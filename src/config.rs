use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::fields::{self, FieldSpec, Kind};

pub const MAX_RECORDS: usize = 500_000;
const MAX_CONFIG_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub source_id: u32,
    pub flows: Vec<FlowConfig>,
    /// Directory that relative random_file paths resolve against; empty means the working directory.
    #[serde(skip)]
    pub base: PathBuf,
    /// Set from --ipfix: selects the IPFIX field table and message encoding.
    #[serde(skip)]
    pub ipfix: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowConfig {
    pub template_id: u16,
    #[serde(default)]
    pub scope: Vec<FieldConfig>,
    #[serde(default)]
    pub fields: Vec<FieldConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldConfig {
    pub name: String,
    pub value: Value,
}

#[derive(Debug)]
pub enum Values {
    List {
        bytes: Vec<u8>,
        length: usize,
    },
    RandomField {
        min: u32,
        max: u32,
        length: usize,
    },
    /// Addresses (length bytes each); one is drawn per record on every send.
    RandomFile {
        bytes: Arc<[u8]>,
        length: usize,
    },
    Range {
        start: u128,
        step: u128,
        count: usize,
        length: usize,
        shift: u32,
    },
}

impl Values {
    pub fn count(&self) -> usize {
        match self {
            Self::List { bytes, length } => bytes.len() / length,
            Self::RandomField { .. } | Self::RandomFile { .. } => 1,
            Self::Range { count, .. } => *count,
        }
    }

    pub fn append(&self, index: usize, output: &mut Vec<u8>) {
        match self {
            Self::List { bytes, length } => {
                output.extend_from_slice(&bytes[index * length..(index + 1) * length]);
            }
            Self::RandomField { min, length, .. } => {
                output.extend_from_slice(&min.to_be_bytes()[4 - length..]);
            }
            Self::RandomFile { bytes, length } => output.extend_from_slice(&bytes[..*length]),
            Self::Range {
                start,
                step,
                length,
                shift,
                ..
            } => {
                let bytes = ((start + *step * index as u128) << shift).to_be_bytes();
                output.extend_from_slice(&bytes[16 - length..]);
            }
        }
    }
}

#[derive(Debug)]
pub struct Column {
    pub name: String,
    pub spec: FieldSpec,
    pub values: Values,
}

#[derive(Debug)]
pub struct Flow {
    pub template_id: u16,
    pub scope_count: usize,
    pub columns: Vec<Column>,
    pub record_length: usize,
    pub record_count: usize,
    /// Record offsets of generated IPv4 headers whose checksum must follow the field values.
    pub ipv4_headers: Vec<usize>,
}

impl Flow {
    /// Columns described in the template; id 0 marks bytes inside a variable-length element.
    pub fn template_columns(&self) -> impl Iterator<Item = &Column> {
        self.columns.iter().filter(|column| column.spec.id != 0)
    }

    pub fn same_template(&self, other: &Self) -> bool {
        // A variable-length element may differ in size between flows sharing a template.
        let shape = |flow: &'_ Self| {
            flow.template_columns()
                .map(|c| {
                    let length = if c.spec.variable { 0 } else { c.spec.length };
                    (c.name.clone(), c.spec.id, c.spec.variable, length)
                })
                .collect::<Vec<_>>()
        };
        self.scope_count == other.scope_count && shape(self) == shape(other)
    }
}

pub fn load(path: &Path) -> Result<Config> {
    let file = File::open(path)
        .with_context(|| format!("Failed to read configuration file {}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(MAX_CONFIG_BYTES + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_CONFIG_BYTES,
        "Configuration file must not exceed 8 MiB"
    );
    let mut config: Config = serde_json::from_slice(&bytes).with_context(|| {
        format!(
            "{}: invalid JSON configuration; check brackets, commas, and quotes at the error location. See examples/basic.json",
            path.display()
        )
    })?;
    config.base = path.parent().unwrap_or(Path::new("")).to_path_buf();
    Ok(config)
}

fn random_file(path: &Path, spec: FieldSpec) -> Result<Values> {
    ensure!(
        matches!(spec.kind, Kind::Ipv4 | Kind::Ipv6),
        "random_file is only supported for IP fields"
    );
    let mut bytes = Vec::new();
    File::open(path)
        .with_context(|| format!("Failed to read random_file {}", path.display()))?
        .take(MAX_CONFIG_BYTES + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_CONFIG_BYTES,
        "random_file must not exceed 8 MiB"
    );
    let text = String::from_utf8(bytes)
        .with_context(|| format!("random_file {} must be UTF-8 text", path.display()))?;
    let mut addresses = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (number, length) = address(line, spec.kind)
            .with_context(|| format!("{} line {}", path.display(), index + 1))?;
        addresses.extend_from_slice(&number.to_be_bytes()[16 - length..]);
    }
    ensure!(
        !addresses.is_empty(),
        "random_file {} contains no addresses",
        path.display()
    );
    Ok(Values::RandomFile {
        bytes: addresses.into(),
        length: spec.length,
    })
}

fn address(value: &str, kind: Kind) -> Result<(u128, usize)> {
    match (value.parse::<IpAddr>(), kind) {
        (Ok(IpAddr::V4(ip)), Kind::Ipv4) => Ok((u32::from(ip) as u128, 4)),
        (Ok(IpAddr::V6(ip)), Kind::Ipv6) => Ok((u128::from(ip), 16)),
        _ => {
            bail!(
                "Address {value:?} is invalid or has the wrong IP version; IPv4 example: 192.0.2.1, IPv6 example: 2001:db8::1"
            )
        }
    }
}

fn asn(text: &str) -> Result<u32> {
    let text = text.trim().to_ascii_uppercase();
    let text = text.strip_prefix("AS").unwrap_or(&text).trim();
    let decimal = |part: &str| -> Result<u32> {
        let part = part.trim();
        ensure!(
            !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()),
            "ASN must be decimal digits, optionally prefixed with AS or written as high.low"
        );
        part.parse::<u32>()
            .context("ASN must be within 0..=4294967295")
    };
    if let Some((high, low)) = text.split_once('.') {
        let (high, low) = (decimal(high)?, decimal(low)?);
        ensure!(
            high <= u16::MAX as u32 && low <= u16::MAX as u32,
            "asdot high and low must both be within 0..=65535"
        );
        Ok((high << 16) | low)
    } else {
        decimal(text)
    }
}

fn route_distinguisher(text: &str) -> Result<Vec<u8>> {
    let parts: Vec<_> = text.split(':').map(str::trim).collect();
    let (kind, admin, assigned) = match parts.as_slice() {
        [admin, assigned] => {
            let kind = if admin.parse::<Ipv4Addr>().is_ok() {
                1
            } else if asn(admin).context("RD administrator must be an ASN or IPv4 address")?
                <= u16::MAX as u32
            {
                0
            } else {
                2
            };
            (kind, *admin, *assigned)
        }
        [kind, admin, assigned] => (
            kind.parse::<u16>().context("RD type must be 0, 1, or 2")?,
            *admin,
            *assigned,
        ),
        _ => bail!("RD format: ASN:number, IPv4:number, or type:administrator:number"),
    };
    let mut bytes = Vec::with_capacity(8);
    bytes.extend_from_slice(&kind.to_be_bytes());
    match kind {
        0 => {
            bytes.extend_from_slice(
                &u16::try_from(asn(admin)?)
                    .context("Type 0 RD ASN must be within 0..=65535")?
                    .to_be_bytes(),
            );
            bytes.extend_from_slice(
                &assigned
                    .parse::<u32>()
                    .context("Type 0 RD number must be within 0..=4294967295")?
                    .to_be_bytes(),
            );
        }
        1 => {
            bytes.extend_from_slice(
                &admin
                    .parse::<Ipv4Addr>()
                    .context("Type 1 RD administrator must be IPv4")?
                    .octets(),
            );
            bytes.extend_from_slice(
                &assigned
                    .parse::<u16>()
                    .context("Type 1 RD number must be within 0..=65535")?
                    .to_be_bytes(),
            );
        }
        2 => {
            bytes.extend_from_slice(
                &asn(admin)
                    .context("Type 2 RD ASN must be within 0..=4294967295")?
                    .to_be_bytes(),
            );
            bytes.extend_from_slice(
                &assigned
                    .parse::<u16>()
                    .context("Type 2 RD number must be within 0..=65535")?
                    .to_be_bytes(),
            );
        }
        _ => bail!("RD type must be 0, 1, or 2"),
    }
    Ok(bytes)
}

fn tcp_flags(text: &str) -> Result<u64> {
    let text = text.trim().to_ascii_uppercase();
    if let Some(digits) = text.strip_prefix("0X") {
        return u64::from_str_radix(digits, 16).context("Invalid TCP_FLAGS hexadecimal value");
    }
    let mut flags = 0;
    for token in text.split('|') {
        let token = token.trim();
        ensure!(
            !token.is_empty(),
            "TCP_FLAGS must not be empty; flags are required on both sides of |"
        );
        flags |= match token {
            "FIN" => 0x01,
            "SYN" => 0x02,
            "RST" => 0x04,
            "PSH" => 0x08,
            "ACK" => 0x10,
            "URG" => 0x20,
            "ECE" => 0x40,
            "CWR" => 0x80,
            _ => {
                let mut bits = 0;
                for letter in token.chars() {
                    bits |= match letter {
                        'F' => 0x01,
                        'S' => 0x02,
                        'R' => 0x04,
                        'P' => 0x08,
                        'A' => 0x10,
                        'U' => 0x20,
                        'E' => 0x40,
                        'C' => 0x80,
                        _ => bail!(
                            "Invalid TCP flag {token:?}; use F/S/R/P/A/U/E/C or full names separated by |, such as SYN|ACK"
                        ),
                    };
                }
                bits
            }
        };
    }
    Ok(flags)
}

fn icmp_type(value: &Value, ipv6: bool) -> Result<u8> {
    if let Some(name) = value.as_str() {
        return match (name.trim().to_ascii_lowercase().as_str(), ipv6) {
            ("echo-request", false) => Ok(8),
            ("echo-request", true) => Ok(128),
            ("echo-reply", false) => Ok(0),
            ("echo-reply", true) => Ok(129),
            ("destination-unreachable", false) => Ok(3),
            ("destination-unreachable", true) => Ok(1),
            ("time-exceeded", false) => Ok(11),
            ("time-exceeded", true) => Ok(3),
            ("parameter-problem", false) => Ok(12),
            ("parameter-problem", true) => Ok(4),
            ("packet-too-big", true) => Ok(2),
            _ => bail!(
                "Unknown ICMP name {name:?} or unsupported IP version; a numeric type is also accepted"
            ),
        };
    }
    value
        .as_u64()
        .and_then(|number| u8::try_from(number).ok())
        .context("ICMP type must be an integer within 0..=255 or a supported name")
}

fn hex(text: &str) -> Result<Vec<u8>> {
    ensure!(
        !text.is_empty() && text.len() % 2 == 0 && text.bytes().all(|b| b.is_ascii_hexdigit()),
        "Frame bytes must be an even-length hexadecimal string without separators, such as 0200000000020200000000010800"
    );
    Ok((0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
        .collect())
}

/// Expands DATALINK_FRAME_SECTION into its length prefix plus fixed-length inner columns.
/// Returns the columns and, for generated IPv4 frames, the IP header offset within them.
fn frame_section(value: &Value, base: &Path) -> Result<(Vec<Column>, Option<usize>)> {
    const NAME: &str = "DATALINK_FRAME_SECTION";
    let inner = |length, kind| FieldSpec {
        id: 0,
        length,
        kind,
        variable: false,
    };
    let fixed = |bytes: Vec<u8>| Column {
        name: format!("{NAME}.FIXED"),
        spec: inner(bytes.len(), Kind::Bytes),
        values: Values::List {
            length: bytes.len(),
            bytes,
        },
    };
    let prefix = |length: usize| {
        // RFC 7011 section 7: one length byte below 255, otherwise 255 plus two bytes.
        let bytes = if length < 255 {
            vec![length as u8]
        } else {
            vec![255, (length >> 8) as u8, length as u8]
        };
        Column {
            name: NAME.into(),
            spec: FieldSpec {
                id: 315,
                length: bytes.len(),
                kind: Kind::Bytes,
                variable: true,
            },
            values: Values::List {
                length: bytes.len(),
                bytes,
            },
        }
    };

    let Some(frame) = value.as_object().and_then(|object| object.get("frame")) else {
        let first = match value {
            Value::Array(items) => items.first(),
            other => Some(other),
        };
        let text = first.and_then(Value::as_str).context(
            "DATALINK_FRAME_SECTION must be a hexadecimal string, an array of them, or a {\"frame\": {...}} object",
        )?;
        let length = hex(text)?.len();
        ensure!(length <= 65535, "Frame section must not exceed 65535 bytes");
        let spec = inner(length, Kind::Bytes);
        let column = Column {
            name: format!("{NAME}.BYTES"),
            spec,
            values: values(value, spec, NAME, base)?,
        };
        return Ok((vec![prefix(length), column], None));
    };

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Frame {
        #[serde(default = "default_clip")]
        clip: usize,
        length: Option<usize>,
        #[serde(default)]
        ipv6: bool,
        protocol: u8,
        src: Value,
        dst: Value,
        src_port: Value,
        dst_port: Value,
        tcp_flags: Option<Value>,
        tos: Option<Value>,
    }
    fn default_clip() -> usize {
        126
    }
    let frame: Frame = serde_json::from_value(frame.clone()).context(
        "frame requires protocol, src, dst, src_port, dst_port; optional clip, length, ipv6, tcp_flags, tos",
    )?;
    ensure!(
        value.as_object().is_some_and(|object| object.len() == 1),
        "frame must be the only key in the value object"
    );
    let tcp = match frame.protocol {
        6 => true,
        17 => false,
        other => bail!("frame.protocol must be 6 (TCP) or 17 (UDP), got {other}"),
    };
    ensure!(
        tcp || frame.tcp_flags.is_none(),
        "frame.tcp_flags requires protocol 6"
    );
    ensure!(
        !frame.ipv6 || frame.tos.is_none(),
        "frame.tos is only supported for IPv4"
    );
    let ip = if frame.ipv6 { 40 } else { 20 };
    let headers = 14 + ip + if tcp { 20 } else { 8 };
    let length = frame.length.unwrap_or(frame.clip);
    ensure!(
        frame.clip >= headers,
        "frame.clip must be at least {headers} bytes to hold the Ethernet, IP, and L4 headers"
    );
    ensure!(
        frame.clip <= length && length <= 65535,
        "frame.length must be within clip..=65535 (the original frame is at least as long as the exported section)"
    );
    let field = |key: &str, rule: &str, kind, size, value: &Value| -> Result<Column> {
        let spec = inner(size, kind);
        Ok(Column {
            name: format!("{NAME}.{}", key.to_ascii_uppercase()),
            spec,
            // `rule` borrows the value rules of the matching top-level field.
            values: values(value, spec, rule, base).with_context(|| format!("frame.{key}"))?,
        })
    };
    let word = |value: usize| (value as u16).to_be_bytes();
    let first = prefix(frame.clip);
    let mut ethernet = vec![2, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 1];
    let mut columns;
    let mut ipv4_header = None;
    if frame.ipv6 {
        ethernet.extend_from_slice(&[0x86, 0xdd, 0x60, 0, 0, 0]);
        ethernet.extend_from_slice(&word(length - 54));
        ethernet.extend_from_slice(&[frame.protocol, 64]);
        columns = vec![
            first,
            fixed(ethernet),
            field("src", "IPV6_SRC_ADDR", Kind::Ipv6, 16, &frame.src)?,
            field("dst", "IPV6_DST_ADDR", Kind::Ipv6, 16, &frame.dst)?,
        ];
    } else {
        ipv4_header = Some(first.spec.length + 14);
        ethernet.extend_from_slice(&[0x08, 0x00, 0x45]);
        let mut rest = word(length - 14).to_vec();
        // ID, flags/fragment, TTL 64, protocol, checksum (filled in per record).
        rest.extend_from_slice(&[0, 0, 0, 0, 64, frame.protocol, 0, 0]);
        columns = vec![
            first,
            fixed(ethernet),
            field(
                "tos",
                "SRC_TOS",
                Kind::Number,
                1,
                frame.tos.as_ref().unwrap_or(&json!(0)),
            )?,
            fixed(rest),
            field("src", "IPV4_SRC_ADDR", Kind::Ipv4, 4, &frame.src)?,
            field("dst", "IPV4_DST_ADDR", Kind::Ipv4, 4, &frame.dst)?,
        ];
    }
    columns.push(field(
        "src_port",
        "L4_SRC_PORT",
        Kind::Number,
        2,
        &frame.src_port,
    )?);
    columns.push(field(
        "dst_port",
        "L4_DST_PORT",
        Kind::Number,
        2,
        &frame.dst_port,
    )?);
    if tcp {
        // Sequence and acknowledgment 0, data offset 5.
        columns.push(fixed(vec![0, 0, 0, 0, 0, 0, 0, 0, 0x50]));
        let syn = json!(2);
        columns.push(field(
            "tcp_flags",
            "TCP_FLAGS",
            Kind::Number,
            1,
            frame.tcp_flags.as_ref().unwrap_or(&syn),
        )?);
        // Window 65535; checksum and urgent pointer 0.
        columns.push(fixed(vec![0xff, 0xff, 0, 0, 0, 0]));
    } else {
        let mut udp = word(length - 14 - ip).to_vec();
        // shortcut: L4 checksums are placeholders (the clipped payload cannot be summed); compute them if a collector verifies.
        // UDP over IPv6 forbids the zero "no checksum" value, so use a nonzero one there.
        udp.extend_from_slice(if frame.ipv6 { &[0xff, 0xff] } else { &[0, 0] });
        columns.push(fixed(udp));
    }
    if frame.clip > headers {
        columns.push(fixed(vec![0; frame.clip - headers]));
    }
    Ok((columns, ipv4_header))
}

fn scalar(value: &Value, spec: FieldSpec, name: &str) -> Result<Vec<u8>> {
    match spec.kind {
        Kind::Bytes => {
            let bytes = hex(value
                .as_str()
                .context("Frame bytes must be a hexadecimal string")?)?;
            ensure!(
                bytes.len() == spec.length,
                "Frame sections in one flow must all be {} bytes (got {}); use a separate flow for another length",
                spec.length,
                bytes.len()
            );
            Ok(bytes)
        }
        Kind::Ipv4 | Kind::Ipv6 => {
            let value = value.as_str().context("IP field must be a string")?;
            let (number, length) = address(value, spec.kind)?;
            Ok(number.to_be_bytes()[16 - length..].to_vec())
        }
        Kind::RouteDistinguisher => route_distinguisher(
            value
                .as_str()
                .context("MPLS_VPN_RD must be an RD string, such as 65000:100")?,
        ),
        Kind::Asn => {
            let number = if let Some(text) = value.as_str() {
                asn(text)?
            } else {
                value
                    .as_u64()
                    .and_then(|number| u32::try_from(number).ok())
                    .context("ASN must be an integer within 0..=4294967295 or a string (such as AS65000 or 1.10)")?
            };
            Ok(number.to_be_bytes().to_vec())
        }
        Kind::Icmp4 | Kind::Icmp6 => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Icmp {
                #[serde(rename = "type")]
                kind: Value,
                #[serde(default)]
                code: u8,
            }
            let ipv6 = matches!(spec.kind, Kind::Icmp6);
            if value.is_object() {
                let icmp: Icmp = serde_json::from_value(value.clone()).context(
                    "ICMP object requires type and optional code (default 0, range 0..=255)",
                )?;
                Ok(vec![icmp_type(&icmp.kind, ipv6)?, icmp.code])
            } else if value.is_string() {
                Ok(vec![icmp_type(value, ipv6)?, 0])
            } else {
                let raw = value
                    .as_u64()
                    .and_then(|n| u16::try_from(n).ok())
                    .context("Raw ICMP integer must be within 0..=65535; alternatively, use a {type, code} object")?;
                Ok(raw.to_be_bytes().to_vec())
            }
        }
        Kind::MplsLabel => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Label {
                label: u32,
                #[serde(default)]
                tc: u8,
                #[serde(default)]
                bos: bool,
            }
            let label: Label = if value.is_object() {
                serde_json::from_value(value.clone())
                    .context("MPLS label object requires label and optional tc and bos (no TTL)")?
            } else {
                Label {
                    label: value
                        .as_u64()
                        .filter(|label| *label <= 0xfffff)
                        .context("MPLS label must be an integer within 0..=1048575")?
                        as u32,
                    tc: 0,
                    bos: false,
                }
            };
            ensure!(
                label.label <= 0xfffff,
                "MPLS label must be within 0..=1048575 (20 bits)"
            );
            ensure!(label.tc <= 7, "MPLS tc must be within 0..=7 (3 bits)");
            let encoded = (label.label << 4) | ((label.tc as u32) << 1) | u32::from(label.bos);
            Ok(encoded.to_be_bytes()[1..].to_vec())
        }
        Kind::Number => {
            let number = if name == "TCP_FLAGS" && value.is_string() {
                tcp_flags(value.as_str().unwrap())?
            } else {
                value
                    .as_u64()
                    .context("This field must be a nonnegative integer")?
            };
            let limit = (1u64 << (spec.length * 8)) - 1;
            ensure!(
                number <= limit,
                "Value {number} exceeds the {}-byte range 0..={limit}",
                spec.length
            );
            if matches!(name, "SRC_MASK" | "DST_MASK") {
                ensure!(number <= 32, "IPv4 mask must be within 0..=32");
            }
            if matches!(name, "IPV6_SRC_MASK" | "IPV6_DST_MASK") {
                ensure!(number <= 128, "IPv6 mask must be within 0..=128");
            }
            if name == "IP_PROTOCOL_VERSION" {
                ensure!(
                    matches!(number, 4 | 6),
                    "IP_PROTOCOL_VERSION must be 4 or 6"
                );
            }
            if name == "MPLS_TOP_LABEL_PREFIX_LEN" {
                ensure!(
                    number <= 128,
                    "MPLS FEC prefix length must be within 0..=128"
                );
            }
            if name == "MPLS_TOP_LABEL_EXP" {
                ensure!(number <= 7, "MPLS EXP/TC must be within 0..=7");
            }
            Ok(number.to_be_bytes()[8 - spec.length..].to_vec())
        }
    }
}

fn prefix_values(text: &str, hosts_only: bool, spec: FieldSpec) -> Result<Values> {
    ensure!(
        matches!(spec.kind, Kind::Ipv4 | Kind::Ipv6),
        "prefix is only supported for IP fields"
    );
    let (ip, prefix) = text
        .trim()
        .split_once('/')
        .context("IP prefix must use address/length format, such as 192.0.2.0/24")?;
    let (number, length) = address(ip, spec.kind)?;
    ensure!(
        !prefix.is_empty() && prefix.bytes().all(|b| b.is_ascii_digit()),
        "prefix length must be a nonnegative integer"
    );
    let prefix: u32 = prefix.parse().context("Invalid prefix length")?;
    let width = length as u32 * 8;
    ensure!(
        prefix <= width,
        "prefix length for this IP version must be within 0..={width}"
    );
    ensure!(
        !hosts_only || length == 4,
        "hosts_only is only supported for IPv4; IPv6 prefixes include all addresses"
    );
    let host_bits = width - prefix;
    let mut count = 1u128
        .checked_shl(host_bits)
        .with_context(|| format!("prefix exceeds {MAX_RECORDS} addresses"))?;
    ensure!(
        count <= MAX_RECORDS as u128,
        "A single prefix must not exceed {MAX_RECORDS} addresses"
    );
    let mut start = (number >> host_bits) << host_bits;
    if hosts_only && count > 2 {
        start += 1;
        count -= 2;
    }
    Ok(Values::Range {
        start,
        step: 1,
        count: count as usize,
        length,
        shift: 0,
    })
}

fn values(value: &Value, spec: FieldSpec, name: &str, base: &Path) -> Result<Values> {
    if value
        .as_object()
        .is_some_and(|object| object.contains_key("random_file"))
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RandomFile {
            random_file: PathBuf,
        }
        let random: RandomFile = serde_json::from_value(value.clone())
            .context("random_file must be a file path string")?;
        return random_file(&base.join(random.random_file), spec);
    }
    if value
        .as_object()
        .is_some_and(|object| object.contains_key("random_between"))
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RandomBetween {
            random_between: [u32; 2],
        }
        // id 0 marks numeric fields inside a generated frame, which may all be randomized.
        ensure!(
            spec.id == 0 || matches!(name, "L4_SRC_PORT" | "L4_DST_PORT" | "IN_PKTS" | "IN_BYTES"),
            "random_between is only supported for L4_SRC_PORT / L4_DST_PORT / IN_PKTS / IN_BYTES"
        );
        let random: RandomBetween = serde_json::from_value(value.clone())
            .context("random_between must be [min,max], exactly two nonnegative 32-bit integers")?;
        let [min, max] = random.random_between;
        ensure!(min <= max, "random_between min must not exceed max");
        scalar(&json!(min), spec, name)?;
        scalar(&json!(max), spec, name)?;
        return Ok(Values::RandomField {
            min,
            max,
            length: spec.length,
        });
    }
    if matches!(spec.kind, Kind::Ipv4 | Kind::Ipv6) {
        if let Some(prefix) = value.as_str().filter(|text| text.contains('/')) {
            return prefix_values(prefix, false, spec);
        }
    }
    if value
        .as_object()
        .is_some_and(|object| object.contains_key("prefix") || object.contains_key("hosts_only"))
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Prefix {
            prefix: String,
            #[serde(default)]
            hosts_only: bool,
        }
        let prefix: Prefix = serde_json::from_value(value.clone())
            .context("prefix object requires a prefix string and optional hosts_only boolean")?;
        return prefix_values(&prefix.prefix, prefix.hosts_only, spec);
    }
    let is_range = value.as_object().is_some_and(|object| {
        ["start", "end", "step"]
            .iter()
            .any(|key| object.contains_key(*key))
    });
    if value.is_object()
        && !is_range
        && matches!(
            spec.kind,
            Kind::MplsLabel | Kind::RouteDistinguisher | Kind::Icmp4 | Kind::Icmp6
        )
    {
        return Ok(Values::List {
            bytes: scalar(value, spec, name)?,
            length: spec.length,
        });
    }
    if value.is_object() {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Range {
            start: Value,
            end: Value,
            #[serde(default = "one")]
            step: u64,
        }
        fn one() -> u64 {
            1
        }
        ensure!(
            !matches!(spec.kind, Kind::RouteDistinguisher),
            "RD does not support ranges; use an array of RD values"
        );
        let range: Range = serde_json::from_value(value.clone())
            .context("range requires start and end; step is optional")?;
        ensure!(range.step > 0, "range step must be greater than 0");
        let shift = if matches!(spec.kind, Kind::MplsLabel) {
            4
        } else {
            0
        };
        let endpoint = |value: &Value| -> Result<u128> {
            if matches!(spec.kind, Kind::MplsLabel) {
                ensure!(
                    value.is_u64(),
                    "MPLS label range start/end must be label integers"
                );
            }
            Ok(scalar(value, spec, name)?
                .iter()
                .fold(0u128, |number, byte| (number << 8) | *byte as u128)
                >> shift)
        };
        let start = endpoint(&range.start).context("range.start")?;
        let end = endpoint(&range.end).context("range.end")?;
        ensure!(end >= start, "range end must not be less than start");
        let intervals = (end - start) / range.step as u128;
        ensure!(
            intervals < MAX_RECORDS as u128,
            "A single range must not exceed {MAX_RECORDS} values"
        );
        let count = intervals as usize + 1;
        if name == "IP_PROTOCOL_VERSION" {
            ensure!(
                (0..count).all(|index| matches!(start + range.step as u128 * index as u128, 4 | 6)),
                "IP_PROTOCOL_VERSION range must only produce 4 or 6"
            );
        }
        return Ok(Values::Range {
            start,
            step: range.step as u128,
            count,
            length: spec.length,
            shift,
        });
    }
    let list = match value {
        Value::Array(items) => items.as_slice(),
        _ => std::slice::from_ref(value),
    };
    ensure!(!list.is_empty(), "Value array must not be empty");
    ensure!(
        list.len() <= MAX_RECORDS,
        "Value array must not exceed {MAX_RECORDS} values"
    );
    let mut bytes = Vec::with_capacity(list.len() * spec.length);
    for (index, value) in list.iter().enumerate() {
        let encoded = scalar(value, spec, name)
            .with_context(|| format!("Value[{index}]: {}", rejected_value(value)))?;
        bytes.extend_from_slice(&encoded);
    }
    Ok(Values::List {
        bytes,
        length: spec.length,
    })
}

fn rejected_value(value: &Value) -> String {
    // Bound diagnostics even when a rejected field contains a huge array.
    let text = value.to_string();
    if text.chars().count() > 160 {
        format!("{}…", text.chars().take(160).collect::<String>())
    } else {
        text
    }
}

pub fn validate(config: &Config) -> Result<Vec<Flow>> {
    ensure!(!config.flows.is_empty(), "flows must not be empty");
    let mut templates: HashMap<u16, usize> = HashMap::new();
    let mut total = 0usize;
    let mut flows: Vec<Flow> = Vec::with_capacity(config.flows.len());
    for (index, input) in config.flows.iter().enumerate() {
        let context = format!("flows[{index}] (Template {})", input.template_id);
        let result = (|| -> Result<Flow> {
            ensure!(
                input.template_id >= 256,
                "template_id must be within 256..=65535"
            );
            ensure!(!input.fields.is_empty(), "fields must not be empty");
            let mut columns = Vec::new();
            let mut record_count = 1usize;
            let mut record_length = 0usize;
            let mut ipv4_headers = Vec::new();
            for (scope, fields) in [(true, &input.scope), (false, &input.fields)] {
                let mut names = HashSet::new();
                for field in fields {
                    let name = field.name.to_ascii_uppercase();
                    let result = (|| -> Result<Vec<Column>> {
                        ensure!(
                            names.insert(name.clone()),
                            "Duplicate fields are not allowed"
                        );
                        let spec = match fields::lookup(&name, scope) {
                            _ if config.ipfix => fields::lookup_ipfix(&name)?,
                            Err(_)
                                if fields::lookup(&name, false).is_err()
                                    && fields::lookup_ipfix(&name).is_ok() =>
                            {
                                bail!("{name} is an IPFIX-only field; rerun with --ipfix")
                            }
                            other => other?,
                        };
                        let added = if spec.variable {
                            ensure!(!scope, "{name} cannot be a scope field");
                            let (added, ipv4_header) = frame_section(&field.value, &config.base)?;
                            ipv4_headers.extend(ipv4_header.map(|at| record_length + at));
                            added
                        } else {
                            vec![Column {
                                name: name.clone(),
                                spec,
                                values: values(&field.value, spec, &name, &config.base)?,
                            }]
                        };
                        for column in &added {
                            record_count = record_count
                                .checked_mul(column.values.count())
                                .context("Expanded record count overflow")?;
                            ensure!(
                                record_count <= MAX_RECORDS,
                                "Expansion must not exceed {MAX_RECORDS} records"
                            );
                            record_length += column.spec.length;
                        }
                        Ok(added)
                    })();
                    columns.extend(result.with_context(|| {
                        format!(
                            "{}.{name}, input value: {}",
                            if scope { "scope" } else { "fields" },
                            rejected_value(&field.value)
                        )
                    })?);
                }
            }
            Ok(Flow {
                template_id: input.template_id,
                scope_count: input.scope.len(),
                columns,
                record_length,
                record_count,
                ipv4_headers,
            })
        })();
        let flow = result.with_context(|| context)?;
        if let Some(previous) = templates.insert(flow.template_id, flows.len()) {
            ensure!(
                flow.same_template(&flows[previous]),
                "flows[{index}] (Template {}): shared templates must have matching scope, field names, order, and lengths",
                flow.template_id
            );
        }
        total = total
            .checked_add(flow.record_count)
            .context("Total record count overflow")?;
        ensure!(
            total <= MAX_RECORDS,
            "All flows combined must not exceed {MAX_RECORDS} records"
        );
        flows.push(flow);
    }
    Ok(flows)
}
