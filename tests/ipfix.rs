use std::{
    process::Command,
    sync::atomic::AtomicBool,
    time::{Duration, UNIX_EPOCH},
};

use neural_ensemble_telemetry_fabric_luminal_oracle_wizard9::{
    config::{self, Config},
    protocol::{self, Plan},
    sender::{self, Kind, Mode, Settings},
};
use serde_json::{Value, json};

// Expected bytes below are taken from the customer sample capture (Junos inline monitoring,
// template 2000 + options template 2600).
const SAMPLE_HEADERS: &str = "020000000002020000000001080045000128000000004006323a0a000164cb007132c00001bb00000000000000005012ffff";

fn build(value: Value, limit: usize) -> anyhow::Result<Plan> {
    let mut config: Config = serde_json::from_value(value)?;
    config.ipfix = true;
    protocol::build_ipfix(config.source_id, &config::validate(&config)?, limit)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn options() -> Value {
    json!({"template_id": 2600,
        "scope": [{"name": "FLOW_EXPORTER", "value": 1}],
        "fields": [{"name": "SAMPLING_INTERVAL", "value": 1000}]})
}

fn sampled(section: Value) -> Value {
    json!({"template_id": 2000, "fields": [
        {"name": "INPUT_SNMP", "value": 1},
        {"name": "OUTPUT_SNMP", "value": 2},
        {"name": "DIRECTION", "value": 0},
        {"name": "DATALINK_FRAME_SIZE", "value": 310},
        {"name": "DATALINK_FRAME_SECTION", "value": section}]})
}

fn frame(extra: Value) -> Value {
    let mut frame = json!({"length": 310, "protocol": 6, "src": "10.0.1.100", "dst": "203.0.113.50",
        "src_port": 49152, "dst_port": 443, "tcp_flags": "SYN|ACK"});
    frame
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    json!({ "frame": frame })
}

fn ipv4_checksum_ok(header: &[u8]) -> bool {
    let sum: u32 = header[..20]
        .chunks(2)
        .map(|pair| u32::from(u16::from_be_bytes([pair[0], pair[1]])))
        .sum();
    (sum & 0xffff) + (sum >> 16) == 0xffff
}

#[test]
fn messages_match_the_sample_capture() {
    let section = format!("{SAMPLE_HEADERS}{}", "ab".repeat(126 - 50));
    let mut plan = build(
        json!({"source_id": 1, "flows": [options(), sampled(json!(section))]}),
        1400,
    )
    .unwrap();
    assert_eq!(
        hex(&plan.templates[0].bytes[16..]),
        "000300140a280002000100900004002200040000"
    );
    assert_eq!(
        hex(&plan.templates[1].bytes[16..]),
        "0002001c07d00005000a0004000e0004003d000101380002013bffff"
    );
    assert_eq!(hex(&plan.data[0].bytes[16..]), "0a28000c00000001000003e8");
    // Set header, the four fixed fields, a one-byte length prefix, the section, zero padding.
    assert_eq!(
        hex(&plan.data[1].bytes[16..]),
        format!("07d0009000000001000000020001367e{section}0000")
    );

    for packet in plan.templates.iter_mut().chain(&mut plan.data) {
        packet
            .stamp(
                7,
                9,
                Duration::from_secs(99),
                UNIX_EPOCH + Duration::from_secs(5000),
            )
            .unwrap();
        let bytes = &packet.bytes;
        assert!(packet.is_ipfix());
        // Version, message length, export time, sequence, observation domain.
        assert_eq!(
            hex(&bytes[..16]),
            format!("000a{:04x}000013880000000900000007", bytes.len())
        );
    }
}

#[test]
fn generated_frames_reproduce_sample_headers_and_keep_checksums_valid() {
    let plan = build(json!({"flows": [sampled(frame(json!({})))]}), 1400).unwrap();
    let section = &plan.data[0].bytes[20 + 12..];
    assert_eq!(hex(&section[..50]), SAMPLE_HEADERS);
    assert_eq!(section.len(), 126 + 2, "126-byte clip plus set padding");

    let random = json!({"random_between": [0, 255]});
    let mut plan = build(
        json!({"flows": [sampled(frame(json!({
            "tos": random, "tcp_flags": random, "src": ["10.0.0.1", "10.0.0.2", "10.9.9.9"],
            "src_port": {"random_between": [1, 65535]}})))]}),
        1400,
    )
    .unwrap();
    let packet = &mut plan.data[0];
    let noise: Vec<u8> = (0..64u8).map(|n| n.wrapping_mul(37)).collect();
    for round in 0..2 {
        if round == 1 {
            packet.randomize_fields(&mut noise.as_slice()).unwrap();
        }
        let headers: Vec<_> = (0..3)
            .map(|record| &packet.bytes[20 + record * 138 + 12 + 14..][..20])
            .collect();
        assert!(headers.iter().all(|header| ipv4_checksum_ok(header)));
        // TOS byte: constant before randomizing, drawn per record afterwards.
        assert_eq!(headers.iter().any(|header| header[1] != 0), round == 1);
    }
}

#[test]
fn clip_lengths_and_ip_versions_share_one_template() {
    let long = frame(json!({"clip": 256, "length": 1200, "protocol": 17, "tcp_flags": null}));
    let v6 = json!({"frame": {"clip": 62, "ipv6": true, "protocol": 17, "src": "2001:db8:1::10",
        "dst": "2001:db8:2::20", "src_port": 53000, "dst_port": 53}});
    let plan = build(
        json!({"flows": [sampled(frame(json!({}))), sampled(long), sampled(v6)]}),
        1400,
    )
    .unwrap();
    assert_eq!((plan.templates.len(), plan.data.len()), (1, 3));
    // 255 and above use the three-byte length prefix; the UDP length follows the frame length.
    let wide = &plan.data[1].bytes[20 + 11..];
    assert_eq!(hex(&wide[..3]), "ff0100");
    assert_eq!(hex(&wide[3 + 34..3 + 42]), "c00001bb048e0000");
    assert_eq!(wide.len(), 3 + 256 + 2);
    let v6 = &plan.data[2].bytes[20 + 11..];
    assert_eq!(
        hex(&v6[..1 + 22]),
        "3e02000000000202000000000186dd6000000000081140"
    );
    assert_eq!(hex(&v6[1 + 54..1 + 62]), "cf0800350008ffff");
}

#[test]
fn sequence_counts_data_records_including_options_records() {
    let mut plan = build(
        json!({"flows": [options(), sampled(frame(json!({"dst_port": [80, 443, 8080]})))]}),
        1400,
    )
    .unwrap();
    let settings = Settings {
        kind: Kind::All,
        count: Some(4),
        duration: None,
        rate: 1_000_000_000,
        mode: Mode::Stream,
        burst_size: 64,
        template_interval: Duration::from_secs(30),
        template_every: 20,
    };
    let mut seen = Vec::new();
    let stats = sender::run(&mut plan, &settings, &AtomicBool::new(false), |bytes| {
        let word = |at: usize| u16::from_be_bytes([bytes[at], bytes[at + 1]]);
        seen.push((
            word(16),
            u32::from_be_bytes(bytes[8..12].try_into().unwrap()),
        ));
        Ok(bytes.len())
    })
    .unwrap();
    // Two template sets (no records), then options data (1 record) and flow data (3) twice.
    assert_eq!(
        seen,
        [(3, 0), (2, 0), (2600, 0), (2000, 1), (2600, 4), (2000, 5)]
    );
    assert_eq!(stats.records, 8);
}

#[test]
fn invalid_frame_sections_are_rejected_with_corrections() {
    let odd = json!(["0200", "020000"]);
    for (section, expected) in [
        (odd, "must all be 2 bytes"),
        (json!("02zz"), "hexadecimal"),
        (json!(5), "hexadecimal string"),
        (frame(json!({"clip": 53})), "at least 54 bytes"),
        (frame(json!({"clip": 200, "length": 100})), "clip..=65535"),
        (frame(json!({"protocol": 1})), "6 (TCP) or 17 (UDP)"),
        (
            frame(json!({"protocol": 17})),
            "tcp_flags requires protocol 6",
        ),
        (
            frame(json!({"ipv6": true, "tos": 1})),
            "only supported for IPv4",
        ),
        (frame(json!({"src": "2001:db8::1"})), "frame.src"),
        (frame(json!({"typo": 1})), "frame requires"),
    ] {
        let error = format!(
            "{:#}",
            build(json!({"flows": [sampled(section)]}), 1400).unwrap_err()
        );
        assert!(error.contains(expected), "{expected}: {error}");
    }
    let scope = json!({"flows": [{"template_id": 256,
        "scope": [{"name": "DATALINK_FRAME_SECTION", "value": "0200"}],
        "fields": [{"name": "IN_PKTS", "value": 1}]}]});
    assert!(format!("{:#}", build(scope, 1400).unwrap_err()).contains("cannot be a scope field"));

    // Without --ipfix the same configuration must fail instead of emitting v9 with IPFIX fields.
    let config: Config = serde_json::from_value(json!({"flows": [options()]})).unwrap();
    let error = format!("{:#}", config::validate(&config).unwrap_err());
    assert!(error.contains("rerun with --ipfix"), "{error}");
}

#[test]
fn cli_selects_ipfix_with_a_flag() {
    let run = |args: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_nfv9gen"))
            .args(args)
            .output()
            .unwrap();
        (
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).into_owned()
                + &String::from_utf8_lossy(&output.stderr),
        )
    };
    let example = "examples/ipfix-inline-monitoring.json";
    let (ok, text) = run(&["validate", example, "--ipfix"]);
    assert!(ok && text.contains("Validation passed"), "{text}");
    let (ok, text) = run(&["validate", example]);
    assert!(!ok && text.contains("rerun with --ipfix"), "{text}");
    let (ok, text) = run(&[
        "preview",
        example,
        "--ipfix",
        "--hex",
        "--kind",
        "templates",
    ]);
    assert!(
        ok && text.contains("0002001c07d00005000a0004000e0004003d000101380002013bffff"),
        "{text}"
    );
    // The flag must not change NetFlow v9 output for existing configurations.
    let (ok, text) = run(&[
        "preview",
        "examples/basic.json",
        "--hex",
        "--kind",
        "templates",
    ]);
    assert!(ok && text.contains("\n0009"), "{text}");
}
