use std::{collections::HashSet, sync::atomic::AtomicBool, time::Duration};

use neural_ensemble_telemetry_fabric_luminal_oracle_wizard9::{
    config::{self, Config},
    fields, protocol,
    sender::{self, Kind, Mode, Settings},
};

#[test]
fn collector_fixture_exports_every_field_and_scope_with_documented_counts() {
    let config: Config =
        serde_json::from_str(include_str!("../examples/collector-all-fields.json")).unwrap();
    let registry: Vec<_> = include_str!("../src/fields.rs")
        .lines()
        .filter_map(|line| line.trim().strip_prefix('"')?.split('"').next())
        .collect();
    for scope in [false, true] {
        let expected: HashSet<_> = registry
            .iter()
            .copied()
            .filter(|name| fields::lookup(name, scope).is_ok())
            .collect();
        let actual: HashSet<_> = config
            .flows
            .iter()
            .flat_map(|flow| if scope { &flow.scope } else { &flow.fields })
            .map(|field| field.name.as_str())
            .collect();
        assert_eq!(actual, expected, "missing or extra fields (scope={scope})");
    }
    let flows = config::validate(&config).unwrap();
    let mut plan = protocol::build(config.source_id, &flows, 1400).unwrap();
    assert_eq!(plan.source_id, 424242);
    assert_eq!(
        (plan.templates.len(), plan.data.len(), plan.records),
        (14, 14, 22)
    );
    assert_eq!(
        flows
            .iter()
            .filter(|flow| flow.scope_count == 0)
            .map(|flow| flow.record_count)
            .sum::<usize>(),
        17
    );
    for (flow, packet) in flows.iter().zip(&plan.data) {
        assert_eq!(packet.template_id, flow.template_id);
        assert_eq!(packet.data_records as usize, flow.record_count);
        assert!(packet.bytes.len() <= 1400);
    }
    let settings = Settings {
        kind: Kind::All,
        count: Some(140),
        duration: None,
        rate: 1_000_000_000,
        mode: Mode::Stream,
        burst_size: 64,
        template_interval: Duration::from_secs(30),
        template_every: 14,
    };
    let mut traffic_records = 0u64;
    let stats = sender::run(&mut plan, &settings, &AtomicBool::new(false), |bytes| {
        let flowset = u16::from_be_bytes([bytes[20], bytes[21]]);
        if (1000..=1008).contains(&flowset) {
            traffic_records += u16::from_be_bytes([bytes[2], bytes[3]]) as u64;
        }
        Ok(bytes.len())
    })
    .unwrap();
    assert_eq!(
        (stats.template_packets, stats.data_packets, stats.records),
        (140, 140, 220)
    );
    assert_eq!(traffic_records, 170);
}
