use std::{
    collections::HashSet,
    io,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use neural_ensemble_telemetry_fabric_luminal_oracle_wizard9::{
    config::{self, Config},
    protocol::{self, Plan},
    sender::{self, Kind, Mode, Settings},
};
use serde_json::json;

fn build(value: serde_json::Value, limit: usize) -> anyhow::Result<Plan> {
    let config: Config = serde_json::from_value(value)?;
    protocol::build(config.source_id, &config::validate(&config)?, limit)
}

fn basic() -> serde_json::Value {
    serde_json::from_str(include_str!("../examples/basic.json")).unwrap()
}
fn options() -> serde_json::Value {
    serde_json::from_str(include_str!("../examples/options.json")).unwrap()
}
fn short(value: serde_json::Value) -> serde_json::Value {
    json!({"flows": [{"template_id": 256, "fields": [{"name": "PROTOCOL", "value": value}]}]})
}
fn word(bytes: &[u8], at: usize) -> u16 {
    u16::from_be_bytes(bytes[at..at + 2].try_into().unwrap())
}
fn long(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap())
}

#[test]
fn next_hops_encode_both_ip_families_and_reject_wrong_versions() {
    let fixture = serde_json::from_str(include_str!("../examples/nexthop.json")).unwrap();
    let plan = build(fixture, 1400).unwrap();
    for (index, ids, length, first, second) in [
        (0, [15, 18], 4, "192.0.2.1", "192.0.2.254"),
        (1, [62, 63], 16, "2001:db8::1", "2001:db8::ffff"),
    ] {
        let template = &plan.templates[index].bytes;
        for (column, id) in ids.iter().enumerate() {
            assert_eq!(word(template, 28 + column * 4), *id);
            assert_eq!(word(template, 30 + column * 4), length as u16);
        }
        let mut expected = Vec::new();
        for address in [first, second] {
            match address.parse::<std::net::IpAddr>().unwrap() {
                std::net::IpAddr::V4(ip) => expected.extend_from_slice(&ip.octets()),
                std::net::IpAddr::V6(ip) => expected.extend_from_slice(&ip.octets()),
            }
        }
        assert_eq!(&plan.data[index].bytes[24..24 + 2 * length], expected);
    }
    for (name, invalid) in [
        ("IPV4_NEXT_HOP", "2001:db8::1"),
        ("BGP_IPV4_NEXT_HOP", "2001:db8::1"),
        ("IPV6_NEXT_HOP", "192.0.2.1"),
        ("BGP_IPV6_NEXT_HOP", "192.0.2.1"),
        ("BGP_IPV6_NEXT_HOP", "invalid"),
    ] {
        let error = build(
            json!({"flows":[{"template_id":256,"fields":[{"name":name,"value":invalid}]}]}),
            1400,
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains(name));
    }
    for value in [
        json!(["2001:db8::1", "2001:db8::2"]),
        json!({"start":"2001:db8::1","end":"2001:db8::2"}),
        json!("2001:db8::/127"),
    ] {
        let plan = build(json!({"flows":[{"template_id":256,"fields":[{"name":"bgp_ipv6_next_hop","value":value}]}]}), 1400).unwrap();
        assert_eq!(plan.records, 2);
        assert_eq!(word(&plan.templates[0].bytes, 28), 63);
    }
}

#[test]
fn normal_template_and_data_are_independently_decodable() {
    let plan = build(basic(), 1400).unwrap();
    let template = &plan.templates[0].bytes;
    assert_eq!((word(template, 0), word(template, 2)), (9, 1));
    assert_eq!(
        (
            word(template, 20),
            word(template, 22),
            word(template, 24),
            word(template, 26)
        ),
        (0, 40, 256, 8)
    );
    let fields: Vec<_> = (28..60)
        .step_by(4)
        .map(|at| (word(template, at), word(template, at + 2)))
        .collect();
    assert_eq!(
        fields,
        [
            (8, 4),
            (12, 4),
            (7, 2),
            (11, 2),
            (4, 1),
            (6, 1),
            (1, 4),
            (2, 4)
        ]
    );
    let data = &plan.data[0].bytes;
    assert_eq!((word(data, 20), word(data, 2)), (256, 1));
    assert_eq!(&data[24..32], &[192, 0, 2, 1, 198, 51, 100, 2]);
    assert_eq!(
        (word(data, 32), word(data, 34), data[36], data[37]),
        (12345, 443, 6, 0x12)
    );
    assert_eq!((long(data, 38), long(data, 42)), (1000, 10));
}

#[test]
fn options_keep_distinct_scope_values_and_correct_descriptors() {
    let plan = build(options(), 1400).unwrap();
    let template = &plan.templates[0].bytes;
    assert_eq!(
        (
            word(template, 20),
            word(template, 24),
            word(template, 26),
            word(template, 28)
        ),
        (1, 300, 8, 8)
    );
    assert_eq!(
        (
            word(template, 30),
            word(template, 32),
            word(template, 34),
            word(template, 36)
        ),
        (2, 4, 3, 4)
    );
    let data = &plan.data[0].bytes;
    assert_eq!(
        (long(data, 24), long(data, 28), long(data, 32), data[36]),
        (1, 2, 100, 1)
    );
    assert_eq!(word(data, 2), 1);
}

#[test]
fn ip_and_integer_arrays_form_cartesian_products() {
    let fixture = serde_json::from_str(include_str!("../examples/ranges.json")).unwrap();
    let plan = build(fixture, 1400).unwrap();
    assert_eq!(plan.records, 11);
    assert_eq!(word(&plan.data[0].bytes, 2), 8);
    let data = &plan.data[0].bytes;
    let mut combinations = HashSet::new();
    for at in (24..24 + 8 * 11).step_by(11) {
        combinations.insert((data[at + 3], data[at + 7], word(data, at + 8)));
    }
    assert_eq!(combinations.len(), 8);
    let ipv6 = &plan.data[1].bytes;
    assert_eq!(word(ipv6, 2), 3);
    for i in 0..3 {
        assert_eq!(ipv6[24 + i * 33 + 15], (i + 1) as u8);
    }
}

#[test]
fn large_value_lists_preserve_order_across_packets() {
    let value = json!({"flows":[{"template_id":256,"fields":[
        {"name":"IN_BYTES","value":(0..500_000u32).collect::<Vec<_>>()}
    ]}]});
    let plan = build(value, 1400).unwrap();
    assert_eq!(plan.records, 500_000);
    let mut expected = 0u32;
    for packet in &plan.data {
        for index in 0..packet.data_records as usize {
            assert_eq!(long(&packet.bytes, 24 + index * 4), expected);
            expected += 1;
        }
    }
    assert_eq!(expected, 500_000);
}

#[test]
fn packet_size_and_padding_preserve_short_record_counts() {
    for record_length in 1..=3 {
        let field = match record_length {
            1 => "PROTOCOL",
            2 => "L4_DST_PORT",
            _ => "MPLS_LABEL_1",
        };
        for count in 1..100 {
            let value = json!({"flows": [{"template_id": 256, "fields": [{"name": field, "value": (0..count).collect::<Vec<_>>()}]}]});
            for limit in [64, 65, 67, 128] {
                let plan = build(value.clone(), limit).unwrap();
                let mut records = 0;
                for packet in plan.data {
                    let data = packet.bytes;
                    assert!(data.len() <= limit);
                    assert_eq!(word(&data, 22) as usize, data.len() - 20);
                    let count = word(&data, 2) as usize;
                    assert_eq!((data.len() - 24) / record_length, count);
                    assert!(data[24 + count * record_length..].iter().all(|b| *b == 0));
                    records += count;
                }
                assert_eq!(records, count);
            }
        }
    }
}

#[test]
fn invalid_values_fail_with_field_context() {
    for value in [
        json!(-1),
        json!(256),
        json!(true),
        json!(1.5),
        json!([]),
        json!("six"),
    ] {
        let error = format!("{:#}", build(short(value), 1400).unwrap_err());
        assert!(error.contains("flows[0]"), "{error}");
        assert!(error.contains("PROTOCOL"), "{error}");
    }
    let mut unknown = basic();
    unknown["flows"][0]["fields"][0]["name"] = json!("TYPO");
    assert!(
        format!("{:#}", build(unknown, 1400).unwrap_err()).contains("Unknown regular field TYPO")
    );
    let mut missing = basic();
    missing["flows"][0]["fields"][0]
        .as_object_mut()
        .unwrap()
        .remove("value");
    assert!(build(missing, 1400).is_err());
    let mut extra = basic();
    extra["soruce_id"] = json!(1);
    assert!(build(extra, 1400).is_err());
}

#[test]
fn shared_templates_preserve_flow_values_and_refresh_once() {
    let value = json!({"flows":[
        {"template_id":256,"fields":[{"name":"L4_SRC_PORT","value":1000},{"name":"IN_BYTES","value":100}]},
        {"template_id":256,"fields":[{"name":"l4_src_port","value":2000},{"name":"IN_BYTES","value":200}]}
    ]});
    let mut plan = build(value.clone(), 1400).unwrap();
    assert_eq!(
        (plan.templates.len(), plan.records, plan.data.len()),
        (1, 2, 2)
    );
    assert_eq!(
        (word(&plan.data[0].bytes, 24), long(&plan.data[0].bytes, 26)),
        (1000, 100)
    );
    assert_eq!(
        (word(&plan.data[1].bytes, 24), long(&plan.data[1].bytes, 26)),
        (2000, 200)
    );
    let mut options = settings();
    options.count = Some(4);
    options.template_every = 2;
    let stats = sender::run(&mut plan, &options, &AtomicBool::new(false), |bytes| {
        Ok(bytes.len())
    })
    .unwrap();
    assert_eq!(
        (stats.template_packets, stats.data_packets, stats.records),
        (2, 4, 4)
    );
    for fields in [
        json!([{"name":"IN_BYTES","value":200},{"name":"L4_SRC_PORT","value":2000}]),
        json!([{"name":"L4_DST_PORT","value":2000},{"name":"IN_BYTES","value":200}]),
        json!([{"name":"L4_SRC_PORT","value":2000}]),
    ] {
        let mut mismatch = value.clone();
        mismatch["flows"][1]["fields"] = fields;
        assert!(format!("{:#}", build(mismatch, 1400).unwrap_err()).contains("shared template"));
    }
    let mut mixed = value.clone();
    mixed["flows"][0]["fields"][1]["value"] = json!({"random_between":[100,200]});
    let mixed = build(mixed, 1400).unwrap();
    assert_eq!(mixed.templates.len(), 1);
    assert_eq!(mixed.data[0].random_fields.len(), 1);
    assert!(mixed.data[1].random_fields.is_empty());
    let mut mismatch = value.clone();
    mismatch["flows"][1]["scope"] = json!([{"name":"SYSTEM","value":1}]);
    assert!(build(mismatch, 1400).is_err());
    let options = json!({"flows":[
        {"template_id":300,"scope":[{"name":"SYSTEM","value":1}],"fields":[{"name":"SAMPLING_INTERVAL","value":1}]},
        {"template_id":300,"scope":[{"name":"SYSTEM","value":2}],"fields":[{"name":"SAMPLING_INTERVAL","value":10}]}
    ]});
    let plan = build(options.clone(), 1400).unwrap();
    assert_eq!((plan.templates.len(), plan.records), (1, 2));
    assert_eq!(
        (long(&plan.data[1].bytes, 24), long(&plan.data[1].bytes, 28)),
        (2, 10)
    );
    let mut mismatch = options.clone();
    mismatch["flows"][1]["scope"][0]["name"] = json!("INTERFACE");
    assert!(build(mismatch, 1400).is_err());
    // The public packet builder must also reject conflicting prevalidated flows.
    let config: Config = serde_json::from_value(options).unwrap();
    let mut flows = config::validate(&config).unwrap();
    flows[1].columns[0].spec.length = 2;
    assert!(protocol::build(0, &flows, 1400).is_err());
}

#[test]
fn ids_duplicates_and_size_are_checked() {
    let mut duplicate = basic();
    let flow = duplicate["flows"][0].clone();
    duplicate["flows"].as_array_mut().unwrap().push(flow);
    assert_eq!(build(duplicate, 1400).unwrap().templates.len(), 1);
    let mut fields = basic();
    let field = fields["flows"][0]["fields"][0].clone();
    fields["flows"][0]["fields"]
        .as_array_mut()
        .unwrap()
        .push(field);
    assert!(build(fields, 1400).is_err());
    for id in [0, 255, 65536] {
        let mut value = basic();
        value["flows"][0]["template_id"] = json!(id);
        assert!(build(value, 1400).is_err());
    }
    assert!(build(basic(), 63).is_err());
    assert!(build(basic(), 65508).is_err());
    let mut value = basic();
    value["flows"][0]["fields"] = json!([
        {"name":"IPV6_SRC_ADDR","value":"2001:db8::1"},
        {"name":"IPV6_DST_ADDR","value":"2001:db8::2"},
        {"name":"IPV6_NEXT_HOP","value":"2001:db8::3"}
    ]);
    assert!(format!("{:#}", build(value, 64).unwrap_err()).contains("record of"));
}

#[test]
fn bad_ranges_are_rejected_before_expansion() {
    for range in [
        json!({"start":"192.0.2.2","end":"192.0.2.1"}),
        json!({"start":"192.0.2.1","end":"192.0.2.2","step":0}),
        json!({"start":"192.0.2.1","end":"::1"}),
        json!({"start":"0.0.0.0","end":"255.255.255.255"}),
        json!({"start":"bad","end":"192.0.2.2"}),
        json!({"start":"192.0.2.1","end":"192.0.2.2","stpe":1}),
    ] {
        let mut value = basic();
        value["flows"][0]["fields"][0]["value"] = range;
        assert!(build(value, 1400).is_err());
    }
    let value = json!({"flows":[{"template_id":256,"fields":[
        {"name":"IPV6_SRC_ADDR","value":{"start":"::","end":"ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff"}}
    ]}]});
    assert!(build(value, 1400).is_err());
    let values: Vec<_> = (0..800).collect();
    let value = json!({"flows":[{"template_id":256,"fields":[
        {"name":"L4_SRC_PORT","value":values}, {"name":"L4_DST_PORT","value":values}
    ]}]});
    assert!(format!("{:#}", build(value, 1400).unwrap_err()).contains("500000"));
}

#[test]
fn expanded_record_limit_accepts_500000_and_rejects_one_more() {
    let mut value = json!({"flows":[{"template_id":256,"fields":[
        {"name":"IN_BYTES","value":{"start":0,"end":499999}}
    ]}]});
    let plan = build(value.clone(), 1400).unwrap();
    assert_eq!(plan.records, 500_000);
    assert_eq!(
        plan.data
            .iter()
            .map(|packet| packet.data_records as usize)
            .sum::<usize>(),
        500_000
    );
    value["flows"][0]["fields"][0]["value"]["end"] = json!(500000);
    assert!(format!("{:#}", build(value, 1400).unwrap_err()).contains("500000"));

    let mut total = json!({"flows":[
        {"template_id":256,"fields":[{"name":"IN_BYTES","value":{"start":0,"end":249999}}]},
        {"template_id":257,"fields":[{"name":"IN_BYTES","value":{"start":0,"end":249999}}]}
    ]});
    assert_eq!(build(total.clone(), 1400).unwrap().records, 500_000);
    total["flows"][1]["fields"][0]["value"]["end"] = json!(250000);
    assert!(format!("{:#}", build(total, 1400).unwrap_err()).contains("All flows combined"));
}

#[test]
fn header_fields_use_network_order_and_wrapping_counters() {
    let mut plan = build(basic(), 1400).unwrap();
    let packet = &mut plan.data[0];
    packet
        .stamp(
            42,
            u32::MAX,
            Duration::from_millis(1234),
            UNIX_EPOCH + Duration::from_secs(5000),
        )
        .unwrap();
    assert_eq!(
        (
            long(&packet.bytes, 4),
            long(&packet.bytes, 8),
            long(&packet.bytes, 12),
            long(&packet.bytes, 16)
        ),
        (1234, 5000, u32::MAX, 42)
    );
    packet
        .stamp(
            42,
            0,
            Duration::from_millis(u32::MAX as u64 + 1),
            SystemTime::now(),
        )
        .unwrap();
    assert_eq!((long(&packet.bytes, 4), long(&packet.bytes, 12)), (0, 0));
}

fn settings() -> Settings {
    Settings {
        kind: Kind::All,
        count: Some(5),
        duration: None,
        rate: 1_000_000_000,
        mode: Mode::Stream,
        burst_size: 64,
        template_interval: Duration::from_secs(30),
        template_every: 2,
    }
}

#[test]
fn sender_refreshes_templates_and_counts_actual_datagrams() {
    for mode in [Mode::Stream, Mode::Burst] {
        let mut plan = build(basic(), 1400).unwrap();
        let mut settings = settings();
        settings.mode = mode;
        let mut packets = Vec::new();
        let stats = sender::run(&mut plan, &settings, &AtomicBool::new(false), |b| {
            packets.push(b.to_vec());
            Ok(b.len())
        })
        .unwrap();
        assert_eq!(
            (
                stats.packets,
                stats.template_packets,
                stats.data_packets,
                stats.records
            ),
            (8, 3, 5, 5)
        );
        assert_eq!(
            stats.bytes,
            packets.iter().map(|p| p.len() as u64).sum::<u64>()
        );
        for (i, packet) in packets.iter().enumerate() {
            assert_eq!(long(packet, 12), i as u32);
            assert!(long(packet, 8) > 0);
            assert_eq!(long(packet, 16), 42);
        }
        assert_eq!(
            packets.iter().map(|p| word(p, 20)).collect::<Vec<_>>(),
            [0, 256, 256, 0, 256, 256, 0, 256]
        );
    }
}

#[test]
fn sender_supports_separate_modes_and_dataset_once_default() {
    for kind in [Kind::Data, Kind::Templates, Kind::All] {
        let mut plan = build(basic(), 1400).unwrap();
        let mut options = settings();
        options.kind = kind;
        options.count = None;
        let stats = sender::run(
            &mut plan,
            &options,
            &AtomicBool::new(false),
            |b| Ok(b.len()),
        )
        .unwrap();
        assert_eq!(stats.packets, if kind == Kind::All { 2 } else { 1 });
        assert_eq!(stats.records, if kind == Kind::Templates { 0 } else { 1 });
    }
}

#[test]
fn cancellation_and_transport_errors_never_report_successful_sends() {
    let mut plan = build(basic(), 1400).unwrap();
    let stop = AtomicBool::new(true);
    let stats = sender::run(&mut plan, &settings(), &stop, |_| panic!("must not send")).unwrap();
    assert!(stats.interrupted);
    assert_eq!(stats.packets, 0);
    stop.store(false, Ordering::Relaxed);
    let mut calls = 0;
    let stats = sender::run(&mut plan, &settings(), &stop, |b| {
        calls += 1;
        if calls == 2 {
            stop.store(true, Ordering::Relaxed);
        }
        Ok(b.len())
    })
    .unwrap();
    assert_eq!(stats.packets, 2);
    assert!(stats.interrupted);
    stop.store(false, Ordering::Relaxed);
    assert!(
        sender::run(&mut plan, &settings(), &stop, |_| Err(io::Error::other(
            "injected failure"
        )))
        .is_err()
    );
    assert!(sender::run(&mut plan, &settings(), &stop, |_| Ok(0)).is_err());
}

#[test]
fn zero_duration_repeats_until_manually_stopped() {
    for kind in [Kind::All, Kind::Data, Kind::Templates] {
        let mut plan = build(basic(), 1400).unwrap();
        let mut options = settings();
        options.kind = kind;
        options.count = None;
        options.duration = Some(Duration::ZERO);
        options.rate = 1_000_000;
        let stop = AtomicBool::new(false);
        let mut sent = 0;
        let stats = sender::run(&mut plan, &options, &stop, |bytes| {
            sent += 1;
            if sent == 5 {
                stop.store(true, Ordering::Relaxed);
            }
            Ok(bytes.len())
        })
        .unwrap();
        assert_eq!(sent, 5, "{kind:?} must continue beyond one round");
        assert!(stats.interrupted);
        assert_eq!(stats.packets, 5);
    }
}

#[test]
fn duration_does_not_sleep_until_the_next_low_rate_packet() {
    let mut plan = build(basic(), 1400).unwrap();
    let mut options = settings();
    options.count = None;
    options.rate = 1;
    options.duration = Some(Duration::from_millis(5));
    let stats = sender::run(
        &mut plan,
        &options,
        &AtomicBool::new(false),
        |b| Ok(b.len()),
    )
    .unwrap();
    assert_eq!(stats.data_packets, 1);
    assert!(stats.elapsed < Duration::from_secs(1));
}

#[test]
#[ignore = "Requires OS permission for UDP sockets; run with cargo test -- --ignored"]
fn udp_loopback_receives_real_netflow_packets() {
    use std::net::UdpSocket;
    let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
    receiver
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.connect(receiver.local_addr().unwrap()).unwrap();
    let mut plan = build(basic(), 1400).unwrap();
    let mut options = settings();
    options.count = Some(1);
    sender::run(&mut plan, &options, &AtomicBool::new(false), |b| {
        socket.send(b)
    })
    .unwrap();
    let mut bytes = [0u8; 2048];
    for sequence in 0..2 {
        let size = receiver.recv(&mut bytes).unwrap();
        assert!(size >= 24);
        assert_eq!(long(&bytes, 12), sequence);
    }
}

#[test]
fn time_refresh_and_runtime_validation_work_without_network() {
    let mut plan = build(basic(), 1400).unwrap();
    let mut options = settings();
    options.count = Some(2);
    options.template_every = u64::MAX;
    options.template_interval = Duration::from_nanos(1);
    let stats = sender::run(
        &mut plan,
        &options,
        &AtomicBool::new(false),
        |b| Ok(b.len()),
    )
    .unwrap();
    assert_eq!((stats.template_packets, stats.data_packets), (2, 2));
    for invalid in 0..5 {
        let mut options = settings();
        match invalid {
            0 => options.rate = 0,
            1 => options.count = Some(0),
            2 => options.template_every = 0,
            3 => options.template_interval = Duration::ZERO,
            _ => options.duration = Some(Duration::from_secs(1)),
        }
        assert!(
            sender::run(&mut plan, &options, &AtomicBool::new(false), |_| panic!(
                "invalid input must not send"
            ))
            .is_err()
        );
    }
}

#[test]
#[ignore = "Offline throughput observation; run in release mode; does not measure real network throughput"]
fn offline_throughput() {
    for observe in [false, true] {
        let mut plan = build(basic(), 1400).unwrap();
        let mut options = settings();
        options.count = Some(1_000_000);
        options.mode = Mode::Burst;
        options.template_every = u64::MAX;
        let mut checksum = 0u64;
        let mut reports = 0;
        let mut report = |stats: &sender::Stats| {
            reports += 1;
            std::hint::black_box(stats);
        };
        let stats = sender::run_with_progress(
            &mut plan,
            &options,
            &AtomicBool::new(false),
            |b| {
                checksum = checksum.wrapping_add(std::hint::black_box(b)[15] as u64);
                Ok(b.len())
            },
            if observe { Some(&mut report) } else { None },
        )
        .unwrap();
        assert_eq!(stats.data_packets, 1_000_000);
        assert_eq!(reports > 0, observe);
        std::hint::black_box(checksum);
        eprintln!(
            "offline (progress={observe}): {} data packets in {:.3}s = {:.0} packets/s (no UDP syscalls or terminal writes)",
            stats.data_packets,
            stats.elapsed.as_secs_f64(),
            stats.data_packets as f64 / stats.elapsed.as_secs_f64()
        );
    }
}

#[test]
fn mpls_all_ten_labels_and_metadata_have_correct_wire_definitions() {
    let mut fields = vec![
        json!({"name":"MPLS_TOP_LABEL_TYPE", "value":3}),
        json!({"name":"MPLS_TOP_LABEL_IP_ADDR", "value":"192.0.2.254"}),
    ];
    for index in 1..=10 {
        fields.push(json!({"name":format!("MPLS_LABEL_{index}"),
            "value":{"label":16000+index,"tc":3,"bos":index==10}}));
    }
    fields.extend([
        json!({"name":"MPLS_VPN_RD", "value":"65000:100"}),
        json!({"name":"MPLS_TOP_LABEL_PREFIX_LEN", "value":64}),
        json!({"name":"MPLS_TOP_LABEL_IPV6_ADDR", "value":"2001:db8::1"}),
        json!({"name":"MPLS_TOP_LABEL_TTL", "value":64}),
        json!({"name":"MPLS_LABEL_STACK_LENGTH", "value":40}),
        json!({"name":"MPLS_LABEL_STACK_DEPTH", "value":10}),
        json!({"name":"MPLS_TOP_LABEL_EXP", "value":3}),
    ]);
    let plan = build(json!({"flows":[{"template_id":400,"fields":fields}]}), 1400).unwrap();
    let template = &plan.templates[0].bytes;
    let mut expected = vec![(46, 1), (47, 4)];
    expected.extend((70..=79).map(|id| (id, 3)));
    expected.extend([
        (90, 8),
        (91, 1),
        (140, 16),
        (200, 1),
        (201, 4),
        (202, 4),
        (203, 1),
    ]);
    assert_eq!(word(template, 26), 19);
    for (index, pair) in expected.iter().enumerate() {
        assert_eq!(
            (
                word(template, 28 + index * 4),
                word(template, 30 + index * 4)
            ),
            *pair
        );
    }
    let data = &plan.data[0].bytes;
    assert_eq!(word(data, 2), 1);
    assert_eq!(data[24], 3);
    assert_eq!(&data[25..29], &[192, 0, 2, 254]);
    for index in 0..10 {
        let at = 29 + index * 3;
        let bits = u32::from_be_bytes([0, data[at], data[at + 1], data[at + 2]]);
        assert_eq!(bits >> 4, 16001 + index as u32);
        assert_eq!((bits >> 1) & 7, 3);
        assert_eq!(bits & 1, u32::from(index == 9));
    }
    assert_eq!(&data[59..67], &[0, 0, 0xfd, 0xe8, 0, 0, 0, 100]);
    assert_eq!(data[67], 64);
    assert_eq!(
        &data[68..84],
        &"2001:db8::1"
            .parse::<std::net::Ipv6Addr>()
            .unwrap()
            .octets()
    );
    assert_eq!(
        (data[84], long(data, 85), long(data, 89), data[93]),
        (64, 40, 10, 3)
    );
}

#[test]
fn mpls_label_objects_and_plain_labels_can_be_mixed_in_arrays() {
    let value = json!({"flows":[{"template_id":400,"fields":[
        {"name":"MPLS_LABEL_1","value":[{"label":16000,"tc":3,"bos":false},16000]},
        {"name":"MPLS_LABEL_10","value":[{"label":1048575,"tc":7,"bos":true},1048575]}
    ]}]});
    let plan = build(value, 1400).unwrap();
    assert_eq!(plan.records, 4);
    let data = &plan.data[0].bytes;
    for index in 0..4 {
        let at = 24 + index * 6;
        let first = u32::from_be_bytes([0, data[at], data[at + 1], data[at + 2]]);
        let last = u32::from_be_bytes([0, data[at + 3], data[at + 4], data[at + 5]]);
        assert_eq!((first >> 4, last >> 4), (16000, 1048575));
        assert_eq!(first & 15, if index < 2 { 6 } else { 0 });
        assert_eq!(last & 15, if index % 2 == 0 { 15 } else { 0 });
    }
    let plan = build(
        json!({"flows":[{"template_id":400,"fields":[
            {"name":"MPLS_LABEL_1","value":{"label":16}}
        ]}]}),
        1400,
    )
    .unwrap();
    assert_eq!(&plan.data[0].bytes[24..27], &[0, 1, 0]);
}

#[test]
fn rd_types_zero_one_and_two_encode_eight_octets() {
    let cases = [
        ("65000:100", [0, 0, 0xfd, 0xe8, 0, 0, 0, 100]),
        ("0:65535:4294967295", [0, 0, 255, 255, 255, 255, 255, 255]),
        ("192.0.2.9:100", [0, 1, 192, 0, 2, 9, 0, 100]),
        ("1:192.0.2.9:65535", [0, 1, 192, 0, 2, 9, 255, 255]),
        ("4200000000:100", [0, 2, 0xfa, 0x56, 0xea, 0, 0, 100]),
        ("2:65000:100", [0, 2, 0, 0, 0xfd, 0xe8, 0, 100]),
        ("4294967295:65535", [0, 2, 255, 255, 255, 255, 255, 255]),
    ];
    let values: Vec<_> = cases.iter().map(|(text, _)| *text).collect();
    let plan = build(
        json!({"flows":[{"template_id":400,"fields":[{"name":"MPLS_VPN_RD","value":values}]}]}),
        1400,
    )
    .unwrap();
    assert_eq!(plan.records, cases.len());
    for (index, (_, bytes)) in cases.iter().enumerate() {
        assert_eq!(&plan.data[0].bytes[24 + index * 8..32 + index * 8], bytes);
    }
}

#[test]
fn mpls_rejects_bad_labels_metadata_and_rd_with_context() {
    for (name, value) in [
        ("MPLS_LABEL_1", json!({"label":1048576})),
        ("MPLS_LABEL_1", json!(1048576)),
        ("MPLS_LABEL_1", json!(-1)),
        ("MPLS_LABEL_1", json!("16000")),
        ("MPLS_LABEL_1", json!(16000.5)),
        ("MPLS_LABEL_1", json!(true)),
        ("MPLS_LABEL_1", json!({"label":-1})),
        ("MPLS_LABEL_1", json!({"label":16,"tc":8})),
        ("MPLS_LABEL_1", json!({"label":16,"bos":1})),
        ("MPLS_LABEL_1", json!({"label":16,"ttl":64})),
        ("MPLS_LABEL_1", json!({"tc":3})),
        ("MPLS_LABEL_10", json!(16777216)),
        ("MPLS_TOP_LABEL_EXP", json!(8)),
        ("MPLS_TOP_LABEL_TTL", json!(256)),
        ("MPLS_TOP_LABEL_PREFIX_LEN", json!(129)),
        ("MPLS_LABEL_STACK_LENGTH", json!(4294967296u64)),
        ("MPLS_LABEL_STACK_DEPTH", json!(-1)),
        ("MPLS_TOP_LABEL_IPV6_ADDR", json!("192.0.2.1")),
        ("MPLS_VPN_RD", json!(123)),
    ] {
        let value = json!({"flows":[{"template_id":400,"fields":[{"name":name,"value":value}]}]});
        let error = format!("{:#}", build(value, 1400).unwrap_err());
        assert!(
            error.contains(name) && error.contains("flows[0]"),
            "{error}"
        );
    }
    for rd in [
        "",
        "bad:1",
        "3:1:1",
        "0:65536:1",
        "0:1:4294967296",
        "1:bad:1",
        "192.0.2.1:65536",
        "4200000000:65536",
        "4294967296:1",
        "2:1:-1",
        "::1:100",
    ] {
        let value =
            json!({"flows":[{"template_id":400,"fields":[{"name":"MPLS_VPN_RD","value":rd}]}]});
        let error = format!("{:#}", build(value, 1400).unwrap_err());
        assert!(error.contains("MPLS_VPN_RD"), "{rd}: {error}");
    }
}

#[test]
fn tcp_flag_letters_cover_every_eight_bit_combination() {
    let letters = [
        ('F', 1u16),
        ('S', 2),
        ('R', 4),
        ('P', 8),
        ('A', 16),
        ('U', 32),
        ('E', 64),
        ('C', 128),
    ];
    for mask in 0u16..=255 {
        let text = if mask == 0 {
            "0x00".to_owned()
        } else {
            letters
                .iter()
                .filter(|(_, bit)| mask & bit != 0)
                .map(|(letter, _)| *letter)
                .collect()
        };
        let value =
            json!({"flows":[{"template_id":256,"fields":[{"name":"TCP_FLAGS","value":text}]}]});
        let plan = build(value, 1400).unwrap();
        assert_eq!(plan.data[0].bytes[24], mask as u8, "{mask}: {text}");
    }
}

#[test]
fn tcp_flags_accept_names_hex_numbers_and_value_arrays() {
    let cases = [
        (json!("SFR"), 0x07),
        (json!("SA"), 0x12),
        (json!("pa"), 0x18),
        (json!("FIN"), 1),
        (json!("SYN"), 2),
        (json!("RST"), 4),
        (json!("PSH"), 8),
        (json!("ACK"), 16),
        (json!("URG"), 32),
        (json!("ECE"), 64),
        (json!("CWR"), 128),
        (json!("SYN|ACK"), 0x12),
        (json!(" syn | ack "), 0x12),
        (json!("S|A"), 0x12),
        (json!("SAS|SYN|ACK"), 0x12),
        (json!("ECE|CWR"), 0xc0),
        (json!("0x12"), 0x12),
        (json!(" 0Xff "), 0xff),
        (json!(18), 0x12),
        (json!(0), 0),
    ];
    let values: Vec<_> = cases.iter().map(|(value, _)| value.clone()).collect();
    let plan = build(
        json!({"flows":[{"template_id":256,"fields":[{"name":"TCP_FLAGS","value":values}]}]}),
        1400,
    )
    .unwrap();
    assert_eq!(plan.records, cases.len());
    let expected: Vec<u8> = cases.iter().map(|(_, bits)| *bits).collect();
    assert_eq!(&plan.data[0].bytes[24..24 + cases.len()], expected);
}

#[test]
fn tcp_flags_reject_typos_empty_tokens_and_overflow_with_context() {
    for value in [
        json!(""),
        json!(" "),
        json!("SYN||ACK"),
        json!("|S"),
        json!("SA|"),
        json!("SYNACK"),
        json!("SYN,ACK"),
        json!("SYN|ACKK"),
        json!("Q"),
        json!("Ａ"),
        json!("18"),
        json!("0x"),
        json!("0xGG"),
        json!("0x100"),
        json!(256),
        json!(-1),
        json!(true),
    ] {
        let input =
            json!({"flows":[{"template_id":256,"fields":[{"name":"TCP_FLAGS","value":value}]}]});
        let error = format!("{:#}", build(input, 1400).unwrap_err());
        assert!(
            error.contains("flows[0]") && error.contains("TCP_FLAGS"),
            "{value}: {error}"
        );
    }
}

#[test]
fn every_mpls_label_position_decodes_to_the_users_plain_input() {
    let inputs = [0u32, 1, 15, 16, 16000, 24000, 256006, 1048575];
    for position in 1..=10 {
        let name = format!("MPLS_LABEL_{position}");
        let value = json!({"flows":[{"template_id":400,"fields":[{"name":name,"value":inputs}]}]});
        let plan = build(value, 1400).unwrap();
        assert_eq!(plan.records, inputs.len());
        for (index, expected) in inputs.iter().enumerate() {
            let at = 24 + index * 3;
            let bytes = &plan.data[0].bytes;
            let encoded = u32::from_be_bytes([0, bytes[at], bytes[at + 1], bytes[at + 2]]);
            assert_eq!(encoded >> 4, *expected, "{name}: {expected}");
            assert_eq!(encoded & 15, 0, "plain labels default to TC=0, BoS=false");
        }
    }
    for input in inputs {
        let number = build(
            json!({"flows":[{"template_id":400,"fields":[{"name":"MPLS_LABEL_1","value":input}]}]}),
            1400,
        )
        .unwrap();
        let object = build(json!({"flows":[{"template_id":400,"fields":[{"name":"MPLS_LABEL_1","value":{"label":input}}]}]}), 1400).unwrap();
        assert_eq!(number.data[0].bytes, object.data[0].bytes);
    }
}

#[test]
fn asn_notations_decode_identically_in_source_and_destination_fields() {
    let cases = [
        (json!(0), 0),
        (json!(65000), 65000),
        (json!("65000"), 65000),
        (json!("AS65000"), 65000),
        (json!("as65000"), 65000),
        (json!(" As 65000 "), 65000),
        (json!("0.65000"), 65000),
        (json!("AS0.65000"), 65000),
        (json!("1.10"), 65546),
        (json!("AS1.10"), 65546),
        (json!(" AS1 . 10 "), 65546),
        (json!(65546), 65546),
        (json!("65535.65535"), u32::MAX),
        (json!("AS65535.65535"), u32::MAX),
        (json!(4294967295u64), u32::MAX),
        (json!("4294967295"), u32::MAX),
    ];
    for (name, id) in [("SRC_AS", 16), ("DST_AS", 17)] {
        let values: Vec<_> = cases.iter().map(|(value, _)| value.clone()).collect();
        let plan = build(
            json!({"flows":[{"template_id":500,"fields":[{"name":name,"value":values}]}]}),
            1400,
        )
        .unwrap();
        assert_eq!(
            (
                word(&plan.templates[0].bytes, 28),
                word(&plan.templates[0].bytes, 30)
            ),
            (id, 4)
        );
        assert_eq!(plan.records, cases.len());
        for (index, (value, expected)) in cases.iter().enumerate() {
            assert_eq!(
                long(&plan.data[0].bytes, 24 + index * 4),
                *expected,
                "{name}: {value}"
            );
        }
    }
}

#[test]
fn rd_asn_notation_uses_the_same_parser_and_preserves_type_limits() {
    let pairs = [
        ("AS65000:100", "65000:100"),
        (" 65000 : 100 ", "65000:100"),
        (" 0 : AS0 . 65000 : 100 ", "65000:100"),
        (" AS1 . 10 : 100 ", "65546:100"),
        (" 1 : 192.0.2.1 : 100 ", "192.0.2.1:100"),
        (" 2 : AS1 . 10 : 100 ", "65546:100"),
        ("0.65000:100", "65000:100"),
        ("0:AS0.65000:100", "65000:100"),
        ("AS1.10:100", "65546:100"),
        ("1.10:100", "65546:100"),
        ("2:AS0.65000:100", "2:65000:100"),
        ("AS65535.65535:65535", "4294967295:65535"),
    ];
    for (alias, plain) in pairs {
        let make = |rd| {
            build(
                json!({"flows":[{"template_id":500,"fields":[{"name":"MPLS_VPN_RD","value":rd}]}]}),
                1400,
            )
            .unwrap()
        };
        assert_eq!(
            make(alias).data[0].bytes,
            make(plain).data[0].bytes,
            "{alias}"
        );
    }
    for rd in [
        "0:AS1.10:100",
        "AS1.10:65536",
        "AS65536.1:1",
        "0:-1:1",
        "AS:100",
    ] {
        let error = build(
            json!({"flows":[{"template_id":500,"fields":[{"name":"MPLS_VPN_RD","value":rd}]}]}),
            1400,
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("MPLS_VPN_RD"), "{rd}");
    }
}

#[test]
fn asn_invalid_values_fail_before_packet_generation_with_context() {
    let values = [
        json!(-1),
        json!(4294967296u64),
        json!(1.10),
        json!(true),
        json!(""),
        json!("AS"),
        json!("AS-1"),
        json!("+65000"),
        json!("65000x"),
        json!("0xFDE8"),
        json!("1."),
        json!(".10"),
        json!("1.2.3"),
        json!("65536.0"),
        json!("0.65536"),
        json!("4294967296"),
        json!("ASAS65000"),
        json!("６５０００"),
    ];
    for name in ["SRC_AS", "DST_AS"] {
        for value in &values {
            let error = build(
                json!({"flows":[{"template_id":500,"fields":[{"name":name,"value":value}]}]}),
                1400,
            )
            .unwrap_err();
            let error = format!("{error:#}");
            assert!(
                error.contains("flows[0]") && error.contains(name),
                "{name} {value}: {error}"
            );
        }
    }
}

#[test]
fn progress_reports_successful_sends_while_waiting_and_is_throttled() {
    let mut plan = build(basic(), 1400).unwrap();
    let mut options = settings();
    options.count = None;
    options.rate = 1;
    options.duration = Some(Duration::from_millis(560));
    let mut snapshots = Vec::new();
    let mut report = |stats: &sender::Stats| {
        snapshots.push((stats.elapsed, stats.data_packets, stats.template_packets));
    };
    let stats = sender::run_with_progress(
        &mut plan,
        &options,
        &AtomicBool::new(false),
        |b| Ok(b.len()),
        Some(&mut report),
    )
    .unwrap();
    assert_eq!((stats.data_packets, stats.template_packets), (1, 1));
    assert!(snapshots.len() >= 2, "{snapshots:?}");
    assert_eq!((snapshots[0].1, snapshots[0].2), (0, 0));
    for pair in snapshots.windows(2) {
        assert!(pair[1].0 - pair[0].0 >= Duration::from_millis(250));
        assert_eq!((pair[1].1, pair[1].2), (1, 1));
    }
}

#[test]
fn completed_slow_burst_does_not_report_again() {
    for kind in [Kind::All, Kind::Data, Kind::Templates] {
        for timed in [false, true] {
            let mut config = basic();
            let mut second = config["flows"][0].clone();
            second["template_id"] = json!(257);
            config["flows"].as_array_mut().unwrap().push(second);
            let mut plan = build(config, 1400).unwrap();
            let mut options = settings();
            options.kind = kind;
            options.mode = Mode::Burst;
            options.rate = 4;
            options.burst_size = 4;
            options.count = if timed { None } else { Some(1) };
            options.duration = timed.then_some(Duration::from_millis(250));
            let stop = AtomicBool::new(false);
            let mut report = |stats: &sender::Stats| {
                if (timed && stats.packets > 0)
                    || stats.data_packets > 0
                    || (kind == Kind::Templates && stats.template_packets > 0)
                {
                    stop.store(true, Ordering::Relaxed);
                }
            };
            let stats = sender::run_with_progress(
                &mut plan,
                &options,
                &stop,
                |bytes| {
                    std::thread::sleep(Duration::from_millis(270));
                    Ok(bytes.len())
                },
                Some(&mut report),
            )
            .unwrap();
            assert!(!stats.interrupted, "kind={kind:?}, timed={timed}");
            assert_eq!(
                stats.packets,
                if kind == Kind::All && !timed { 3 } else { 1 }
            );
        }
    }
}

#[test]
fn progress_can_cancel_during_a_slow_burst() {
    for kind in [Kind::All, Kind::Data, Kind::Templates] {
        let mut config = basic();
        let mut second = config["flows"][0].clone();
        second["template_id"] = json!(257);
        config["flows"].as_array_mut().unwrap().push(second);
        let mut plan = build(config, 1400).unwrap();
        let mut options = settings();
        options.kind = kind;
        options.mode = Mode::Burst;
        options.rate = 4;
        options.burst_size = 4;
        options.count = Some(4);
        let stop = AtomicBool::new(false);
        let mut report = |stats: &sender::Stats| {
            if stats.packets > 0 {
                stop.store(true, Ordering::Relaxed);
            }
        };
        let stats = sender::run_with_progress(
            &mut plan,
            &options,
            &stop,
            |bytes| {
                std::thread::sleep(Duration::from_millis(270));
                Ok(bytes.len())
            },
            Some(&mut report),
        )
        .unwrap();
        assert!(stats.interrupted);
        assert_eq!(stats.packets, 1, "must stop within the burst");
    }
}

#[test]
fn progress_can_cancel_without_sending_an_extra_packet() {
    let mut plan = build(basic(), 1400).unwrap();
    let mut options = settings();
    options.rate = 1;
    let stop = AtomicBool::new(false);
    let mut report = |stats: &sender::Stats| {
        if stats.data_packets == 1 {
            stop.store(true, Ordering::Relaxed);
        }
    };
    let stats = sender::run_with_progress(
        &mut plan,
        &options,
        &stop,
        |b| Ok(b.len()),
        Some(&mut report),
    )
    .unwrap();
    assert!(stats.interrupted);
    assert_eq!((stats.data_packets, stats.template_packets), (1, 1));
}

#[test]
fn random_fields_redraw_records_without_touching_headers_or_other_fields() {
    let mut plan = build(
        json!({"flows":[{"template_id":800,"fields":[
            {"name":"PROTOCOL","value":[6,17]},
            {"name":"L4_SRC_PORT","value":{"random_between":[100,102]}},
            {"name":"L4_DST_PORT","value":{"random_between":[0,65535]}}
        ]}]}),
        1400,
    )
    .unwrap();
    assert_eq!(plan.records, 2);
    assert!(plan.templates[0].random_fields.is_empty());
    let packet = &mut plan.data[0];
    let original = packet.bytes.clone();
    // 65535 is rejected for a three-value range; the remaining draws cover both bounds.
    let draws: Vec<u8> = [65535u16, 0, 2, 0, 65535]
        .into_iter()
        .flat_map(u16::to_be_bytes)
        .collect();
    packet.randomize_fields(&mut draws.as_slice()).unwrap();
    assert_eq!(&packet.bytes[..24], &original[..24]);
    assert_eq!((packet.bytes[24], packet.bytes[29]), (6, 17));
    assert_eq!(
        (word(&packet.bytes, 25), word(&packet.bytes, 30)),
        (100, 102)
    );
    assert_eq!(
        (word(&packet.bytes, 27), word(&packet.bytes, 32)),
        (0, 65535)
    );
    assert_eq!(&packet.bytes[34..], &original[34..]);
    packet.randomize_fields(&mut [0u8; 8].as_slice()).unwrap();
    assert_eq!((word(&packet.bytes, 30), word(&packet.bytes, 32)), (100, 0));
    assert!(packet.randomize_fields(&mut [].as_slice()).is_err());
}

#[test]
fn random_fields_validate_bounds_and_do_not_expand_combinations() {
    let one = |name: &str, value: serde_json::Value| json!({"flows":[{"template_id":800,"fields":[{"name":name,"value":value}]}]});
    let plan = build(one("l4_src_port", json!({"random_between":[1,65535]})), 64).unwrap();
    assert_eq!(plan.records, 1);
    let port = &plan.data[0].random_fields[0];
    assert_eq!((port.min, port.max), (1, 65535));
    let mut plan = build(one("L4_DST_PORT", json!({"random_between":[0,0]})), 64).unwrap();
    plan.data[0]
        .randomize_fields(&mut [255, 255].as_slice())
        .unwrap();
    assert_eq!(word(&plan.data[0].bytes, 24), 0);
    for (name, value) in [
        ("OUT_BYTES", json!({"random_between":[1,2]})),
        ("L4_SRC_PORT", json!({"random_between":[1]})),
        ("L4_SRC_PORT", json!({"random_between":[1,2,3]})),
        ("L4_SRC_PORT", json!({"random_between":[2,1]})),
        ("L4_SRC_PORT", json!({"random_between":[-1,2]})),
        ("L4_SRC_PORT", json!({"random_between":[1,65536]})),
        ("L4_SRC_PORT", json!({"random_between":[1.5,2]})),
        ("L4_SRC_PORT", json!({"random_between":"1,2"})),
        ("L4_SRC_PORT", json!({"random_between":[1,2],"random":true})),
        ("L4_SRC_PORT", json!({"random_between":[1,2],"step":1})),
        ("L4_SRC_PORT", json!([{"random_between":[1,2]}])),
        ("L4_SRC_PORT", json!({"random":true})),
        ("L4_DST_PORT", json!({"random":true,"min":1,"max":65535})),
    ] {
        let error = build(one(name, value), 1400).unwrap_err();
        assert!(format!("{error:#}").contains(name));
    }
}

#[test]
fn sender_accepts_random_fields_on_reused_data_packets() {
    let value = serde_json::from_str(include_str!("../examples/random-ports.json")).unwrap();
    let mut plan = build(value, 1400).unwrap();
    let mut settings = settings();
    settings.count = Some(3);
    let mut data = 0;
    let stats = sender::run(&mut plan, &settings, &AtomicBool::new(false), |bytes| {
        if word(bytes, 20) == 800 {
            data += 1;
            assert!((49152..=65535).contains(&word(bytes, 33)));
            assert_eq!(word(bytes, 35), 443);
        }
        Ok(bytes.len())
    })
    .unwrap();
    assert_eq!(data, 3);
    assert_eq!(stats.data_packets, 3);
}

#[test]
fn random_counters_cover_u32_bounds_and_redraw_each_record() {
    let mut plan = build(
        json!({"flows":[{"template_id":801,"fields":[
            {"name":"PROTOCOL","value":[6,17]},
            {"name":"IN_PKTS","value":{"random_between":[65536,65538]}},
            {"name":"IN_BYTES","value":{"random_between":[0,4294967295u32]}}
        ]}]}),
        64,
    )
    .unwrap();
    assert_eq!(plan.records, 2);
    let packet = &mut plan.data[0];
    assert_eq!(packet.record_length, 9);
    assert_eq!(
        (long(&packet.bytes, 25), long(&packet.bytes, 29)),
        (65536, 0)
    );
    let original = packet.bytes.clone();
    // Reject the biased tail, then cover both ends of each counter range.
    let draws: Vec<_> = [u32::MAX, 0, 2, 0, u32::MAX]
        .into_iter()
        .flat_map(u32::to_be_bytes)
        .collect();
    packet.randomize_fields(&mut draws.as_slice()).unwrap();
    assert_eq!(&packet.bytes[..24], &original[..24]);
    assert_eq!((packet.bytes[24], packet.bytes[33]), (6, 17));
    assert_eq!(
        (long(&packet.bytes, 25), long(&packet.bytes, 34)),
        (65536, 65538)
    );
    assert_eq!(
        (long(&packet.bytes, 29), long(&packet.bytes, 38)),
        (0, u32::MAX)
    );
    assert_eq!(&packet.bytes[42..], &original[42..]);
    packet.randomize_fields(&mut [0u8; 16].as_slice()).unwrap();
    assert_eq!(
        (long(&packet.bytes, 34), long(&packet.bytes, 38)),
        (65536, 0)
    );
    assert!(packet.randomize_fields(&mut [].as_slice()).is_err());

    for name in ["IN_PKTS", "IN_BYTES"] {
        for value in [
            json!({"random_between":[0,0]}),
            json!({"random_between":[4294967295u32,4294967295u32]}),
        ] {
            let expected = value["random_between"][0].as_u64().unwrap() as u32;
            let mut fixed = build(
                json!({"flows":[{"template_id":801,"fields":[{"name":name,"value":value}]}]}),
                64,
            )
            .unwrap();
            fixed.data[0]
                .randomize_fields(&mut [255u8; 4].as_slice())
                .unwrap();
            assert_eq!(long(&fixed.data[0].bytes, 24), expected);
        }
        for value in [
            json!({"random_between":[0,4294967296u64]}),
            json!({"random_between":[-1,2]}),
            json!({"random_between":[2,1]}),
            json!({"random_between":[1.5,2]}),
        ] {
            assert!(
                build(
                    json!({"flows":[{"template_id":801,"fields":[{"name":name,"value":value}]}]}),
                    64
                )
                .is_err()
            );
        }
    }
}

#[test]
fn sender_redraws_random_counters_on_reused_packets() {
    let mut plan = build(
        serde_json::from_str(include_str!("../examples/random-counters.json")).unwrap(),
        1400,
    )
    .unwrap();
    let mut options = settings();
    options.count = Some(3);
    let mut data = 0;
    let stats = sender::run(&mut plan, &options, &AtomicBool::new(false), |bytes| {
        if word(bytes, 20) == 801 {
            data += 1;
            assert!((10..=100).contains(&long(bytes, 24)));
            assert!((1000..=100000).contains(&long(bytes, 28)));
        }
        Ok(bytes.len())
    })
    .unwrap();
    assert_eq!((data, stats.records), (3, 3));
}

#[test]
fn random_file_draws_host_addresses_relative_to_the_config() {
    let config = config::load(std::path::Path::new("examples/random-ips.json")).unwrap();
    let mut plan =
        protocol::build(config.source_id, &config::validate(&config).unwrap(), 1400).unwrap();
    assert_eq!(plan.records, 1);
    let packet = &mut plan.data[0];
    // Preview bytes use the first address; draws index the three file entries.
    assert_eq!(&packet.bytes[24..28], &[192, 0, 2, 10]);
    packet
        .randomize_fields(&mut [0u8, 0, 0, 2].as_slice())
        .unwrap();
    assert_eq!(&packet.bytes[24..28], &[198, 51, 100, 0]);
    packet
        .randomize_fields(&mut [0u8, 0, 0, 1].as_slice())
        .unwrap();
    assert_eq!(&packet.bytes[24..28], &[192, 0, 2, 20]);
    assert_eq!(&packet.bytes[28..32], &[203, 0, 113, 1]);

    let dir = std::env::temp_dir().join(format!(
        "NeuralEnsembleTelemetryFabricLuminalOracleWizard9-random-file-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let one = |name: &str, file: &str, text: &str| {
        std::fs::write(dir.join(file), text).unwrap();
        let mut config: Config = serde_json::from_value(json!({"flows":[{"template_id":800,"fields":[{"name":name,"value":{"random_file":file}}]}]})).unwrap();
        config.base = dir.clone();
        config::validate(&config).map(|_| ())
    };
    assert!(one("IPV6_SRC_ADDR", "v6.txt", "2001:db8::1\n\n2001:db8::2\n").is_ok());
    for (name, file, text) in [
        (
            "IPV4_SRC_ADDR",
            "cidr-v4.txt",
            "192.0.2.1\n198.51.100.0/24\n",
        ),
        (
            "IPV6_SRC_ADDR",
            "cidr-v6.txt",
            "2001:db8::1\n2001:db8::2/64\n",
        ),
        ("IPV4_SRC_ADDR", "v6-in-v4.txt", "2001:db8::1\n"),
        ("IPV4_SRC_ADDR", "empty.txt", "# nothing\n\n"),
        ("IPV4_SRC_ADDR", "bad.txt", "192.0.2.1\nnot-an-ip\n"),
        ("L4_SRC_PORT", "port.txt", "192.0.2.1\n"),
    ] {
        let error = format!("{:#}", one(name, file, text).unwrap_err());
        assert!(error.contains(name), "{error}");
        if file.starts_with("cidr-") {
            assert!(error.contains(file) && error.contains("line 2"), "{error}");
        }
    }
    assert!(
        format!(
            "{:#}",
            one("IPV4_SRC_ADDR", "bad.txt", "192.0.2.1\nnot-an-ip\n").unwrap_err()
        )
        .contains("line 2")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
