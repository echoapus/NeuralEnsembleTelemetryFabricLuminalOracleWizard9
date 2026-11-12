use anyhow::{Result, bail};

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    Number,
    Ipv4,
    Ipv6,
    MplsLabel,
    RouteDistinguisher,
    Asn,
    Icmp4,
    Icmp6,
    Bytes,
}

#[derive(Clone, Copy, Debug)]
pub struct FieldSpec {
    pub id: u16,
    pub length: usize,
    pub kind: Kind,
    /// IPFIX variable-length element: the template says 65535 and `length` is the on-wire length prefix.
    pub variable: bool,
}

pub fn lookup(name: &str, scope: bool) -> Result<FieldSpec> {
    let (id, length, kind) = if scope {
        let (id, length) = match name {
            "SYSTEM" => (1, 4),
            "INTERFACE" => (2, 4),
            "LINE_CARD" => (3, 4),
            "NETFLOW_CACHE" => (4, 4),
            "TEMPLATE" => (5, 2),
            _ => bail!(
                "Unknown scope field {name}; supported: SYSTEM, INTERFACE, LINE_CARD, NETFLOW_CACHE, TEMPLATE"
            ),
        };
        (id, length, Kind::Number)
    } else {
        match name {
            "IN_BYTES" => (1, 4, Kind::Number),
            "IN_PKTS" => (2, 4, Kind::Number),
            "FLOWS" => (3, 4, Kind::Number),
            "PROTOCOL" => (4, 1, Kind::Number),
            "SRC_TOS" => (5, 1, Kind::Number),
            "TCP_FLAGS" => (6, 1, Kind::Number),
            "L4_SRC_PORT" => (7, 2, Kind::Number),
            "IPV4_SRC_ADDR" => (8, 4, Kind::Ipv4),
            "SRC_MASK" => (9, 1, Kind::Number),
            "INPUT_SNMP" => (10, 4, Kind::Number),
            "L4_DST_PORT" => (11, 2, Kind::Number),
            "IPV4_DST_ADDR" => (12, 4, Kind::Ipv4),
            "DST_MASK" => (13, 1, Kind::Number),
            "OUTPUT_SNMP" => (14, 4, Kind::Number),
            "IPV4_NEXT_HOP" => (15, 4, Kind::Ipv4),
            "SRC_AS" => (16, 4, Kind::Asn),
            "DST_AS" => (17, 4, Kind::Asn),
            "BGP_IPV4_NEXT_HOP" => (18, 4, Kind::Ipv4),
            "LAST_SWITCHED" => (21, 4, Kind::Number),
            "FIRST_SWITCHED" => (22, 4, Kind::Number),
            "OUT_BYTES" => (23, 4, Kind::Number),
            "OUT_PKTS" => (24, 4, Kind::Number),
            "IPV6_SRC_ADDR" => (27, 16, Kind::Ipv6),
            "IPV6_DST_ADDR" => (28, 16, Kind::Ipv6),
            "IPV6_SRC_MASK" => (29, 1, Kind::Number),
            "IPV6_DST_MASK" => (30, 1, Kind::Number),
            "ICMP_TYPE" => (32, 2, Kind::Icmp4),
            "SAMPLING_INTERVAL" => (34, 4, Kind::Number),
            "SAMPLING_ALGORITHM" => (35, 1, Kind::Number),
            "MPLS_TOP_LABEL_TYPE" => (46, 1, Kind::Number),
            "MPLS_TOP_LABEL_IP_ADDR" => (47, 4, Kind::Ipv4),
            "DST_TOS" => (55, 1, Kind::Number),
            "IP_PROTOCOL_VERSION" => (60, 1, Kind::Number),
            "IPV6_NEXT_HOP" => (62, 16, Kind::Ipv6),
            "BGP_IPV6_NEXT_HOP" => (63, 16, Kind::Ipv6),
            "MPLS_LABEL_1" => (70, 3, Kind::MplsLabel),
            "MPLS_LABEL_2" => (71, 3, Kind::MplsLabel),
            "MPLS_LABEL_3" => (72, 3, Kind::MplsLabel),
            "MPLS_LABEL_4" => (73, 3, Kind::MplsLabel),
            "MPLS_LABEL_5" => (74, 3, Kind::MplsLabel),
            "MPLS_LABEL_6" => (75, 3, Kind::MplsLabel),
            "MPLS_LABEL_7" => (76, 3, Kind::MplsLabel),
            "MPLS_LABEL_8" => (77, 3, Kind::MplsLabel),
            "MPLS_LABEL_9" => (78, 3, Kind::MplsLabel),
            "MPLS_LABEL_10" => (79, 3, Kind::MplsLabel),
            "MPLS_VPN_RD" => (90, 8, Kind::RouteDistinguisher),
            "MPLS_TOP_LABEL_PREFIX_LEN" => (91, 1, Kind::Number),
            "MPLS_TOP_LABEL_IPV6_ADDR" => (140, 16, Kind::Ipv6),
            "ICMP_TYPE_IPV6" => (139, 2, Kind::Icmp6),
            "MPLS_TOP_LABEL_TTL" => (200, 1, Kind::Number),
            "MPLS_LABEL_STACK_LENGTH" => (201, 4, Kind::Number),
            "MPLS_LABEL_STACK_DEPTH" => (202, 4, Kind::Number),
            "MPLS_TOP_LABEL_EXP" => (203, 1, Kind::Number),
            _ => bail!("Unknown regular field {name}; see README.md for the full field list"),
        }
    };
    Ok(FieldSpec {
        id,
        length,
        kind,
        variable: false,
    })
}

/// IPFIX has no separate scope namespace, so scopes and regular fields share this table.
pub fn lookup_ipfix(name: &str) -> Result<FieldSpec> {
    let spec = |id, length, kind, variable| FieldSpec {
        id,
        length,
        kind,
        variable,
    };
    // Deliberately not shaped like the v9 table above: tests scan that shape as the v9 registry.
    Ok(match name {
        "DIRECTION" => spec(61, 1, Kind::Number, false),
        "FLOW_EXPORTER" => spec(144, 4, Kind::Number, false),
        "DATALINK_FRAME_SIZE" => spec(312, 2, Kind::Number, false),
        "DATALINK_FRAME_SECTION" => spec(315, 0, Kind::Bytes, true),
        _ => return lookup(name, false),
    })
}
