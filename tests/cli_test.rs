use std::process::Command;

#[test]
fn exposes_uspto_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_flowleap"))
        .args(["uspto", "--help"])
        .output()
        .expect("run flowleap uspto --help");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    assert!(stdout.contains("USPTO Open Data Portal commands"));
    assert!(stdout.contains("search"));
    assert!(stdout.contains("build-query"));
}

#[test]
fn dry_run_succeeds_without_credentials() {
    let temp_home = tempfile::tempdir().expect("create temp home");
    let output = Command::new(env!("CARGO_BIN_EXE_flowleap"))
        .env("HOME", temp_home.path())
        .env_remove("FLOWLEAP_API_KEY")
        .env_remove("FLOWLEAP_TOKEN")
        .args([
            "patent",
            "search",
            "--query",
            "ti=\"wireless charging\"",
            "--limit",
            "5",
            "--countries",
            "EP,WO",
            "--dry-run",
            "--output",
            "json",
        ])
        .output()
        .expect("run dry-run search");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is json");

    assert_eq!(value["dryRun"], true);
    assert_eq!(value["method"], "POST");
    assert_eq!(value["authenticated"], false);
    // The backend contract is { query (CQL), range: "start-end", countries? }.
    assert_eq!(value["body"]["query"], "ti=\"wireless charging\"");
    assert_eq!(value["body"]["range"], "1-5");
    assert_eq!(value["body"]["countries"], "EP,WO");
    assert!(value["body"].get("source").is_none());
    assert!(value["body"].get("limit").is_none());
}

#[test]
fn uspto_search_dry_run_uses_odp_request_shape() {
    let temp_home = tempfile::tempdir().expect("create temp home");
    let output = Command::new(env!("CARGO_BIN_EXE_flowleap"))
        .env("HOME", temp_home.path())
        .env("FLOWLEAP_API_KEY", "fl_org_test_secret")
        .env_remove("FLOWLEAP_TOKEN")
        .args([
            "--json",
            "uspto",
            "search",
            "--query",
            "wireless charging",
            "--limit",
            "1",
            "--dry-run",
        ])
        .output()
        .expect("run uspto search dry-run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is json");

    assert_eq!(value["dryRun"], true);
    assert_eq!(value["method"], "POST");
    assert_eq!(value["authenticated"], true);
    assert_eq!(
        value["url"],
        "https://api.flowleap.co/v1/patent-search-uspto/search"
    );
    assert_eq!(value["body"]["q"], "wireless charging");
    assert_eq!(value["body"]["pagination"]["limit"], 1);
    assert_eq!(value["body"]["pagination"]["offset"], 0);
    assert!(value["body"].get("query").is_none());
    assert!(value["body"].get("limit").is_none());
}

#[test]
fn exposes_agent_first_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_flowleap"))
        .arg("--help")
        .output()
        .expect("run flowleap --help");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    for command in [
        "doctor",
        "api",
        "health",
        "uspto",
        "npl",
        "legal",
        "citation",
        "ocr",
        "analyze-claim",
    ] {
        assert!(stdout.contains(command), "missing command {command}");
    }
}

#[test]
fn raw_request_dry_run_succeeds_without_credentials() {
    let temp_home = tempfile::tempdir().expect("create temp home");
    let output = Command::new(env!("CARGO_BIN_EXE_flowleap"))
        .env("HOME", temp_home.path())
        .env_remove("FLOWLEAP_API_KEY")
        .env_remove("FLOWLEAP_TOKEN")
        .args([
            "--json",
            "api",
            "request",
            "post",
            "/v1/patent-search",
            "--body",
            r#"{"query":"solar","limit":1}"#,
            "--dry-run",
        ])
        .output()
        .expect("run raw request dry-run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is json");

    assert_eq!(value["dryRun"], true);
    assert_eq!(value["method"], "POST");
    assert_eq!(value["authenticated"], false);
    assert_eq!(value["body"]["query"], "solar");
}

#[test]
fn org_api_key_dry_run_is_authenticated_without_leaking_key() {
    let temp_home = tempfile::tempdir().expect("create temp home");
    let output = Command::new(env!("CARGO_BIN_EXE_flowleap"))
        .env("HOME", temp_home.path())
        .env("FLOWLEAP_API_KEY", "fl_org_test_secret")
        .env_remove("FLOWLEAP_TOKEN")
        .args([
            "--json",
            "--verbose",
            "api",
            "request",
            "get",
            "/v1/health",
            "--dry-run",
        ])
        .output()
        .expect("run org-key dry-run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr is utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is json");

    assert_eq!(value["dryRun"], true);
    assert_eq!(value["authenticated"], true);
    assert!(!stderr.contains("fl_org_test_secret"));
    assert!(!stdout.contains("fl_org_test_secret"));
}

#[test]
fn parse_errors_honor_json_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_flowleap"))
        .args(["--json", "not-a-command"])
        .output()
        .expect("run invalid command");

    assert!(!output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is json");

    assert_eq!(value["ok"], false);
    assert!(value["error"]["message"]
        .as_str()
        .unwrap()
        .contains("unrecognized subcommand"));
}

#[test]
fn ocr_url_dry_run_sends_url_field() {
    let temp_home = tempfile::tempdir().expect("create temp home");
    let output = Command::new(env!("CARGO_BIN_EXE_flowleap"))
        .env("HOME", temp_home.path())
        .env_remove("FLOWLEAP_API_KEY")
        .env_remove("FLOWLEAP_TOKEN")
        .args(["--json", "ocr", "https://example.com/spec.pdf", "--dry-run"])
        .output()
        .expect("run ocr url dry-run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is json");

    assert_eq!(value["dryRun"], true);
    assert_eq!(value["method"], "POST");
    assert_eq!(value["url"], "https://api.flowleap.co/v1/ocr");
    assert_eq!(value["body"]["url"], "https://example.com/spec.pdf");
    assert!(value["body"].get("file").is_none());
    assert!(value["body"].get("filename").is_none());
}

#[test]
fn ocr_local_file_dry_run_sends_base64_and_filename() {
    let temp_home = tempfile::tempdir().expect("create temp home");
    let file_path = temp_home.path().join("scan.png");
    std::fs::write(&file_path, b"fake png bytes").expect("write sample file");

    let output = Command::new(env!("CARGO_BIN_EXE_flowleap"))
        .env("HOME", temp_home.path())
        .env_remove("FLOWLEAP_API_KEY")
        .env_remove("FLOWLEAP_TOKEN")
        .args([
            "--json",
            "ocr",
            file_path.to_str().expect("path is utf8"),
            "--dry-run",
        ])
        .output()
        .expect("run ocr file dry-run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is json");

    assert_eq!(value["dryRun"], true);
    assert_eq!(value["method"], "POST");
    assert_eq!(value["url"], "https://api.flowleap.co/v1/ocr");
    // base64("fake png bytes")
    assert_eq!(value["body"]["file"], "ZmFrZSBwbmcgYnl0ZXM=");
    assert_eq!(value["body"]["filename"], "scan.png");
    assert!(value["body"].get("url").is_none());
}

#[test]
fn ocr_rejects_unsupported_file_type_locally() {
    let temp_home = tempfile::tempdir().expect("create temp home");
    let file_path = temp_home.path().join("notes.txt");
    std::fs::write(&file_path, b"plain text").expect("write sample file");

    let output = Command::new(env!("CARGO_BIN_EXE_flowleap"))
        .env("HOME", temp_home.path())
        .args([
            "--json",
            "ocr",
            file_path.to_str().expect("path is utf8"),
            "--dry-run",
        ])
        .output()
        .expect("run ocr unsupported-type");

    assert!(!output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is json");

    assert_eq!(value["ok"], false);
    let message = value["error"]["message"].as_str().unwrap();
    assert!(message.contains("Unsupported file type 'txt'"));
    assert!(message.contains("pdf"));
}

#[test]
fn ocr_rejects_missing_file_locally() {
    let temp_home = tempfile::tempdir().expect("create temp home");
    let output = Command::new(env!("CARGO_BIN_EXE_flowleap"))
        .env("HOME", temp_home.path())
        .args(["--json", "ocr", "/nonexistent/never.pdf", "--dry-run"])
        .output()
        .expect("run ocr missing-file");

    assert!(!output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is json");

    assert_eq!(value["ok"], false);
    assert!(value["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Cannot read /nonexistent/never.pdf"));
}

#[test]
fn ocr_rejects_oversized_file_locally() {
    let temp_home = tempfile::tempdir().expect("create temp home");
    let file_path = temp_home.path().join("huge.pdf");
    // Sparse file: over the 36 MB limit without materializing the bytes.
    let file = std::fs::File::create(&file_path).expect("create sparse file");
    file.set_len(37 * 1024 * 1024).expect("set sparse length");

    let output = Command::new(env!("CARGO_BIN_EXE_flowleap"))
        .env("HOME", temp_home.path())
        .args([
            "--json",
            "ocr",
            file_path.to_str().expect("path is utf8"),
            "--dry-run",
        ])
        .output()
        .expect("run ocr oversized");

    assert!(!output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is json");

    assert_eq!(value["ok"], false);
    assert!(value["error"]["message"]
        .as_str()
        .unwrap()
        .contains("exceeds the 36 MB OCR upload limit"));
}

#[test]
fn analyze_claim_argument_dry_run_request_shape() {
    let temp_home = tempfile::tempdir().expect("create temp home");
    let output = Command::new(env!("CARGO_BIN_EXE_flowleap"))
        .env("HOME", temp_home.path())
        .env_remove("FLOWLEAP_API_KEY")
        .env_remove("FLOWLEAP_TOKEN")
        .args([
            "--json",
            "analyze-claim",
            "A method for wireless charging comprising a coil.",
            "--dry-run",
        ])
        .output()
        .expect("run analyze-claim arg dry-run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is json");

    assert_eq!(value["dryRun"], true);
    assert_eq!(value["method"], "POST");
    assert_eq!(value["url"], "https://api.flowleap.co/v1/analyze-claim");
    assert_eq!(
        value["body"]["claimText"],
        "A method for wireless charging comprising a coil."
    );
    assert!(value["body"].get("focus").is_none());
}

#[test]
fn analyze_claim_forwards_focus_flag() {
    let temp_home = tempfile::tempdir().expect("create temp home");
    let output = Command::new(env!("CARGO_BIN_EXE_flowleap"))
        .env("HOME", temp_home.path())
        .args([
            "--json",
            "analyze-claim",
            "A battery pack comprising lithium cells.",
            "--focus",
            "search",
            "--dry-run",
        ])
        .output()
        .expect("run analyze-claim focus dry-run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is json");

    assert_eq!(value["dryRun"], true);
    assert_eq!(value["body"]["focus"], "search");
}

#[test]
fn analyze_claim_reads_file_input() {
    let temp_home = tempfile::tempdir().expect("create temp home");
    let file_path = temp_home.path().join("claim.txt");
    std::fs::write(&file_path, "A device comprising a sensor.\n").expect("write claim file");

    let output = Command::new(env!("CARGO_BIN_EXE_flowleap"))
        .env("HOME", temp_home.path())
        .args([
            "--json",
            "analyze-claim",
            "--file",
            file_path.to_str().expect("path is utf8"),
            "--dry-run",
        ])
        .output()
        .expect("run analyze-claim file dry-run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is json");

    assert_eq!(value["dryRun"], true);
    assert_eq!(value["body"]["claimText"], "A device comprising a sensor.");
}

#[test]
fn analyze_claim_reads_stdin_when_no_arg_or_file() {
    use std::io::Write;
    use std::process::Stdio;

    let temp_home = tempfile::tempdir().expect("create temp home");
    let mut child = Command::new(env!("CARGO_BIN_EXE_flowleap"))
        .env("HOME", temp_home.path())
        .args(["--json", "analyze-claim", "--dry-run"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn analyze-claim stdin dry-run");

    child
        .stdin
        .take()
        .expect("stdin handle")
        .write_all(b"A system comprising a processor.\n")
        .expect("write claim to stdin");

    let output = child.wait_with_output().expect("wait for analyze-claim");
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is json");

    assert_eq!(value["dryRun"], true);
    assert_eq!(
        value["body"]["claimText"],
        "A system comprising a processor."
    );
}

#[test]
fn ocr_and_analyze_claim_help_include_examples() {
    for command in ["ocr", "analyze-claim"] {
        let output = Command::new(env!("CARGO_BIN_EXE_flowleap"))
            .args([command, "--help"])
            .output()
            .expect("run --help");

        assert!(output.status.success());

        let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
        assert!(
            stdout.contains("Examples:"),
            "{command} --help is missing examples"
        );
        assert!(
            stdout.contains(&format!("flowleap {command}")),
            "{command} --help examples don't show an invocation"
        );
    }
}

#[test]
fn init_honors_json_flag() {
    let temp_home = tempfile::tempdir().expect("create temp home");
    let output = Command::new(env!("CARGO_BIN_EXE_flowleap"))
        .env("HOME", temp_home.path())
        .args(["--json", "init", "--base-url", "http://localhost:8000"])
        .output()
        .expect("run json init");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is json");

    assert_eq!(value["ok"], true);
    assert_eq!(value["baseUrl"], "http://localhost:8000");
}
