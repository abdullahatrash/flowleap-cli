//! `flowleap mcp` — a stdio MCP server bridging the `/v1/tools` facade.
//!
//! A thin bridge with no per-tool code and no name mapping: the backend's
//! `/v1/tools` vocabulary is canonical. `tools/list` passes each registry
//! entry through verbatim (the backend already publishes MCP-shaped
//! `{ name, description, inputSchema }` objects), so new backend tools appear
//! in every MCP harness without a CLI release. `tools/call` runs a tool via
//! the same facade path as `flowleap tools run`.
//!
//! Protocol discipline: nothing but JSON-RPC frames on stdout (one per line);
//! all logging goes to stderr. Tool-level failures — including the backend's
//! structured `providerKeysHint` / subscription / rate-limit envelopes — are
//! returned as MCP tool results with `isError: true`, never as JSON-RPC
//! transport errors, so the calling agent can read the hint and act.

use anyhow::Result;
use clap::Parser;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::client::Context;
use crate::commands::tools;

/// Protocol version offered when the client requests one we don't know.
const FALLBACK_PROTOCOL_VERSION: &str = "2024-11-05";
/// Versions we can mirror back — the tools-only surface is identical in all.
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2024-11-05", "2025-03-26", "2025-06-18"];

const AUTH_REQUIRED_MESSAGE: &str = "Not authenticated with FlowLeap. Run 'flowleap auth login' \
     (or set FLOWLEAP_API_KEY / FLOWLEAP_TOKEN), then restart the MCP server.";

const HARNESS_WIRING_HELP: &str = "\
Wire into an MCP-capable harness:

  Claude Code:
    claude mcp add flowleap -- flowleap mcp

  Cursor (~/.cursor/mcp.json or .cursor/mcp.json):
    {
      \"mcpServers\": {
        \"flowleap\": { \"command\": \"flowleap\", \"args\": [\"mcp\"] }
      }
    }

  Codex (~/.codex/config.toml):
    [mcp_servers.flowleap]
    command = \"flowleap\"
    args = [\"mcp\"]

Authentication reuses stored credentials: run 'flowleap auth login' once
before wiring the server (unauthenticated servers still start, but every
tool call returns an error explaining how to log in).";

/// Serve FlowLeap backend tools over the Model Context Protocol (stdio).
///
/// Bridges the /v1/tools facade: tools/list mirrors every backend tool with
/// its JSON input schema verbatim, tools/call runs one. Speaks JSON-RPC 2.0,
/// one frame per line, on stdin/stdout.
#[derive(Parser)]
#[command(after_long_help = HARNESS_WIRING_HELP)]
pub struct McpArgs {
    /// Print readiness diagnostics (backend, auth, provider keys, tool count)
    /// and exit instead of serving — the one-command onboarding check.
    #[arg(long)]
    pub check: bool,
}

pub async fn run(ctx: &Context, args: McpArgs) -> Result<()> {
    if args.check {
        return run_check(ctx).await;
    }
    // stdout carries protocol frames only; everything human goes to stderr.
    eprintln!(
        "flowleap mcp v{}: serving MCP over stdio (backend: {})",
        env!("CARGO_PKG_VERSION"),
        ctx.config.base_url
    );
    if ctx.credentials.auth_header().is_none() {
        eprintln!(
            "flowleap mcp: no stored credentials — tool calls will ask for 'flowleap auth login'"
        );
    }

    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let Some(response) = handle_line(ctx, &line).await else {
            continue; // notification — no frame goes back
        };
        let mut frame = serde_json::to_string(&response)?;
        frame.push('\n');
        stdout.write_all(frame.as_bytes()).await?;
        stdout.flush().await?;
    }
    Ok(())
}

/// `flowleap mcp --check` — readiness diagnostics for harness onboarding.
/// This is NOT protocol mode: results go to stdout (JSON when requested),
/// remediation hints to stderr, and the exit code reflects readiness.
async fn run_check(ctx: &Context) -> Result<()> {
    let wants_json = ctx.output_format == "json";

    // 1. Backend reachability (no auth required).
    let backend_ok = ctx
        .http
        .get(format!("{}/health", ctx.config.base_url))
        .send()
        .await
        .map(|resp| resp.status().is_success())
        .unwrap_or(false);

    // 2. Stored/ambient FlowLeap auth.
    let auth_kind = if ctx.credentials.token.is_some() {
        Some("session token")
    } else if ctx.credentials.api_key.is_some() {
        Some("personal token / API key")
    } else {
        None
    };

    // 3. Provider keys (presence only — never values).
    let epo_ok = ctx.credentials.epo_key.is_some() && ctx.credentials.epo_secret.is_some();
    let uspto_ok = ctx.credentials.uspto_key.is_some();

    // 4. Live tool list — also proves the auth actually works.
    let tool_count = if auth_kind.is_some() && backend_ok {
        match tools::fetch_tools_envelope(ctx).await {
            Ok(envelope) => envelope
                .get("body")
                .and_then(|body| body.get("tools"))
                .and_then(Value::as_array)
                .map(|tools| tools.len()),
            Err(_) => None,
        }
    } else {
        None
    };

    let ready = backend_ok && auth_kind.is_some() && tool_count.is_some();

    if wants_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ready": ready,
                "backend": { "url": ctx.config.base_url, "reachable": backend_ok },
                "auth": { "configured": auth_kind.is_some(), "kind": auth_kind, "verified": tool_count.is_some() },
                "providerKeys": { "epoOps": epo_ok, "usptoOdp": uspto_ok },
                "toolCount": tool_count,
            }))?
        );
    } else {
        let mark = |ok: bool| if ok { "ok " } else { "MISSING" };
        println!("flowleap mcp readiness check");
        println!("  backend   {}  {}", mark(backend_ok), ctx.config.base_url);
        println!(
            "  auth      {}  {}",
            mark(auth_kind.is_some() && tool_count.is_some()),
            auth_kind.unwrap_or("no credentials stored")
        );
        println!(
            "  epo ops   {}  (optional: unlocks EPO-gated data)",
            mark(epo_ok)
        );
        println!(
            "  uspto odp {}  (optional: unlocks USPTO-gated data)",
            mark(uspto_ok)
        );
        match tool_count {
            Some(count) => println!("  tools     ok   {count} tools served over MCP"),
            None => println!("  tools     n/a  (needs backend + working auth)"),
        }
    }

    if !ready {
        if auth_kind.is_none() {
            eprintln!(
                "fix: run 'flowleap auth login' (or set FLOWLEAP_TOKEN in the MCP config env)"
            );
        } else if backend_ok && tool_count.is_none() {
            eprintln!("fix: credentials present but rejected — re-run 'flowleap auth login'");
        }
        if !backend_ok {
            eprintln!("fix: backend unreachable — check network or base URL ('flowleap doctor')");
        }
        anyhow::bail!("mcp is not ready to serve");
    }
    if !epo_ok || !uspto_ok {
        eprintln!("note: provider keys are optional; add via 'flowleap keys set' to unlock provider-gated data");
    }
    Ok(())
}

/// Parse one inbound line and produce the response frame, if any.
async fn handle_line(ctx: &Context, line: &str) -> Option<Value> {
    let message: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(err) => {
            return Some(error_response(
                Value::Null,
                -32700,
                &format!("Parse error: {err}"),
                None,
            ))
        }
    };

    let method = message
        .get("method")
        .and_then(|m| m.as_str())
        .unwrap_or_default()
        .to_string();
    // Absent id = notification (no response). A present id — even null — is
    // echoed back so the client can correlate.
    let id = message.get("id").cloned();
    let params = message.get("params").cloned().unwrap_or(Value::Null);

    match method.as_str() {
        "initialize" => id.map(|id| result_response(id, initialize_result(&params))),
        "notifications/initialized" => None,
        "ping" => id.map(|id| result_response(id, json!({}))),
        "tools/list" => match id {
            Some(id) => Some(tools_list(ctx, id).await),
            None => None,
        },
        "tools/call" => match id {
            Some(id) => Some(tools_call(ctx, id, &params).await),
            None => None,
        },
        _ => id.map(|id| error_response(id, -32601, &format!("Method not found: {method}"), None)),
    }
}

/// Mirror the client's protocol version when we support it; otherwise offer
/// the oldest version we speak, per the MCP version-negotiation rules.
fn initialize_result(params: &Value) -> Value {
    let requested = params.get("protocolVersion").and_then(|v| v.as_str());
    let version = match requested {
        Some(v) if SUPPORTED_PROTOCOL_VERSIONS.contains(&v) => v,
        _ => FALLBACK_PROTOCOL_VERSION,
    };
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {} },
        "serverInfo": {
            "name": "flowleap",
            "version": env!("CARGO_PKG_VERSION"),
        },
    })
}

/// tools/list ← GET /v1/tools. Registry entries pass through verbatim — the
/// backend already publishes MCP-shaped `{ name, description, inputSchema }`
/// objects, and extra fields are legal for MCP clients to ignore.
async fn tools_list(ctx: &Context, id: Value) -> Value {
    if ctx.credentials.auth_header().is_none() {
        return error_response(id, -32002, AUTH_REQUIRED_MESSAGE, None);
    }
    let envelope = match tools::fetch_tools_envelope(ctx).await {
        Ok(envelope) => envelope,
        Err(err) => return error_response(id, -32603, &err.to_string(), None),
    };
    if envelope.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        let status = envelope.get("status").and_then(|v| v.as_u64()).unwrap_or(0);
        return error_response(
            id,
            -32002,
            &format!("FlowLeap backend rejected tools/list (HTTP {status})"),
            Some(envelope),
        );
    }
    let tools = envelope
        .get("body")
        .and_then(|body| body.get("tools"))
        .and_then(|tools| tools.as_array())
        .cloned()
        .unwrap_or_default();
    result_response(id, json!({ "tools": tools }))
}

/// tools/call ← POST /v1/tools/{name}. Success returns the tool payload as
/// pretty JSON text; backend error envelopes (providerKeysHint, subscription,
/// rate-limit) come back as `isError` tool results carrying the full
/// structured envelope — never as JSON-RPC transport errors.
async fn tools_call(ctx: &Context, id: Value, params: &Value) -> Value {
    if ctx.credentials.auth_header().is_none() {
        return tool_error(id, json!({ "error": AUTH_REQUIRED_MESSAGE }));
    }
    let Some(name) = params.get("name").and_then(|n| n.as_str()) else {
        return error_response(id, -32602, "Invalid params: missing tool 'name'", None);
    };
    let arguments = match params.get("arguments") {
        None | Some(Value::Null) => json!({}),
        Some(value @ Value::Object(_)) => value.clone(),
        Some(_) => {
            return error_response(
                id,
                -32602,
                "Invalid params: 'arguments' must be an object",
                None,
            )
        }
    };

    let envelope = match tools::call_tool_envelope(ctx, name, &arguments).await {
        Ok(envelope) => envelope,
        // Transport failure (timeout, connection refused): still a tool-level
        // error the agent should read, not a protocol error.
        Err(err) => return tool_error(id, json!({ "error": err.to_string() })),
    };

    if envelope.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        // Backend tool envelope: { success, tool, data, executionTimeMs } —
        // surface `data` (like `flowleap tools run`), whole body otherwise.
        let body = envelope.get("body").cloned().unwrap_or(Value::Null);
        let payload = body.get("data").cloned().unwrap_or(body);
        return text_result(id, false, &payload);
    }
    // The envelope already carries status, body, and any structured hints
    // (providerKeysHint, retryAfterSeconds) — pass it through whole.
    tool_error(id, envelope)
}

fn result_response(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Value, code: i64, message: &str, data: Option<Value>) -> Value {
    let mut error = json!({ "code": code, "message": message });
    if let Some(data) = data {
        error["data"] = data;
    }
    json!({ "jsonrpc": "2.0", "id": id, "error": error })
}

/// A successful (or `isError`) MCP tool result with one pretty-JSON text block.
fn text_result(id: Value, is_error: bool, payload: &Value) -> Value {
    let text = serde_json::to_string_pretty(payload).unwrap_or_else(|_| payload.to_string());
    let mut result = json!({ "content": [{ "type": "text", "text": text }] });
    if is_error {
        result["isError"] = json!(true);
    }
    result_response(id, result)
}

fn tool_error(id: Value, payload: Value) -> Value {
    text_result(id, true, &payload)
}
