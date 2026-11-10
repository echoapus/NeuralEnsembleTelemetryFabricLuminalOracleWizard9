use std::{
    io::{self, Write},
    net::{Ipv4Addr, Ipv6Addr},
};

use neural_ensemble_telemetry_fabric_luminal_oracle_wizard9::{
    config::{Flow, Values},
    fields::{FieldSpec, Kind},
};

fn value(bytes: &[u8], spec: FieldSpec) -> String {
    let number = || bytes.iter().fold(0u128, |n, byte| (n << 8) | *byte as u128);
    match spec.kind {
        Kind::Ipv4 => Ipv4Addr::from(<[u8; 4]>::try_from(bytes).unwrap()).to_string(),
        Kind::Ipv6 => Ipv6Addr::from(<[u8; 16]>::try_from(bytes).unwrap()).to_string(),
        Kind::Asn => format!("AS{}", number()),
        Kind::Icmp4 | Kind::Icmp6 => format!("type={} code={}", bytes[0], bytes[1]),
        Kind::MplsLabel => {
            let n = number();
            format!("label={} tc={} bos={}", n >> 4, (n >> 1) & 7, n & 1)
        }
        Kind::RouteDistinguisher => {
            let kind = u16::from_be_bytes([bytes[0], bytes[1]]);
            let admin = match kind {
                1 => Ipv4Addr::from(<[u8; 4]>::try_from(&bytes[2..6]).unwrap()).to_string(),
                0 => u16::from_be_bytes([bytes[2], bytes[3]]).to_string(),
                _ => u32::from_be_bytes(bytes[2..6].try_into().unwrap()).to_string(),
            };
            let assigned = if kind == 0 {
                u32::from_be_bytes(bytes[4..8].try_into().unwrap())
            } else {
                u16::from_be_bytes([bytes[6], bytes[7]]) as u32
            };
            format!("{kind}:{admin}:{assigned}")
        }
        Kind::Number if spec.id == 6 => {
            let flags: Vec<_> = ["FIN", "SYN", "RST", "PSH", "ACK", "URG", "ECE", "CWR"]
                .into_iter()
                .enumerate()
                .filter_map(|(bit, name)| (bytes[0] & (1 << bit) != 0).then_some(name))
                .collect();
            format!(
                "{} (0x{:02x})",
                if flags.is_empty() {
                    "None".into()
                } else {
                    flags.join("|")
                },
                bytes[0]
            )
        }
        Kind::Number => number().to_string(),
    }
}

pub fn flow(flow: &Flow, out: &mut impl Write) -> io::Result<()> {
    writeln!(
        out,
        "Template {} · {} · {} combinations (Cartesian product of field values) · {} B per record",
        flow.template_id,
        if flow.scope_count == 0 {
            "Regular flow"
        } else {
            "Options"
        },
        flow.record_count,
        flow.record_length
    )?;
    if flow.record_count > 1 {
        let factors: Vec<_> = flow
            .columns
            .iter()
            .enumerate()
            .filter(|(_, column)| column.values.count() > 1)
            .map(|(index, column)| {
                format!(
                    "{}{} {}",
                    if index < flow.scope_count {
                        "scope."
                    } else {
                        ""
                    },
                    column.name,
                    column.values.count()
                )
            })
            .collect();
        writeln!(
            out,
            "  Expansion: {} = {} records",
            factors.join(" × "),
            flow.record_count
        )?;
    }
    writeln!(
        out,
        "  Field / Length / Value count / First value (last value shown for multiple values)"
    )?;
    for (index, column) in flow.columns.iter().enumerate() {
        if let Values::RandomField { min, max, length } = column.values {
            writeln!(
                out,
                "  {} / {length} B / random_between {min}..={max} (resampled per record on each send; preview hex uses min)",
                column.name
            )?;
            continue;
        }
        if let Values::RandomFile { bytes, length } = &column.values {
            writeln!(
                out,
                "  {} / {length} B / random_file {} addresses (resampled per record on each send; preview hex uses the first)",
                column.name,
                bytes.len() / length
            )?;
            continue;
        }
        let mut bytes = Vec::with_capacity(column.spec.length);
        column.values.append(0, &mut bytes);
        // Scope IDs use a separate namespace; ID 6 only means TCP flags in data fields.
        let sample = if index < flow.scope_count {
            bytes
                .iter()
                .fold(0u128, |n, byte| (n << 8) | *byte as u128)
                .to_string()
        } else {
            value(&bytes, column.spec)
        };
        writeln!(
            out,
            "  {}{} / {} B / {} values / {sample}",
            if index < flow.scope_count {
                "scope."
            } else {
                ""
            },
            column.name,
            column.spec.length,
            column.values.count()
        )?;
        if column.values.count() > 1 {
            bytes.clear();
            column.values.append(column.values.count() - 1, &mut bytes);
            let last = if index < flow.scope_count {
                bytes
                    .iter()
                    .fold(0u128, |n, byte| (n << 8) | *byte as u128)
                    .to_string()
            } else {
                value(&bytes, column.spec)
            };
            writeln!(out, "    Last value: {last}")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use neural_ensemble_telemetry_fabric_luminal_oracle_wizard9::config::{self, Config};

    #[test]
    fn samples_decode_encoded_values_and_distinguish_scopes() {
        for (file, expected) in [
            (
                include_str!("../examples/basic.json"),
                vec!["192.0.2.1", "SYN|ACK (0x12)"],
            ),
            (
                include_str!("../examples/options.json"),
                vec!["Options", "scope.INTERFACE / 4 B / 1 values / 1"],
            ),
            (
                include_str!("../examples/convenience.json"),
                vec![
                    "2001:db8::",
                    "type=8 code=0",
                    "label=24000 tc=0 bos=1",
                    "2 values",
                ],
            ),
            (
                include_str!("../examples/mpls.json"),
                vec!["0:65000:100", "label=16000 tc=3 bos=0"],
            ),
            (include_str!("../examples/asn.json"), vec!["AS65000"]),
        ] {
            let config: Config = serde_json::from_str(file).unwrap();
            let mut output = Vec::new();
            for item in config::validate(&config).unwrap() {
                flow(&item, &mut output).unwrap();
            }
            let output = String::from_utf8(output).unwrap();
            for text in expected {
                assert!(output.contains(text), "{text}: {output}");
            }
        }
        for (rd, expected) in [("192.0.2.1:5", "1:192.0.2.1:5"), ("65546:7", "2:65546:7")] {
            let config: Config = serde_json::from_value(serde_json::json!({"flows":[{"template_id":256,"fields":[{"name":"MPLS_VPN_RD","value":rd}]}]})).unwrap();
            let mut output = Vec::new();
            flow(&config::validate(&config).unwrap()[0], &mut output).unwrap();
            assert!(String::from_utf8(output).unwrap().contains(expected));
        }
    }
}
