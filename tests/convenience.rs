use anyhow::Result;
use nfgen::{
    config::{self, Config},
    protocol::{self, Plan},
};
use serde_json::{Value, json};

fn build(value: Value, size: usize) -> Result<Plan> {
    let config: Config = serde_json::from_value(value)?;
    protocol::build(config.source_id, &config::validate(&config)?, size)
}
fn one(name: &str, value: Value) -> Value {
    json!({"flows":[{"template_id":600,"fields":[{"name":name,"value":value}]}]})
}
fn word(bytes: &[u8], at: usize) -> u16 {
    u16::from_be_bytes(bytes[at..at + 2].try_into().unwrap())
}

#[test]
fn icmp_type_code_objects_and_names_have_exact_wire_bytes() {
    for (name, id, values, expected) in [
        (
            "ICMP_TYPE",
            32,
            json!([{"type":8,"code":0}, "echo-request", "ECHO-REPLY", {"type":"destination-unreachable","code":3}, 2048, {"type":255,"code":255}]),
            vec![2048, 2048, 0, 771, 2048, 65535],
        ),
        (
            "ICMP_TYPE_IPV6",
            139,
            json!([{"type":128}, "echo-request", "echo-reply", {"type":"time-exceeded","code":1}, "packet-too-big", "parameter-problem"]),
            vec![32768, 32768, 33024, 769, 512, 1024],
        ),
    ] {
        let plan = build(one(name, values), 1400).unwrap();
        assert_eq!(
            (
                word(&plan.templates[0].bytes, 28),
                word(&plan.templates[0].bytes, 30)
            ),
            (id, 2)
        );
        for (index, expected) in expected.iter().enumerate() {
            assert_eq!(word(&plan.data[0].bytes, 24 + index * 2), *expected);
        }
    }
}

#[test]
fn icmp_rejects_invalid_codes_names_and_unknown_keys() {
    for (name, value) in [
        ("ICMP_TYPE", json!({"type":256})),
        ("ICMP_TYPE", json!({"type":8,"code":256})),
        ("ICMP_TYPE", json!({"type":-1})),
        ("ICMP_TYPE", json!({"code":0})),
        ("ICMP_TYPE", json!({"type":8,"cod":0})),
        ("ICMP_TYPE", json!("packet-too-big")),
        ("ICMP_TYPE", json!(65536)),
        ("ICMP_TYPE_IPV6", json!({"type":128,"code":true})),
        ("ICMP_TYPE_IPV6", json!("not-a-type")),
    ] {
        let error = format!("{:#}", build(one(name, value), 1400).unwrap_err());
        assert!(
            error.contains("flows[0]") && error.contains(name),
            "{error}"
        );
    }
}

#[test]
fn ipv4_prefix_normalizes_host_bits_and_includes_all_addresses_by_default() {
    let plan = build(one("IPV4_SRC_ADDR", json!("192.0.2.7/30")), 1400).unwrap();
    assert_eq!(plan.records, 4);
    for index in 0..4 {
        assert_eq!(
            &plan.data[0].bytes[24 + index * 4..28 + index * 4],
            &[192, 0, 2, (4 + index) as u8]
        );
    }
    let plan = build(
        one(
            "IPV4_SRC_ADDR",
            json!({"prefix":"192.0.2.7/30","hosts_only":true}),
        ),
        1400,
    )
    .unwrap();
    assert_eq!(plan.records, 2);
    assert_eq!(&plan.data[0].bytes[24..32], &[192, 0, 2, 5, 192, 0, 2, 6]);
}

#[test]
fn point_to_point_and_single_host_prefixes_keep_their_addresses() {
    for (prefix, count, first) in [("192.0.2.1/31", 2, 0), ("192.0.2.1/32", 1, 1)] {
        let plan = build(
            one("IPV4_SRC_ADDR", json!({"prefix":prefix,"hosts_only":true})),
            1400,
        )
        .unwrap();
        assert_eq!(plan.records, count);
        assert_eq!(plan.data[0].bytes[27], first);
    }
    let plan = build(one("IPV6_SRC_ADDR", json!("2001:db8::3/126")), 1400).unwrap();
    assert_eq!(plan.records, 4);
    for index in 0..4 {
        assert_eq!(plan.data[0].bytes[24 + index * 16 + 15], index as u8);
    }
    let plan = build(
        one(
            "IPV6_SRC_ADDR",
            json!("ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff/128"),
        ),
        1400,
    )
    .unwrap();
    assert_eq!(&plan.data[0].bytes[24..40], &[255; 16]);
}

#[test]
fn prefix_rejects_bad_versions_lengths_and_excessive_expansion() {
    for (name, value) in [
        ("IPV4_SRC_ADDR", json!("192.0.2.1/33")),
        ("IPV6_SRC_ADDR", json!("2001:db8::/129")),
        ("IPV4_SRC_ADDR", json!("192.0.2.1/-1")),
        ("IPV4_SRC_ADDR", json!("192.0.2.1/")),
        ("IPV4_SRC_ADDR", json!("0.0.0.0/0")),
        ("IPV6_SRC_ADDR", json!("::/0")),
        ("IPV4_SRC_ADDR", json!("2001:db8::/126")),
        ("IPV6_SRC_ADDR", json!("192.0.2.0/30")),
        (
            "IPV6_SRC_ADDR",
            json!({"prefix":"2001:db8::/126","hosts_only":true}),
        ),
        (
            "IPV4_SRC_ADDR",
            json!({"prefix":"192.0.2.0/30","hosts_only":1}),
        ),
        ("L4_SRC_PORT", json!({"prefix":"192.0.2.0/30"})),
    ] {
        let error = format!("{:#}", build(one(name, value), 1400).unwrap_err());
        assert!(error.contains(name), "{error}");
    }
}

#[test]
fn numerical_ranges_apply_field_encoders_and_step_without_truncation() {
    let cases = [
        (
            "L4_SRC_PORT",
            json!({"start":10000,"end":10005,"step":2}),
            vec![10000u32, 10002, 10004],
            2,
            0,
        ),
        (
            "MPLS_LABEL_10",
            json!({"start":16000,"end":16002}),
            vec![16000, 16001, 16002],
            3,
            4,
        ),
        (
            "SRC_AS",
            json!({"start":"AS1.10","end":"1.12"}),
            vec![65546, 65547, 65548],
            4,
            0,
        ),
        (
            "IP_PROTOCOL_VERSION",
            json!({"start":4,"end":6,"step":2}),
            vec![4, 6],
            1,
            0,
        ),
        (
            "IN_BYTES",
            json!({"start":4294967294u64,"end":4294967295u64}),
            vec![4294967294, 4294967295],
            4,
            0,
        ),
    ];
    for (name, value, expected, length, shift) in cases {
        let plan = build(one(name, value), 1400).unwrap();
        assert_eq!(plan.records, expected.len());
        for (index, expected) in expected.iter().enumerate() {
            let at = 24 + index * length;
            let encoded = plan.data[0].bytes[at..at + length]
                .iter()
                .fold(0u32, |v, b| (v << 8) | *b as u32);
            assert_eq!(encoded >> shift, *expected, "{name}");
        }
    }
    let max_step = build(
        one(
            "L4_DST_PORT",
            json!({"start":1,"end":65535,"step":u64::MAX}),
        ),
        1400,
    )
    .unwrap();
    assert_eq!(max_step.records, 1);
}

#[test]
fn range_rejects_bad_bounds_steps_and_noncontiguous_enumerated_values() {
    for (name, value) in [
        ("L4_SRC_PORT", json!({"start":2,"end":1})),
        ("L4_SRC_PORT", json!({"start":1,"end":3,"step":0})),
        ("L4_SRC_PORT", json!({"start":1,"end":65536})),
        ("IN_BYTES", json!({"start":0,"end":500000})),
        ("IP_PROTOCOL_VERSION", json!({"start":4,"end":6})),
        ("MPLS_TOP_LABEL_EXP", json!({"start":0,"end":8})),
        ("MPLS_LABEL_1", json!({"start":0,"end":1048576})),
        ("MPLS_LABEL_1", json!({"start":{"label":1},"end":3})),
        ("MPLS_VPN_RD", json!({"start":1,"end":3})),
        ("SRC_AS", json!({"start":-1,"end":5})),
    ] {
        let error = format!("{:#}", build(one(name, value), 1400).unwrap_err());
        assert!(error.contains(name), "{error}");
    }
}

#[test]
fn prefix_and_number_ranges_share_cartesian_limits_and_packet_splitting() {
    let value = json!({"flows":[{"template_id":600,"fields":[
        {"name":"IPV4_SRC_ADDR","value":"192.0.2.0/30"},
        {"name":"L4_DST_PORT","value":{"start":10000,"end":10002}}
    ]}]});
    let plan = build(value, 64).unwrap();
    assert_eq!(plan.records, 12);
    assert!(plan.data.len() > 1);
    let mut pairs = std::collections::HashSet::new();
    for packet in plan.data {
        assert!(packet.bytes.len() <= 64);
        for index in 0..packet.data_records as usize {
            let at = 24 + index * 6;
            pairs.insert((packet.bytes[at + 3], word(&packet.bytes, at + 4)));
        }
    }
    assert_eq!(pairs.len(), 12);
    let value = json!({"flows":[{"template_id":600,"fields":[
        {"name":"IPV4_SRC_ADDR","value":"192.0.0.0/16"},
        {"name":"L4_DST_PORT","value":{"start":1,"end":8}}
    ]}]});
    assert!(format!("{:#}", build(value, 1400).unwrap_err()).contains("500000"));
}

#[test]
fn removed_mpls_stack_syntax_is_rejected() {
    let error = build(
        json!({"flows":[{"template_id":600,"mpls_stack":[16],"fields":[{"name":"PROTOCOL","value":6}]}]}),
        1400,
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("unknown field `mpls_stack`"));
}
