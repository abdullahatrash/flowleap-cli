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
            "wireless charging",
            "--source",
            "uspto",
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
    assert_eq!(value["body"]["query"], "wireless charging");
    assert_eq!(value["body"]["source"], "uspto");
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
        "doctor", "api", "health", "uspto", "npl", "legal", "citation",
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
