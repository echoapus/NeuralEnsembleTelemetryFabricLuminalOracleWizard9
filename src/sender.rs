use std::{
    fs::File,
    io::{self, BufReader},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant, SystemTime},
};

use anyhow::{Context, Result, ensure};
use clap::ValueEnum;

use crate::protocol::{Packet, Plan};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Mode {
    Stream,
    Burst,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Kind {
    All,
    Templates,
    Data,
}

pub struct Settings {
    pub kind: Kind,
    pub count: Option<u64>,
    /// None sends one round by default; Some(Duration::ZERO) runs until stopped.
    pub duration: Option<Duration>,
    pub rate: u32,
    pub mode: Mode,
    pub burst_size: u32,
    pub template_interval: Duration,
    pub template_every: u64,
}

#[derive(Debug, Default)]
pub struct Stats {
    pub packets: u64,
    pub template_packets: u64,
    pub data_packets: u64,
    pub records: u64,
    pub bytes: u64,
    pub elapsed: Duration,
    pub interrupted: bool,
}

fn transmit(
    packet: &mut Packet,
    source_id: u32,
    start: Instant,
    stats: &mut Stats,
    send: &mut impl FnMut(&[u8]) -> io::Result<usize>,
    random: &mut Option<BufReader<File>>,
) -> Result<()> {
    if let Some(random) = random.as_mut() {
        packet
            .randomize_fields(random)
            .context("Failed to generate random field values")?;
    }
    packet.stamp(
        source_id,
        stats.packets as u32,
        start.elapsed(),
        SystemTime::now(),
    )?;
    let size = send(&packet.bytes).context("UDP send failed")?;
    ensure!(
        size == packet.bytes.len(),
        "Incomplete UDP send: {size}/{} bytes",
        packet.bytes.len()
    );
    stats.packets += 1;
    stats.bytes += size as u64;
    stats.records += packet.data_records as u64;
    if packet.data_records == 0 {
        stats.template_packets += 1;
    } else {
        stats.data_packets += 1;
    }
    Ok(())
}

fn wait_until(deadline: Instant, stop: &AtomicBool, mut tick: impl FnMut()) {
    while !stop.load(Ordering::Relaxed) {
        tick();
        let Some(left) = deadline.checked_duration_since(Instant::now()) else {
            break;
        };
        thread::sleep(left.min(Duration::from_millis(20)));
    }
}

pub fn run(
    plan: &mut Plan,
    settings: &Settings,
    stop: &AtomicBool,
    send: impl FnMut(&[u8]) -> io::Result<usize>,
) -> Result<Stats> {
    run_with_progress(plan, settings, stop, send, None)
}

/// Reports successful sends at most four times a second, including while pacing.
/// The observer runs on the sender thread and should return promptly.
pub fn run_with_progress(
    plan: &mut Plan,
    settings: &Settings,
    stop: &AtomicBool,
    mut send: impl FnMut(&[u8]) -> io::Result<usize>,
    mut progress: Option<&mut dyn FnMut(&Stats)>,
) -> Result<Stats> {
    ensure!(settings.rate > 0, "rate must be greater than 0");
    ensure!(settings.burst_size > 0, "burst-size must be greater than 0");
    ensure!(
        settings.template_every > 0,
        "template-every must be greater than 0"
    );
    ensure!(
        !settings.template_interval.is_zero(),
        "template-interval must be greater than 0"
    );
    ensure!(settings.count != Some(0), "count must be greater than 0");
    ensure!(
        !(settings.count.is_some() && settings.duration.is_some()),
        "count and duration cannot be set together"
    );
    ensure!(
        !plan.data.is_empty() && !plan.templates.is_empty(),
        "No packets to send"
    );
    let start = Instant::now();
    let mut random = if settings.kind != Kind::Templates
        && plan
            .data
            .iter()
            .any(|packet| !packet.random_fields.is_empty())
    {
        Some(BufReader::new(
            File::open("/dev/urandom").context("Failed to open /dev/urandom")?,
        ))
    } else {
        None
    };
    let end = settings
        .duration
        .filter(|d| !d.is_zero())
        .map(|d| {
            start
                .checked_add(d)
                .context("duration exceeds the system timer range")
        })
        .transpose()?;
    let active_length = if settings.kind == Kind::Templates {
        plan.templates.len()
    } else {
        plan.data.len()
    };
    let count = settings
        .count
        .or_else(|| settings.duration.is_none().then_some(active_length as u64));
    let batch = match settings.mode {
        Mode::Stream => 1,
        Mode::Burst => settings.burst_size.min(settings.rate),
    } as u64;
    let mut stats = Stats::default();
    let mut sent = 0u64;
    let mut next = start;
    let mut refreshed = start;
    let mut since_refresh = 0u64;
    let mut next_progress = start;
    let mut report_progress = |stats: &mut Stats| {
        if let Some(report) = progress.as_mut() {
            let now = Instant::now();
            if now >= next_progress {
                stats.elapsed = now.duration_since(start);
                report(stats);
                next_progress = now + Duration::from_millis(250);
            }
        }
    };

    while count.is_none_or(|limit| sent < limit) {
        if stop.load(Ordering::Relaxed) || end.is_some_and(|end| Instant::now() >= end) {
            break;
        }
        wait_until(end.map_or(next, |end| end.min(next)), stop, || {
            report_progress(&mut stats);
        });
        if stop.load(Ordering::Relaxed) || end.is_some_and(|end| Instant::now() >= end) {
            break;
        }
        for _ in 0..batch {
            if count.is_some_and(|limit| sent >= limit)
                || stop.load(Ordering::Relaxed)
                || end.is_some_and(|end| Instant::now() >= end)
            {
                break;
            }
            report_progress(&mut stats);
            if stop.load(Ordering::Relaxed) || end.is_some_and(|end| Instant::now() >= end) {
                break;
            }
            if settings.kind == Kind::All
                && (sent == 0
                    || since_refresh >= settings.template_every
                    || refreshed.elapsed() >= settings.template_interval)
            {
                for template in &mut plan.templates {
                    if stop.load(Ordering::Relaxed) || end.is_some_and(|end| Instant::now() >= end)
                    {
                        break;
                    }
                    report_progress(&mut stats);
                    if stop.load(Ordering::Relaxed) || end.is_some_and(|end| Instant::now() >= end)
                    {
                        break;
                    }
                    transmit(
                        template,
                        plan.source_id,
                        start,
                        &mut stats,
                        &mut send,
                        &mut random,
                    )?;
                }
                refreshed = Instant::now();
                since_refresh = 0;
                if stop.load(Ordering::Relaxed) || end.is_some_and(|end| Instant::now() >= end) {
                    break;
                }
            }
            let index = (sent % active_length as u64) as usize;
            let packet = if settings.kind == Kind::Templates {
                &mut plan.templates[index]
            } else {
                &mut plan.data[index]
            };
            transmit(
                packet,
                plan.source_id,
                start,
                &mut stats,
                &mut send,
                &mut random,
            )?;
            sent += 1;
            since_refresh += 1;
        }
        // Do not turn scheduler delays into an unbounded catch-up burst.
        let interval = Duration::from_secs_f64(batch as f64 / settings.rate as f64);
        next = (next + interval).max(Instant::now());
    }
    stats.elapsed = start.elapsed();
    stats.interrupted = stop.load(Ordering::Relaxed);
    Ok(stats)
}
