# All-fields FlowSets: PC → nfcapd → nfdump

[English (en-US)](COLLECTOR-TEST.md) | [繁體中文 (zh-TW)](COLLECTOR-TEST.zh-TW.md)

Configuration: [examples/collector-all-fields.json](examples/collector-all-fields.json). It covers all **53 regular fields and five Options scopes** in [field.md](field.md). See the [README](README.md) for workflows and CLI options. Source ID is `424242`, with 14 templates. Each datagram contains one FlowSet: a regular template, Options template, or corresponding data FlowSet.

These instructions assume a Linux or Windows WSL2 PC and nfdump/nfcapd installed on the collector. This project currently builds Linux executables; Windows users should build and run in WSL2. IPv4/IPv6 addresses inside records are simulated, need not be configured on the PC, and do not receive connections. Actual UDP packets go only to `--target`.

The fixture uses explicit MPLS label and metadata entries in `fields`. Keep their values and order when adapting it; stack metadata and BoS are not inferred. For older configurations, see [Updating older configurations](README.md#updating-older-configurations).

## Start the collector

Run on the collector and retain the versions:

```sh
nfcapd -V
nfdump -V
```

Collect into a new dedicated directory to avoid mixing with existing traffic. These commands use UDP 2055. If that port is occupied, choose another, such as 12055, and update the PC target too. The collector firewall must allow the PC to reach that UDP port.

```sh
NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR=$(mktemp -d "$HOME/NeuralEnsembleTelemetryFabricLuminalOracleWizard9-collector.XXXXXX")
printf '%s\n' "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR"
nfcapd -l "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -p 2055 -t 60 -E
```

Leave this terminal open and send from the PC. nfcapd runs in the foreground. `-E` allows decoding observation; newer versions also offer `-v 3`. The nfdump 1.7 series dynamically stores supported extensions by default. For the 1.6 series, check local `nfcapd -h` and add `-T all` so interface, ASN, MPLS, and other extra data are retained. See the [official nfcapd manual](https://github.com/phaag/nfdump/blob/master/man/nfcapd.1).

For an existing collector, confirm that the PC source IP is allowed. Source allowlists, enforced sampling, and extension restrictions can affect results.

## Validate and send from the PC

Run from the project root:

```sh
cargo build --release --locked
./target/release/nfv9gen validate examples/collector-all-fields.json
./target/release/nfv9gen preview examples/collector-all-fields.json --kind data --limit 14 --rate 10
```

Expected validation summary:

```text
Validation passed: Source ID=424242, 14 templates, 22 records, 14 data packets
```

### Expected collector rates per minute and templates

This example intentionally uses 14 different template IDs to make each group easy to verify. Other configurations may share an ID if scope, field names, order, and lengths match. The template is then sent once initially and once per refresh, while each flow expands independently; see [shared-template.json](examples/shared-template.json). Do not merge different structures just to reduce template packets.

Preview and the send plan show a table with one row per regular template, listing IN/OUT bps and pps with bandwidth first. Shared IDs are combined. The preview above explicitly sets rate=10 to match the sending command below. Without rate, both commands default to **20 packets/second**, shared by all templates' data packets.

This configuration has 14 data packets per cycle. At rate=10, Template 1000 estimates are approximately IN **5714.29 bps / 7.14 pps** and OUT **11428.57 bps / 14.29 pps**. Other regular templates get their own rows. Missing OUT counters show `-`; the five Options templates show only ID and `Options`, without simulated traffic estimates. Options data still consumes sending time.

Send shows a short configuration summary and the complete rate table, without repeating per-field random ranges or the first few template structures. For details, use `nfv9gen preview examples/collector-all-fields.json --template-id 1000 --kind data --hex --limit 1 --rate 10`. After stopping, a two-line summary includes elapsed hours/minutes/seconds, data/template packets, records, UDP payload MiB, and average rate.

Estimates assume continuous cycles, complete receipt, aggregation by receipt time, and no sampling multiplier. Short runs still show long-term averages. These are not actual UDP bandwidth or rates derived by a collector from record duration, sampling, or aggregation. limit/template-id filter packet previews without narrowing estimates. Periodic sending statistics do not recalculate per-minute estimates.

### Send ten cycles

Replace the target below with your actual collector IP. Use `all` so templates and data are sent in one run; root and `--source` are unnecessary.

```sh
./target/release/nfv9gen send examples/collector-all-fields.json \
  --target 192.0.2.20:2055 \
  --kind all --rate 10 --count 140 --template-every 14
```

This is **ten cycles**, each with 14 data packets, including five Options data packets. All templates refresh before each cycle. At this rate, the usual totals are 140 template packets, 140 data packets, and 220 records. System delays triggering the 30-second refresh threshold may increase the template count.

The 220 records comprise **170 regular flow records and 50 Options records**. Options describe sampling state and are not nfdump regular flow rows. With no packet loss/filtering and all regular templates accepted, expect 170 unaggregated regular flow rows.

All transport may use IPv4 UDP while still testing IPv6 fields inside records. For an IPv6-only collector, use `--target '[ACTUAL_IPV6]:2055'`.

### Continuous testing and logs

After verifying ten cycles, test continuous sending separately. This command has no fixed cycle count, so the later ten-cycle row totals do not apply:

```sh
./target/release/nfv9gen send examples/collector-all-fields.json \
  --target 192.0.2.20:2055 --kind all --rate 20 \
  --duration 0 --template-every 14 >send.log 2>&1
```

Only exactly `--duration 0` enables unlimited mode until Ctrl-C/SIGTERM. Other zero/tiny forms such as `0.0`, `-0`, and `1e-400` are rejected. count and duration conflict. Manual interruption exits 130, invalid arguments exit 2, and transport/statistics failures exit 1.

When stdout and stderr are terminals, progress updates in place. With redirection, stderr receives actual cumulative packets, records, UDP payload bytes, and average rate every five seconds. Progress checks also run within large bursts and initial template sends, with callbacks at most every 250 ms. Reaching count or duration stops further per-packet progress callbacks. Read with `tail -f send.log` in another terminal. Statistics write/flush failures stop sending and preserve the first error. If stderr also fails, the message may be lost but the program does not panic. This differs from normal successful exit on a closed stdout pipe.

## Stop collection and read completed files

After PC sending completes, press Ctrl-C in the collector terminal. Wait for nfcapd to exit, then run the following. Do not read an active `nfcapd.current.*` file. If you switch terminals, first set `NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR` to the directory printed earlier.

```sh
ls -lh "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR"
nfdump -R "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -N -6 -o raw
```

`-o raw` shows extensions retained in the file, **not a field-by-field dump of the original UDP packets**. `-N` disables numeric abbreviation and `-6` keeps complete IPv6 addresses. Do not aggregate with `-a` or `-A`, which can merge repeated records. See the [official nfdump manual](https://github.com/phaag/nfdump/blob/master/man/nfdump.1) for output and filtering.

Summary table:

```sh
nfdump -R "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -N -6 \
  -o 'fmt:%sa %da %pr %sp %dp %in %out %sas %das %ipkt %ibyt %opkt %obyt'
```

Columns are source/destination IP, protocol, source/destination port, input/output ifIndex, source/destination ASN, input packets/bytes, and output packets/bytes. If an older version rejects a format token, start with `-o raw`.

## Expected results by group

Counts below assume ten cycles. Distinguish groups by source IP inside records because nfdump files may not preserve original template IDs.

| Template | Test group | Record source IP | Records/cycle | Regular flow rows in ten cycles |
|---|---|---|---:|---:|
| 1000 | IPv4 TCP: full counters, ToS, masks, interfaces, ASNs, next hops | `192.0.2.10` | 1 | 10 |
| 1001 | IPv4 UDP: single `.0/32` address | `1.1.1.0` | 1 | 10 |
| 1002 | IPv6 TCP: masks, interfaces, ASNs, both IPv6 next hops | `2001:db8:1002::10` | 1 | 10 |
| 1003 | ICMPv4: type/code `8/0`, `3/1` | `192.0.2.13` | 2 | 20 |
| 1004 | ICMPv6: type/code `128/0`, `2/0` | `2001:db8:1004::10` | 2 | 20 |
| 1005 | Interface: 0, 1, 65535, 65536, 4294967295 | `192.0.2.15` | 5 | 50 |
| 1006 | MPLS labels 1–10 | `192.0.2.16` | 1 | 10 |
| 1007 | IPv4 MPLS: three RD encodings, FEC, extra metadata | `192.0.2.17` | 3 | 30 |
| 1008 | IPv6 MPLS: IPv6 FEC, extra metadata | `2001:db8:1008::10` | 1 | 10 |
| 1100 | Options, scope `SYSTEM=1` | No regular flow IP | 1 | Excluded |
| 1101 | Options, scope `INTERFACE=70001` | No regular flow IP | 1 | Excluded |
| 1102 | Options, scope `LINE_CARD=2` | No regular flow IP | 1 | Excluded |
| 1103 | Options, scope `NETFLOW_CACHE=1` | No regular flow IP | 1 | Excluded |
| 1104 | Options, scope `TEMPLATE=1000` | No regular flow IP | 1 | Excluded |

Runnable checks:

```sh
# .0/32 keeps 1.1.1.0: one address, not zero records or a full /24
nfdump -R "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -N -o raw 'src ip 1.1.1.0'

# Interface: five INPUT_SNMP values for one source; OUTPUT_SNMP stays 70002
nfdump -R "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -N \
  -o 'fmt:%sa %in %out %ipkt %ibyt' 'src ip 192.0.2.15'

# Full IPv4 baseline: per-record input/output packets/bytes are 10/1000 and 20/2000
nfdump -R "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -N -o raw 'src ip 192.0.2.10'

# MPLS: first label 16000, TC=3; tenth label 16009, BoS=1
nfdump -R "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -N \
  -o 'fmt:%sa %mpls1 %mpls2 %mpls3 %mpls4 %mpls5 %mpls6 %mpls7 %mpls8 %mpls9 %mpls10' \
  'src ip 192.0.2.16'

# ICMPv4/v6: inspect type/code in raw output; TCP/UDP ports do not apply
nfdump -R "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -N -6 -o raw 'proto 1 or proto 58'
```

For Template 1000, SRC_AS should be `65546` (the integer equivalent of `AS1.10`) and DST_AS `4200000000`. Next hops are `192.0.2.254` and `198.51.100.254`; INPUT/OUTPUT_SNMP are `70001/70002`. Template 1002's IPv6 next hops are `2001:db8:1002::ffff` and `2001:db8:2002::ffff`, with ASNs `65002/65556`.

All regular flow FIRST_SWITCHED/LAST_SWITCHED values are 0, meaning exporter startup. Record duration (LAST−FIRST) is zero milliseconds; this differs from CLI `--duration 0` for unlimited sending. These values do not update on repeated sends, and PKTS/BYTES do not accumulate. Valid relative uptime avoids configuring future times immediately after startup. Collector absolute times are derived from packet time and uptime and may vary slightly because headers have second-level precision.

## Interpret Options and extra MPLS fields

The five scopes use separate Options templates to test collector acceptance; a collector may use only some scopes. SAMPLING_INTERVAL and SAMPLING_ALGORITHM are both **1**, avoiding sampling multipliers during this check. Options may not appear in `nfdump -o raw`. Check nfcapd decoding messages first, then inspect a completed `nfcapd.YYYY...` file with:

```sh
nfdump -E /actual/path/nfcapd.YYYYMMddhhmm
```

This displays available exporter/sampler information. Absence of Options fields from regular flow rows alone does not prove a packet problem. See [nfdump sampling documentation](https://github.com/phaag/nfdump#sampling) for multiplier behavior.

MPLS labels 1–10 and extra metadata use different templates to simplify diagnosis. This tool exports RD (90), FEC prefix (91), IPv6 FEC (140), and TTL/depth/length/EXP (200–203) in v9 using IANA definitions. **Your nfdump version may skip or not retain them.** Compare its implementation with the upstream [NetFlow v9 conversion table](https://github.com/phaag/nfdump/blob/master/src/netflow/netflow_v9.c). Seeing labels does not prove RDs or metadata were stored.

To check whether a missing field exists on the wire, start capture in another collector terminal before sending again:

```sh
sudo tcpdump -ni any -s 0 -w /tmp/NeuralEnsembleTelemetryFabricLuminalOracleWizard9-all-fields.pcap 'udp dst port 2055'
```

After sending, press Ctrl-C and open the pcap in Wireshark. If needed, Decode As CFLOW/NetFlow for UDP 2055. Compare template IDs, field IDs, lengths, and data values. If the pcap contains correct fields/values that nfdump does not display, record this as that collector version not retaining/displaying them. For nfcapd length errors, discarded data, or misaligned fields, compare against the pcap to locate the problem.

## Coverage of all 53 regular fields

The table lists every field ID and its templates. A field may appear in several groups.

| Field | ID | Length (bytes) | Template |
|---|---:|---:|---|
| IN_BYTES | 1 | 4 | 1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008 |
| IN_PKTS | 2 | 4 | 1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008 |
| FLOWS | 3 | 4 | 1000 |
| PROTOCOL | 4 | 1 | 1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008 |
| SRC_TOS | 5 | 1 | 1000 |
| TCP_FLAGS | 6 | 1 | 1000, 1002, 1006, 1007, 1008 |
| L4_SRC_PORT | 7 | 2 | 1000, 1001, 1002, 1005, 1006, 1007, 1008 |
| IPV4_SRC_ADDR | 8 | 4 | 1000, 1001, 1003, 1005, 1006, 1007 |
| SRC_MASK | 9 | 1 | 1000, 1001 |
| INPUT_SNMP | 10 | 4 | 1000, 1001, 1002, 1005 |
| L4_DST_PORT | 11 | 2 | 1000, 1001, 1002, 1005, 1006, 1007, 1008 |
| IPV4_DST_ADDR | 12 | 4 | 1000, 1001, 1003, 1005, 1006, 1007 |
| DST_MASK | 13 | 1 | 1000, 1001 |
| OUTPUT_SNMP | 14 | 4 | 1000, 1001, 1002, 1005 |
| IPV4_NEXT_HOP | 15 | 4 | 1000 |
| SRC_AS | 16 | 4 | 1000, 1002 |
| DST_AS | 17 | 4 | 1000, 1002 |
| BGP_IPV4_NEXT_HOP | 18 | 4 | 1000 |
| LAST_SWITCHED | 21 | 4 | 1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008 |
| FIRST_SWITCHED | 22 | 4 | 1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008 |
| OUT_BYTES | 23 | 4 | 1000 |
| OUT_PKTS | 24 | 4 | 1000 |
| IPV6_SRC_ADDR | 27 | 16 | 1002, 1004, 1008 |
| IPV6_DST_ADDR | 28 | 16 | 1002, 1004, 1008 |
| IPV6_SRC_MASK | 29 | 1 | 1002 |
| IPV6_DST_MASK | 30 | 1 | 1002 |
| ICMP_TYPE | 32 | 2 | 1003 |
| SAMPLING_INTERVAL | 34 | 4 | 1100, 1101, 1102, 1103, 1104 |
| SAMPLING_ALGORITHM | 35 | 1 | 1100, 1101, 1102, 1103, 1104 |
| MPLS_TOP_LABEL_TYPE | 46 | 1 | 1007, 1008 |
| MPLS_TOP_LABEL_IP_ADDR | 47 | 4 | 1007 |
| DST_TOS | 55 | 1 | 1000 |
| IP_PROTOCOL_VERSION | 60 | 1 | 1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008 |
| IPV6_NEXT_HOP | 62 | 16 | 1002 |
| BGP_IPV6_NEXT_HOP | 63 | 16 | 1002 |
| MPLS_LABEL_1 | 70 | 3 | 1006, 1007, 1008 |
| MPLS_LABEL_2 | 71 | 3 | 1006 |
| MPLS_LABEL_3 | 72 | 3 | 1006 |
| MPLS_LABEL_4 | 73 | 3 | 1006 |
| MPLS_LABEL_5 | 74 | 3 | 1006 |
| MPLS_LABEL_6 | 75 | 3 | 1006 |
| MPLS_LABEL_7 | 76 | 3 | 1006 |
| MPLS_LABEL_8 | 77 | 3 | 1006 |
| MPLS_LABEL_9 | 78 | 3 | 1006 |
| MPLS_LABEL_10 | 79 | 3 | 1006 |
| MPLS_VPN_RD | 90 | 8 | 1007 |
| MPLS_TOP_LABEL_PREFIX_LEN | 91 | 1 | 1007, 1008 |
| MPLS_TOP_LABEL_IPV6_ADDR | 140 | 16 | 1008 |
| ICMP_TYPE_IPV6 | 139 | 2 | 1004 |
| MPLS_TOP_LABEL_TTL | 200 | 1 | 1007, 1008 |
| MPLS_LABEL_STACK_LENGTH | 201 | 4 | 1007, 1008 |
| MPLS_LABEL_STACK_DEPTH | 202 | 4 | 1007, 1008 |
| MPLS_TOP_LABEL_EXP | 203 | 1 | 1007, 1008 |

Offline checks cover complete field coverage, configuration validation, and expected send counts. Sending/receiving on your PC and collector has not been verified. Retain nfdump/nfcapd versions, send summaries, raw output, and pcaps for problematic cases.
