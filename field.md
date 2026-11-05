# Supported fields and value syntax

[English (en-US)](field.md) | [繁體中文 (zh-TW)](field.zh-TW.md)

[Field/value support matrix (English)](FIELD-SUPPORT.md)

This reference follows the current [field definitions](src/fields.rs) and [JSON validator](src/config.rs), covering **53 regular fields and five Options scopes**. See the [README](README.md) for workflows and CLI options.

Multiple flows may share a `template_id` when scope and field structures match. Values expand independently without cross-flow combinations; see [shared templates](README.md#share-a-template-across-flows).

Put regular fields in `flows[].fields` as `{"name":"FIELD_NAME","value":value}`. Names are case-insensitive; this reference uses uppercase. All numeric ranges are inclusive. Numbers must be JSON integers; text such as IP addresses, flag names, and RDs must be double-quoted.

```json
{
  "source_id": 42,
  "flows": [{
    "template_id": 256,
    "fields": [
      {"name":"IPV4_SRC_ADDR","value":"192.0.2.1"},
      {"name":"IPV4_DST_ADDR","value":"198.51.100.2"},
      {"name":"PROTOCOL","value":6},
      {"name":"TCP_FLAGS","value":"SYN|ACK"},
      {"name":"IN_PKTS","value":10},
      {"name":"IN_BYTES","value":1000}
    ]
  }]
}
```

## Regular fields

An array is a nonempty list of single values, such as `[80,443]`. A range is a start/end object. Prefixes apply only to IP fields. random_between and random_file resample every record on every send. See the sections below for syntax. Type ID and Bytes refer to NetFlow v9 encoding, not JSON string length.

| Field name | Type ID | Bytes | Single value format/range | Other supported formats | Example value |
|---|---:|---:|---|---|---|
| `IN_BYTES` | 1 | 4 | Integer `0..=4294967295` | Array, range, random_between | `1000` |
| `IN_PKTS` | 2 | 4 | Integer `0..=4294967295` | Array, range, random_between | `10` |
| `FLOWS` | 3 | 4 | Integer `0..=4294967295` | Array, range | `1` |
| `PROTOCOL` | 4 | 1 | Integer `0..=255` | Array, range | `6` |
| `SRC_TOS` | 5 | 1 | Integer `0..=255` | Array, range | `0` |
| `TCP_FLAGS` | 6 | 1 | Integer `0..=255`, flag string, or hexadecimal string | Array, range | `"SYN\|ACK"` |
| `L4_SRC_PORT` | 7 | 2 | Integer `0..=65535` | Array, range, random_between | `12345` |
| `IPV4_SRC_ADDR` | 8 | 4 | IPv4 address string | Array, range, prefix, random_file | `"192.0.2.1"` |
| `SRC_MASK` | 9 | 1 | Integer `0..=32` | Array, range | `24` |
| `INPUT_SNMP` | 10 | 4 | Integer `0..=4294967295` | Array, range | `70001` |
| `L4_DST_PORT` | 11 | 2 | Integer `0..=65535` | Array, range, random_between | `443` |
| `IPV4_DST_ADDR` | 12 | 4 | IPv4 address string | Array, range, prefix, random_file | `"198.51.100.2"` |
| `DST_MASK` | 13 | 1 | Integer `0..=32` | Array, range | `24` |
| `OUTPUT_SNMP` | 14 | 4 | Integer `0..=4294967295` | Array, range | `70002` |
| `IPV4_NEXT_HOP` | 15 | 4 | IPv4 address string | Array, range, prefix, random_file | `"192.0.2.254"` |
| `SRC_AS` | 16 | 4 | Integer `0..=4294967295` or ASN string | Array, range | `"AS1.10"` |
| `DST_AS` | 17 | 4 | Integer `0..=4294967295` or ASN string | Array, range | `4200000000` |
| `BGP_IPV4_NEXT_HOP` | 18 | 4 | IPv4 address string | Array, range, prefix, random_file | `"198.51.100.254"` |
| `LAST_SWITCHED` | 21 | 4 | Integer `0..=4294967295`; exporter uptime in milliseconds | Array, range | `1000` |
| `FIRST_SWITCHED` | 22 | 4 | Integer `0..=4294967295`; exporter uptime in milliseconds | Array, range | `0` |
| `OUT_BYTES` | 23 | 4 | Integer `0..=4294967295` | Array, range | `3000` |
| `OUT_PKTS` | 24 | 4 | Integer `0..=4294967295` | Array, range | `20` |
| `IPV6_SRC_ADDR` | 27 | 16 | IPv6 address string | Array, range, prefix, random_file | `"2001:db8::1"` |
| `IPV6_DST_ADDR` | 28 | 16 | IPv6 address string | Array, range, prefix, random_file | `"2001:db8:1::2"` |
| `IPV6_SRC_MASK` | 29 | 1 | Integer `0..=128` | Array, range | `64` |
| `IPV6_DST_MASK` | 30 | 1 | Integer `0..=128` | Array, range | `64` |
| `ICMP_TYPE` | 32 | 2 | Integer `0..=65535`, type name, or `{type,code}` | Array, range | `{"type":"echo-request","code":0}` |
| `SAMPLING_INTERVAL` | 34 | 4 | Integer `0..=4294967295` | Array, range | `1` |
| `SAMPLING_ALGORITHM` | 35 | 1 | Integer `0..=255` | Array, range | `1` |
| `MPLS_TOP_LABEL_TYPE` | 46 | 1 | Integer `0..=255` | Array, range | `0` |
| `MPLS_TOP_LABEL_IP_ADDR` | 47 | 4 | IPv4 address string | Array, range, prefix, random_file | `"192.0.2.254"` |
| `DST_TOS` | 55 | 1 | Integer `0..=255` | Array, range | `0` |
| `IP_PROTOCOL_VERSION` | 60 | 1 | Integer `4` or `6` | Array, range | `4` |
| `IPV6_NEXT_HOP` | 62 | 16 | IPv6 address string | Array, range, prefix, random_file | `"2001:db8::ffff"` |
| `BGP_IPV6_NEXT_HOP` | 63 | 16 | IPv6 address string | Array, range, prefix, random_file | `"2001:db8:1::ffff"` |
| `MPLS_LABEL_1` | 70 | 3 | Integer label `0..=1048575` or `{label,tc,bos}` | Array, range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_LABEL_2` | 71 | 3 | Integer label `0..=1048575` or `{label,tc,bos}` | Array, range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_LABEL_3` | 72 | 3 | Integer label `0..=1048575` or `{label,tc,bos}` | Array, range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_LABEL_4` | 73 | 3 | Integer label `0..=1048575` or `{label,tc,bos}` | Array, range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_LABEL_5` | 74 | 3 | Integer label `0..=1048575` or `{label,tc,bos}` | Array, range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_LABEL_6` | 75 | 3 | Integer label `0..=1048575` or `{label,tc,bos}` | Array, range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_LABEL_7` | 76 | 3 | Integer label `0..=1048575` or `{label,tc,bos}` | Array, range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_LABEL_8` | 77 | 3 | Integer label `0..=1048575` or `{label,tc,bos}` | Array, range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_LABEL_9` | 78 | 3 | Integer label `0..=1048575` or `{label,tc,bos}` | Array, range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_LABEL_10` | 79 | 3 | Integer label `0..=1048575` or `{label,tc,bos}` | Array, range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_VPN_RD` | 90 | 8 | RD string, Type 0/1/2 | Array | `"65000:100"` |
| `MPLS_TOP_LABEL_PREFIX_LEN` | 91 | 1 | Integer `0..=128` | Array, range | `24` |
| `ICMP_TYPE_IPV6` | 139 | 2 | Integer `0..=65535`, type name, or `{type,code}` | Array, range | `{"type":128,"code":0}` |
| `MPLS_TOP_LABEL_IPV6_ADDR` | 140 | 16 | IPv6 address string | Array, range, prefix, random_file | `"2001:db8:2::"` |
| `MPLS_TOP_LABEL_TTL` | 200 | 1 | Integer `0..=255` | Array, range | `64` |
| `MPLS_LABEL_STACK_LENGTH` | 201 | 4 | Integer `0..=4294967295`; bytes in the original stack | Array, range | `8` |
| `MPLS_LABEL_STACK_DEPTH` | 202 | 4 | Integer `0..=4294967295`; stack layers | Array, range | `2` |
| `MPLS_TOP_LABEL_EXP` | 203 | 1 | Integer `0..=7` | Array, range | `3` |

## Options scope

These names belong only in `flows[].scope`, not regular `fields`. Scope and regular fields use separate Type ID namespaces. A nonempty scope creates an Options template. Each scope accepts integers, nonempty integer arrays, and ranges.

| Scope name | Type ID | Bytes | Value range | Example value |
|---|---:|---:|---|---|
| `SYSTEM` | 1 | 4 | Integer `0..=4294967295` | `1` |
| `INTERFACE` | 2 | 4 | Integer `0..=4294967295`; ifIndex | `70001` |
| `LINE_CARD` | 3 | 4 | Integer `0..=4294967295` | `2` |
| `NETFLOW_CACHE` | 4 | 4 | Integer `0..=4294967295` | `1` |
| `TEMPLATE` | 5 | 2 | Integer `0..=65535` | `256` |

Scope `TEMPLATE` is a field value; its range differs from flow `template_id` (256–65535). See [options.json](examples/options.json) for a complete example.

## Arrays, ranges, and IP prefixes

Each example is a single entry that can be added to `fields`:

```json
{"name":"L4_DST_PORT","value":[80,443]}
```

```json
{"name":"L4_DST_PORT","value":{"start":10000,"end":10005,"step":2}}
```

Range step defaults to 1 and must be a positive integer; the CLI maximum is `18446744073709551615`. The example produces 10000, 10002, and 10004. Endpoints use the single-value formats accepted by that field. MPLS label endpoints must be label integers; RDs do not support ranges; each expanded IP_PROTOCOL_VERSION value must be 4 or 6.

```json
{"name":"IPV4_SRC_ADDR","value":"192.0.2.0/24"}
```

```json
{"name":"IPV4_DST_ADDR","value":{"prefix":"198.51.100.0/24","hosts_only":true}}
```

```json
{"name":"IPV6_SRC_ADDR","value":"2001:db8::/126"}
```

IPv4 prefixes include network and broadcast addresses by default, so `/24` contains 256 addresses. hosts_only excludes those endpoints, except `/31` and `/32` retain all addresses. IPv6 `/126` contains four addresses and rejects hosts_only=true. Prefixes clear host bits and do not add mask fields.

Multiple field values form a Cartesian product, not positional pairs. Source `/24` × destination `/24` yields 65,536 records; each source combines with every destination. Scope values also participate in multiplication. All flows combined must not exceed **500,000 records**. Put prefix/range/random objects directly in value, not nested inside arrays for expansion.

## Random values

Only `L4_SRC_PORT`, `L4_DST_PORT`, `IN_PKTS`, and `IN_BYTES` support random_between, within each field's integer bounds:

```json
{"name":"IN_PKTS","value":{"random_between":[10,100]}}
```

Bounds are inclusive and min≤max. Every record is resampled on each send; duplicates are allowed. Each field counts as one configured value. Fields are sampled independently without enforcing a PKTS/BYTES ratio. OUT_PKTS/OUT_BYTES do not support random values. `random_between` is the only random numeric syntax; the old `random`/`min`/`max` object is rejected.

Every IP field accepts `random_file`, which draws one address per record from a text file on every send:

```json
{"name":"IPV4_SRC_ADDR","value":{"random_file":"random-ips.txt"}}
```

One address per line; blank lines and `#` lines are skipped. Entries must be plain IP addresses; CIDR prefixes are rejected with the file path and line number. The address version must match the field. Relative paths resolve against the JSON file's folder. The file may be at most 8 MiB and counts as one configured value.

## TCP flags

These values are equivalent and all mean SYN+ACK:

| value | Format |
|---|---|
| `18` | JSON integer |
| `"0x12"` | Hexadecimal string |
| `"SA"` | Short flag letters |
| `"SYN\|ACK"` | Full names separated by `\|` |
| `"syn \| ack"` | Case-insensitive, with whitespace around separators |

| Letter | Full name | Bit value |
|---|---|---:|
| F | FIN | 1 |
| S | SYN | 2 |
| R | RST | 4 |
| P | PSH | 8 |
| A | ACK | 16 |
| U | URG | 32 |
| E | ECE | 64 |
| C | CWR | 128 |

Repeated flags are combined with OR. Do not concatenate full names into `"SYNACK"`. Empty strings, empty separator entries such as `"SYN|"`, and decimal strings such as `"18"` are rejected.

## ASNs, ICMP, and MPLS

ASN dots and RD colons allow surrounding whitespace, such as `"AS1 . 10"` and `"65000 : 100"`.

| Type | Example values | Limits |
|---|---|---|
| ASN | `65000`, `"AS65000"`, `"1.10"`, `"AS1.10"` | 0–4294967295; asdot parts each 0–65535; 1.10 = 65546 |
| ICMPv4 | `2048`, `"echo-request"`, `{"type":8,"code":0}` | Integer means type×256+code; object type/code each 0–255; code defaults to 0 |
| ICMPv6 | `32768`, `"echo-request"`, `{"type":128,"code":0}` | Use ICMP_TYPE_IPV6; PROTOCOL is not set automatically |
| MPLS label | `16000`, `{"label":16000,"tc":3,"bos":true}` | label 0–1048575; tc 0–7; bos boolean; defaults tc=0, bos=false |
| RD Type 0 | `"65000:100"`, `"0:65000:100"` | 16-bit ASN + 32-bit number |
| RD Type 1 | `"192.0.2.1:100"`, `"1:192.0.2.1:100"` | IPv4 + 16-bit number |
| RD Type 2 | `"4200000000:100"`, `"2:65000:100"` | 32-bit ASN + 16-bit number |

| ICMP type name | IPv4 | IPv6 |
|---|---:|---:|
| echo-request | 8 | 128 |
| echo-reply | 0 | 129 |
| destination-unreachable | 3 | 1 |
| time-exceeded | 11 | 3 |
| parameter-problem | 12 | 4 |
| packet-too-big | Not accepted | 2 |

ICMP type accepts an integer or one of these names. Set PROTOCOL=1 for IPv4 ICMP and PROTOCOL=58 for IPv6 ICMP. CLI ICMP range endpoints pass through single-value encoding; HTML accepts only combined integer endpoints.

Set MPLS labels and stack metadata explicitly in `fields`; flow-level `mpls_stack` is no longer accepted. MPLS labels export as 3 bytes without TTL. Manual fields do not fill in BoS or stack metadata automatically. Original MPLS stacks use four bytes per layer, so two layers have MPLS_LABEL_STACK_LENGTH=8. RDs are fixed at 8 bytes, and ASN administrators support AS prefixes/asdot. Without an explicit type, an IPv4 administrator selects Type 1, ASN≤65535 selects Type 0, and larger ASNs select Type 2. See [MPLS configuration](README.md#mpls-configuration).

## Counters and expected collector average rates per minute

`IN_PKTS`/`OUT_PKTS` are input/output packet counts inside records; `IN_BYTES`/`OUT_BYTES` are byte counts. They do not accumulate automatically on every send. bps uses BYTES × 8; pps uses PKTS. Per-minute totals divided by 60 seconds express average rates:

```sh
./target/release/nfgen preview examples/shared-template.json --kind data --rate 20
```

The default rate is 20. Both example flows share Template 256; expected combined output:

```text
Template         IN bps         IN pps        OUT bps        OUT pps
     256      320000.00         300.00              -              -
```

Each regular template occupies one row with IN/OUT bps and pps, bandwidth first. Different IDs stay separate; shared IDs are summed. Rate is the data packets/s shared by the configuration. Each group uses template counter totals per cycle × rate ÷ all data packets per cycle, with BYTES multiplied by 8 for bps. Options do not estimate simulated traffic, but their data packets count in the denominator. Random counters use the mean of the bounds.

If any regular flow in a template lacks PKTS/BYTES, only that direction's corresponding cell shows `-`. Missing counters are not zero and do not affect other templates. Preview limit/template-id restrict packet previews without narrowing the full estimates. Send shows the same groups before sending; periodic statistics do not recalculate per-minute estimates.

These estimates assume continuous sending, complete receipt, aggregation by receipt time, and no sampling multiplier. Short runs still show the same long-term extrapolation. This is neither actual UDP throughput nor an original flow rate calculated over LAST_SWITCHED−FIRST_SWITCHED. Those fields are configured uptime milliseconds and do not update on repeats; equal values mean zero record duration, separate from CLI `--duration 0` for unlimited sending. See the [README](README.md#continuous-sending-and-logs) for options, logs, and exit codes.

## JSON and HTML differences

All four JSON counters store actual values, without formulas. HTML can calculate IN_BYTES/OUT_BYTES as same-direction PKTS × average packet size, but requires a single PKTS value and exports an ordinary BYTES integer. Calculation settings are retained only in drafts; see [IN/OUT counters](README.md#inout-counters).

This reference describes formats accepted by CLI JSON. HTML does not import `random_file`, ICMP ranges with named/object endpoints, or integers above JavaScript's safe range. Regular field values do not change actual UDP source/destination IPs, ports, or sending interfaces; CLI options control those.
