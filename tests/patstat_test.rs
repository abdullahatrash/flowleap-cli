//! `flowleap patstat portfolio` (issue #32, PRD 0011): success rendering in
//! both output modes, the 422 ambiguous-applicant interaction step, the
//! typed `patstat_unavailable` unavailability, and an auth failure — driven
//! through the real binary against a wiremock backend, matching the
//! exit-code contract's test harness (see tests/exit_codes_test.rs).

mod support;

use serde_json::json;
use support::{run_cli, stdout_json};
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const API_KEY_ENV: (&str, &str) = ("FLOWLEAP_API_KEY", "fl_pat_test_key");

/// Canned successful /v1/patstat/portfolio body, shaped like the real
/// backend response (see flowleap-backend src/lib/patstat/portfolio.ts).
fn success_body() -> serde_json::Value {
    json!({
        "success": true,
        "applicant": {
            "query": "Siemens",
            "matched_name": "SIEMENS AG",
            "matched_psn_names": ["SIEMENS AG", "SIEMENS AKTIENGESELLSCHAFT"],
            "other_matches": [],
        },
        "filters": { "from_year": 2015, "to_year": 2024 },
        "totals": { "applications": 120, "granted": 80 },
        "by_year": [
            { "year": 2015, "applications": 10, "granted": 8 },
            { "year": 2016, "applications": 12, "granted": 9 },
        ],
        "by_office": [
            { "office": "EP", "applications": 70, "granted": 50 },
            { "office": "WO", "applications": 50, "granted": null },
        ],
        "by_year_office": [
            { "year": 2015, "office": "EP", "applications": 6, "granted": 5 },
        ],
        "grant_status_caveats": [
            "WO: PCT applications never grant at WIPO — grant status is structurally meaningless",
            "Grant counts for the flagged authorities are reported as null and excluded from totals.",
        ],
        "notes": [],
        "summary": "SIEMENS AG: 120 patent applications filed 2015–2024 across 2 offices \
                     (top: EP 70, WO 50); 80 granted among offices with reliable grant status. \
                     Source: 2024 Autumn.",
        "data_edition": "2024 Autumn",
    })
}

/// Canned 422 ambiguous-applicant error body (unified FlowLeap envelope).
fn ambiguous_body() -> serde_json::Value {
    json!({
        "success": false,
        "error": {
            "code": "patstat_applicant_ambiguous",
            "message": "\"Kia\" matches 2 distinct applicant entities: KIA MOTORS (500 \
                         applications), KIA CORPORATION (10 applications). These may be \
                         separate companies, so they are not merged automatically — retry \
                         with a more specific name (one of the entities listed).",
            "candidates": [
                { "name": "KIA MOTORS", "applications": 500 },
                { "name": "KIA CORPORATION", "applications": 10 },
            ],
        },
        "status": 422,
    })
}

/// Canned 503 `patstat_unavailable` error body.
fn unavailable_body() -> serde_json::Value {
    json!({
        "success": false,
        "error": {
            "code": "patstat_unavailable",
            "message": "The PATSTAT analytics layer is not configured on this deployment \
                         (PATSTAT_DATABASE_URL is unset). Aggregate portfolio analytics are \
                         unavailable — for individual documents use /v1/patent-search (EPO) or \
                         /v1/patent-search-uspto (USPTO) instead.",
        },
        "status": 503,
    })
}

async fn mount_portfolio(server: &MockServer, template: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/v1/patstat/portfolio"))
        .respond_with(template)
        .mount(server)
        .await;
}

#[tokio::test]
async fn portfolio_sends_the_documented_request_shape() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/patstat/portfolio"))
        .and(body_json(json!({
            "applicant": "Siemens",
            "fromYear": 2015,
            "toYear": 2024,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(success_body()))
        .mount(&server)
        .await;

    let output = run_cli(
        &server.uri(),
        &[API_KEY_ENV],
        &[
            "--json",
            "patstat",
            "portfolio",
            "Siemens",
            "--from-year",
            "2015",
            "--to-year",
            "2024",
        ],
    )
    .await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn portfolio_omits_absent_year_bounds() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/patstat/portfolio"))
        .and(body_json(json!({ "applicant": "Siemens" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(success_body()))
        .mount(&server)
        .await;

    let output = run_cli(
        &server.uri(),
        &[API_KEY_ENV],
        &["--json", "patstat", "portfolio", "Siemens"],
    )
    .await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn success_human_mode_renders_summary_tables_and_provenance() {
    let server = MockServer::start().await;
    mount_portfolio(
        &server,
        ResponseTemplate::new(200).set_body_json(success_body()),
    )
    .await;

    let output = run_cli(
        &server.uri(),
        &[API_KEY_ENV],
        &["patstat", "portfolio", "Siemens"],
    )
    .await;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");

    assert!(stdout.contains("SIEMENS AG: 120 patent applications"));
    assert!(stdout.contains("Filings by Year"));
    assert!(stdout.contains("Filings by Office"));
    assert!(stdout.contains("Grant status caveats:"));
    assert!(stdout.contains("PCT applications never grant at WIPO"));
    assert!(stdout.contains("Source: PATSTAT data edition 2024 Autumn"));
    // Real table cells, not a raw JSON dump.
    assert!(stdout.contains("2015"));
    assert!(!stdout.contains("\"by_year\""));
}

#[tokio::test]
async fn success_json_mode_emits_the_endpoint_body_untouched() {
    let server = MockServer::start().await;
    mount_portfolio(
        &server,
        ResponseTemplate::new(200).set_body_json(success_body()),
    )
    .await;

    let output = run_cli(
        &server.uri(),
        &[API_KEY_ENV],
        &["--json", "patstat", "portfolio", "Siemens"],
    )
    .await;

    assert!(output.status.success());
    let value = stdout_json(&output);

    assert_eq!(value["success"], true);
    assert_eq!(value["applicant"]["matched_name"], "SIEMENS AG");
    assert_eq!(value["totals"]["applications"], 120);
    assert_eq!(value["by_year"][0]["year"], 2015);
    assert_eq!(value["by_office"][1]["office"], "WO");
    assert!(value["by_office"][1]["granted"].is_null());
    assert_eq!(value["data_edition"], "2024 Autumn");
    // json mode passes the endpoint envelope through untouched — no CLI
    // wrapper fields.
    assert!(value.get("ok").is_none());
}

#[tokio::test]
async fn ambiguous_422_renders_candidates_in_human_mode() {
    let server = MockServer::start().await;
    mount_portfolio(
        &server,
        ResponseTemplate::new(422).set_body_json(ambiguous_body()),
    )
    .await;

    let output = run_cli(
        &server.uri(),
        &[API_KEY_ENV],
        &["patstat", "portfolio", "Kia"],
    )
    .await;

    assert!(!output.status.success());
    assert_ne!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");

    assert!(stdout.contains("Ambiguous applicant"));
    assert!(stdout.contains("Candidates:"));
    assert!(stdout.contains("KIA MOTORS (500 applications)"));
    assert!(stdout.contains("KIA CORPORATION (10 applications)"));
    assert!(stdout.contains("Re-run with one exact candidate name"));
    // Never a silent pick between the two entities.
    assert!(!stdout.contains("\"error\""));
}

#[tokio::test]
async fn ambiguous_422_renders_candidates_in_json_mode() {
    let server = MockServer::start().await;
    mount_portfolio(
        &server,
        ResponseTemplate::new(422).set_body_json(ambiguous_body()),
    )
    .await;

    let output = run_cli(
        &server.uri(),
        &[API_KEY_ENV],
        &["--json", "patstat", "portfolio", "Kia"],
    )
    .await;

    assert!(!output.status.success());
    assert_ne!(output.status.code(), Some(0));
    let value = stdout_json(&output);

    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "patstat_applicant_ambiguous");
    assert_eq!(value["error"]["candidates"][0]["name"], "KIA MOTORS");
    assert_eq!(value["error"]["candidates"][0]["applications"], 500);
    assert_eq!(value["error"]["candidates"][1]["name"], "KIA CORPORATION");
}

#[tokio::test]
async fn patstat_unavailable_renders_plainly_in_human_mode() {
    let server = MockServer::start().await;
    mount_portfolio(
        &server,
        ResponseTemplate::new(503).set_body_json(unavailable_body()),
    )
    .await;

    let output = run_cli(
        &server.uri(),
        &[API_KEY_ENV],
        &["patstat", "portfolio", "Siemens"],
    )
    .await;

    assert!(!output.status.success());
    assert_ne!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    assert!(stdout
        .contains("PATSTAT analytics unavailable: backend has no PATSTAT dataset configured."));
}

#[tokio::test]
async fn patstat_unavailable_renders_plainly_in_json_mode() {
    let server = MockServer::start().await;
    mount_portfolio(
        &server,
        ResponseTemplate::new(503).set_body_json(unavailable_body()),
    )
    .await;

    let output = run_cli(
        &server.uri(),
        &[API_KEY_ENV],
        &["--json", "patstat", "portfolio", "Siemens"],
    )
    .await;

    assert!(!output.status.success());
    assert_ne!(output.status.code(), Some(0));
    let value = stdout_json(&output);
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "patstat_unavailable");
    assert!(value["error"]["message"]
        .as_str()
        .is_some_and(|m| m.contains("not configured")));
}

/// An HTTP 401 (rejected/expired credentials) is not retried and exits with
/// the documented auth-required code, same as every other authenticated
/// command.
#[tokio::test]
async fn auth_failure_401_exits_with_auth_required_code() {
    let server = MockServer::start().await;
    mount_portfolio(
        &server,
        ResponseTemplate::new(401).set_body_json(json!({ "error": "unauthorized" })),
    )
    .await;

    let output = run_cli(
        &server.uri(),
        &[API_KEY_ENV],
        &["--json", "patstat", "portfolio", "Siemens"],
    )
    .await;

    assert_eq!(output.status.code(), Some(3));
    let value = stdout_json(&output);
    assert_eq!(value["ok"], false);
    assert_eq!(value["status"], 401);
}

/// With no credentials configured at all, the command fails fast locally
/// (never reaches the network) — same `require_auth` guard every other
/// authenticated command uses.
#[tokio::test]
async fn missing_credentials_fails_locally_without_a_network_call() {
    let output = run_cli(
        "http://127.0.0.1:9",
        &[],
        &["--json", "patstat", "portfolio", "Siemens"],
    )
    .await;

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stderr.contains("Not authenticated") || stdout.contains("Not authenticated"),
        "stdout: {stdout}\nstderr: {stderr}"
    );
}
