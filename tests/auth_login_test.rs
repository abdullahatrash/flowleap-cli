//! Structured device-flow login (issue #41): `flowleap --json auth login`
//! must emit a blocking NDJSON event stream on stdout — the
//! `device_authorization` event first (URL + user code for the agent to
//! relay), then exactly one terminal event (`authorized` exit 0 / `failed`
//! nonzero) — with nothing but NDJSON on stdout and the session token stored
//! exactly as the human flow stores it. Driven through the real binary
//! against a wiremock backend mocking the two unauthenticated device
//! endpoints.

mod support;

use std::process::Output;

use serde_json::json;
use support::run_cli;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Mount `POST /oauth/device` answering with a device authorization. The
/// `interval` drives the CLI's poll sleep — 0 keeps tests fast.
async fn mount_device_authorization(server: &MockServer, interval: u64, expires_in: u64) {
    Mock::given(method("POST"))
        .and(path("/oauth/device"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_code": "dev-code-123",
            "user_code": "ABCD-1234",
            "verification_uri": "https://flowleap.co/device",
            "verification_uri_complete": "https://flowleap.co/device?code=ABCD-1234",
            "expires_in": expires_in,
            "interval": interval,
        })))
        .mount(server)
        .await;
}

/// Mount `POST /oauth/device/token` answering `authorization_pending` exactly
/// once, so the next mounted token mock serves the terminal response. Proves
/// the process blocks through polling rather than stopping at the first poll.
async fn mount_pending_once(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/oauth/device/token"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(json!({ "error": "authorization_pending" })),
        )
        .up_to_n_times(1)
        .mount(server)
        .await;
}

/// Mount a terminal `POST /oauth/device/token` response.
async fn mount_token_response(server: &MockServer, template: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/oauth/device/token"))
        .respond_with(template)
        .mount(server)
        .await;
}

/// Parse stdout as NDJSON: every line must be a JSON object, so any stray
/// human-formatted output fails the test.
fn ndjson_events(output: &Output) -> Vec<serde_json::Value> {
    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout is utf8");
    stdout
        .lines()
        .map(|line| {
            serde_json::from_str(line).unwrap_or_else(|_| {
                panic!("stdout line was not JSON: {line:?}\nfull stdout: {stdout}")
            })
        })
        .collect()
}

/// pending → authorized: the first NDJSON line carries the URL and user code,
/// the terminal event is `authorized` with exit 0, and the session token is
/// stored in credentials.toml exactly as the human flow stores it.
#[tokio::test]
async fn json_login_pending_then_authorized_streams_events_and_stores_token() {
    let server = MockServer::start().await;
    mount_device_authorization(&server, 0, 300).await;
    mount_pending_once(&server).await;
    mount_token_response(
        &server,
        ResponseTemplate::new(200).set_body_json(json!({ "access_token": "jwt-session-token" })),
    )
    .await;

    // Own the HOME so the credentials file survives the run for inspection.
    let home = tempfile::tempdir().expect("create temp home");
    let home_str = home.path().to_str().expect("utf8 home").to_string();
    let xdg = home.path().join(".config");
    let xdg_str = xdg.to_str().expect("utf8 xdg").to_string();

    let output = run_cli(
        &server.uri(),
        &[("HOME", &home_str), ("XDG_CONFIG_HOME", &xdg_str)],
        &["--json", "auth", "login"],
    )
    .await;

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let events = ndjson_events(&output);
    assert_eq!(events.len(), 2, "exactly two NDJSON events: {events:?}");
    assert_eq!(events[0]["event"], "device_authorization");
    assert_eq!(events[0]["verification_uri"], "https://flowleap.co/device");
    assert_eq!(
        events[0]["verification_uri_complete"],
        "https://flowleap.co/device?code=ABCD-1234"
    );
    assert_eq!(events[0]["user_code"], "ABCD-1234");
    assert_eq!(events[0]["expires_in"], 300);
    assert_eq!(events[0]["interval"], 0);
    assert_eq!(events[1], json!({ "event": "authorized", "stored": true }));

    // Session token stored where the human flow stores it.
    let credentials_path = if cfg!(target_os = "macos") {
        home.path()
            .join("Library/Application Support/flowleap/credentials.toml")
    } else {
        xdg.join("flowleap/credentials.toml")
    };
    let credentials =
        std::fs::read_to_string(&credentials_path).expect("credentials.toml was written");
    assert!(
        credentials.contains("jwt-session-token"),
        "stored credentials must carry the session token: {credentials}"
    );
}

/// pending → access_denied: terminal event is `failed` with a denial error
/// and the documented generic-failure exit code (1).
#[tokio::test]
async fn json_login_denied_emits_failed_event_and_exits_nonzero() {
    let server = MockServer::start().await;
    mount_device_authorization(&server, 0, 300).await;
    mount_pending_once(&server).await;
    mount_token_response(
        &server,
        ResponseTemplate::new(400).set_body_json(json!({ "error": "access_denied" })),
    )
    .await;

    let output = run_cli(&server.uri(), &[], &["--json", "auth", "login"]).await;

    assert_eq!(output.status.code(), Some(1));
    let events = ndjson_events(&output);
    assert_eq!(events.len(), 2, "exactly two NDJSON events: {events:?}");
    assert_eq!(events[0]["event"], "device_authorization");
    assert_eq!(events[0]["user_code"], "ABCD-1234");
    assert_eq!(events[1]["event"], "failed");
    assert!(
        events[1]["error"]
            .as_str()
            .is_some_and(|e| e.contains("denied")),
        "failed event must describe the denial: {}",
        events[1]
    );
}

/// pending → expired_token: terminal event is `failed` with an expiry error
/// and the documented generic-failure exit code (1).
#[tokio::test]
async fn json_login_expired_emits_failed_event_and_exits_nonzero() {
    let server = MockServer::start().await;
    mount_device_authorization(&server, 0, 300).await;
    mount_pending_once(&server).await;
    mount_token_response(
        &server,
        ResponseTemplate::new(400).set_body_json(json!({ "error": "expired_token" })),
    )
    .await;

    let output = run_cli(&server.uri(), &[], &["--json", "auth", "login"]).await;

    assert_eq!(output.status.code(), Some(1));
    let events = ndjson_events(&output);
    assert_eq!(events.len(), 2, "exactly two NDJSON events: {events:?}");
    assert_eq!(events[0]["event"], "device_authorization");
    assert_eq!(events[1]["event"], "failed");
    assert!(
        events[1]["error"]
            .as_str()
            .is_some_and(|e| e.contains("expired")),
        "failed event must describe the expiry: {}",
        events[1]
    );
}
