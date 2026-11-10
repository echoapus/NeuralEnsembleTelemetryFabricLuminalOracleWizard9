use std::{
    io::{self, Write},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use anyhow::{Context, Result};

use neural_ensemble_telemetry_fabric_luminal_oracle_wizard9::{
    protocol::Plan,
    sender::{self, Kind, Settings, Stats},
};

pub fn run(
    plan: &mut Plan,
    settings: &Settings,
    stop: &AtomicBool,
    send: impl FnMut(&[u8]) -> io::Result<usize>,
    out: &mut impl Write,
    interactive: bool,
) -> Result<Stats> {
    let default_count = if settings.kind == Kind::Templates {
        plan.templates.len()
    } else {
        plan.data.len()
    } as u64;
    let mut next_log = Duration::from_secs(5);
    let mut progress_error = None;
    let mut report = |stats: &Stats| {
        if !interactive && stats.elapsed < next_log {
            return;
        }
        if let Err(error) = write(out, stats, settings, default_count, interactive) {
            if progress_error.is_none() {
                progress_error = Some(error);
            }
            stop.store(true, Ordering::Relaxed);
        }
        next_log = stats.elapsed + Duration::from_secs(5);
    };
    let result = sender::run_with_progress(plan, settings, stop, send, Some(&mut report));
    if interactive {
        let _ = write!(out, "\r\x1b[2K");
        let _ = out.flush();
    }
    if let Some(error) = progress_error {
        // A stderr BrokenPipe is a failure; only stdout BrokenPipe may exit successfully.
        return Err(io::Error::other(error)).context("Statistics output failed; sending stopped");
    }
    result
}

pub fn write(
    out: &mut impl Write,
    stats: &Stats,
    settings: &Settings,
    default_count: u64,
    interactive: bool,
) -> io::Result<()> {
    let seconds = stats.elapsed.as_secs_f64();
    if interactive {
        write!(out, "\r\x1b[2K")?;
    }
    write!(out, "Sending {seconds:.1}s")?;
    if settings.duration == Some(std::time::Duration::ZERO) {
        write!(out, " · Unlimited (Ctrl-C to stop)")?;
    } else if let Some(duration) = settings.duration {
        write!(out, " / {:.1}s", duration.as_secs_f64())?;
    }
    let (name, sent) = if settings.kind == Kind::Templates {
        ("Template", stats.template_packets)
    } else {
        ("Data", stats.data_packets)
    };
    write!(out, " · {name} {sent}")?;
    if settings.duration.is_none() {
        write!(out, " / {}", settings.count.unwrap_or(default_count))?;
    }
    write!(
        out,
        " · {:.1} {name} packets/s · Template {}",
        sent as f64 / seconds.max(f64::EPSILON),
        stats.template_packets
    )?;
    write!(
        out,
        " · Total {} records / {} bytes",
        stats.records, stats.bytes
    )?;
    if !interactive {
        writeln!(out)?;
    }
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use neural_ensemble_telemetry_fabric_luminal_oracle_wizard9::sender::Mode;
    use std::time::Duration;

    #[test]
    fn failed_progress_stops_sending_and_returns_failure() {
        struct FailedWriter {
            flush_only: bool,
            kind: io::ErrorKind,
            attempts: usize,
        }
        impl Write for FailedWriter {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if self.flush_only {
                    return Ok(bytes.len());
                }
                self.attempts += 1;
                Err(io::Error::new(
                    self.kind,
                    if self.attempts == 1 {
                        "first write failure"
                    } else {
                        "later failure"
                    },
                ))
            }
            fn flush(&mut self) -> io::Result<()> {
                if self.flush_only {
                    Err(io::Error::new(self.kind, "first flush failure"))
                } else {
                    Ok(())
                }
            }
        }
        for (flush_only, kind, interactive) in [
            (false, io::ErrorKind::Other, true),
            (true, io::ErrorKind::Other, true),
            (false, io::ErrorKind::BrokenPipe, false),
        ] {
            let config: neural_ensemble_telemetry_fabric_luminal_oracle_wizard9::config::Config =
                serde_json::from_str(include_str!("../examples/basic.json")).unwrap();
            let flows =
                neural_ensemble_telemetry_fabric_luminal_oracle_wizard9::config::validate(&config)
                    .unwrap();
            let mut plan =
                neural_ensemble_telemetry_fabric_luminal_oracle_wizard9::protocol::build(
                    config.source_id,
                    &flows,
                    1400,
                )
                .unwrap();
            let settings = Settings {
                kind: Kind::Data,
                count: None,
                duration: Some(Duration::ZERO),
                rate: 1,
                mode: Mode::Stream,
                burst_size: 64,
                template_interval: Duration::from_secs(30),
                template_every: 20,
            };
            let stop = AtomicBool::new(false);
            let mut sent = 0;
            let mut writer = FailedWriter {
                flush_only,
                kind,
                attempts: 0,
            };
            let error = run(
                &mut plan,
                &settings,
                &stop,
                |bytes| {
                    sent += 1;
                    if sent == 10 {
                        stop.store(true, Ordering::Relaxed);
                    }
                    Ok(bytes.len())
                },
                &mut writer,
                interactive,
            )
            .unwrap_err();
            assert!(stop.load(Ordering::Relaxed));
            assert!(
                sent < 10,
                "must stop on the progress error, not the fallback guard"
            );
            if interactive {
                assert_eq!(sent, 0);
            }
            assert!(format!("{error:#}").contains("Statistics output failed; sending stopped"));
            assert!(
                format!("{error:#}").contains("first"),
                "retain the first error"
            );
            assert_eq!(
                error.downcast_ref::<io::Error>().unwrap().kind(),
                io::ErrorKind::Other,
                "stderr BrokenPipe must not use stdout's successful-exit path"
            );
        }
    }

    #[test]
    fn progress_uses_successful_active_packet_counts_and_duration() {
        let stats = Stats {
            data_packets: 20,
            template_packets: 3,
            records: 200,
            bytes: 4096,
            elapsed: Duration::from_secs(2),
            ..Stats::default()
        };
        let mut settings = Settings {
            kind: Kind::All,
            count: Some(100),
            duration: None,
            rate: 10,
            mode: Mode::Stream,
            burst_size: 64,
            template_interval: Duration::from_secs(30),
            template_every: 20,
        };
        let mut out = Vec::new();
        write(&mut out, &stats, &settings, 1, true).unwrap();
        assert!(
            String::from_utf8(out)
                .unwrap()
                .contains("Data 20 / 100 · 10.0 Data packets/s · Template 3")
        );
        settings.kind = Kind::Templates;
        settings.count = None;
        let mut out = Vec::new();
        write(&mut out, &stats, &settings, 8, true).unwrap();
        assert!(String::from_utf8(out).unwrap().contains("Template 3 / 8"));
        settings.duration = Some(Duration::from_secs(5));
        let mut out = Vec::new();
        write(&mut out, &stats, &settings, 8, true).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("2.0s / 5.0s"));
        assert!(!text.contains("3 / 8"));
        settings.duration = Some(Duration::ZERO);
        let mut out = Vec::new();
        write(&mut out, &stats, &settings, 8, true).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("Unlimited"));
        assert!(!text.contains(" / 0.0s"));
        assert!(!text.contains("3 / 8"));
        let mut log = Vec::new();
        write(&mut log, &stats, &settings, 8, false).unwrap();
        write(&mut log, &stats, &settings, 8, false).unwrap();
        let log = String::from_utf8(log).unwrap();
        assert_eq!(log.lines().count(), 2);
        assert!(log.ends_with('\n'));
        assert!(!log.contains('\r') && !log.contains('\x1b'));
        assert!(log.contains("Total 200 records / 4096 bytes"));
        assert!(log.contains("1.5 Template packets/s"));
    }
}
