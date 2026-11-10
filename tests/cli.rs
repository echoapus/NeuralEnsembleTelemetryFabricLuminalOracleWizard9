use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

static ID: AtomicUsize = AtomicUsize::new(0);
fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nfv9gen"))
        .args(args)
        .output()
        .unwrap()
}
fn traffic_row(text: &str, id: u16) -> Vec<&str> {
    let id = id.to_string();
    text.lines()
        .find(|line| line.split_whitespace().next() == Some(id.as_str()))
        .unwrap()
        .split_whitespace()
        .collect()
}
struct Fixture(PathBuf);
impl Fixture {
    fn new(text: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "nfv9gen-test-{}-{}.json",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, text).unwrap();
        Self(path)
    }
    fn path(&self) -> &str {
        self.0.to_str().unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[test]
fn examples_validate_and_preview_without_a_socket() {
    for path in [
        "examples/basic.json",
        "examples/interfaces.json",
        "examples/nexthop.json",
        "examples/random-ports.json",
        "examples/random-ips.json",
        "examples/shared-template.json",
        "examples/random-counters.json",
        "examples/options.json",
        "examples/ranges.json",
        "examples/mpls.json",
        "examples/asn.json",
        "examples/convenience.json",
        "examples/collector-all-fields.json",
    ] {
        let output = cli(&["validate", path]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("Validation passed"));
    }
    let output = cli(&["preview", "examples/random-ips.json"]);
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("random_file 3 addresses")
    );
    let output = cli(&["preview", "examples/basic.json", "--hex", "--limit", "1"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Template template=256 records=1"));
    assert!(text.contains("00090001"));
    assert!(text.contains("1 remaining packets omitted"));
    let output = cli(&["preview", "examples/basic.json", "--kind", "data"]);
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("#0 Data")
    );
    let output = cli(&[
        "preview",
        "examples/random-ports.json",
        "--kind",
        "data",
        "--hex",
    ]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("1 records, 1 data packets"));
    assert!(text.contains("L4_SRC_PORT / 2 B / random_between 49152..=65535"));
    assert!(text.contains("preview hex uses min"));
    assert!(text.contains("L4_DST_PORT / 2 B / 1 values / 443"));
}

#[test]
fn malformed_input_and_unknown_fields_exit_nonzero() {
    for input in [
        "{",
        "{}",
        r#"{"flows":[]}"#,
        r#"{"flows":[{"template_id":256,"fields":[{"name":"NOPE","value":1}]}]}"#,
    ] {
        let file = Fixture::new(input);
        let output = cli(&["validate", file.path()]);
        assert_eq!(output.status.code(), Some(1));
        let text = String::from_utf8_lossy(&output.stderr);
        assert!(text.contains("Error:"));
        assert!(!text.contains("panicked"));
    }
    let file =
        Fixture::new(r#"{"flows":[{"template_id":256,"fields":[{"name":"NOPE","value":1}]}]}"#);
    let output = cli(&["send", file.path(), "--target", "invalid.invalid:1234"]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("NOPE"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("resolve target"));
}

#[test]
fn cli_rejects_conflicts_zero_and_nonfinite_durations() {
    for flags in [
        vec!["--count", "1", "--duration", "1"],
        vec!["--count", "0"],
        vec!["--count", "1", "--duration", "0"],
        vec!["--duration", "NaN"],
        vec!["--duration", "inf"],
        vec!["--duration", "-1"],
        vec!["--duration", "1e300"],
        vec!["--duration", "1e-400"],
        vec!["--duration=-1e-400"],
        vec!["--duration", "0.0"],
        vec!["--rate", "0"],
        vec!["--burst-size", "0"],
        vec!["--packet-size", "63"],
        vec!["--template-every", "0"],
        vec!["--template-interval", "0"],
    ] {
        let mut args = vec!["send", "examples/basic.json"];
        args.extend(flags);
        let output = cli(&args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert!(cli(&["--help"]).status.success());
    assert!(cli(&["send", "--help"]).status.success());
    assert!(String::from_utf8_lossy(&cli(&["--version"]).stdout).contains("nfv9gen 0.1.0"));
}

#[test]
fn help_explains_modes_limits_and_examples_in_short_and_long_forms() {
    for (command, expected) in [
        (
            None,
            vec![
                "<COMMAND> -h",
                "config-builder.html",
                "README.md",
                "Exit codes",
                "random_between=[min,max]",
                "nexthop.json",
                "random-ports.json",
            ],
        ),
        (
            Some("validate"),
            vec![
                "8 MiB",
                "500,000",
                "64..=65507",
                "nfv9gen validate",
                "random_between counts as one value",
                "not cumulative sends",
            ],
        ),
        (
            Some("preview"),
            vec![
                "all shows templates then data",
                "not a record count",
                "sequence",
                "--template-id 601",
                "hex uses the minimum value",
            ],
        ),
        (
            Some("send"),
            vec![
                "stream schedules individual packets",
                "reusing prebuilt packets",
                "conflicts with count",
                "CAP_NET_RAW",
                "before the next data packet",
                "--mode burst",
                "cumulative sent records may exceed it",
                "random_between resamples each record on every send",
            ],
        ),
    ] {
        for flag in ["-h", "--help"] {
            let mut args = Vec::new();
            if let Some(command) = command {
                args.push(command);
            }
            args.push(flag);
            let output = cli(&args);
            assert!(output.status.success());
            let text = String::from_utf8(output.stdout).unwrap();
            for phrase in &expected {
                assert!(
                    text.contains(phrase),
                    "{args:?}: missing {phrase:?}\n{text}"
                );
            }
        }
    }
}

#[test]
fn source_validation_precedes_raw_socket_creation() {
    for (source, target, expected) in [
        ("0.0.0.0:5000", "127.0.0.1:2055", "unicast"),
        (
            "192.0.2.1:0",
            "127.0.0.1:2055",
            "ports must be greater than 0",
        ),
        ("192.0.2.1:5000", "[::1]:2055", "IP version"),
    ] {
        let output = cli(&[
            "send",
            "examples/basic.json",
            "--source",
            source,
            "--target",
            target,
        ]);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
    }
    let output = cli(&[
        "send",
        "examples/basic.json",
        "--source",
        "192.0.2.1:5000",
        "--bind",
        "127.0.0.1:5000",
    ]);
    assert_eq!(output.status.code(), Some(2));
    let output = cli(&["send", "examples/basic.json", "--source", "bad"]);
    assert_eq!(output.status.code(), Some(2));
    let file = Fixture::new(r#"{"flows":[]}"#);
    let output = cli(&["send", file.path(), "--source", "192.0.2.1:5000"]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("flows must not be empty"));
}

#[test]
fn readable_preview_filters_templates_and_keeps_hex_and_packet_limits() {
    let output = cli(&[
        "preview",
        "examples/convenience.json",
        "--template-id",
        "601",
        "--kind",
        "data",
        "--hex",
        "--limit",
        "1",
    ]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Template 601 · Regular flow · 4 combinations"));
    assert!(text.contains("IPV6_SRC_ADDR / 16 B / 4 values / 2001:db8::"));
    assert!(text.contains("type=129 code=0"));
    assert!(text.contains("#0 Data template=601 records=4"));
    assert!(text.lines().any(|line| line.starts_with("00090004")));
    assert!(!text.contains("Template 600 ·"));
    assert!(!text.contains("Template 602 ·"));
    assert!(!text.contains("packets omitted"));
    assert!(!text.contains('\x1b'));

    let output = cli(&["preview", "examples/convenience.json", "--limit", "1"]);
    let text = String::from_utf8(output.stdout).unwrap();
    assert_eq!(text.matches("Field / Length").count(), 1);
    assert!(text.contains("5 remaining packets omitted"));
    let output = cli(&["preview", "examples/basic.json", "--template-id", "999"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Template 999 not found"));
    assert!(output.stdout.is_empty());

    // Filtering must not hide errors in another template.
    let file = Fixture::new(
        r#"{"flows":[{"template_id":256,"fields":[{"name":"PROTOCOL","value":6}]},{"template_id":257,"fields":[{"name":"NOPE","value":1}]}]}"#,
    );
    let output = cli(&["preview", file.path(), "--template-id", "256"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("NOPE"));
}

#[test]
fn preview_explains_expansion_and_last_values() {
    let output = cli(&["preview", "examples/ranges.json", "--kind", "data"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("Expansion: IPV4_SRC_ADDR 2 × IPV4_DST_ADDR 2 × L4_DST_PORT 2 = 8 records")
    );
    for last in ["192.0.2.2", "198.51.100.3", "443", "2001:db8::3"] {
        assert!(text.contains(&format!("Last value: {last}")), "{text}");
    }
    let output = cli(&["preview", "examples/basic.json"]);
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Expansion:"));
}

#[test]
fn errors_show_rejected_values_locations_and_corrections() {
    for (field, value, expected) in [
        (
            "TCP_FLAGS",
            serde_json::json!("SYNACK"),
            vec!["SYNACK", "SYN|ACK"],
        ),
        (
            "IPV4_SRC_ADDR",
            serde_json::json!("2001:db8::1"),
            vec!["wrong IP version", "192.0.2.1"],
        ),
        (
            "L4_DST_PORT",
            serde_json::json!(65536),
            vec!["65536", "0..=65535"],
        ),
        (
            "IN_BYTES",
            serde_json::json!({"start":0,"end":500000}),
            vec!["500000", "single range"],
        ),
        (
            "NOPE",
            serde_json::json!(1),
            vec!["Unknown regular field", "README.md"],
        ),
        (
            "TCP_FLAGS",
            serde_json::json!(["SA", "BAD"]),
            vec!["Value[1]", "BAD"],
        ),
    ] {
        let file = Fixture::new(&serde_json::json!({"flows":[{"template_id":256,"fields":[{"name":field,"value":value}]}]}).to_string());
        let output = cli(&["send", file.path(), "--target", "invalid.invalid:2055"]);
        assert_eq!(output.status.code(), Some(1));
        let error = String::from_utf8_lossy(&output.stderr);
        for text in ["flows[0]", "Template 256", field, "input value"]
            .into_iter()
            .chain(expected)
        {
            assert!(error.contains(text), "{text}: {error}");
        }
        assert!(!error.contains("resolve target"));
        assert!(!error.contains('\x1b'));
    }
    let file = Fixture::new("{bad");
    let output = cli(&["validate", file.path()]);
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("examples/basic.json"));
    assert!(error.contains("line 1"));

    let file = Fixture::new(&serde_json::json!({"flows":[{"template_id":256,"fields":[{"name":"NOPE","value":"界".repeat(1000)}]}]}).to_string());
    let output = cli(&["validate", file.path()]);
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains('…'));
    assert!(error.len() < 1000, "long diagnostic: {} bytes", error.len());
}

#[test]
fn preview_random_counters_uses_four_byte_minimum_placeholders() {
    let output = cli(&[
        "preview",
        "examples/random-counters.json",
        "--kind",
        "data",
        "--hex",
    ]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("IN_PKTS / 4 B / random_between 10..=100"));
    assert!(text.contains("IN_BYTES / 4 B / random_between 1000..=100000"));
    assert!(text.contains("0000000a000003e8"));
}

#[test]
fn shared_templates_are_reported_once_and_preview_all_flows() {
    let file = Fixture::new(
        r#"{"flows":[{"template_id":256,"fields":[{"name":"IN_BYTES","value":100}]},{"template_id":256,"fields":[{"name":"IN_BYTES","value":200}]}]}"#,
    );
    let output = cli(&["validate", file.path()]);
    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 templates, 2 records, 2 data packets")
    );
    let output = cli(&[
        "preview",
        file.path(),
        "--kind",
        "data",
        "--template-id",
        "256",
    ]);
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("Flow 1: ") && text.contains("Flow 2: "));
    assert_eq!(traffic_row(&text, 256), ["256", "24000.00", "-", "-", "-"]);
    assert!(
        text.contains("IN_BYTES / 4 B / 1 values / 100")
            && text.contains("IN_BYTES / 4 B / 1 values / 200")
    );
}

#[test]
fn preview_estimates_simulated_traffic_without_sending() {
    let output = cli(&["preview", "examples/basic.json", "--kind", "data"]);
    assert!(output.status.success());
    assert_eq!(
        traffic_row(&String::from_utf8_lossy(&output.stdout), 256),
        ["256", "160000.00", "200.00", "-", "-"]
    );
    let output = cli(&[
        "preview",
        "examples/basic.json",
        "--rate",
        "40",
        "--template-id",
        "256",
    ]);
    assert!(output.status.success());
    assert_eq!(
        traffic_row(&String::from_utf8_lossy(&output.stdout), 256),
        ["256", "320000.00", "400.00", "-", "-"]
    );
    assert!(
        !cli(&["preview", "examples/basic.json", "--rate", "0"])
            .status
            .success()
    );
}

#[cfg(target_os = "linux")]
#[test]
fn error_reporting_to_full_stderr_exits_without_panicking() {
    let file = Fixture::new("{");
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_nfv9gen"))
        .args(["validate", file.path()])
        .stderr(full)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn silent_prints_only_done_or_fail() {
    let collector = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let target = collector.local_addr().unwrap().to_string();
    for (args, expected, code) in [
        (
            vec!["validate", "examples/basic.json", "--silent"],
            "done\n",
            0,
        ),
        (
            vec!["-s", "preview", "examples/basic.json", "--hex"],
            "done\n",
            0,
        ),
        (
            vec!["send", "examples/basic.json", "--target", &target, "-s"],
            "done\n",
            0,
        ),
        (vec!["validate", "examples/missing.json", "-s"], "fail\n", 1),
    ] {
        let output = cli(&args);
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            expected,
            "{args:?}"
        );
        assert!(output.stderr.is_empty(), "{args:?}");
        assert_eq!(output.status.code(), Some(code), "{args:?}");
    }
}
