//! Shared mock-HTTP test harness.
//!
//! Spins up a `wiremock` server and drives the built `flowleap` binary against
//! it in an isolated `HOME`, so tests exercise the real client (timeouts,
//! User-Agent, retry) end-to-end without touching a live backend or the user's
//! credentials. This is the foundation the exit-code (#20) and hint (#21)
//! slices build on — keep the entry points stable.
//!
//! Entry points:
//! - [`run_cli`] — run `flowleap <args>` against a base URL with extra env.
//! - [`stdout_json`] — parse a run's stdout as JSON.
//! - Callers create the server themselves via `wiremock::MockServer::start()`
//!   and pass `server.uri()` as the base URL.
#![allow(dead_code)]

use std::process::{Command, Output};

/// Run the built `flowleap` binary against `base_url` (typically a
/// `wiremock::MockServer::uri()`), with `envs` layered on top of a clean
/// environment: an isolated temp `HOME`, no ambient credentials, and the update
/// check disabled. The blocking subprocess runs off the async runtime so the
/// mock server keeps serving while it executes.
pub async fn run_cli(base_url: &str, envs: &[(&str, &str)], args: &[&str]) -> Output {
    let base_url = base_url.to_string();
    let envs: Vec<(String, String)> = envs
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();

    tokio::task::spawn_blocking(move || {
        let home = tempfile::tempdir().expect("create temp home");
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_flowleap"));
        cmd.env("HOME", home.path())
            .env("FLOWLEAP_BASE_URL", &base_url)
            .env("FLOWLEAP_NO_UPDATE_CHECK", "1")
            .env_remove("FLOWLEAP_TOKEN")
            .env_remove("FLOWLEAP_API_KEY");
        for (key, value) in &envs {
            cmd.env(key, value);
        }
        cmd.args(&args);
        cmd.output().expect("run flowleap binary")
    })
    .await
    .expect("join flowleap subprocess")
}

/// Parse a run's stdout as JSON. Panics with the raw stdout on failure.
pub fn stdout_json(output: &Output) -> serde_json::Value {
    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout is utf8");
    serde_json::from_str(&stdout).unwrap_or_else(|_| panic!("stdout was not json: {stdout}"))
}
