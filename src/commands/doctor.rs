use anyhow::Result;
use serde_json::{json, Value};

use crate::client::Context;
use crate::config::{Config, Credentials};
use crate::output;

pub async fn run(ctx: &Context) -> Result<()> {
    let config_path = Config::config_path()?;
    let credentials_path = Credentials::credentials_path()?;
    let auth_source = auth_source(ctx);

    let health = ctx
        .execute_json_envelope(ctx.request(reqwest::Method::GET, "/health", None))
        .await;

    let (reachable, status, error, error_kind, hint) = match health {
        Ok(value) => {
            let reachable = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            let status = value.get("status").and_then(|v| v.as_u64());
            let hint = if reachable {
                None
            } else {
                Some("Backend responded but not with a successful status.")
            };
            (reachable, status, None, None, hint)
        }
        Err(err) => {
            let message = err.to_string();
            let (kind, hint) = classify_error(&message);
            (false, None, Some(message), Some(kind), Some(hint))
        }
    };

    let authenticated = ctx.credentials.auth_header().is_some();

    // Best-effort server verdicts (POST /v1/keys/validate) so "key missing
    // locally but covered by the server" produces no next step. Any failure —
    // unauthenticated, unreachable, HTTP error — falls back to local key
    // presence; doctor never errors because of this call.
    let verdicts: Option<Value> = if reachable && authenticated && !ctx.dry_run {
        crate::commands::keys::validate(ctx, ctx.credentials.clone())
            .await
            .ok()
    } else {
        None
    };

    let next_steps = next_steps(ctx, authenticated, verdicts.as_ref());
    // Ready means nothing blocks work — stricter than `ok` (reachability).
    let ready = reachable && authenticated && next_steps.is_empty();

    let key_validation = match &verdicts {
        Some(_) => json!({ "source": "server", "note": Value::Null }),
        None => json!({
            "source": "local",
            "note": "Server key validation was unavailable (unauthenticated, unreachable, or the call failed) — provider verdicts reflect local key presence only. Missing keys may still be covered by the server; check with 'flowleap keys test' once authenticated.",
        }),
    };

    let report = json!({
        "ok": reachable,
        "ready": ready,
        "command": "flowleap",
        "baseUrl": ctx.config.base_url,
        "auth": {
            "available": ctx.credentials.auth_header().is_some(),
            "source": auth_source,
            "setup": if ctx.credentials.auth_header().is_some() {
                serde_json::Value::Null
            } else {
                json!("Run `flowleap auth login --api-key <key>` or set FLOWLEAP_API_KEY/FLOWLEAP_TOKEN.")
            },
        },
        "config": {
            "path": config_path,
            "credentialsPath": credentials_path,
            "defaultModel": ctx.config.default_model,
            "outputFormat": ctx.config.output_format,
        },
        "providerKeys": {
            "epo": ctx.credentials.epo_pair().is_some(),
            "epoIncompletePair": ctx.credentials.epo_pair().is_none()
                && (ctx.credentials.epo_key.is_some() || ctx.credentials.epo_secret.is_some()),
            "uspto": ctx.credentials.uspto_key.is_some(),
            "setup": if ctx.credentials.epo_pair().is_some() && ctx.credentials.uspto_key.is_some() {
                serde_json::Value::Null
            } else {
                json!("Missing keys may be fine if the server has its own — check with 'flowleap keys test'. Add local keys via 'flowleap setup' (interactive) or 'flowleap keys set <provider>'.")
            },
        },
        "keyValidation": key_validation,
        "backend": {
            "reachable": reachable,
            "healthStatus": status,
            "error": error,
            "errorKind": error_kind,
            "hint": hint,
        },
        "cli": cli_status(),
        "skills": crate::commands::skills::doctor_skills_status(env!("CARGO_PKG_VERSION")),
        "nextSteps": next_steps,
    });

    output::print_value(&ctx.output_format, &report, &[]);

    // Exit contract: 0 iff ready, else the generic-failure code (1). The
    // checklist above is always fully emitted first; PrintedError tells the
    // top-level handler not to print anything more. Dry-run sends nothing and
    // so can never prove readiness — it keeps its historical success exit.
    if ready || ctx.dry_run {
        Ok(())
    } else {
        Err(crate::client::PrintedError::new().into())
    }
}

/// The pending, blocking onboarding steps in dependency order. Step ids are a
/// public contract (see docs/adr/0001): `auth-login`, `mint-personal-token`,
/// `obtain-epo-keys`, `store-epo-keys`, `obtain-uspto-key`, `store-uspto-key`,
/// `verify-keys`. Steps whose need is already covered (e.g. a provider the
/// server has its own keys for) are omitted — the list means "what blocks
/// you", not "what could be configured".
fn next_steps(ctx: &Context, authenticated: bool, verdicts: Option<&Value>) -> Vec<Value> {
    let mut steps = Vec::new();

    if !authenticated {
        steps.push(step(
            "auth-login",
            "human",
            "Sign in to FlowLeap (browser device flow — run the command, relay the verification URL and code to the human, and wait for approval)",
            Some("flowleap --json auth login"),
            None,
        ));
    } else if session_only(&ctx.credentials) {
        steps.push(step(
            "mint-personal-token",
            "agent",
            "Mint and store a long-lived personal token — the session token expires on its own",
            Some("flowleap --json auth create-token --name <n> --store"),
            None,
        ));
    }

    let epo_pending = provider_pending(verdicts, "epo", ctx.credentials.epo_pair().is_some());
    let uspto_pending = provider_pending(verdicts, "uspto", ctx.credentials.uspto_key.is_some());

    if epo_pending {
        steps.push(step(
            "obtain-epo-keys",
            "human",
            "Sign up for free EPO OPS credentials (browser: 'My apps' → create app)",
            None,
            Some(crate::commands::keys::EPO_SIGNUP),
        ));
        steps.push(step(
            "store-epo-keys",
            "agent",
            "Store the EPO consumer key and secret",
            Some("flowleap keys set epo --key <k> --secret <s>"),
            None,
        ));
    }
    if uspto_pending {
        steps.push(step(
            "obtain-uspto-key",
            "human",
            "Sign up for a free USPTO ODP API key (browser)",
            None,
            Some(crate::commands::keys::USPTO_SIGNUP),
        ));
        steps.push(step(
            "store-uspto-key",
            "agent",
            "Store the USPTO ODP API key",
            Some("flowleap keys set uspto --key <k>"),
            None,
        ));
    }
    if epo_pending || uspto_pending {
        steps.push(step(
            "verify-keys",
            "agent",
            "Verify the stored provider keys against the live providers",
            Some("flowleap --json keys test"),
            None,
        ));
    }

    steps
}

/// Session-only auth: signed in with a short-lived Clerk session token and no
/// durable fl_pat_ personal token to fall back on. Durability is a pending
/// step (`mint-personal-token`) until one exists.
fn session_only(creds: &Credentials) -> bool {
    creds.token.is_some()
        && !creds
            .api_key
            .as_deref()
            .is_some_and(|key| key.starts_with("fl_pat_"))
}

/// Whether a provider blocks work. With server verdicts (source
/// user|server|none, valid true|false|null): server-covered providers never
/// block, and — mirroring `keys test` — a provider blocks only when provably
/// absent everywhere (`source: "none"`) or provably invalid (`valid: false`).
/// Without verdicts (unauthenticated / call failed), fall back to local key
/// presence.
fn provider_pending(verdicts: Option<&Value>, provider: &str, local_present: bool) -> bool {
    match verdicts {
        Some(verdicts) => {
            let verdict = &verdicts[provider];
            let source = verdict["source"].as_str().unwrap_or("unknown");
            source != "server" && (verdict["valid"] == Value::Bool(false) || source == "none")
        }
        None => !local_present,
    }
}

/// One next step: stable kebab-case id, exactly one actor ("human" |
/// "agent"), a title, and optionally a runnable command and/or a URL.
fn step(id: &str, actor: &str, title: &str, run: Option<&str>, url: Option<&str>) -> Value {
    let mut step = json!({
        "id": id,
        "actor": actor,
        "title": title,
    });
    if let Some(run) = run {
        step["run"] = json!(run);
    }
    if let Some(url) = url {
        step["url"] = json!(url);
    }
    step
}

/// CLI self-status: install channel and any known-available upgrade, read
/// from the daily update cache (no extra network call) so doctor stays
/// offline-safe. Always recommends the channel-aware `flowleap upgrade`.
fn cli_status() -> serde_json::Value {
    let current = env!("CARGO_PKG_VERSION");
    let channel = crate::commands::upgrade::current_channel();
    let latest = crate::update::cached_latest();
    let update_available = latest
        .as_deref()
        .map(|l| crate::update::is_newer(l, current));
    let hint = if update_available == Some(true) {
        Some(format!(
            "flowleap {} is available (you have {current}) — run 'flowleap upgrade'",
            latest.as_deref().unwrap_or_default()
        ))
    } else {
        None
    };
    json!({
        "channel": channel.as_str(),
        "currentVersion": current,
        "latestVersion": latest,
        "updateAvailable": update_available,
        "upgradeCommand": "flowleap upgrade",
        "hint": hint,
    })
}

fn classify_error(message: &str) -> (&'static str, &'static str) {
    let lower = message.to_ascii_lowercase();
    if lower.contains("operation not permitted") {
        (
            "network-permission",
            "The runtime blocked network access. Retry with network permission or run from a normal shell.",
        )
    } else if lower.contains("connection refused") || lower.contains("couldn't connect") {
        (
            "connection-refused",
            "No backend accepted the connection. Start the backend or check --base-url.",
        )
    } else if lower.contains("dns") || lower.contains("failed to lookup") {
        (
            "dns",
            "The host could not be resolved. Check --base-url and network connectivity.",
        )
    } else if lower.contains("timed out") || lower.contains("timeout") {
        (
            "timeout",
            "The backend did not respond before the request timed out.",
        )
    } else {
        (
            "request-error",
            "The health request failed. Check --base-url, backend status, and network access.",
        )
    }
}

fn auth_source(ctx: &Context) -> &'static str {
    if std::env::var("FLOWLEAP_TOKEN").is_ok() {
        "env-token"
    } else if std::env::var("FLOWLEAP_API_KEY").is_ok() {
        "env-api-key"
    } else if ctx.credentials.token.is_some() {
        "config-token"
    } else if ctx.credentials.api_key.is_some() {
        "config-api-key"
    } else {
        "missing"
    }
}
