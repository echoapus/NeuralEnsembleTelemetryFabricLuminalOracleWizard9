# Field and value support matrix

This table covers the current **53 regular fields and five Options scopes**. It describes JSON accepted by the CLI, based on [field definitions](src/fields.rs) and [configuration validation](src/config.rs). See the [full field reference](field.md) for encoding details.

Use regular fields in `flows[].fields`, for example `{"name":"L4_DST_PORT","value":443}`. Field names are case-insensitive. Numeric bounds include both ends; numbers must be nonnegative JSON integers.

## Regular fields

**Yes** means the format is supported; **—** means it is rejected. Every array must contain valid single values for that field. A range must produce only valid values and stay within the expansion limit.

| Field | ID | Single value type / bounds | Single value example | Array | Range | IP prefix | `random_between` | `random_file` |
|---|---:|---|---|:---:|:---:|:---:|:---:|:---:|
| `IN_BYTES` | 1 | Integer `0..=4294967295` | `1000` | Yes | Yes | — | Yes | — |
| `IN_PKTS` | 2 | Integer `0..=4294967295` | `10` | Yes | Yes | — | Yes | — |
| `FLOWS` | 3 | Integer `0..=4294967295` | `1` | Yes | Yes | — | — | — |
| `PROTOCOL` | 4 | Integer `0..=255` | `6` | Yes | Yes | — | — | — |
| `SRC_TOS` | 5 | Integer `0..=255` | `0` | Yes | Yes | — | — | — |
| `TCP_FLAGS` | 6 | Integer `0..=255`, flag string, or hexadecimal string | `"SYN\|ACK"` | Yes | Yes | — | — | — |
| `L4_SRC_PORT` | 7 | Integer `0..=65535` | `12345` | Yes | Yes | — | Yes | — |
| `IPV4_SRC_ADDR` | 8 | IPv4 address string | `"192.0.2.1"` | Yes | Yes | Yes | — | Yes |
| `SRC_MASK` | 9 | Integer `0..=32` | `24` | Yes | Yes | — | — | — |
| `INPUT_SNMP` | 10 | Integer `0..=4294967295` | `70001` | Yes | Yes | — | — | — |
| `L4_DST_PORT` | 11 | Integer `0..=65535` | `443` | Yes | Yes | — | Yes | — |
| `IPV4_DST_ADDR` | 12 | IPv4 address string | `"198.51.100.2"` | Yes | Yes | Yes | — | Yes |
| `DST_MASK` | 13 | Integer `0..=32` | `24` | Yes | Yes | — | — | — |
| `OUTPUT_SNMP` | 14 | Integer `0..=4294967295` | `70002` | Yes | Yes | — | — | — |
| `IPV4_NEXT_HOP` | 15 | IPv4 address string | `"192.0.2.254"` | Yes | Yes | Yes | — | Yes |
| `SRC_AS` | 16 | Integer `0..=4294967295` or ASN string | `"AS1.10"` | Yes | Yes | — | — | — |
| `DST_AS` | 17 | Integer `0..=4294967295` or ASN string | `4200000000` | Yes | Yes | — | — | — |
| `BGP_IPV4_NEXT_HOP` | 18 | IPv4 address string | `"198.51.100.254"` | Yes | Yes | Yes | — | Yes |
| `LAST_SWITCHED` | 21 | Integer `0..=4294967295`; exporter uptime in milliseconds | `1000` | Yes | Yes | — | — | — |
| `FIRST_SWITCHED` | 22 | Integer `0..=4294967295`; exporter uptime in milliseconds | `0` | Yes | Yes | — | — | — |
| `OUT_BYTES` | 23 | Integer `0..=4294967295` | `3000` | Yes | Yes | — | — | — |
| `OUT_PKTS` | 24 | Integer `0..=4294967295` | `20` | Yes | Yes | — | — | — |
| `IPV6_SRC_ADDR` | 27 | IPv6 address string | `"2001:db8::1"` | Yes | Yes | Yes | — | Yes |
| `IPV6_DST_ADDR` | 28 | IPv6 address string | `"2001:db8:1::2"` | Yes | Yes | Yes | — | Yes |
| `IPV6_SRC_MASK` | 29 | Integer `0..=128` | `64` | Yes | Yes | — | — | — |
| `IPV6_DST_MASK` | 30 | Integer `0..=128` | `64` | Yes | Yes | — | — | — |
| `ICMP_TYPE` | 32 | Integer `0..=65535`, type name, or `{type,code}` | `{"type":"echo-request","code":0}` | Yes | Yes | — | — | — |
| `SAMPLING_INTERVAL` | 34 | Integer `0..=4294967295` | `1` | Yes | Yes | — | — | — |
| `SAMPLING_ALGORITHM` | 35 | Integer `0..=255` | `1` | Yes | Yes | — | — | — |
| `MPLS_TOP_LABEL_TYPE` | 46 | Integer `0..=255` | `0` | Yes | Yes | — | — | — |
| `MPLS_TOP_LABEL_IP_ADDR` | 47 | IPv4 address string | `"192.0.2.254"` | Yes | Yes | Yes | — | Yes |
| `DST_TOS` | 55 | Integer `0..=255` | `0` | Yes | Yes | — | — | — |
| `IP_PROTOCOL_VERSION` | 60 | Integer `4` or `6` | `4` | Yes | Yes | — | — | — |
| `IPV6_NEXT_HOP` | 62 | IPv6 address string | `"2001:db8::ffff"` | Yes | Yes | Yes | — | Yes |
| `BGP_IPV6_NEXT_HOP` | 63 | IPv6 address string | `"2001:db8:1::ffff"` | Yes | Yes | Yes | — | Yes |
| `MPLS_LABEL_1` | 70 | Integer label `0..=1048575` or `{label,tc,bos}` | `{"label":16000,"tc":3,"bos":true}` | Yes | Yes | — | — | — |
| `MPLS_LABEL_2` | 71 | Integer label `0..=1048575` or `{label,tc,bos}` | `{"label":16000,"tc":3,"bos":true}` | Yes | Yes | — | — | — |
| `MPLS_LABEL_3` | 72 | Integer label `0..=1048575` or `{label,tc,bos}` | `{"label":16000,"tc":3,"bos":true}` | Yes | Yes | — | — | — |
| `MPLS_LABEL_4` | 73 | Integer label `0..=1048575` or `{label,tc,bos}` | `{"label":16000,"tc":3,"bos":true}` | Yes | Yes | — | — | — |
| `MPLS_LABEL_5` | 74 | Integer label `0..=1048575` or `{label,tc,bos}` | `{"label":16000,"tc":3,"bos":true}` | Yes | Yes | — | — | — |
| `MPLS_LABEL_6` | 75 | Integer label `0..=1048575` or `{label,tc,bos}` | `{"label":16000,"tc":3,"bos":true}` | Yes | Yes | — | — | — |
| `MPLS_LABEL_7` | 76 | Integer label `0..=1048575` or `{label,tc,bos}` | `{"label":16000,"tc":3,"bos":true}` | Yes | Yes | — | — | — |
| `MPLS_LABEL_8` | 77 | Integer label `0..=1048575` or `{label,tc,bos}` | `{"label":16000,"tc":3,"bos":true}` | Yes | Yes | — | — | — |
| `MPLS_LABEL_9` | 78 | Integer label `0..=1048575` or `{label,tc,bos}` | `{"label":16000,"tc":3,"bos":true}` | Yes | Yes | — | — | — |
| `MPLS_LABEL_10` | 79 | Integer label `0..=1048575` or `{label,tc,bos}` | `{"label":16000,"tc":3,"bos":true}` | Yes | Yes | — | — | — |
| `MPLS_VPN_RD` | 90 | RD string, Type 0/1/2 | `"65000:100"` | Yes | — | — | — | — |
| `MPLS_TOP_LABEL_PREFIX_LEN` | 91 | Integer `0..=128` | `24` | Yes | Yes | — | — | — |
| `ICMP_TYPE_IPV6` | 139 | Integer `0..=65535`, type name, or `{type,code}` | `{"type":128,"code":0}` | Yes | Yes | — | — | — |
| `MPLS_TOP_LABEL_IPV6_ADDR` | 140 | IPv6 address string | `"2001:db8:2::"` | Yes | Yes | Yes | — | Yes |
| `MPLS_TOP_LABEL_TTL` | 200 | Integer `0..=255` | `64` | Yes | Yes | — | — | — |
| `MPLS_LABEL_STACK_LENGTH` | 201 | Integer `0..=4294967295`; bytes in the original stack | `8` | Yes | Yes | — | — | — |
| `MPLS_LABEL_STACK_DEPTH` | 202 | Integer `0..=4294967295`; stack layers | `2` | Yes | Yes | — | — | — |
| `MPLS_TOP_LABEL_EXP` | 203 | Integer `0..=7` | `3` | Yes | Yes | — | — | — |

## Options scopes

These names belong in `flows[].scope`, rather than `fields`. A nonempty scope creates an Options template; the flow still needs at least one regular field. Scope IDs use a separate namespace from regular field IDs.

| Scope | ID | Single value bounds | Single value example | Array | Range | IP prefix | `random_between` | `random_file` |
|---|---:|---|---|:---:|:---:|:---:|:---:|:---:|
| `SYSTEM` | 1 | Integer `0..=4294967295` | `1` | Yes | Yes | — | — | — |
| `INTERFACE` | 2 | Integer `0..=4294967295` | `70001` | Yes | Yes | — | — | — |
| `LINE_CARD` | 3 | Integer `0..=4294967295` | `2` | Yes | Yes | — | — | — |
| `NETFLOW_CACHE` | 4 | Integer `0..=4294967295` | `1` | Yes | Yes | — | — | — |
| `TEMPLATE` | 5 | Integer `0..=65535` | `256` | Yes | Yes | — | — | — |

The `TEMPLATE` scope value may be 0–65535; a flow's `template_id` must be 256–65535.

## Value format examples

Each example below is a complete field entry for a flow's `fields` array.

| Format | Example | Meaning |
|---|---|---|
| Single value | `{"name":"L4_DST_PORT","value":443}` | One configured value |
| Array | `{"name":"L4_DST_PORT","value":[80,443]}` | Two configured values |
| Range | `{"name":"L4_DST_PORT","value":{"start":10000,"end":10005,"step":2}}` | 10000, 10002, 10004; step defaults to 1 |
| IPv4 prefix | `{"name":"IPV4_SRC_ADDR","value":"192.0.2.0/30"}` | Four addresses, including network and broadcast addresses |
| IPv4 hosts only | `{"name":"IPV4_SRC_ADDR","value":{"prefix":"192.0.2.0/30","hosts_only":true}}` | Two addresses; /31 and /32 retain all addresses |
| IPv6 prefix | `{"name":"IPV6_SRC_ADDR","value":"2001:db8::/126"}` | Four addresses; `hosts_only:true` is rejected |
| Random number | `{"name":"IN_PKTS","value":{"random_between":[10,100]}}` | One value, redrawn per record on each send; both bounds are inclusive |
| Random IP file | `{"name":"IPV4_SRC_ADDR","value":{"random_file":"random-ips.txt"}}` | One address redrawn per record on each send from the file |

Arrays and ranges across fields and scopes form a Cartesian product, rather than being paired by position. The total across all flows must not exceed **500,000 configured records**. Random formats each count as one configured value; repeated draws may return the same value. Range, prefix, and random objects must be the outermost `value`, not nested inside an array.

## Field-specific rules

| Field or format | Rules |
|---|---|
| `TCP_FLAGS` | Integer 0–255, hexadecimal string, short letters F/S/R/P/A/U/E/C, or full names separated by `\|`; for example `18`, `"0x12"`, `"SA"`, `"SYN\|ACK"` |
| `SRC_AS`, `DST_AS` | Integer or decimal/asplain, AS-prefixed, or asdot string; `"AS1.10"` is 65546. Both asdot parts are 0–65535. Whitespace around the dot is allowed. |
| `ICMP_TYPE`, `ICMP_TYPE_IPV6` | A bare integer is type × 256 + code. Object `type` and `code` are 0–255; `type` may also be a supported name, and omitted `code` defaults to 0. See [ICMP names](field.md#asns-icmp-and-mpls). |
| `MPLS_LABEL_1`–`MPLS_LABEL_10` | Label 0–1048575; object `tc` is 0–7 and `bos` is boolean. Defaults are tc=0 and bos=false. Range endpoints must be label integers. |
| `MPLS_VPN_RD` | Type 0: 16-bit ASN + 32-bit number; Type 1: IPv4 + 16-bit number; Type 2: 32-bit ASN + 16-bit number. Examples: `"65000:100"`, `"192.0.2.1:100"`, `"4200000000:100"`. An explicit type prefix such as `"2:65000:100"` is accepted. Whitespace around colons is allowed. No ranges. |
| `IP_PROTOCOL_VERSION` | Every expanded value must be 4 or 6; `{"start":4,"end":6,"step":2}` is valid, but step=1 is not. |
| MPLS stack metadata | Set labels, BoS, depth, length, and EXP explicitly in `fields`; no automatic stack expansion. Original stack length is four bytes per layer. |
| IP prefix | Host bits are cleared. Prefixes do not automatically add mask fields. Expansion must fit the record limit. |
| Range | End must be at least start. Step must be a positive integer, at most 18446744073709551615 in the CLI. IP endpoints must match the field's IP version. |
| `random_between` | Supported only for the four marked fields. Both bounds are required, min ≤ max, within the field's bounds. Equal bounds give a fixed value. |
| `random_file` | UTF-8, at most 8 MiB, one plain IP per line matching the field's IP version. Blank lines and lines starting with `#` are skipped. CIDR entries are rejected with file path and line number. Relative paths resolve against the configuration file's directory. |

## CLI and HTML differences

The [HTML builder](config-builder.html) supports the marked formats except `random_file`. It accepts only combined integer endpoints for ICMP ranges, and range steps up to **9007199254740991**. It rejects JSON integers that JavaScript cannot represent exactly.

HTML additionally offers **Calculate from PKTS** for `IN_BYTES` and `OUT_BYTES`. This requires a single corresponding PKTS value; export stores an ordinary integer, not a formula. Calculation settings survive only in browser drafts.

The old `{"random":true}` syntax and flow-level `mpls_stack` are rejected by both interfaces. See [Updating older configurations](README.md#updating-older-configurations).

## IPFIX-only fields

These fields are accepted only with `--ipfix`, in `fields` or (except `DATALINK_FRAME_SECTION`) in `scope`. With `--ipfix`, `scope` takes regular field names instead of the five v9 Options scopes. They are not available in the HTML builder. See [IPFIX output](README.md#ipfix-output) for the `frame` object.

| Field | ID | Length | Value |
|---|---:|---:|---|
| `DIRECTION` | 61 | 1 | Integer; 0 = ingress, 1 = egress. Arrays and ranges accepted |
| `FLOW_EXPORTER` | 144 | 4 | Integer 0..=4294967295. Arrays and ranges accepted |
| `DATALINK_FRAME_SIZE` | 312 | 2 | Integer 0..=65535. Arrays and ranges accepted |
| `DATALINK_FRAME_SECTION` | 315 | Variable | Hexadecimal string, array of equal-length hexadecimal strings, or a `frame` object |
