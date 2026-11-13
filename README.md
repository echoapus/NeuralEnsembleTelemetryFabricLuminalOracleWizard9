# NeuralEnsembleTelemetryFabricLuminalOracleWizard9

[English (en-US)](README.md) | [繁體中文 (zh-TW)](README.zh-TW.md)

`nfv9gen` is a NetFlow v9 / IPFIX traffic generator for testing collectors. Describe flows in a JSON file, check it offline, then send it over UDP.

- 53 regular fields, five Options scopes, IPv4/IPv6, ASN, ICMP, MPLS
- Arrays, ranges, IP prefixes, and random values expand into many records
- `validate` and `preview` need no collector and open no socket
- `--ipfix` sends IPFIX instead of v9, including sampled packet headers (Junos inline monitoring)

Linux only (Windows: use WSL2). Arbitrary field IDs and enterprise PENs are not supported.

<a id="build-and-quick-start"></a>

## Build and quick start

Get the binary from the repository's **Releases** page (`nfv9gen-linux-x86_64.tar.gz`, Linux x86-64, glibc 2.35+), or build from source:

```sh
bash build.sh          # run without sudo; installs missing tools and builds target/release/nfv9gen
```

Then check, preview, and send:

```sh
nfv9gen validate examples/basic.json
nfv9gen preview examples/basic.json --hex
nfv9gen send examples/basic.json --target 192.0.2.20:7777
```

- `send` needs a collector listening on the target UDP port; otherwise it stops with `Connection refused`.
- By default `send` sends one cycle (all templates, then each data packet once) at 20 packets/s. Use `--count N` or `--duration SECONDS` (`0` runs until Ctrl-C) to send more.
- A successful send only means the local OS accepted the packets. Confirm receipt in the collector.

You can also fill out a form in [config-builder.html](config-builder.html) (offline, NetFlow v9 only) and download the JSON.

<a id="configuration-format"></a>

## Configuration format

```json
{
  "source_id": 42,
  "flows": [
    {
      "template_id": 256,
      "fields": [
        {"name":"IPV4_SRC_ADDR","value":"192.0.2.1"},
        {"name":"IPV4_DST_ADDR","value":"198.51.100.2"},
        {"name":"PROTOCOL","value":6},
        {"name":"L4_SRC_PORT","value":12345},
        {"name":"L4_DST_PORT","value":443},
        {"name":"IN_PKTS","value":10},
        {"name":"IN_BYTES","value":1000}
      ]
    }
  ]
}
```

| Key | Rule |
|---|---|
| `source_id` | Optional, default 0. Not an IP address (becomes the IPFIX observation domain ID) |
| `template_id` | `256..=65535`; field order in `fields` is the template order |
| `fields` | `name` (case-insensitive) and `value` are both required |
| `scope` | Optional; a non-empty scope makes an Options template (see [options.json](examples/options.json)) |

Unknown keys or fields fail validation. Limits: 8 MiB file, 500,000 expanded records in total.

### Value syntax

| Value | Meaning |
|---|---|
| `443` or `"192.0.2.1"` | Single value |
| `[80, 443]` | Several values |
| `{"start":"192.0.2.1","end":"192.0.2.3","step":1}` | Range (`step` defaults to 1) |
| `"192.0.2.0/30"` or `{"prefix":"192.0.2.0/30","hosts_only":true}` | IP prefix |
| `{"random_between":[49152,65535]}` | New random value on every send (L4 ports, `IN_PKTS`, `IN_BYTES`) |
| `{"random_file":"ips.txt"}` | One random IP per record from a file, one address per line |

Values across fields multiply: 2 sources × 2 ports = 4 records, and the last field varies fastest. TCP flags accept `"SYN|ACK"`, `"SA"`, `18`, or `"0x12"`. The full list of fields and accepted values is in [field.md](field.md) (MPLS, ASN, ICMP, and counters are covered there).

### Share a template across flows

Flows with the same `template_id` share one template, so you can keep groups of values together without cross-combining them. They must have identical scope, field names, order, and lengths; each flow expands on its own. See [shared-template.json](examples/shared-template.json).

### Updating older configurations

- `{"random":true,"min":…,"max":…}` → `{"random_between":[min,max]}`.
- Flow-level `mpls_stack` → explicit `MPLS_LABEL_n` and metadata fields (see [convenience.json](examples/convenience.json)).
- `random_file` lists take plain IPs only; `198.51.100.0/24` must become `198.51.100.0`.

## IPFIX output

Add `--ipfix` to `validate`, `preview`, or `send` to emit IPFIX (version 10) instead of NetFlow v9. IPFIX collectors usually listen on 4739, so set `--target`.

```sh
nfv9gen send examples/ipfix-inline-monitoring.json --ipfix --target 192.0.2.10:4739
```

- `scope` takes regular field names, such as `FLOW_EXPORTER`, not the v9 scope names.
- `DIRECTION` (61), `FLOW_EXPORTER` (144), `DATALINK_FRAME_SIZE` (312), and `DATALINK_FRAME_SECTION` (315) work only with `--ipfix`.
- The sequence number counts data records, not packets.

`DATALINK_FRAME_SECTION` carries the first bytes of a sampled Ethernet frame. Give hex bytes, or a `frame` object that generates the headers:

```json
{"name": "DATALINK_FRAME_SECTION", "value": {"frame": {
  "clip": 126, "length": 310, "protocol": 6,
  "src": {"start": "10.0.1.1", "end": "10.0.1.8"}, "dst": "198.51.100.50",
  "src_port": {"random_between": [1024, 65535]}, "dst_port": [80, 443],
  "tcp_flags": "SYN"
}}}
```

| Key | Default | Meaning |
|---|---|---|
| `protocol`, `src`, `dst`, `src_port`, `dst_port` | required | `protocol` is 6 (TCP) or 17 (UDP); addresses and ports accept the value syntax above |
| `clip` | 126 | Exported bytes, starting at the Ethernet header |
| `length` | `clip` | Original frame length; set `DATALINK_FRAME_SIZE` to the same value |
| `ipv6` | false | Generate an IPv6 header (`src`/`dst` are then IPv6) |
| `tcp_flags`, `tos` | SYN, 0 | TCP flags (TCP only), TOS (IPv4 only) |

All sections in one flow have the same length; add another flow (it may share the `template_id`) for a different `clip` or IP version. MACs are fixed, TTL is 64, bytes after the headers are zero, and the IPv4 header checksum is always valid. Example: [ipfix-inline-monitoring.json](examples/ipfix-inline-monitoring.json).

## CLI

```text
nfv9gen validate CONFIG [--packet-size BYTES] [--ipfix]
nfv9gen preview  CONFIG [--packet-size BYTES] [--ipfix] [--kind all|templates|data] [--hex] [--limit N] [--template-id ID] [--rate N]
nfv9gen send     CONFIG [--target HOST:PORT] [OPTIONS]
```

Every command also accepts `-s/--silent` (print only `done` or `fail`). `preview` also prints the expected collector IN/OUT bps and pps per template.

| `send` option | Default | Meaning |
|---|---|---|
| `--target HOST:PORT` | `127.0.0.1:2055` | Collector address (`[::1]:2055` for IPv6) |
| `--kind all\|templates\|data` | all | `all` sends templates first and refreshes them; `data` needs templates already known to the collector |
| `--count N` | one cycle | Stop after N packets (templates count only in `templates` mode) |
| `--duration SECONDS` | — | Run for this long; `0` runs until Ctrl-C/SIGTERM; cannot be combined with `--count` |
| `--rate N` | 20 | Target packets per second |
| `--mode stream\|burst` | stream | `burst` sends batches of `--burst-size` (default 64) |
| `--template-interval S`, `--template-every N` | 30, 20 | Refresh templates every S seconds or N data packets |
| `--packet-size BYTES` | 1400 | Maximum UDP payload (64–65507); also for `validate`/`preview` |
| `--bind IP:PORT` | OS choice | Local address and port to send from |
| `--source IP:PORT` | — | Spoof the UDP source address (raw socket: root or `CAP_NET_RAW`); conflicts with `--bind` |
| `--ipfix` | off | Send IPFIX instead of NetFlow v9 |

`--bind` and `--source` do not change the addresses inside the JSON records. When sending templates and data separately, keep `source_id`, the source IP:port, and the template definitions the same.

### Continuous sending and logs

```sh
# 100 data packets
nfv9gen send examples/basic.json --target 192.0.2.20:7777 --rate 10 --count 100

# Run until Ctrl-C, logging statistics every 5 seconds
nfv9gen send examples/basic.json --target 192.0.2.20:7777 --duration 0 >send.log 2>&1
```

Progress updates in place on a terminal; when output is redirected, statistics go to stderr every 5 seconds (hence `2>&1`). A final summary is printed on completion or Ctrl-C.

Exit codes: `0` success, `1` configuration or runtime error, `2` argument error, `130` interrupted.

## Examples

| File | Contents |
|---|---|
| [basic.json](examples/basic.json) | One IPv4 TCP record |
| [shared-template.json](examples/shared-template.json) | Two value groups on one template |
| [ranges.json](examples/ranges.json) | IP ranges, arrays, combinations |
| [random-ports.json](examples/random-ports.json), [random-counters.json](examples/random-counters.json), [random-ips.json](examples/random-ips.json) | Random values resampled on each send |
| [interfaces.json](examples/interfaces.json), [nexthop.json](examples/nexthop.json) | Interface indexes and next hops |
| [options.json](examples/options.json) | Sampling Options with several scopes |
| [asn.json](examples/asn.json), [mpls.json](examples/mpls.json), [convenience.json](examples/convenience.json) | ASN notation, MPLS, ICMP |
| [collector-all-fields.json](examples/collector-all-fields.json) | All 53 fields and five scopes; see [COLLECTOR-TEST.md](COLLECTOR-TEST.md) |
| [ipfix-inline-monitoring.json](examples/ipfix-inline-monitoring.json) | IPFIX inline monitoring (use `--ipfix`) |

## More documentation

- [field.md](field.md): every field, its length, and accepted values
- [FIELD-SUPPORT.md](FIELD-SUPPORT.md): value formats per field
- [COLLECTOR-TEST.md](COLLECTOR-TEST.md): all-fields test with nfcapd/nfdump

## License

[PolyForm Noncommercial 1.0.0](LICENSE). Free for study, research, personal and other noncommercial use, including modified builds. Commercial use is not allowed.
