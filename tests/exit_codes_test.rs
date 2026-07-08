//! Exit-code contract + structured 402/429 hints (issue #20): each HTTP
//! status class must produce its documented exit code, and the 402/429
//! envelopes must carry their additive hints — driven through the real binary
//! against a wiremock backend. The contract table lives in AGENTS.md.

mod support;

use std::time::Duration;

use serde_json::json;
use support::{run_cli, stdout_json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Mount a GET /v1/thing mock answering `template`, exactly once expected.
async fn mount_thing(server: &MockServer, template: ResponseTemplate) {
    Mock::given(method("GET"))
        .and(path("/v1/thing"))
        .respond_with(template)
        .mount(server)
        .await;
}

/// Run `api request get /v1/thing --output json` against the server.
async fn request_thing_json(server: &MockServer) -> std::process::Output {
    run_cli(
        &server.uri(),
        &[],
        &["api", "request", "get", "/v1/thing", "--output", "json"],
    )
    .await
}

/// HTTP 401 → exit 3 (auth required).
#[tokio::test]
async fn auth_required_401_exits_3() {
    let server = MockServer::start().await;
    mount_thing(
        &server,
        ResponseTemplate::new(401).set_body_json(json!({ "error": "unauthorized" })),
    )
    .await;

    let output = request_thing_json(&server).await;

    assert_eq!(output.status.code(), Some(3));
    let value = stdout_json(&output);
    assert_eq!(value["ok"], false);
    assert_eq!(value["status"], 401);
}

/// HTTP 402 → exit 4, with a subscription hint carrying the upgrade URL the
/// backend sent.
#[tokio::test]
async fn subscription_required_402_exits_4_with_hint() {
    let server = MockServer::start().await;
    mount_thing(
        &server,
        ResponseTemplate::new(402).set_body_json(json!({
            "error": "subscription_required",
            "upgradeUrl": "https://flowleap.co/upgrade-here",
        })),
    )
    .await;

    let output = request_thing_json(&server).await;

    assert_eq!(output.status.code(), Some(4));
    let value = stdout_json(&output);
    assert_eq!(value["ok"], false);
    assert_eq!(value["status"], 402);
    let hint = &value["subscriptionHint"];
    assert_eq!(hint["requiresHumanIntervention"], true);
    assert_eq!(hint["plan"], "Basic");
    assert_eq!(hint["upgradeUrl"], "https://flowleap.co/upgrade-here");
    assert!(
        hint["message"].as_str().is_some_and(|m| !m.is_empty()),
        "hint must carry a message: {hint}"
    );
}

/// A 402 body without an upgrade URL falls back to the pricing page.
#[tokio::test]
async fn subscription_hint_falls_back_to_pricing_url() {
    let server = MockServer::start().await;
    mount_thing(
        &server,
        ResponseTemplate::new(402).set_body_json(json!({ "error": "subscription_required" })),
    )
    .await;

    let output = request_thing_json(&server).await;

    assert_eq!(output.status.code(), Some(4));
    let value = stdout_json(&output);
    assert_eq!(
        value["subscriptionHint"]["upgradeUrl"],
        "https://flowleap.co/pricing"
    );
}

/// In human mode the 402 hint renders as an upgrade box on stderr, and stdout
/// stays free of it.
#[tokio::test]
async fn human_mode_402_renders_upgrade_box_on_stderr() {
    let server = MockServer::start().await;
    mount_thing(
        &server,
        ResponseTemplate::new(402).set_body_json(json!({ "error": "subscription_required" })),
    )
    .await;

    let output = run_cli(&server.uri(), &[], &["api", "request", "get", "/v1/thing"]).await;

    assert_eq!(output.status.code(), Some(4));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("subscription required") && stderr.contains("https://flowleap.co/pricing"),
        "expected an upgrade box on stderr, got: {stderr}"
    );
}

/// HTTP 404 → exit 5 (not found).
#[tokio::test]
async fn not_found_404_exits_5() {
    let server = MockServer::start().await;
    mount_thing(
        &server,
        ResponseTemplate::new(404).set_body_json(json!({ "error": "not found" })),
    )
    .await;

    let output = request_thing_json(&server).await;

    assert_eq!(output.status.code(), Some(5));
    let value = stdout_json(&output);
    assert_eq!(value["status"], 404);
}

/// HTTP 429 → exit 6, with a rate-limit hint carrying retryAfterSeconds.
#[tokio::test]
async fn rate_limited_429_exits_6_with_hint() {
    let server = MockServer::start().await;
    mount_thing(
        &server,
        ResponseTemplate::new(429)
            .insert_header("retry-after", "30")
            .set_body_json(json!({ "error": "rate limited" })),
    )
    .await;

    let output = request_thing_json(&server).await;

    assert_eq!(output.status.code(), Some(6));
    let value = stdout_json(&output);
    assert_eq!(value["status"], 429);
    let hint = &value["rateLimitHint"];
    assert_eq!(hint["retryAfterSeconds"], 30);
    assert!(
        hint["message"].as_str().is_some_and(|m| !m.is_empty()),
        "hint must carry a message: {hint}"
    );
}

/// A 429 without Retry-After still exits 6 and still carries the hint.
#[tokio::test]
async fn rate_limited_without_retry_after_still_hints() {
    let server = MockServer::start().await;
    mount_thing(
        &server,
        ResponseTemplate::new(429).set_body_json(json!({ "error": "rate limited" })),
    )
    .await;

    let output = request_thing_json(&server).await;

    assert_eq!(output.status.code(), Some(6));
    let value = stdout_json(&output);
    let hint = &value["rateLimitHint"];
    assert!(hint["retryAfterSeconds"].is_null());
    assert!(
        hint["message"].as_str().is_some_and(|m| !m.is_empty()),
        "hint must carry a message: {hint}"
    );
}

/// A request timeout → exit 7 (network).
#[tokio::test]
async fn timeout_exits_7() {
    let server = MockServer::start().await;
    mount_thing(
        &server,
        ResponseTemplate::new(200).set_delay(Duration::from_secs(5)),
    )
    .await;

    let output = run_cli(
        &server.uri(),
        &[("FLOWLEAP_TIMEOUT_SECS", "1")],
        &["api", "request", "get", "/v1/thing", "--output", "json"],
    )
    .await;

    assert_eq!(output.status.code(), Some(7));
}

/// A connection failure → exit 7 (network).
#[tokio::test]
async fn connection_refused_exits_7() {
    // Port 9 (discard) is closed on any sane test machine; no server started.
    let output = run_cli(
        "http://127.0.0.1:9",
        &[("FLOWLEAP_MAX_RETRIES", "0")],
        &["api", "request", "get", "/v1/thing", "--output", "json"],
    )
    .await;

    assert_eq!(output.status.code(), Some(7));
}

/// A 5xx without a dedicated code stays a generic failure → exit 1.
#[tokio::test]
async fn server_error_500_exits_1() {
    let server = MockServer::start().await;
    mount_thing(
        &server,
        ResponseTemplate::new(500).set_body_json(json!({ "error": "boom" })),
    )
    .await;

    let output = run_cli(
        &server.uri(),
        &[("FLOWLEAP_MAX_RETRIES", "0")],
        &["api", "request", "get", "/v1/thing", "--output", "json"],
    )
    .await;

    assert_eq!(output.status.code(), Some(1));
    let value = stdout_json(&output);
    assert_eq!(value["status"], 500);
}

/// The 402 hint fields are additive: the pre-existing envelope shape
/// (ok/status/contentType/body) is untouched.
#[tokio::test]
async fn hint_fields_are_additive_to_the_envelope() {
    let server = MockServer::start().await;
    mount_thing(
        &server,
        ResponseTemplate::new(402).set_body_json(json!({ "error": "subscription_required" })),
    )
    .await;

    let output = request_thing_json(&server).await;
    let value = stdout_json(&output);

    assert_eq!(value["ok"], false);
    assert_eq!(value["status"], 402);
    assert_eq!(value["body"]["error"], "subscription_required");
    assert!(value["contentType"].is_string());
}
