use std::{
    io,
    net::{SocketAddr, UdpSocket},
    os::fd::{AsRawFd, FromRawFd},
    time::Duration,
};

use anyhow::{Context, Result, ensure};

pub fn validate(source: SocketAddr, target: SocketAddr) -> Result<()> {
    ensure!(
        source.is_ipv4() == target.is_ipv4(),
        "source and target must use the same IP version"
    );
    ensure!(
        source.port() != 0 && target.port() != 0,
        "source and target ports must be greater than 0"
    );
    ensure!(
        !source.ip().is_unspecified() && !source.ip().is_multicast(),
        "source must be an explicit unicast IP"
    );
    ensure!(
        source.ip() != std::net::Ipv4Addr::BROADCAST,
        "source must not be a broadcast IP"
    );
    ensure!(
        !target.ip().is_unspecified(),
        "target must not be an unspecified IP"
    );
    if let SocketAddr::V6(address) = source {
        ensure!(
            address.scope_id() == 0,
            "source does not accept IPv6 scope IDs; the interface is determined by the target route"
        );
    }
    Ok(())
}

pub(crate) fn checksum(bytes: &[u8], mut sum: u32) -> u16 {
    for pair in bytes.chunks(2) {
        sum += (pair[0] as u32) << 8 | pair.get(1).copied().unwrap_or(0) as u32;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

fn encode(source: SocketAddr, target: SocketAddr, payload: &[u8], bytes: &mut Vec<u8>) {
    let ip_length = if source.is_ipv4() { 20 } else { 40 };
    let udp_length = (8 + payload.len()) as u16;
    bytes.clear();
    bytes.resize(ip_length + 8, 0);
    match (source, target) {
        (SocketAddr::V4(source), SocketAddr::V4(target)) => {
            bytes[0] = 0x45;
            bytes[2..4].copy_from_slice(&((ip_length + udp_length as usize) as u16).to_be_bytes());
            bytes[6] = 0x40; // Do not fragment; oversized datagrams report an error.
            bytes[8] = 64;
            bytes[9] = 17;
            bytes[12..16].copy_from_slice(&source.ip().octets());
            bytes[16..20].copy_from_slice(&target.ip().octets());
            let check = checksum(bytes, 0);
            bytes[10..12].copy_from_slice(&check.to_be_bytes());
        }
        (SocketAddr::V6(source), SocketAddr::V6(target)) => {
            bytes[0] = 0x60;
            bytes[4..6].copy_from_slice(&udp_length.to_be_bytes());
            bytes[6] = 17;
            bytes[7] = 64;
            bytes[8..24].copy_from_slice(&source.ip().octets());
            bytes[24..40].copy_from_slice(&target.ip().octets());
        }
        _ => unreachable!("validated address families"),
    }
    bytes[ip_length..ip_length + 2].copy_from_slice(&source.port().to_be_bytes());
    bytes[ip_length + 2..ip_length + 4].copy_from_slice(&target.port().to_be_bytes());
    bytes[ip_length + 4..ip_length + 6].copy_from_slice(&udp_length.to_be_bytes());
    bytes.extend_from_slice(payload);
    let addresses = if source.is_ipv4() {
        &bytes[12..20]
    } else {
        &bytes[8..40]
    };
    let address_sum = !checksum(addresses, 0) as u32 & 0xffff;
    let check = checksum(&bytes[ip_length..], address_sum + 17 + udp_length as u32);
    // A computed zero checksum must be transmitted as all ones (RFC 768).
    let check = if check == 0 { 0xffff } else { check };
    bytes[ip_length + 6..ip_length + 8].copy_from_slice(&check.to_be_bytes());
}

pub struct RawSender {
    socket: UdpSocket,
    source: SocketAddr,
    target: SocketAddr,
    bytes: Vec<u8>,
}

impl RawSender {
    pub fn new(source: SocketAddr, target: SocketAddr) -> Result<Self> {
        validate(source, target)?;
        let domain = if source.is_ipv4() {
            libc::AF_INET
        } else {
            libc::AF_INET6
        };
        // SAFETY: socket returns a new owned descriptor or -1; no borrowed pointers.
        let fd = unsafe {
            libc::socket(
                domain,
                libc::SOCK_RAW | libc::SOCK_CLOEXEC,
                libc::IPPROTO_RAW,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error())
                .context("Failed to create raw socket; --source requires root or CAP_NET_RAW");
        }
        // SAFETY: fd is newly created and ownership is transferred exactly once.
        let socket = unsafe { UdpSocket::from_raw_fd(fd) };
        let enabled: libc::c_int = 1;
        let (level, option) = if source.is_ipv4() {
            (libc::IPPROTO_IP, libc::IP_HDRINCL)
        } else {
            (libc::IPPROTO_IPV6, libc::IPV6_HDRINCL)
        };
        // SAFETY: enabled is a valid integer and its exact size is passed.
        let result = unsafe {
            libc::setsockopt(
                socket.as_raw_fd(),
                level,
                option,
                (&enabled as *const libc::c_int).cast(),
                std::mem::size_of_val(&enabled) as libc::socklen_t,
            )
        };
        if result < 0 {
            return Err(io::Error::last_os_error()).context("Failed to enable raw IP header");
        }
        let mut route = target;
        route.set_port(0); // Raw socket ports are zero; UDP ports live in our header.
        socket
            .connect(route)
            .context("Failed to connect raw socket; check the target route")?;
        socket.set_write_timeout(Some(Duration::from_secs(1)))?;
        Ok(Self {
            socket,
            source,
            target,
            bytes: Vec::with_capacity(65555),
        })
    }

    pub fn send(&mut self, payload: &[u8]) -> io::Result<usize> {
        if payload.len() > 65507 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "UDP payload exceeds 65507 bytes",
            ));
        }
        encode(self.source, self.target, payload, &mut self.bytes);
        let size = self.socket.send(&self.bytes)?;
        if size != self.bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "Incomplete raw IP send",
            ));
        }
        Ok(payload.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_and_checksums_match_independent_vectors() {
        for (source, target, ip_length, expected) in [
            ("192.0.2.123:5000", "198.51.100.2:2055", 20, 0xcdccu16),
            ("[2001:db8::123]:5000", "[2001:db8::2]:2055", 40, 0x5de7u16),
        ] {
            let source: SocketAddr = source.parse().unwrap();
            let target: SocketAddr = target.parse().unwrap();
            validate(source, target).unwrap();
            let mut bytes = Vec::new();
            encode(source, target, b"abcde", &mut bytes);
            assert_eq!(bytes.len(), ip_length + 13);
            assert_eq!(
                &bytes[ip_length..ip_length + 6],
                &[0x13, 0x88, 0x08, 0x07, 0, 13]
            );
            assert_eq!(&bytes[ip_length + 8..], b"abcde");
            assert_eq!(
                u16::from_be_bytes(bytes[ip_length + 6..ip_length + 8].try_into().unwrap()),
                expected
            );
            if ip_length == 20 {
                assert_eq!(&bytes[12..20], &[192, 0, 2, 123, 198, 51, 100, 2]);
                assert_eq!(&bytes[2..4], &[0, 33]);
                assert_eq!(checksum(&bytes[..20], 0), 0);
                assert_eq!(bytes[9], 17);
            } else {
                assert_eq!(
                    &bytes[8..24],
                    &"2001:db8::123"
                        .parse::<std::net::Ipv6Addr>()
                        .unwrap()
                        .octets()
                );
                assert_eq!(
                    &bytes[24..40],
                    &"2001:db8::2"
                        .parse::<std::net::Ipv6Addr>()
                        .unwrap()
                        .octets()
                );
                assert_eq!(&bytes[4..8], &[0, 13, 17, 64]);
            }
            let capacity = bytes.capacity();
            encode(source, target, b"even", &mut bytes);
            assert_eq!(bytes.capacity(), capacity);
            assert_eq!(&bytes[ip_length + 8..], b"even");
        }
    }

    #[test]
    fn invalid_sources_fail_before_socket_creation() {
        for (source, target) in [
            ("0.0.0.0:5000", "127.0.0.1:2055"),
            ("224.0.0.1:5000", "127.0.0.1:2055"),
            ("255.255.255.255:5000", "127.0.0.1:2055"),
            ("192.0.2.1:0", "127.0.0.1:2055"),
            ("192.0.2.1:5000", "127.0.0.1:0"),
            ("192.0.2.1:5000", "0.0.0.0:2055"),
            ("192.0.2.1:5000", "[::1]:2055"),
            ("[::]:5000", "[::1]:2055"),
        ] {
            assert!(validate(source.parse().unwrap(), target.parse().unwrap()).is_err());
        }
    }

    #[test]
    fn zero_udp_checksum_is_encoded_as_all_ones() {
        let source = "192.0.2.123:5000".parse().unwrap();
        let target = "198.51.100.2:2055".parse().unwrap();
        let mut bytes = Vec::new();
        encode(source, target, &[], &mut bytes);
        let cancelling = u16::from_be_bytes([bytes[26], bytes[27]]).wrapping_sub(4);
        encode(source, target, &cancelling.to_be_bytes(), &mut bytes);
        assert_eq!(&bytes[26..28], &[255, 255]);
    }

    #[test]
    #[ignore = "Requires Linux, CAP_NET_RAW, and IPv4/IPv6 loopback sockets"]
    fn raw_loopback_preserves_source_for_templates_and_data() {
        use crate::{config, protocol, sender};
        use std::sync::atomic::AtomicBool;
        for (source, bind) in [
            ("127.0.0.123:5000", "127.0.0.1:0"),
            ("[::1]:5000", "[::1]:0"),
        ] {
            let collector = UdpSocket::bind(bind).unwrap();
            collector
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let source = source.parse().unwrap();
            let mut raw = RawSender::new(source, collector.local_addr().unwrap()).unwrap();
            let config: config::Config =
                serde_json::from_str(include_str!("../examples/basic.json")).unwrap();
            let flows = config::validate(&config).unwrap();
            let mut plan = protocol::build(config.source_id, &flows, 1400).unwrap();
            let settings = sender::Settings {
                kind: sender::Kind::All,
                count: Some(1),
                duration: None,
                rate: 1000,
                mode: sender::Mode::Stream,
                burst_size: 64,
                template_interval: Duration::from_secs(30),
                template_every: 20,
            };
            let stats = sender::run(&mut plan, &settings, &AtomicBool::new(false), |bytes| {
                raw.send(bytes)
            })
            .unwrap();
            assert_eq!(stats.packets, 2);
            let mut bytes = [0; 2048];
            let mut total = 0;
            for (sequence, flowset) in [(0u32, 0u16), (1, 256)] {
                let (size, peer) = collector.recv_from(&mut bytes).unwrap();
                assert_eq!(peer, source);
                assert_eq!(&bytes[..2], &9u16.to_be_bytes());
                assert_eq!(&bytes[12..16], &sequence.to_be_bytes());
                assert_eq!(&bytes[20..22], &flowset.to_be_bytes());
                total += size as u64;
            }
            assert_eq!(stats.bytes, total);
        }
    }
}
