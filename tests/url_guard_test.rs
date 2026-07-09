//! End-to-end tests for the base-URL credential guard (issue #21): sending
//! credentials to a non-FlowLeap host warns once per invocation on stderr in
//! non-interactive mode while the request proceeds and stdout stays clean
//! JSON; trusted hosts stay silent.
//!
//! The wiremock server listens on 127.0.0.1 (a trusted host), so untrusted
//! hostnames are pinned to it via the test-only FLOWLEAP_TEST_RESOLVE DNS
//! override in `build_http_client` — the guard still classifies the original
//! hostname.

mod support;

use serde_json::json;
use support::{run_cli, stdout_json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Stable marker of the guard warning, asserted on stderr.
const WARNING_MARKER: &str = "non-FlowLeap host";

const UNTRUSTED_HOST: &str = "guard-test.example";

async fn mock_thing_ok(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/v1/thing"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "result": "ok" })))
        .mount(server)
        .await;
}

/// Base URL that names `host` but resolves to the mock server, plus the
/// FLOWLEAP_TEST_RESOLVE value pinning it there.
fn pinned_base(host: &str, server: &MockServer) -> (String, String) {
    (
        format!("http://{host}:{}", server.address().port()),
        format!("{host}={}", server.address()),
    )
}

/// Non-TTY + credentials + untrusted host: exactly one stderr warning naming
/// the host and the present credential kinds (never values, never absent
/// kinds), the request still proceeds — even across an in-band retry, proving
/// once-per-invocation rather than once-per-request — and stdout stays clean
/// JSON.
#[tokio::test]
async fn untrusted_host_warns_once_and_proceeds_non_tty() {
    let server = MockServer::start().await;
    // 503 then 200: the client retries, so two requests leave one invocation.
    Mock::given(method("GET"))
        .and(path("/v1/thing"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    mock_thing_ok(&server).await;
    let (base, resolve) = pinned_base(UNTRUSTED_HOST, &server);

    let output = run_cli(
        &base,
        &[
            ("FLOWLEAP_TEST_RESOLVE", &resolve),
            ("FLOWLEAP_TOKEN", "tok-secret-123"),
            ("FLOWLEAP_USPTO_KEY", "uspto-secret-456"),
        ],
        &["api", "request", "get", "/v1/thing", "--output", "json"],
    )
    .await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value = stdout_json(&output);
    assert_eq!(value["ok"], true);
    assert_eq!(value["body"]["result"], "ok");

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(
        stderr.matches(WARNING_MARKER).count(),
        1,
        "exactly one warning per invocation, stderr: {stderr}"
    );
    assert!(stderr.contains(UNTRUSTED_HOST), "names the host: {stderr}");
    assert!(
        stderr.contains("session token") && stderr.contains("USPTO ODP key"),
        "names the present credential kinds: {stderr}"
    );
    assert!(
        !stderr.contains("EPO") && !stderr.contains("personal API token"),
        "absent credential kinds are not named: {stderr}"
    );
    assert!(
        !stderr.contains("tok-secret-123") && !stderr.contains("uspto-secret-456"),
        "credential values never appear: {stderr}"
    );
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        2,
        "the request proceeds (and retries) in non-interactive mode"
    );
}

/// 127.0.0.1, localhost, and *.flowleap.co produce no warning at all.
#[tokio::test]
async fn trusted_hosts_produce_no_warning() {
    let server = MockServer::start().await;
    mock_thing_ok(&server).await;
    let port = server.address().port();
    let resolve = format!(
        "api.flowleap.co={addr},localhost={addr}",
        addr = server.address()
    );

    for base in [
        server.uri(),
        format!("http://localhost:{port}"),
        format!("http://api.flowleap.co:{port}"),
    ] {
        let output = run_cli(
            &base,
            &[
                ("FLOWLEAP_TEST_RESOLVE", &resolve),
                ("FLOWLEAP_TOKEN", "tok"),
            ],
            &["api", "request", "get", "/v1/thing", "--output", "json"],
        )
        .await;

        assert!(
            output.status.success(),
            "base {base}, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            !stderr.contains(WARNING_MARKER),
            "no warning for trusted base {base}: {stderr}"
        );
    }
}

/// --dry-run against an untrusted host still warns (so the misdirection is
/// visible) but sends nothing.
#[tokio::test]
async fn dry_run_untrusted_host_warns_but_sends_nothing() {
    let server = MockServer::start().await;
    mock_thing_ok(&server).await;
    let (base, _resolve) = pinned_base(UNTRUSTED_HOST, &server);

    let output = run_cli(
        &base,
        &[("FLOWLEAP_TOKEN", "tok")],
        &[
            "api",
            "request",
            "get",
            "/v1/thing",
            "--output",
            "json",
            "--dry-run",
        ],
    )
    .await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value = stdout_json(&output);
    assert_eq!(value["dryRun"], true);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(
        stderr.matches(WARNING_MARKER).count(),
        1,
        "stderr: {stderr}"
    );
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        0,
        "dry-run never sends"
    );
}

/// No credentials in play → nothing to exfiltrate → no warning, even for an
/// untrusted host.
#[tokio::test]
async fn untrusted_host_without_credentials_stays_silent() {
    let server = MockServer::start().await;
    mock_thing_ok(&server).await;
    let (base, resolve) = pinned_base(UNTRUSTED_HOST, &server);

    let output = run_cli(
        &base,
        &[("FLOWLEAP_TEST_RESOLVE", &resolve)],
        &["api", "request", "get", "/v1/thing", "--output", "json"],
    )
    .await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        !stderr.contains(WARNING_MARKER),
        "no credentials, no warning: {stderr}"
    );
}

/// The global --yes flag parses and, in non-interactive mode, behaves like
/// the default: warn and proceed.
#[tokio::test]
async fn yes_flag_warns_and_proceeds() {
    let server = MockServer::start().await;
    mock_thing_ok(&server).await;
    let (base, resolve) = pinned_base(UNTRUSTED_HOST, &server);

    let output = run_cli(
        &base,
        &[
            ("FLOWLEAP_TEST_RESOLVE", &resolve),
            ("FLOWLEAP_TOKEN", "tok"),
        ],
        &[
            "api",
            "request",
            "get",
            "/v1/thing",
            "--output",
            "json",
            "--yes",
        ],
    )
    .await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(stdout_json(&output)["ok"], true);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(
        stderr.matches(WARNING_MARKER).count(),
        1,
        "--yes skips the prompt, not the warning: {stderr}"
    );
}
