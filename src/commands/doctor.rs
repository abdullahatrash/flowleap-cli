use anyhow::Result;
use serde_json::json;

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

    let report = json!({
        "ok": reachable,
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
        "backend": {
            "reachable": reachable,
            "healthStatus": status,
            "error": error,
            "errorKind": error_kind,
            "hint": hint,
        },
    });

    output::print_value(&ctx.output_format, &report, &[]);
    Ok(())
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
