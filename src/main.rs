mod preview;
mod progress;

use std::{
    io::{self, IsTerminal, Write},
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    path::PathBuf,
    process::ExitCode,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result, ensure};
use clap::{Args, Parser, Subcommand};
use neural_ensemble_telemetry_fabric_luminal_oracle_wizard9::{
    config,
    protocol::{self, Plan},
    sender::{Kind, Mode, Settings},
};

static STOP: AtomicBool = AtomicBool::new(false);

#[derive(Parser)]
#[command(
    name = "nfv9gen",
    version,
    about = "NetFlow v9 / IPFIX traffic generator for testing collectors",
    after_help = "Workflow: validate → preview → send. Run nfv9gen <COMMAND> -h for command options. Add --ipfix for IPFIX output.\nExamples:\n  nfv9gen validate examples/basic.json\n  nfv9gen preview examples/basic.json --hex\n  nfv9gen send examples/basic.json --target 127.0.0.1:2055 --count 100\nConfiguration format and fields: README.md, field.md, examples/. config-builder.html builds JSON in a browser.\nSource and releases: https://github.com/echoapus/NeuralEnsembleTelemetryFabricLuminalOracleWizard9\nExit codes: 0 success, 1 configuration or runtime error, 2 argument error, 130 interrupted by user."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    /// Print nothing but a final "done" or "fail"; the exit code is unchanged
    #[arg(short, long, global = true)]
    silent: bool,
}

#[derive(Args)]
struct Input {
    /// JSON configuration path (maximum 8 MiB and 500,000 expanded records)
    config: PathBuf,
    /// Maximum UDP payload in bytes (64..=65507), including the NetFlow/IPFIX header
    #[arg(long, default_value_t = protocol::DEFAULT_PACKET_SIZE as u32, value_parser = clap::value_parser!(u32).range(64..=65507))]
    packet_size: u32,
    /// Use IPFIX (version 10) instead of NetFlow v9; needed for DATALINK_FRAME_SECTION
    #[arg(long)]
    ipfix: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Validate all fields, ranges, and packet sizes without creating a socket
    #[command(
        after_help = "Checks the JSON, field values, expanded record count, and that templates and records fit in a packet.\nExamples:\n  nfv9gen validate examples/interfaces.json\n  nfv9gen validate examples/ranges.json --packet-size 1200"
    )]
    Validate {
        #[command(flatten)]
        input: Input,
    },
    /// Preview prebuilt packets without creating a socket
    #[command(
        after_help = "Shows field lengths, value counts, and first/last values. Uses the current time, uptime=0, and sequence=0; random_between fields use their minimum value.\nExamples:\n  nfv9gen preview examples/interfaces.json --hex\n  nfv9gen preview examples/convenience.json --template-id 601 --kind data --limit 2"
    )]
    Preview {
        #[command(flatten)]
        input: Input,
        /// Packets to show: all (templates, then data), templates, or data
        #[arg(long, value_enum, default_value_t = Kind::All)]
        kind: Kind,
        /// Show the full UDP payload in hexadecimal
        #[arg(long)]
        hex: bool,
        /// Maximum packets to show (at least 1); not a record count
        #[arg(long, default_value_t = 8, value_parser = clap::value_parser!(u32).range(1..))]
        limit: u32,
        /// Show only this Template ID (256..=65535); totals still cover the full configuration
        #[arg(long, value_parser = clap::value_parser!(u16).range(256..))]
        template_id: Option<u16>,
        /// Target data packets/s for the expected collector rates (sends nothing)
        #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..=1_000_000_000))]
        rate: u32,
    },
    /// Send UDP; by default, send templates followed by one cycle of data
    #[command(
        after_help = "Without --count or --duration, sends all templates and then one cycle of data at 20 packets/s.\n--kind data needs a collector that already knows the templates (same source_id, source IP:port, and definitions).\nUDP success means the OS accepted the packet, not that the collector received it.\nCtrl-C stops sending, prints statistics, and exits with code 130.\nExamples:\n  nfv9gen send examples/basic.json --target 127.0.0.1:2055 --count 100\n  nfv9gen send examples/ranges.json --rate 1000 --duration 10\n  sudo nfv9gen send examples/basic.json --target 192.0.2.10:2055 --source 198.51.100.123:5000"
    )]
    Send {
        #[command(flatten)]
        input: Input,
        /// Collector IP/hostname and port, such as [::1]:2055 for IPv6
        #[arg(long, default_value = "127.0.0.1:2055")]
        target: String,
        /// Local IP:port to send from (default: OS choice); conflicts with source
        #[arg(long)]
        bind: Option<SocketAddr>,
        /// Set the UDP source IP:port, even a nonlocal one (needs root or CAP_NET_RAW); conflicts with bind
        #[arg(long, conflicts_with = "bind")]
        source: Option<SocketAddr>,
        /// all sends templates then data and refreshes templates; templates or data sends only that kind
        #[arg(long, value_enum, default_value_t = Kind::All)]
        kind: Kind,
        /// Stop after N packets (default: one cycle); templates are counted only with --kind templates
        #[arg(long, conflicts_with = "duration", value_parser = clap::value_parser!(u64).range(1..))]
        count: Option<u64>,
        /// Run for this many seconds (0 runs until Ctrl-C); conflicts with count
        #[arg(long, value_parser = send_duration)]
        duration: Option<Duration>,
        /// Target packets per second (1..=1000000000)
        #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..=1_000_000_000))]
        rate: u32,
        /// stream sends packets one by one; burst sends them in batches
        #[arg(long, value_enum, default_value_t = Mode::Stream)]
        mode: Mode,
        /// Packets per batch in burst mode (at least 1)
        #[arg(long, default_value_t = 64, value_parser = clap::value_parser!(u32).range(1..))]
        burst_size: u32,
        /// With kind=all: refresh templates after this many seconds
        #[arg(long, default_value = "30", value_parser = duration)]
        template_interval: Duration,
        /// With kind=all: refresh templates after every N data packets (at least 1)
        #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u64).range(1..))]
        template_every: u64,
    },
}

fn send_duration(text: &str) -> Result<Duration, String> {
    if text == "0" {
        Ok(Duration::ZERO)
    } else {
        duration(text)
    }
}

fn duration(text: &str) -> Result<Duration, String> {
    let seconds = text
        .parse::<f64>()
        .map_err(|_| "Enter a duration greater than 0 seconds, such as 1 or 0.5".to_owned())?;
    let value = Duration::try_from_secs_f64(seconds)
        .map_err(|_| "Seconds must be finite, positive, and within the timer range".to_owned())?;
    if value.is_zero() {
        return Err("Seconds must be greater than 0".to_owned());
    }
    Ok(value)
}

fn prepare(input: &Input) -> Result<Plan> {
    Ok(prepare_flows(input)?.1)
}

fn prepare_flows(input: &Input) -> Result<(Vec<config::Flow>, Plan)> {
    let mut config = config::load(&input.config)?;
    config.ipfix = input.ipfix;
    let flows = config::validate(&config)?;
    let source_id = config.source_id;
    drop(config);
    let build = if input.ipfix {
        protocol::build_ipfix
    } else {
        protocol::build
    };
    let plan = build(source_id, &flows, input.packet_size as usize)?;
    Ok((flows, plan))
}

fn summary(plan: &Plan, out: &mut impl Write) -> io::Result<()> {
    writeln!(
        out,
        "Validation passed: Source ID={}, {} templates, {} records, {} data packets",
        plan.source_id,
        plan.templates.len(),
        plan.records,
        plan.data.len()
    )
}

fn simulated_estimate(
    plan: &Plan,
    flows: &[config::Flow],
    rate: u32,
    out: &mut impl Write,
) -> io::Result<()> {
    let mut groups: std::collections::HashMap<u16, Vec<&config::Flow>> =
        std::collections::HashMap::new();
    for flow in flows {
        groups.entry(flow.template_id).or_default().push(flow);
    }
    let estimate = |group: &[&config::Flow], name: &str, multiplier: f64| {
        let mut total = 0.0;
        for flow in group {
            let Some(column) = flow.columns.iter().find(|column| column.name == name) else {
                return "-".to_owned();
            };
            let average = match &column.values {
                config::Values::List { bytes, length } => {
                    bytes
                        .chunks_exact(*length)
                        .map(|value| {
                            value
                                .iter()
                                .fold(0u64, |n, byte| (n << 8) | u64::from(*byte))
                                as f64
                        })
                        .sum::<f64>()
                        / column.values.count() as f64
                }
                config::Values::Range {
                    start, step, count, ..
                } => *start as f64 + *step as f64 * (*count - 1) as f64 / 2.0,
                config::Values::RandomField { min, max, .. } => (*min as f64 + *max as f64) / 2.0,
                config::Values::RandomFile { .. } => {
                    unreachable!("random_file is limited to IP fields")
                }
            };
            total += average * flow.record_count as f64;
        }
        format!(
            "{:.2}",
            total * multiplier * rate as f64 / plan.data.len() as f64
        )
    };
    writeln!(
        out,
        "Expected collector average rates per minute (continuous sending, all packets received, random values averaged; - = missing counter, Options not estimated)"
    )?;
    writeln!(
        out,
        "{:>8} {:>14} {:>14} {:>14} {:>14}",
        "Template", "IN bps", "IN pps", "OUT bps", "OUT pps"
    )?;
    for template in &plan.templates {
        let group = &groups[&template.template_id];
        if group[0].scope_count > 0 {
            writeln!(out, "{:>8}  Options", template.template_id)?;
        } else {
            writeln!(
                out,
                "{:>8} {:>14} {:>14} {:>14} {:>14}",
                template.template_id,
                estimate(group, "IN_BYTES", 8.0),
                estimate(group, "IN_PKTS", 1.0),
                estimate(group, "OUT_BYTES", 8.0),
                estimate(group, "OUT_PKTS", 1.0)
            )?;
        }
    }
    Ok(())
}

fn send_estimate(
    plan: &Plan,
    flows: &[config::Flow],
    settings: &Settings,
    out: &mut impl Write,
) -> io::Result<()> {
    let (name, packets) = if settings.kind == Kind::Templates {
        ("Template", &plan.templates)
    } else {
        ("Data", &plan.data)
    };
    let min = packets
        .iter()
        .map(|packet| packet.bytes.len())
        .min()
        .unwrap_or(0);
    let max = packets
        .iter()
        .map(|packet| packet.bytes.len())
        .max()
        .unwrap_or(0);
    writeln!(
        out,
        "Configuration: Source ID={} | {} templates | {} records | {} data/cycle",
        plan.source_id,
        plan.templates.len(),
        plan.records,
        plan.data.len()
    )?;
    let batch = match settings.mode {
        Mode::Stream => 1,
        Mode::Burst => settings.burst_size.min(settings.rate),
    };
    let mode = match settings.mode {
        Mode::Stream => "stream".to_owned(),
        Mode::Burst => format!("burst ({batch} packets/batch)"),
    };
    let stop = if settings.duration == Some(Duration::ZERO) {
        "Unlimited (Ctrl-C / SIGTERM to stop)".to_owned()
    } else if let Some(duration) = settings.duration {
        format!("{:.3} seconds", duration.as_secs_f64())
    } else {
        let count = settings.count.unwrap_or(packets.len() as u64);
        let seconds =
            ((count - 1) / u64::from(batch)) as f64 * f64::from(batch) / f64::from(settings.rate);
        format!("{count} {name} packets, scheduled over approximately {seconds:.3} seconds")
    };
    writeln!(
        out,
        "Mode: {mode} | {} {name} packets/s | {stop}",
        settings.rate
    )?;
    if settings.kind == Kind::Templates {
        writeln!(out, "UDP payload: {min}..={max} B/packet")?;
    } else {
        let min_records = plan.data.iter().map(|p| p.data_records).min().unwrap_or(0);
        let max_records = plan.data.iter().map(|p| p.data_records).max().unwrap_or(0);
        writeln!(
            out,
            "UDP payload: {min}..={max} B/packet | {min_records}..={max_records} records/packet"
        )?;
    }
    if settings.kind == Kind::All {
        writeln!(
            out,
            "Templates: {} initial packets; refresh every {} data packets or {:.3}s (extra)",
            plan.templates.len(),
            settings.template_every,
            settings.template_interval.as_secs_f64()
        )?;
    }
    if settings.kind != Kind::Templates {
        simulated_estimate(plan, flows, settings.rate, out)?;
    }
    writeln!(out, "Details: nfv9gen preview CONFIG --template-id ID")
}

extern "C" fn interrupt(_: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}

fn install_signals() -> Result<()> {
    // SAFETY: the handler only sets a lock-free atomic flag; it never allocates or performs I/O.
    unsafe {
        ensure!(
            libc::signal(libc::SIGINT, interrupt as *const () as libc::sighandler_t)
                != libc::SIG_ERR,
            "Failed to install SIGINT handler"
        );
        ensure!(
            libc::signal(libc::SIGTERM, interrupt as *const () as libc::sighandler_t)
                != libc::SIG_ERR,
            "Failed to install SIGTERM handler"
        );
    }
    Ok(())
}

fn connect(target: &str, bind: Option<SocketAddr>) -> Result<UdpSocket> {
    let addresses = target
        .to_socket_addrs()
        .with_context(|| format!("Failed to resolve target {target:?}; use host:port"))?;
    let mut last = None;
    for address in addresses {
        if address.port() == 0 {
            continue;
        }
        if bind.is_some_and(|bind| bind.is_ipv4() != address.is_ipv4()) {
            continue;
        }
        let local = bind.unwrap_or_else(|| {
            if address.is_ipv4() {
                "0.0.0.0:0".parse().unwrap()
            } else {
                "[::]:0".parse().unwrap()
            }
        });
        match UdpSocket::bind(local).and_then(|socket| {
            socket.connect(address)?;
            Ok(socket)
        }) {
            Ok(socket) => return Ok(socket),
            Err(error) => last = Some(error),
        }
    }
    if let Some(error) = last {
        return Err(error).context("Failed to create UDP socket or connect to collector");
    }
    anyhow::bail!("target must have a nonzero port and the same IP version as bind")
}

fn execute(cli: Cli, out: &mut impl Write) -> Result<bool> {
    let silent = cli.silent;
    let mut interrupted = false;
    match cli.command {
        Command::Validate { input } => summary(&prepare(&input)?, out)?,
        Command::Preview {
            input,
            kind,
            hex,
            limit,
            template_id,
            rate,
        } => {
            let (flows, mut plan) = prepare_flows(&input)?;
            if let Some(id) = template_id {
                ensure!(
                    flows.iter().any(|flow| flow.template_id == id),
                    "Template {id} not found; run preview without --template-id to see available IDs"
                );
            }
            summary(&plan, out)?;
            if kind != Kind::Templates {
                simulated_estimate(&plan, &flows, rate, out)?;
            }
            if let Some(id) = template_id {
                writeln!(
                    out,
                    "Filter: Template {id} (totals above cover the full configuration)"
                )?;
            }
            let mut number = 0u32;
            let mut records = 0u32;
            let now = SystemTime::now();
            let source_id = plan.source_id;
            let total = plan
                .templates
                .iter()
                .filter(|packet| {
                    kind != Kind::Data && template_id.is_none_or(|id| packet.template_id == id)
                })
                .count()
                + plan
                    .data
                    .iter()
                    .filter(|packet| {
                        kind != Kind::Templates
                            && template_id.is_none_or(|id| packet.template_id == id)
                    })
                    .count();
            let mut shown = std::collections::HashSet::new();
            let packets = plan
                .templates
                .iter_mut()
                .filter(|_| kind != Kind::Data)
                .chain(plan.data.iter_mut().filter(|_| kind != Kind::Templates))
                .filter(|packet| template_id.is_none_or(|id| packet.template_id == id));
            for packet in packets.take(limit as usize) {
                if shown.insert(packet.template_id) {
                    for (index, flow) in flows
                        .iter()
                        .enumerate()
                        .filter(|(_, flow)| flow.template_id == packet.template_id)
                    {
                        writeln!(out, "Flow {}: ", index + 1)?;
                        preview::flow(flow, out)?;
                    }
                }
                let sequence = if packet.is_ipfix() { records } else { number };
                packet.stamp(source_id, sequence, Duration::ZERO, now)?;
                records += u32::from(packet.data_records);
                // A template packet counts as one record, as in the v9 header.
                let count = packet.data_records.max(1);
                writeln!(
                    out,
                    "#{number} {} template={} records={count} payload={} bytes",
                    if packet.data_records == 0 {
                        "Template"
                    } else {
                        "Data"
                    },
                    packet.template_id,
                    packet.bytes.len()
                )?;
                if hex {
                    for byte in &packet.bytes {
                        write!(out, "{byte:02x}")?;
                    }
                    writeln!(out)?;
                }
                number += 1;
            }
            if total > number as usize {
                writeln!(
                    out,
                    "{} remaining packets omitted; adjust with --limit",
                    total - number as usize
                )?;
            }
        }
        Command::Send {
            input,
            target,
            bind,
            source,
            kind,
            count,
            duration,
            rate,
            mode,
            burst_size,
            template_interval,
            template_every,
        } => {
            let (flows, mut plan) = prepare_flows(&input)?;
            let (socket, mut raw, local, peer) = if let Some(source) = source {
                let peer = target
                    .to_socket_addrs()
                    .with_context(|| format!("Failed to resolve target {target:?}"))?
                    .find(|address| address.is_ipv4() == source.is_ipv4() && address.port() != 0)
                    .context("target must have a nonzero port and the same IP version as source")?;
                let raw =
                    neural_ensemble_telemetry_fabric_luminal_oracle_wizard9::raw::RawSender::new(
                        source, peer,
                    )?;
                (None, Some(raw), source, peer)
            } else {
                let socket = connect(&target, bind)?;
                socket.set_write_timeout(Some(Duration::from_secs(1)))?;
                let local = socket.local_addr()?;
                let peer = socket.peer_addr()?;
                (Some(socket), None, local, peer)
            };
            install_signals()?;
            writeln!(out, "Sending: {local} → {peer}")?;
            let settings = Settings {
                kind,
                count,
                duration,
                rate,
                mode,
                burst_size,
                template_interval,
                template_every,
            };
            send_estimate(&plan, &flows, &settings, out)?;
            drop(flows);
            out.flush()?;
            let stderr = io::stderr();
            let interactive = !silent && stderr.is_terminal() && io::stdout().is_terminal();
            let mut log: Box<dyn Write> = if silent {
                Box::new(io::sink())
            } else {
                Box::new(stderr.lock())
            };
            let stats = progress::run(
                &mut plan,
                &settings,
                &STOP,
                |bytes| match raw.as_mut() {
                    Some(raw) => raw.send(bytes),
                    None => socket.as_ref().unwrap().send(bytes),
                },
                &mut log,
                interactive,
            )?;
            let seconds = stats.elapsed.as_secs_f64();
            let elapsed = stats.elapsed.as_secs();
            writeln!(
                out,
                "{} | {:02}:{:02}:{:02} | Data {} | Template {}\nRecords {} | UDP payload {:.2} MiB | Average {:.1} data packets/s",
                if stats.interrupted {
                    "Stopped"
                } else {
                    "Completed"
                },
                elapsed / 3600,
                (elapsed / 60) % 60,
                elapsed % 60,
                stats.data_packets,
                stats.template_packets,
                stats.records,
                stats.bytes as f64 / (1024.0 * 1024.0),
                stats.data_packets as f64 / seconds.max(f64::EPSILON)
            )?;
            interrupted = stats.interrupted;
        }
    }
    out.flush()?;
    Ok(interrupted)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let silent = cli.silent;
    let result = if silent {
        execute(cli, &mut io::sink())
    } else {
        execute(cli, &mut io::stdout().lock())
    };
    let code = match result {
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::from(130),
        Err(error)
            if error
                .downcast_ref::<io::Error>()
                .is_some_and(|e| e.kind() == io::ErrorKind::BrokenPipe) =>
        {
            ExitCode::SUCCESS
        }
        Err(error) => {
            if silent {
                let _ = writeln!(io::stdout().lock(), "fail");
            } else {
                let _ = writeln!(io::stderr().lock(), "Error: {error:#}");
            }
            return ExitCode::FAILURE;
        }
    };
    if silent {
        let _ = writeln!(io::stdout().lock(), "done");
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    fn traffic_row(text: &str, id: u16) -> Vec<&str> {
        let id = id.to_string();
        text.lines()
            .find(|line| line.split_whitespace().next() == Some(id.as_str()))
            .unwrap()
            .split_whitespace()
            .collect()
    }

    #[test]
    fn send_estimates_match_modes_and_stopping_conditions() {
        let (flows, plan) = prepare_flows(&Input {
            config: "examples/basic.json".into(),
            packet_size: 1400,
            ipfix: false,
        })
        .unwrap();
        let mut settings = Settings {
            kind: Kind::All,
            count: Some(101),
            duration: None,
            rate: 100,
            mode: Mode::Stream,
            burst_size: 64,
            template_interval: Duration::from_secs(30),
            template_every: 20,
        };
        let render = |settings: &Settings| {
            let mut out = Vec::new();
            send_estimate(&plan, &flows, settings, &mut out).unwrap();
            String::from_utf8(out).unwrap()
        };
        let text = render(&settings);
        assert!(text.contains("UDP payload: 48..=48 B/packet | 1..=1 records/packet"));
        assert!(text.contains("101 Data packets, scheduled over approximately 1.000 seconds"));
        assert!(text.contains("Templates: 1 initial packets"));
        assert!(text.contains("stream | 100 Data packets/s"));
        assert_eq!(
            traffic_row(&text, 256),
            ["256", "800000.00", "1000.00", "-", "-"]
        );
        settings.mode = Mode::Burst;
        let text = render(&settings);
        assert!(text.contains("scheduled over approximately 0.640 seconds"));
        assert!(text.contains("burst (64 packets/batch)"));
        settings.count = None;
        settings.kind = Kind::Templates;
        let text = render(&settings);
        assert!(text.contains("1 Template packets, scheduled over approximately 0.000 seconds"));
        assert!(!text.contains("Expected collector average rates per minute"));
        settings.kind = Kind::Data;
        settings.duration = Some(Duration::from_millis(500));
        let text = render(&settings);
        assert!(text.contains("0.500 seconds"));
        assert!(!text.contains("scheduled over approximately"));
        settings.duration = Some(Duration::ZERO);
        let text = render(&settings);
        assert!(text.contains("Unlimited (Ctrl-C / SIGTERM to stop)"));
        assert!(!text.contains("scheduled over approximately"));
    }

    #[test]
    fn simulated_estimates_handle_combinations_random_values_and_missing_counters() {
        let render = |value: serde_json::Value| {
            let config: config::Config = serde_json::from_value(value).unwrap();
            let flows = config::validate(&config).unwrap();
            let plan = protocol::build(config.source_id, &flows, 1400).unwrap();
            let mut out = Vec::new();
            simulated_estimate(&plan, &flows, 20, &mut out).unwrap();
            String::from_utf8(out).unwrap()
        };
        let mut config = serde_json::json!({"flows":[{"template_id":256,"fields":[
            {"name":"IN_BYTES","value":[10,20]},
            {"name":"IN_PKTS","value":{"start":2,"end":7,"step":2}}
        ]}]});
        let text = render(config.clone());
        assert_eq!(
            traffic_row(&text, 256),
            ["256", "14400.00", "480.00", "-", "-"]
        );
        config["flows"].as_array_mut().unwrap().push(serde_json::json!({"template_id":300,"scope":[{"name":"SYSTEM","value":1}],"fields":[{"name":"IN_BYTES","value":999999}]}));
        assert_eq!(
            traffic_row(&render(config), 256),
            ["256", "7200.00", "240.00", "-", "-"]
        );
        let mut config = serde_json::json!({"flows":[{"template_id":256,"fields":[
            {"name":"IN_BYTES","value":{"random_between":[1000,2000]}},
            {"name":"IN_PKTS","value":{"random_between":[10,11]}},
            {"name":"OUT_BYTES","value":0}, {"name":"OUT_PKTS","value":0}
        ]}]});
        let text = render(config.clone());
        assert_eq!(
            traffic_row(&text, 256),
            ["256", "240000.00", "210.00", "0.00", "0.00"]
        );
        assert!(text.contains("Expected collector average rates per minute"));
        assert!(text.contains("IN bps") && text.contains("OUT pps"));
        config["flows"][0]["fields"][1]["value"] =
            serde_json::json!({"random_between":[4294967294u32,4294967295u32]});
        assert_eq!(traffic_row(&render(config), 256)[2], "85899345890.00");
        let text = render(
            serde_json::json!({"flows":[{"template_id":300,"scope":[{"name":"SYSTEM","value":1}],"fields":[{"name":"SAMPLING_INTERVAL","value":1}]}]}),
        );
        assert_eq!(traffic_row(&text, 300), ["300", "Options"]);
        let config: config::Config =
            serde_json::from_str(include_str!("../examples/shared-template.json")).unwrap();
        let flows = config::validate(&config).unwrap();
        let plan = protocol::build(42, &flows, 1400).unwrap();
        let mut out = Vec::new();
        simulated_estimate(&plan, &flows, 20, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(
            traffic_row(&text, 256),
            ["256", "320000.00", "300.00", "-", "-"]
        );
        let text = render(
            serde_json::json!({"flows":[{"template_id":256,"fields":[{"name":"IN_PKTS","value":10},{"name":"IN_BYTES","value":1000}]},{"template_id":257,"fields":[{"name":"IN_BYTES","value":2000}]}]}),
        );
        assert_eq!(
            traffic_row(&text, 256),
            ["256", "80000.00", "100.00", "-", "-"]
        );
        assert_eq!(traffic_row(&text, 257), ["257", "160000.00", "-", "-", "-"]);
        assert_eq!(
            text.lines()
                .filter(|line| line.split_whitespace().next() == Some("256"))
                .count(),
            1
        );
        assert_eq!(
            text.lines()
                .filter(|line| line.split_whitespace().next() == Some("257"))
                .count(),
            1
        );
    }

    #[test]
    fn send_estimate_aggregates_shared_templates() {
        let (flows, plan) = prepare_flows(&Input {
            config: "examples/shared-template.json".into(),
            packet_size: 1400,
            ipfix: false,
        })
        .unwrap();
        let settings = Settings {
            kind: Kind::All,
            count: None,
            duration: None,
            rate: 1000,
            mode: Mode::Stream,
            burst_size: 64,
            template_interval: Duration::from_secs(30),
            template_every: 20,
        };
        let mut out = Vec::new();
        send_estimate(&plan, &flows, &settings, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("1 templates | 2 records | 2 data/cycle"));
        assert!(text.contains("Templates: 1 initial packets"));
        assert_eq!(
            traffic_row(&text, 256),
            ["256", "16000000.00", "15000.00", "-", "-"]
        );
        assert_eq!(
            text.lines()
                .filter(|line| line.split_whitespace().next() == Some("256"))
                .count(),
            1
        );
    }

    #[test]
    fn many_templates_have_one_compact_row_each() {
        let flows_json: Vec<_> = (256..280)
            .map(|id| {
                serde_json::json!({"template_id":id,"fields":[
                    {"name":"L4_SRC_PORT","value":{"random_between":[49152,65535]}},
                    {"name":"IN_PKTS","value":{"random_between":[1,4]}},
                    {"name":"IN_BYTES","value":{"random_between":[1,8]}}
                ]})
            })
            .collect();
        let config: config::Config =
            serde_json::from_value(serde_json::json!({"flows":flows_json})).unwrap();
        let flows = config::validate(&config).unwrap();
        let plan = protocol::build(101, &flows, 1400).unwrap();
        let settings = Settings {
            kind: Kind::All,
            count: None,
            duration: Some(Duration::ZERO),
            rate: 50,
            mode: Mode::Stream,
            burst_size: 64,
            template_interval: Duration::from_secs(30),
            template_every: 20,
        };
        let mut out = Vec::new();
        send_estimate(&plan, &flows, &settings, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.lines().count() <= 32, "{} lines", text.lines().count());
        assert!(!text.contains("random_between") && !text.contains("Cannot estimate"));
        assert!(!text.contains("remaining") && !text.contains("scope /"));
        for id in 256..280 {
            assert_eq!(traffic_row(&text, id).len(), 5);
            let key = id.to_string();
            assert_eq!(
                text.lines()
                    .filter(|line| line.split_whitespace().next() == Some(key.as_str()))
                    .count(),
                1
            );
        }
    }

    #[test]
    fn zero_duration_is_unlimited_only_for_send_duration() {
        let cli =
            Cli::try_parse_from(["nfv9gen", "send", "examples/basic.json", "--duration", "0"])
                .unwrap();
        assert!(matches!(
            cli.command,
            Command::Send {
                duration: Some(Duration::ZERO),
                rate: 20,
                ..
            }
        ));
        assert!(
            Cli::try_parse_from([
                "nfv9gen",
                "send",
                "examples/basic.json",
                "--duration",
                "0",
                "--count",
                "1"
            ])
            .is_err()
        );
        assert!(duration("0").is_err());
        for value in ["0.00000000001", "1e-400", "-1e-400", "0.0", "-0", "0e10"] {
            assert!(
                send_duration(value).is_err(),
                "{value} must not enable unlimited sending"
            );
        }
        assert_eq!(send_duration("0.5").unwrap(), Duration::from_millis(500));
    }

    #[test]
    fn send_details_describe_options_and_packet_splitting() {
        let config: config::Config = serde_json::from_value(serde_json::json!({"flows":[
            {"template_id":256,"fields":[
                {"name":"IN_BYTES","value":[10,20,30,40,50,60]},
                {"name":"IN_PKTS","value":2},
                {"name":"OUT_BYTES","value":100}
            ]},
            {"template_id":257,"scope":[{"name":"SYSTEM","value":1}],
             "fields":[{"name":"IN_BYTES","value":999}]}
        ]}))
        .unwrap();
        let flows = config::validate(&config).unwrap();
        let plan = protocol::build(config.source_id, &flows, 64).unwrap();
        assert_eq!(plan.data.len(), 3);
        let settings = Settings {
            kind: Kind::Data,
            count: Some(4),
            duration: None,
            rate: 5,
            mode: Mode::Stream,
            burst_size: 64,
            template_interval: Duration::from_secs(30),
            template_every: 20,
        };
        let mut out = Vec::new();
        send_estimate(&plan, &flows, &settings, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("2 templates | 7 records | 3 data/cycle"));
        assert!(text.contains("1..=3 records/packet"));
        assert_eq!(
            traffic_row(&text, 256),
            ["256", "2800.00", "20.00", "8000.00", "-"]
        );
        assert_eq!(traffic_row(&text, 257), ["257", "Options"]);
    }
}
