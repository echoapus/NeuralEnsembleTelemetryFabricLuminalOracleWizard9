# NeuralEnsembleTelemetryFabricLuminalOracleWizard9

[English (en-US)](README.md) | [繁體中文 (zh-TW)](README.zh-TW.md)

**NeuralEnsembleTelemetryFabricLuminalOracleWizard9** (executable: `nfv9gen`) is a NetFlow v9 traffic generator written in Rust. Load JSON, validate the configuration, preview it offline, and send it over UDP. It supports 53 regular fields, five Options scopes, IPv4/IPv6, interfaces, ASNs, ICMP, MPLS, range expansion, and random L4 port / IN_PKTS / IN_BYTES values.

| Task | Documentation or tool |
|---|---|
| First use | [Build and quick start](#build-and-quick-start) → [Start the collector](#start-the-collector) → [First send and verification](#first-send-and-verification) |
| Generate JSON in a browser | [Offline HTML builder](config-builder.html) |
| Test every field | [All-fields configuration](examples/collector-all-fields.json), [collector test guide](COLLECTOR-TEST.md) |
| Look up field IDs, lengths, and input rules | [Configuration format](#configuration-format), [complete field/value reference](field.md) |
| Compare value formats by field | [Field/value support matrix](FIELD-SUPPORT.md) |
| Find send options, unlimited runs, and logs | [CLI](#cli), [Continuous sending and logs](#continuous-sending-and-logs) |
| Estimate collector rates | [Expected collector average rates per minute](#expected-collector-average-rates-per-minute) |
| Check combinations, RAM, and limits | [Record counts, packets, and configuration limits](#record-counts-packets-and-configuration-limits), [Memory usage](#memory-usage) |
| Troubleshoot receipt or decoding | [Troubleshooting](#troubleshooting) |

The current build targets Linux; Windows users can build and run it in WSL2. Output is always **NetFlow v9**. IPFIX v10, arbitrary field IDs, and enterprise PENs are not supported. Some extra fields use IANA IPFIX registry definitions, but the packet version remains v9 and the receiver must support those fields.

The CLI and HTML interface use US English. Both documentation editions show the actual English output and interface labels.

<a id="build-and-quick-start"></a>

## Build and quick start

### Download a prebuilt binary

Open this repository's **Releases** page and download `nfv9gen-linux-x86_64.tar.gz` and its `.sha256` file. The binary targets Linux x86-64 with glibc 2.35 or newer (Ubuntu 22.04 or newer, including WSL2). Other systems can build from source below.

```bash
sha256sum -c nfv9gen-linux-x86_64.tar.gz.sha256
tar -xzf nfv9gen-linux-x86_64.tar.gz
cd nfv9gen-linux-x86_64
./nfv9gen --version
./nfv9gen validate examples/basic.json
```

For the commands below, replace `./target/release/nfv9gen` with `./nfv9gen` when using this download.

### Automated builds and releases

GitHub Actions tests and builds every push to `main`. **Actions → Build and release → Run workflow** also builds without publishing when run on `main`. Download the archive and checksum from the completed run's artifacts.

To publish a version, update `Cargo.toml` and `Cargo.lock`, commit and push the changes, then push a matching version tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Use a new tag for each release. After tests and compilation pass, the workflow creates a GitHub Release containing the binary archive and checksum. It uses GitHub's automatic token; no personal access token or repository secret is needed.

### Automatic build (recommended)

Download and extract this project from GitHub (**Code → Download ZIP**), then open a terminal in the folder containing `build.sh`. Run:

```sh
bash build.sh
```

The script detects Linux and your CPU architecture, checks the C compiler/linker, and installs missing system prerequisites on Ubuntu/Debian, Fedora/RHEL-compatible systems, openSUSE, and Arch-based systems. It reuses Rust when it meets the minimum in Cargo.toml; otherwise it installs a minimal stable toolchain using [official rustup](https://rust-lang.github.io/rustup/concepts/profiles.html). It then builds with locked dependencies and checks the version, basic configuration, and offline packet preview. No packets are sent.

Run it **without sudo**. The script requests sudo only if system packages are missing; package installation and the first Rust/dependency downloads need internet access. It automatically accepts package/Rust installation prompts; sudo may still ask for your login password. It does not install the application system-wide. The executable is `target/release/nfv9gen`, and the script prints its absolute path when finished.

You can rerun `bash build.sh` after updating the source. Existing tools and cached build output are reused. On other distributions, install the C compiler/linker, curl, and CA certificates manually first; the script can then set up Rust and build. On Arch, keep the system/package databases current before running the script; it does not perform a full system upgrade. Failures stop the build and print an error; fix that error before rerunning.

### Manual build

If you prefer to install the tools yourself, follow these steps.

This walkthrough uses **Ubuntu or Debian Linux**, including Ubuntu inside WSL2. You need internet access for installation and the first build, plus a user account that can run `sudo` to install packages. On other Linux distributions, install their C compiler/linker and curl packages before following the Rust and build steps.

Run commands in a **Terminal**, one code block at a time. Copy only the commands, not the output examples. When `sudo` asks for your login password, typing shows no characters; enter it and press Enter. Use `sudo` only for the package-installation commands below. Build and run NeuralEnsembleTelemetryFabricLuminalOracleWizard9 as your normal user.

### 1. Install the Linux build tools

```sh
sudo apt update
sudo apt install -y build-essential curl ca-certificates
```

`build-essential` supplies the compiler and linker needed to produce the executable. curl downloads the Rust installer. See [Ubuntu's Rust setup guide](https://ubuntu.com/developers/docs/howto/rust-setup/) for distribution-specific background.

### 2. Install Rust and Cargo

If `rustc --version` and `cargo --version` already work and Rust is current, continue to step 3. The project's declared minimum is Rust 1.85; use the latest stable version for the locked dependencies.

Install using the [official Rust instructions](https://doc.rust-lang.org/book/ch01-01-installation.html):

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

When the installer offers options, press Enter for the default installation. After it finishes, load the environment into this terminal and check both tools:

```sh
. "$HOME/.cargo/env"
rustc --version
cargo --version
```

Each version command should print a version number. Rust compiles the program; Cargo downloads dependencies and manages the build. If you installed through rustup and need a newer version, run `rustup update stable`.

### 3. Download the project and open its folder

1. Open **this project's repository page** on GitHub. Above the files, select **Code → Download ZIP**. See [GitHub's download instructions](https://docs.github.com/en/repositories/working-with-files/using-files/downloading-source-code-archives).
2. In your file manager, extract the downloaded ZIP. Open the extracted project folder rather than the ZIP itself.
3. In the folder containing `Cargo.toml`, right-click an empty area and select **Open in Terminal** if your file manager provides it. If you already have the source, use its existing folder.

Alternatively, change folders in an existing terminal with `cd "/full/path/to/project"`, replacing the quoted path with your actual extracted folder. Keep the quotes if the path contains spaces.

Confirm you are in the project root:

```sh
pwd
ls Cargo.toml Cargo.lock src/main.rs examples/basic.json
```

`pwd` prints your current folder. `ls` should list all four paths without an error. Keep using this folder for the commands below and the rest of this guide.

### 4. Build the executable

```sh
cargo build --release --locked
```

The first build downloads dependencies and compiles them, so it may take several minutes. Wait for a successful `Finished` message and for the terminal prompt to return. `--release` builds the optimized executable; `--locked` uses the dependency versions in `Cargo.lock`.

The finished program is **`target/release/nfv9gen`**, inside the project folder. You do not need `cargo install` or a system-wide installation.

### 5. Check your build without a collector

```sh
./target/release/nfv9gen --version
./target/release/nfv9gen validate examples/basic.json
./target/release/nfv9gen preview examples/basic.json --kind data --hex
```

`--version` should show `nfv9gen 0.1.0`. The `./` prefix tells Linux to run the file at this relative path. Validation should print:

```text
Validation passed: Source ID=42, 1 templates, 1 records, 1 data packets
```

`validate` and `preview` do not create sockets or require a collector. If both finish successfully, your build is ready.

### 6. Send to your collector

The example `192.0.2.20:7777` is a documentation address, not a working collector. Replace it with your real collector IP/UDP port. Complete [Start the collector](#start-the-collector) first, keeping the listener, target, and capture filters consistent, then send:

```sh
./target/release/nfv9gen send examples/basic.json \
  --target 192.0.2.20:7777
```

Without `--rate`, the default target is **20 packets/second**, shared by the entire configuration. Without `--count` or `--duration`, the program sends one cycle: all templates followed by each data packet once. Specify a duration for continuous sending, or use `--duration 0` to run until Ctrl-C/SIGTERM:

```sh
./target/release/nfv9gen send examples/basic.json \
  --target 192.0.2.20:7777 --rate 20 --duration 10
```

After updating the program, run `cargo build --release --locked` again. Add `--offline` when dependencies are cached. All commands assume the project root as the working directory. If you copied the executable to `./nfv9gen`, substitute that path in the commands.

### If a build step fails

| Message or symptom | What to do |
|---|---|
| `cargo: command not found` or `rustc: command not found` | After installing Rust, run `. "$HOME/.cargo/env"` in this terminal, or open a new terminal |
| Linker error mentioning `cc` | Install `build-essential` using step 1, then retry the build |
| `could not find Cargo.toml` | You are in the wrong folder; return to the extracted folder containing Cargo.toml and repeat step 3's `ls` check |
| Rust version is too old / a dependency requires a newer rustc | For a rustup installation, run `rustup update stable` and `rustup default stable`, check `rustc --version`, then build again |
| Dependency download or name-resolution errors | Check internet/proxy access and retry; `--offline` only works after every required dependency is cached |
| `./target/release/nfv9gen: No such file or directory` | Confirm the build finished successfully and you are still in the project root; run `ls -l target/release/nfv9gen` |

## Start the collector

First, confirm that a program is listening on the destination port on the collector host:

```sh
sudo ss -lunp 'sport = :7777'
```

Remote IPv4 sending requires a listener on `192.0.2.20:7777` or `0.0.0.0:7777`; binding only to `127.0.0.1` is insufficient. Allow UDP traffic from the sending PC through the firewall. For a containerized collector, publish the correct UDP port.

If you already have a production collector, enable its NetFlow v9 listening port through that system's settings. With the official nfdump tools, you can start a test receiver with nfcapd. Run this in a terminal on the collector:

```sh
NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR=$(mktemp -d /tmp/NeuralEnsembleTelemetryFabricLuminalOracleWizard9-collector.XXXXXX)
printf '%s\n' "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR"
nfcapd -l "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -b 0.0.0.0 -p 7777 -t 60 -E
```

Leave nfcapd in the foreground and check `ss` from another terminal. Do not start a second receiver on the same port. `-E` enables decoding observation; newer versions also offer `-v 3`. For the 1.6 series, check local `nfcapd -h` and add `-T all` to retain additional fields. See the [official nfcapd manual](https://github.com/phaag/nfdump/blob/master/man/nfcapd.1).

If no program receives UDP on the destination port, the OS may return ICMP Port Unreachable. NeuralEnsembleTelemetryFabricLuminalOracleWizard9 stops when it receives `Connection refused`. A packet capture tool alone does not replace a UDP receiver.

## First send and verification

### Send a configuration and read the results

The CLI accepts both HTML-generated `NeuralEnsembleTelemetryFabricLuminalOracleWizard9.json` and handwritten JSON. Start with the basic example, then substitute your configuration:

```sh
./target/release/nfv9gen send examples/basic.json \
  --target 192.0.2.20:7777
```

Without count/duration, the basic example sends **one template packet and one data packet**, containing one data record. Completion confirms only that the local OS accepted the packets; confirm receipt through the collector's logs or queries.

When collecting with official nfcapd, return to its terminal after sending and press Ctrl-C. Wait for it to exit and finish writing, then run these commands in that same terminal:

```sh
ls -lh "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR"
nfdump -R "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -N -6 -o raw
```

If you switch terminals, set NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR to the directory printed earlier. Do not read `nfcapd.current.*` files while they are being written. Start without aggregation or time filters so received data is not hidden. See the [official nfdump manual](https://github.com/phaag/nfdump/blob/master/man/nfdump.1).

### Distinguish tools named nfdump

If your environment uses `./nfdump -i eth0 -a udp`, that is a live capture/decoding interface. It differs from the workflow above, where nfcapd writes files and official nfdump reads them. Follow that tool's help rather than mixing in options such as `-R` and `-o raw`.

Record each kind of verification separately:

| Evidence | What it confirms |
|---|---|
| `ss` shows a UDP socket | A program listens on that IP/port |
| A network capture contains packets | Packets reached the capture point |
| A capture tool displays correct fields | That tool can decode those fields |
| Collector receipt counters / parsing logs | The collector received and processed data |
| Files, database records, or reports contain data | Data was actually stored |

A complete capture does not prove that a production collector received and stored the data. If needed, open another collector terminal and start capture before sending:

```sh
sudo tcpdump -ni any -nn \
  'host 192.0.2.10 and (udp port 7777 or icmp)'
```

Replace the source IP with the actual PC address. After NAT, the collector may see a different address. To save complete packets for Wireshark, use:

```sh
sudo tcpdump -ni any -s 0 -w /tmp/NeuralEnsembleTelemetryFabricLuminalOracleWizard9-test.pcap 'udp dst port 7777'
```

## HTML JSON builder

Open [config-builder.html](config-builder.html) in a modern browser. The page works offline without a server, account, or network connection, uses muted colors, and does not upload data. Its interface is in US English.

1. For existing JSON, choose a file under **Start here**, or expand **Paste JSON content** and select **Load pasted JSON**.
2. New configurations start with an IPv4 TCP preset. Add IPv6, ICMP, Options, MPLS, or custom flows below.
3. Set the Source ID, template IDs, and field values. Choose single values, multiple values, ranges, prefixes, or random values as needed.
4. Review the preview and field errors. Download `NeuralEnsembleTelemetryFabricLuminalOracleWizard9.json` when the status says **Ready to download**.
5. Run `nfv9gen validate NeuralEnsembleTelemetryFabricLuminalOracleWizard9.json` and `nfv9gen preview NeuralEnsembleTelemetryFabricLuminalOracleWizard9.json`, then send with `nfv9gen send`.

| Feature | Behavior |
|---|---|
| Presets | IPv4/IPv6 TCP, ICMP, Options, MPLS, and custom fields |
| Field search and categories | Search English descriptions, field names, or categories such as `interface` |
| Value modes | Single values, arrays, ranges, IP prefixes, Random between; IN_BYTES/OUT_BYTES also support **Calculate from PKTS** |
| Error locations | Reasons appear beside fields; jump to the first error; errors disable download and copy |
| JSON import | Choose a file or paste JSON; validation completes before replacing the form |
| Duplication and ordering | **Duplicate flow** assigns a new ID; **Duplicate with shared template** keeps the ID; fields and scopes can move up/down |
| Local drafts | Off by default; opt in to save the full form, including incomplete input; restore or delete later |
| Leave-page reminder | Requests a browser warning for changes not yet downloaded; the browser controls the actual dialog |

See [IN/OUT counters](#inout-counters) for linked IN_BYTES/OUT_BYTES controls and JSON syntax.

Import does not silently skip unknown content. Unknown properties, `random_file` IP pools, ICMP ranges with named or object endpoints, and integers that cannot be represented exactly are rejected while preserving the existing form. Valid CLI configurations with these features can still be used from the command line. HTML range steps are limited to `9007199254740991`; the CLI accepts larger u64 steps. ASNs, RDs, and IP prefixes are normalized to equivalent formats before export.

Drafts exist only in the current browser. Enable autosave again each time you open the page. Turning autosave off keeps the existing draft; deleting the draft also disables autosave. Storage restrictions or insufficient space produce an error. Downloading clears the leave-page reminder, but confirm that the file was saved. Copying JSON does not clear the reminder. If clipboard access is restricted, the page selects the JSON for manual copying.

The data packet count assumes the default `--packet-size 1400`. Use CLI results after changing packet-size. The HTML file works by itself; its footer help links additionally require the README.

<a id="configuration-format"></a>

## Configuration format

JSON structural keys are fixed, including `source_id`, `flows`, `template_id`, `fields`, `name`, and `value`. **Short aliases `n`/`v` are not supported.** Field names such as `IPV4_SRC_ADDR` are case-insensitive. Unknown properties or fields fail before socket creation.

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
        {"name":"TCP_FLAGS","value":"SA"},
        {"name":"IN_PKTS","value":10},
        {"name":"IN_BYTES","value":1000}
      ]
    }
  ]
}
```

| Item | Rule |
|---|---|
| `source_id` | Optional; default 0; range `0..=4294967295`; not a source IP |
| `flows` | At least one; values expand independently per flow; flows may share a template |
| `template_id` | `256..=65535`; shared IDs require identical scope, field names, order, and lengths |
| `fields` | Ordered field array; at least one field |
| `scope` | Optional; a nonempty scope creates Options templates/data and is encoded before regular fields |
| `name`, `value` | Both required for every field; scope and fields each reject duplicate names |
| File size limit | 8 MiB |
| Expansion limit | At most 500,000 records across all flows, including Options |

### Share a template across flows

Multiple flows may use the same `template_id` to preserve groups of field values. This works for IP pairs, ports, protocols, flags, counters, and Options. Flows sharing an ID must have identical scope, field names, order, and lengths. Values and value modes may differ; structural conflicts reject the entire configuration.

Each flow still forms its own Cartesian product; values do not combine across flows. A shared template is created once and sent once initially and at each refresh. Data packets are built separately per flow; records from different flows are not merged into one data packet.

[shared-template.json](examples/shared-template.json) uses one template for two groups:

| Flow | Source IP | Destination IP | Destination port | IN_PKTS | IN_BYTES |
|---|---|---|---:|---:|---:|
| 1 | 192.0.2.1 | 198.51.100.1 | 443 | 10 | 1000 |
| 2 | 192.0.2.2 | 198.51.100.2 | 80 | 20 | 3000 |

Only these two records are produced; source .1 does not combine with destination .2 or other cross-flow values:

```sh
./target/release/nfv9gen validate examples/shared-template.json
./target/release/nfv9gen preview examples/shared-template.json --template-id 256 --kind data
```

Expect **one template, two records, and two data packets**. Each flow must supply all its own field values; values are not inherited. In HTML, select **Duplicate with shared template** and edit the values. If you change structure, update all flows sharing the ID or assign a new ID. Different IDs are not merged automatically, even when their structures match.

### Multiple values, IP prefixes, and ranges

These are individual entries for `fields`, not complete configurations:

```json
{"name":"L4_DST_PORT","value":[80,443]}
```

```json
{"name":"IPV4_SRC_ADDR","value":{"start":"192.0.2.1","end":"192.0.2.3","step":1}}
```

```json
{"name":"L4_DST_PORT","value":{"start":10000,"end":10005,"step":2}}
```

Arrays must not be empty. Values across fields form a Cartesian product: two sources × two destinations × two ports = eight records. The last field varies fastest. Range `step` defaults to 1 and must be positive. The final example produces 10000, 10002, and 10004; end need not be included. The CLI accepts steps up to `18446744073709551615`. RDs do not support ranges. MPLS label range endpoints must be label integers. `IP_PROTOCOL_VERSION` ranges may produce only 4 or 6.

IP fields also support CIDR:

```json
{"name":"IPV4_SRC_ADDR","value":"1.1.1.0/32"}
```

```json
{"name":"IPV4_SRC_ADDR","value":{"prefix":"192.0.2.0/30","hosts_only":true}}
```

An address ending in `.0` is valid; the first example produces only `1.1.1.0`. Prefixes clear host bits first and include every address by default; `192.0.2.3/30` produces `.0` through `.3`. `hosts_only:true` excludes IPv4 network and broadcast addresses, except `/31` keeps both addresses and `/32` keeps its one address. IPv6 `/126` produces four addresses and `/128` one; `hosts_only:true` is not accepted for IPv6. Prefixes do not add mask fields automatically.

Prefix, range, and random objects belong directly in `value`. Arrays hold discrete values and do not expand nested objects of these kinds.

### Record counts, packets, and configuration limits

| Term | Meaning |
|---|---|
| Configured records | Multiply scope/field value counts per flow, then add all flows; maximum 500,000 |
| Data packets per cycle | Records split by packet-size; one packet may contain multiple records; includes Options data |
| `--count N` | Stop after N packets; all/data count data packets, templates counts template packets |
| `--rate N` | Target packets per second; not records per second or simulated traffic bytes |
| Cumulative records | Successfully sent data/Options records, including repeats; not limited by configured expansion |

Source `/24` × destination `/24` produces `256 × 256 = 65,536` records: each source combines with all 256 destinations. With `hosts_only:true` on both sides, this becomes `254 × 254 = 64,516`. Adding ten ports exceeds 500,000 records. Single PKTS/BYTES values do not add combinations.

In the default all mode, initial/refreshed templates are extra and do not count toward count/rate. There is no option to stop by cumulative record count. Each random_between field counts as one value, adds no configured expansion, and does not guarantee unique connections.

### Expected collector average rates per minute

`preview` estimates collector IN/OUT average bps and pps after per-minute aggregation, without creating sockets or sending packets. The send plan shows the same estimates:

```sh
./target/release/nfv9gen preview examples/basic.json --kind data --rate 20
```

The basic example has one record and one data packet per cycle, with IN_PKTS=10 and IN_BYTES=1000. Expected table:

```text
Template         IN bps         IN pps        OUT bps        OUT pps
     256      160000.00         200.00              -              -
```

Calculation:

```text
Expected average bps per template = template BYTES total per cycle × 8 × rate ÷ all data packets per cycle
Expected average pps per template = template PKTS total per cycle × rate ÷ all data packets per cycle
```

Each regular template occupies one row, showing IN/OUT bps and pps with bandwidth first. Flows sharing a template_id are summed; different IDs remain separate. Multiple values are summed over their Cartesian product. The rate is shared by the full configuration, rather than granted separately to each template. Options do not represent simulated traffic, but their data packets consume sending time and count in the denominator. Random fields use the expectation `(min + max) / 2`; actual samples vary.

If any regular flow in a template lacks a direction's PKTS or BYTES, the corresponding table cell shows `-`. Other templates are unaffected. Missing values are not treated as zero; explicit zero is counted as zero. Template-only sending has no traffic estimates. Options templates show their IDs and `Options`, without traffic values.

These expected rates appear once in `preview` and before `send`. Periodic sending statistics do not recalculate them; they report actual exported packets, records, and UDP payload bytes. Those bytes are not IN_BYTES/OUT_BYTES.

These are **expected collector average bps/pps after per-minute aggregation**, assuming continuous sending at the target rate, complete receipt, aggregation by receipt time, and no sampling multiplier. They are not actual UDP throughput. These long-term averages need not match every 60-second interval; shorter runs show the same extrapolated rates. Collectors distributing records by FIRST_SWITCHED/LAST_SWITCHED may produce different results. `--limit` and `--template-id` filter packet previews without narrowing estimates. Short runs, burst timing, template overhead, and actual system/network performance affect observations. Both preview and send default to rate 20.

### Memory usage

All packets are built before sending and retained in RAM for reuse. The list is not duplicated according to `--count` or `--duration`. The 500,000-record expansion limit is a program safeguard, not a NetFlow protocol limit or a hard 50 MB RAM limit. More fields and longer records use more memory.

Prefer prefixes/ranges for consecutive addresses or numbers, and arrays for discrete values. Prefixes/ranges store a start and count without first listing every value. Arrays use contiguous bytes. Original JSON is released before packet construction, and used field data before sending; parsing large JSON arrays still adds overhead.

Previously recorded release measurements used `validate` to build 500,000 records with one `IN_BYTES` field, measuring process peak RSS five times per case:

| Value representation | Peak RAM |
|---|---:|
| Range `0..=499999` | About 5 MiB |
| JSON array listing 500,000 integers | About 22 MiB |

The same array case used about 48 MiB before optimization, a reduction of about 55%. These are measurements for one configuration, not guarantees for all configurations or long runs. On Linux, measure your own peak with `/usr/bin/time -v ./target/release/nfv9gen validate NeuralEnsembleTelemetryFabricLuminalOracleWizard9.json`. There is no automatic RAM estimate or memory-budget option.

### IN/OUT counters

`IN_PKTS`, `IN_BYTES`, `OUT_PKTS`, and `OUT_BYTES` are 4-byte nonnegative integers in `0..=4294967295`. They describe traffic inside records, rather than the actual UDP packets sent by NeuralEnsembleTelemetryFabricLuminalOracleWizard9, and do not accumulate automatically on repeated sends.

In HTML, choose **Calculate from PKTS** under **Value mode** for IN_BYTES/OUT_BYTES, then enter a positive integer average packet size in bytes/packet. The same flow must contain the corresponding PKTS field as a single value:

| Direction | PKTS | Average size | Calculated BYTES |
|---|---:|---:|---:|
| IN | 10 | 100 bytes / packet | 1000 |
| OUT | 20 | 150 bytes / packet | 3000 |

Changing PKTS or average size recalculates BYTES. Missing PKTS, multiple/range/random PKTS, or a result above the limit disables download. JSON stores the calculated integer, without a formula or average-size property. Put these four entries in the same flow's `fields`:

```json
[
  {"name":"IN_PKTS","value":10},
  {"name":"IN_BYTES","value":1000},
  {"name":"OUT_PKTS","value":20},
  {"name":"OUT_BYTES","value":3000}
]
```

The calculation mode and average size are retained only in browser drafts. Enable them again after importing JSON. When editing JSON PKTS manually, update BYTES yourself. `random_between` supports IN counters and L4 ports; OUT counters do not support random mode.

### Interfaces and next hops

`INPUT_SNMP` and `OUTPUT_SNMP` are input/output ifIndexes inside records. Both are 4 bytes in `0..=4294967295`; names such as `eth0` are not accepted. They do not select the actual sending interface. Options `scope.INTERFACE` identifies the interface to which options apply.

For IPv4 next hops, use `IPV4_NEXT_HOP` and `BGP_IPV4_NEXT_HOP`; for IPv6, use `IPV6_NEXT_HOP` and `BGP_IPV6_NEXT_HOP`. Single values, arrays, prefixes, and ranges are supported, with the matching IP version. These addresses do not change OS routing.

Add these entries to `fields` to test 32-bit ifIndexes, ASNs, and BGP next hops:

```json
[
  {"name":"INPUT_SNMP","value":70001},
  {"name":"OUTPUT_SNMP","value":70002},
  {"name":"SRC_AS","value":"AS1.10"},
  {"name":"BGP_IPV4_NEXT_HOP","value":"192.0.2.254"}
]
```

`AS1.10` equals 65546. ASN dots and RD colons allow surrounding whitespace, so `"AS1 . 10"` and `"65000 : 100"` are accepted. Records containing IPv6 traffic can be transported over IPv4 UDP; JSON addresses and the actual transport version are configured separately.

### Updating older configurations

Older configuration syntax is no longer accepted:

- Replace `{"random":true,"min":49152,"max":65535}` with `{"random_between":[49152,65535]}`. Specify both bounds; if the old bounds were omitted, use `[1,65535]`.
- Replace flow-level `mpls_stack` with explicit label and metadata entries in `fields`. Preserve field order, label TC/BoS, stack depth, stack length, and top-label EXP; see [convenience.json](examples/convenience.json).
- In `random_file` lists, use plain IP addresses. To preserve the former single-host behavior, change `198.51.100.0/24` to `198.51.100.0`. A list entry does not expand a subnet.

### Random L4 ports, packet counts, and bytes

`L4_SRC_PORT`, `L4_DST_PORT`, `IN_PKTS`, and `IN_BYTES` accept:

```json
{"name":"L4_SRC_PORT","value":{"random_between":[49152,65535]}}
{"name":"IN_PKTS","value":{"random_between":[10,100]}}
{"name":"IN_BYTES","value":{"random_between":[1000,100000]}}
```

Bounds are inclusive and min≤max. L4 ports must be integers in `0..=65535`; IN_PKTS/IN_BYTES must be in `0..=4294967295`. Equal bounds produce a fixed value. Every record is resampled on every send, including repeated packets; duplicates are allowed. Fields are sampled independently without enforcing a packets/bytes ratio. Only record values change, not actual UDP ports.

`validate` and `preview` do not sample; preview hex uses min. Send uses buffered `/dev/urandom` with no fixed seed. Per-record sampling adds work, so fixed-packet performance estimates do not apply.

### Random IP addresses from a file

Any IP field accepts `{"random_file":"PATH"}`. Each record draws one line from the file on every send:

```json
{"name":"IPV4_SRC_ADDR","value":{"random_file":"random-ips.txt"}}
```

Put one address per line. Blank lines and lines starting with `#` are skipped. Each entry must be a plain IP address; CIDR prefixes such as `198.51.100.0/24` are rejected. The address version must match the field. Relative paths are resolved against the JSON file's folder. Invalid entries report the file path and line number. The file must not exceed 8 MiB, and it counts as one configured value. Preview hex uses the first address. See `examples/random-ips.json`.

### TCP flags, ASNs, and ICMP

| Type | Syntax and limits |
|---|---|
| TCP flags | `"SA"`, `"SYN\|ACK"`, integer `18`, and `"0x12"` are equivalent; range 0–255 |
| Short flag letters | F, S, R, P, A, U, E, C mean FIN, SYN, RST, PSH, ACK, URG, ECE, CWR |
| Full flag names | Separate with `\|`; `"SYNACK"`, empty strings, and decimal strings such as `"18"` are rejected |
| ASN | `65000`, `"AS65000"`, `"1.10"`, `"AS1.10"`; full range 0–4294967295 |
| asdot | `high × 65536 + low`; both parts 0–65535; `"1.10"` = 65546 and must be a string |
| ASN encoding | SRC_AS/DST_AS are fixed at 4 bytes; large ASNs are not truncated to 16 bits |
| ICMP | Use ICMP_TYPE for IPv4 and ICMP_TYPE_IPV6 for IPv6; both are 2 bytes |
| ICMP values | `{"type":8,"code":0}` or combined integer `2048`; code defaults to 0 |
| ICMP encoding | `type × 256 + code`; type/code each 0–255; PROTOCOL is not set automatically |

TCP flags are case-insensitive and repeated flags are combined with OR. `"ECE"` is 0x40; `"EC"` is ECE+CWR = 0xc0. These ICMP names work directly as value or inside type:

| ICMP name | IPv4 type | IPv6 type |
|---|---:|---:|
| echo-request | 8 | 128 |
| echo-reply | 0 | 129 |
| destination-unreachable | 3 | 1 |
| time-exceeded | 11 | 3 |
| parameter-problem | 12 | 4 |
| packet-too-big | Not applicable | 2 |

For IPv4 ICMP, set `PROTOCOL` to 1 and use `ICMP_TYPE`; for IPv6 ICMP, set `PROTOCOL` to 58 and use `ICMP_TYPE_IPV6`. Remove inapplicable TCP flags and L4 ports. Their echo-request values encode as 2048 (8/0) and 32768 (128/0), respectively.

```json
{"name":"TCP_FLAGS","value":"SYN|ACK"}
```

The `|` separates flag names. Case and surrounding whitespace are accepted, such as `"syn | ack"`. Do not concatenate full names into `"SYNACK"`.

### MPLS configuration

Manual fields use actual label values; do not shift them beforehand:

```json
{"name":"MPLS_LABEL_1","value":{"label":16000,"tc":3,"bos":false}}
```

Labels range from 0 to 1048575, TC from 0 to 7, and BoS marks the bottom of the stack. Plain integer labels use TC=0 and BoS=false. Each label section exports 3 bytes as `(label << 4) | (tc << 1) | bos`, without TTL. Use `MPLS_TOP_LABEL_TTL` for TTL. Manual fields do not infer BoS, depth, length, or EXP.

Set each `MPLS_LABEL_1`–`MPLS_LABEL_10` and any stack metadata explicitly in `fields`, as shown in [convenience.json](examples/convenience.json). Set BoS=true only on the last label. Stack length uses the original four bytes per MPLS layer, rather than the exported 3-byte section.

RDs are fixed at 8 bytes and encoded according to [RFC 4364 §4.2](https://www.rfc-editor.org/rfc/rfc4364.html#section-4.2):

| RD syntax | Type | Range |
|---|---|---|
| `"65000:100"` | Type 0 | 16-bit ASN + 32-bit number |
| `"192.0.2.1:101"` | Type 1 | IPv4 + 16-bit number |
| `"4200000000:102"` | Type 2 | 32-bit ASN + 16-bit number |
| `"2:65000:100"` | Explicit type | Type 0, 1, or 2; same limits as above |

ASN administrators support AS prefixes and asdot. Without an explicit type, ASN≤65535 selects Type 0; larger ASNs select Type 2. RDs support arrays but not ranges. Extra metadata IDs/definitions follow the [IANA IPFIX registry](https://www.iana.org/assignments/ipfix); the collector must handle their fixed lengths in v9 templates.

### Options

A nonempty scope creates Options templates/data. Each scope retains its own values and order; scope and regular fields participate together in Cartesian expansion.

| Scope | Type ID | Bytes |
|---|---:|---:|
| SYSTEM | 1 | 4 |
| INTERFACE | 2 | 4 |
| LINE_CARD | 3 | 4 |
| NETFLOW_CACHE | 4 | 4 |
| TEMPLATE | 5 | 2 |

A complete configuration with sampling interval=1:

```json
{
  "source_id": 42,
  "flows": [{
    "template_id": 300,
    "scope": [{"name":"SYSTEM","value":1}],
    "fields": [
      {"name":"SAMPLING_INTERVAL","value":1},
      {"name":"SAMPLING_ALGORITHM","value":1}
    ]
  }]
}
```

See [options.json](examples/options.json) for multiple scopes. Its interval is 100, so collectors may scale packet/byte counts. The all-fields test uses interval=1 to avoid that effect. Options generally do not appear as regular flow rows, and supported scopes depend on the collector.

## CLI

```text
nfv9gen validate CONFIG [--packet-size BYTES]
nfv9gen preview CONFIG [--packet-size BYTES] [--kind all|templates|data] [--hex] [--limit N] [--template-id ID] [--rate N]
nfv9gen send CONFIG [OPTIONS]
# Any command also accepts -s/--silent: print only done or fail
```

### Validation and preview

| Option | Default | Description |
|---|---|---|
| `--packet-size BYTES` | 1400 | Shared by all three commands; 64–65507, including the 20-byte NetFlow header, excluding IP/UDP headers |
| `--kind` | all | Preview: all, templates, data; all shows templates before data |
| `--hex` | Off | Show the complete UDP payload in hexadecimal |
| `--limit N` | 8 | Show at most N packets after filtering; at least 1; not records |
| `--template-id ID` | All | Preview only this ID; validate the full configuration and retain full totals and collector rate estimates |
| `--rate N` | 20 | Target data packets/s for preview's collector rate estimate; sends no packets |

Preview lists field lengths, counts, first/last values, and expansion formulas. It uses the current UNIX time, uptime=0, and sequence=0; it does not reuse a send session, read random values, or create sockets.

### Sending

| Option | Default | Description |
|---|---|---|
| `--target host:port` | `127.0.0.1:2055` | Collector address; use `[::1]:2055` for IPv6; port must not be 0 |
| `--bind IP:port` | OS choice | Local source address/port; IP must be configured; port=0 lets the OS select a port |
| `--source IP:port` | None | Source may be nonlocal; requires Linux raw socket permission; conflicts with bind |
| `--kind all` | all | Send initial templates, then data; refresh templates automatically |
| `--kind templates` | — | Templates only; count/rate measure template packets |
| `--kind data` | — | Data only; collector must already have matching templates for that source |
| `--count N` | One cycle of the selected kind | At least 1; all/data count data packets; automatic templates are extra |
| `--duration SECONDS` | None | Exactly `0` means run until Ctrl-C/SIGTERM; positive fractions accepted; tiny/negative/other zero forms rejected; conflicts with count; includes initial templates |
| `--rate N` | 20 | Target packets/s, 1–1000000000 |
| `--mode stream` | stream | Schedule packets individually |
| `--mode burst` | — | Send batches, controlling the average target rate with intervals |
| `--burst-size N` | 64 | At least 1; burst only; actual batch size is capped at rate |
| `--template-interval SECONDS` | 30 | all only; finite positive seconds, fractions accepted |
| `--template-every N` | 20 | all only; at least 1; refresh before the next data packet after every N data packets |
| `--packet-size BYTES` | 1400 | Maximum UDP payload; template descriptions and individual records must fit |

Either refresh threshold triggers all templates before the next data packet. Each run starts sequence at 0; it increments after every successfully sent datagram, shared by templates and data, with natural wrapping.

Before sending, the plan shows source/destination, configuration summary, mode/rate/stop condition, UDP payload range, refresh rules, and rates by template. Use `nfv9gen preview CONFIG --template-id ID` for structure, first/last values, and random ranges. When stdout and stderr are terminals, progress updates in place. If either is redirected, stderr receives statistics every five seconds without terminal control codes: elapsed seconds, cumulative packets/records/bytes, and average packet rate. Use `--duration 0 >send.log 2>&1` to log continuous runs. Completion or Ctrl-C/SIGTERM produces a two-line summary of elapsed hours/minutes/seconds, data/template packets, records, UDP payload MiB, and average data packets/s. Transport errors immediately report failure without a normal completion summary.

High stream rates depend on scheduler precision. Burst reduces individual waiting overhead without guaranteeing the target rate. Configurations and packets are prepared once rather than copied by count. random_between and random_file fields are rewritten before sending. FIRST_SWITCHED/LAST_SWITCHED remain configured values and do not update with time.

### Continuous sending and logs

```sh
# Send 100 data packets; templates are extra
./target/release/nfv9gen send examples/basic.json \
  --target 192.0.2.20:7777 --rate 10 --count 100

# Send for 10 seconds; count and duration cannot be combined
./target/release/nfv9gen send examples/basic.json \
  --target 192.0.2.20:7777 --rate 20 --duration 10

# Run until Ctrl-C/SIGTERM; log statistics every 5 seconds
./target/release/nfv9gen send examples/basic.json \
  --target 192.0.2.20:7777 --rate 20 --duration 0 \
  >send.log 2>&1

# Send in bursts and adjust template refresh thresholds
./target/release/nfv9gen send examples/ranges.json \
  --target 192.0.2.20:7777 --mode burst --burst-size 64 \
  --rate 10000 --count 10000 --template-every 100 --template-interval 5
```

Read the log from another terminal with `tail -f send.log`. Record totals include Options and repeated sends. Bytes mean successfully sent UDP payload; packet rate is the average since startup, not an instantaneous rate. Startup information and the final summary go to stdout; periodic statistics go to stderr. `>send.log` alone omits stderr; add `2>&1`. Runs under five seconds may have only a final summary. Statistics write/flush failures, such as a full disk or closed pipe, stop sending with exit code 1 rather than reporting completion or manual cancellation. If stderr itself fails, the error message may be lost, but output errors do not cause a panic.

`--duration 0` works with all, data, and templates. `--template-interval 0` remains invalid. `0.0`, `-0`, `0e10`, and `1e-400` do not enable unlimited mode and are rejected with exit code 2. Finite positive values too small to represent as a valid Duration are also rejected. Manual interruption exits 130; transport/statistics failures exit 1. Verify receipt and stop signals in your collector environment.

| Send plan item | Meaning |
|---|---|
| Data packets per cycle | Length of the reused packet list, not records per cycle |
| Source → destination | Actual UDP IP:port, separate from simulated JSON addresses |
| Automatic templates | Extra in all mode; excluded from count/rate |
| Records and bytes per packet | Split according to packet-size, which measures UDP payload |
| Expected collector rates | One row per template; IN/OUT bps then pps; missing counters shown as `-` |
| Structure and random ranges | Use preview CONFIG --template-id ID; send does not repeat per-field details |

### Set the actual UDP source IP

```sh
./target/release/nfv9gen send examples/basic.json \
  --target 192.0.2.20:7777 --bind 192.0.2.10:55000
```

```sh
sudo ./target/release/nfv9gen send examples/basic.json \
  --target 192.0.2.20:7777 --source 198.51.100.123:55000
```

Both bind and source must use the same IP version as target. Source must be explicit unicast, both ports must be >0, and IPv6 source scope IDs are rejected. Root or `CAP_NET_RAW` is required for source. Raw mode builds IP/UDP headers and checksums with TTL/hop limit=64 and no fragmentation, subject to routing and path MTU. These options do not rewrite addresses inside JSON records.

When running templates/data separately, preserve source_id, source IP:port, template IDs, and definitions, and check collector cache behavior. For general testing, use `--kind all` in one run.

### Silent mode

Add `-s` / `--silent` to any command to print only `done` or `fail` on stdout, with no progress, statistics, or error details. Exit codes are unchanged. Argument errors are still reported by the argument parser.

```bash
nfv9gen send examples/basic.json --target 127.0.0.1:2055 --count 100 --silent
```

### Exit codes

| Code | Meaning |
|---:|---|
| 0 | Completed; a closed stdout pipe (BrokenPipe) also counts as success |
| 1 | Configuration, parsing, socket, sending, or statistics-output error; stderr BrokenPipe is failure |
| 2 | CLI argument error |
| 130 | Ctrl-C/SIGTERM interruption |

## Supported fields

See [field.md](field.md) for full value ranges, examples, and supported formats.

These 53 regular field IDs, lengths, and types are defined in [src/fields.rs](src/fields.rs). Fixed-width numbers use network byte order. Integers are nonnegative and limited by field width; masks, IP version, MPLS TC, and similar fields have additional semantic limits.

| Name | Type ID | Bytes | Value |
|---|---|---|---|
| IN_BYTES | 1 | 4 | Integer, array, range, or random_between |
| IN_PKTS | 2 | 4 | Integer, array, range, or random_between |
| FLOWS | 3 | 4 | Integer |
| PROTOCOL | 4 | 1 | Integer |
| SRC_TOS | 5 | 1 | Integer |
| TCP_FLAGS | 6 | 1 | Flag letters, full names, integer, or 0x string |
| L4_SRC_PORT | 7 | 2 | Integer, array, range, or random_between |
| IPV4_SRC_ADDR | 8 | 4 | IPv4 |
| SRC_MASK | 9 | 1 | IPv4 mask |
| INPUT_SNMP | 10 | 4 | Input interface ifIndex, 0..=4294967295 |
| L4_DST_PORT | 11 | 2 | Integer, array, range, or random_between |
| IPV4_DST_ADDR | 12 | 4 | IPv4 |
| DST_MASK | 13 | 1 | IPv4 mask |
| OUTPUT_SNMP | 14 | 4 | Output interface ifIndex, 0..=4294967295 |
| IPV4_NEXT_HOP | 15 | 4 | IPv4 |
| SRC_AS | 16 | 4 | ASN integer / ASplain / AS prefix / asdot |
| DST_AS | 17 | 4 | ASN integer / ASplain / AS prefix / asdot |
| BGP_IPV4_NEXT_HOP | 18 | 4 | IPv4 |
| LAST_SWITCHED | 21 | 4 | Integer, milliseconds |
| FIRST_SWITCHED | 22 | 4 | Integer, milliseconds |
| OUT_BYTES | 23 | 4 | Integer |
| OUT_PKTS | 24 | 4 | Integer |
| IPV6_SRC_ADDR | 27 | 16 | IPv6 |
| IPV6_DST_ADDR | 28 | 16 | IPv6 |
| IPV6_SRC_MASK | 29 | 1 | IPv6 mask |
| IPV6_DST_MASK | 30 | 1 | IPv6 mask |
| ICMP_TYPE | 32 | 2 | IPv4 type/code object, name, or combined integer |
| SAMPLING_INTERVAL | 34 | 4 | Integer |
| SAMPLING_ALGORITHM | 35 | 1 | Integer |
| MPLS_TOP_LABEL_TYPE | 46 | 1 | Integer |
| MPLS_TOP_LABEL_IP_ADDR | 47 | 4 | IPv4 |
| DST_TOS | 55 | 1 | Integer |
| IP_PROTOCOL_VERSION | 60 | 1 | 4 or 6 |
| IPV6_NEXT_HOP | 62 | 16 | IPv6 |
| BGP_IPV6_NEXT_HOP | 63 | 16 | IPv6 |
| MPLS_LABEL_1 | 70 | 3 | 20-bit label integer or object |
| MPLS_LABEL_2 | 71 | 3 | 20-bit label integer or object |
| MPLS_LABEL_3 | 72 | 3 | 20-bit label integer or object |
| MPLS_LABEL_4 | 73 | 3 | 20-bit label integer or object |
| MPLS_LABEL_5 | 74 | 3 | 20-bit label integer or object |
| MPLS_LABEL_6 | 75 | 3 | 20-bit label integer or object |
| MPLS_LABEL_7 | 76 | 3 | 20-bit label integer or object |
| MPLS_LABEL_8 | 77 | 3 | 20-bit label integer or object |
| MPLS_LABEL_9 | 78 | 3 | 20-bit label integer or object |
| MPLS_LABEL_10 | 79 | 3 | 20-bit label integer or object |
| MPLS_VPN_RD | 90 | 8 | RD string, Type 0 / 1 / 2 |
| MPLS_TOP_LABEL_PREFIX_LEN | 91 | 1 | FEC prefix length, 0–128 |
| ICMP_TYPE_IPV6 | 139 | 2 | IPv6 type/code object, name, or combined integer |
| MPLS_TOP_LABEL_IPV6_ADDR | 140 | 16 | IPv6 FEC address |
| MPLS_TOP_LABEL_TTL | 200 | 1 | Top label TTL, 0–255 |
| MPLS_LABEL_STACK_LENGTH | 201 | 4 | Original MPLS stack length in bytes |
| MPLS_LABEL_STACK_DEPTH | 202 | 4 | Label count in the original MPLS stack |
| MPLS_TOP_LABEL_EXP | 203 | 1 | Top label EXP/TC, 0–7 |

## Examples and all-fields testing

| Configuration | Contents |
|---|---|
| [basic.json](examples/basic.json) | One IPv4 TCP record |
| [shared-template.json](examples/shared-template.json) | Multiple value groups share one template without cross-flow combinations |
| [interfaces.json](examples/interfaces.json) | INPUT_SNMP/OUTPUT_SNMP |
| [nexthop.json](examples/nexthop.json) | Four IPv4/IPv6 next-hop fields |
| [random-ports.json](examples/random-ports.json) | Resample the source L4 port on each send |
| [random-ips.json](examples/random-ips.json) | Draw IPV4_SRC_ADDR from [random-ips.txt](examples/random-ips.txt) on each send |
| [random-counters.json](examples/random-counters.json) | Resample IN_PKTS/IN_BYTES on each send |
| [ranges.json](examples/ranges.json) | IP ranges, arrays, and field combinations |
| [options.json](examples/options.json) | Sampling Options with multiple scopes |
| [asn.json](examples/asn.json) | ASN notation and RDs |
| [mpls.json](examples/mpls.json) | Manual labels, RDs, IPv4/IPv6 FEC, and metadata |
| [convenience.json](examples/convenience.json) | ICMP, prefixes, ranges, and explicit MPLS stack fields |
| [collector-all-fields.json](examples/collector-all-fields.json) | All 53 regular fields and five scopes across 14 templates |

The all-fields configuration has 14 data packets and 22 records per cycle: 17 regular flow records and five Options records. Start the collector on the destination port, then run:

```sh
./target/release/nfv9gen send examples/collector-all-fields.json \
  --target 192.0.2.20:7777 \
  --kind all --rate 10 --count 140 --template-every 14
```

This sends ten cycles: 170 regular records and 50 Options records. Without loss, filtering, or unsupported templates, expect 170 unaggregated regular flow rows. Options are not regular flow rows. Collector sampling can affect counts; this configuration uses sampling interval 1.

Validation should report Source ID=424242, 14 templates, 22 records, and 14 data packets. Preview one group with:

```sh
./target/release/nfv9gen preview examples/collector-all-fields.json \
  --template-id 1005 --kind data --hex --limit 1
```

See [COLLECTOR-TEST.md](COLLECTOR-TEST.md) for field coverage, expected groups, and nfcapd/nfdump commands. A previously recorded live capture confirmed 28 packets per cycle (sequence 0–27), with 17+5 records, `.0/32`, 32-bit ifIndex boundaries, and ten MPLS labels matching the configuration. That decoder displayed Type 1/2 RDs as two 32-bit integers and some fields only by ID. This confirms packet arrival and that decoder's behavior, not production collector storage or preservation of every field.

## JSON size and distribution

`collector-all-fields.json` is 15,629 bytes. Removing indentation produces 7,259 bytes, about 53.6% smaller, with equivalent configuration and packet contents. Only whitespace is removed; keys remain name/value. Prefixes and ranges can reduce repetition but must preserve the intended fields and values. Changing field order changes templates, so these are not universally lossless substitutions.

Use Python to remove indentation, then validate the result. Running NeuralEnsembleTelemetryFabricLuminalOracleWizard9 itself does not require Python:

```sh
python3 -c 'import json,sys; json.dump(json.load(sys.stdin),sys.stdout,separators=(",",":"))' \
  < NeuralEnsembleTelemetryFabricLuminalOracleWizard9.json > NeuralEnsembleTelemetryFabricLuminalOracleWizard9.min.json
./target/release/nfv9gen validate NeuralEnsembleTelemetryFabricLuminalOracleWizard9.min.json
```

Removing whitespace does not shrink NetFlow packets. Conversion tools may lose precision for large numbers written as floating-point values; use the required integer format.

Release enables `lto="thin"`, `codegen-units=1`, and `strip=true`. No packer, obfuscation, or reverse-engineering protection is included. UPX is an optional executable-compression step, can be unpacked, and does not guarantee protection; test compatibility and performance separately. See the [UPX documentation](https://github.com/upx/upx/blob/devel/doc/upx.pod).

Distribute the release executable, required JSON, HTML, and documentation. Source and target/debug need not be included. Linux executables still require a compatible OS and dynamic libraries; they are not cross-platform single files. Executable size, JSON size, and NetFlow wire packet size are different measurements.

## Troubleshooting

| Symptom | Check and action |
|---|---|
| `Connection refused (os error 111)` | Check the target UDP listener and firewall first; NeuralEnsembleTelemetryFabricLuminalOracleWizard9 stops on the error; do not assume a field-format problem |
| Capture shows packets, production system has no data | Check UDP receipt, source allowlists, template parsing, file writes, and ingestion logs separately |
| `invalid netflow version` | An all-UDP filter may include unrelated traffic; narrow it to the real source IP/destination port and retain a pcap |
| Stops after a short send | Without count/duration, only one cycle is sent; use --duration 0 for continuous sending |
| No periodic statistics in logs | Statistics go to stderr about every five seconds; use >send.log 2>&1; short runs may show only a final summary; write failures exit 1 |
| Duration rejected | Unlimited mode accepts exactly 0; other values must be representable finite positive seconds; other zero forms and underflow are rejected |
| Need simulated rates per template | Use preview CONFIG --rate N; IN/OUT appear per template, shared IDs are summed, and sending does not recalculate them |
| High RAM usage | Prefer prefixes/ranges for regular values; large JSON arrays add parsing overhead; 500,000 records does not guarantee less than 50 MB |
| BYTES does not follow PKTS | Linking exists only in HTML's Calculate from PKTS mode; JSON stores results; enable again after import |
| Shared template structures differ | Scope, names, order, and lengths must match; change values only or assign a new ID |
| Collector cannot find a template | Start with all; for separate templates/data runs, fix the source and source_id and check definitions |
| Invalid JSON | Use double quotes; remove comments/trailing commas; do not abbreviate name/value to n/v |
| random_file cannot be read | Relative paths resolve against the JSON file's folder, not the current directory; use UTF-8, at most 8 MiB, and one plain IP per line without CIDR prefixes; check the reported path and line number |
| Wrong IP version | Match IPv4/IPv6 values to their fields; transport and record IP versions are configured separately |
| More than 500,000 expanded records | Multiply value counts per field, then sum all flows; random_between and random_file each count as one value |
| Template or record does not fit | Increase packet-size reasonably or reduce fields, while respecting path MTU |
| `Message too long` | Reduce packet-size, especially in raw mode without fragmentation; do not rely only on the allowed maximum |
| Bind fails | Use an IP configured locally; check IP version and port availability |
| Raw socket permission denied | source mode requires Linux root or CAP_NET_RAW; regular UDP does not require sudo |
| Interface value differs | Check a 4-byte template length and whether values above 65535 are truncated |
| ICMP appears as 2048 or 32768 | These are combined type/code values: 8/0 and 128/0 |
| MPLS RD Type 1/2 differs | Check type-specific decoding and raw bytes; not every RD is two 32-bit integers |
| Packet/byte counts grow | Check collector sampling multipliers and aggregation; the all-fields baseline interval is 1 |
| Draft not saved or restored | Enable drafts explicitly; they exist only in this browser; check storage restrictions and download a backup |

For issue reports, retain the command, NeuralEnsembleTelemetryFabricLuminalOracleWizard9 and collector versions, send summary, receipt logs, and a pcap when needed. Compare packets with the configuration before attributing the issue to sending, decoding, storage, or display.

## Development checks

```sh
cargo test --locked
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
node --test tests/*.test.cjs
```

Default Rust tests cover validation, field encoding, packet splitting, shared templates and refresh, collector rates by template, scheduling, cancellation, CLI behavior, and full field coverage. They also check duration underflow rejection, stopping on statistics write/flush/BrokenPipe failures, avoiding panics when stderr fails, progress and cancellation within slow bursts, and completing count/duration runs without a trailing progress callback. Configuration tests reject removed syntax and CIDR entries in IP pools; CLI checks validate all 13 bundled JSON examples and the random-file preview. Node.js is only needed for development tests. Tests cover generation cases and import/export round trips, IN/OUT calculations, the 500,000-record boundary, and event handling with DOM stubs. They do not replace browser layout, native dialogs, clipboard, or download tests.

The build-script checks simulate distributions, package managers, and Rust installation without changing the host system. They cover tool reuse, sudo boundaries, download/build failures, and invocation from paths containing spaces.

These three checks do not run by default:

```sh
cargo test --locked --test protocol udp_loopback -- --ignored
sudo cargo test --locked --lib raw_loopback -- --ignored
cargo test --locked --release --test protocol offline_throughput -- --ignored --nocapture
```

The first two need OS permission for sockets; raw loopback additionally requires Linux and CAP_NET_RAW. Offline performance excludes UDP syscalls, networks, and collectors, so it cannot establish wire throughput. Real-system verification still requires send summaries, receiver logs, and pcaps when needed.

## License

[PolyForm Noncommercial 1.0.0](LICENSE). Free for study, research, personal and other noncommercial use, including modified builds. Commercial use (selling it, or selling modified versions) is not allowed.
