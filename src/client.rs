use anyhow::{bail, Result};
use reqwest::{Client, Method, Request, RequestBuilder, Response};
use serde_json::{json, Value};

use crate::config::{Config, Credentials};

#[derive(Debug)]
pub struct PrintedError;

impl std::fmt::Display for PrintedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "request failed")
    }
}

impl std::error::Error for PrintedError {}

/// Headers that must never appear in verbose or debug output.
const SECRET_HEADERS: &[&str] = &[
    "authorization",
    "x-api-key",
    "x-epo-ops-key",
    "x-epo-ops-secret",
    "x-uspto-odp-key",
];

fn is_secret_header(name: &reqwest::header::HeaderName) -> bool {
    SECRET_HEADERS.contains(&name.as_str())
}

/// Detect provider-key problems in an error response and produce a structured,
/// agent-parseable hint. Returns None when the error is unrelated to keys.
///
/// Signals (from the backend's patentKeys middleware and provider libs):
/// - `patent_provider_key_invalid`  → user-supplied keys were rejected
/// - `EPO_CLIENT_ID` / `EPO_CLIENT_SECRET` in an error → no EPO keys anywhere
/// - `USPTO_ODP_API_KEY` / "USPTO ODP API key not configured" → no USPTO key
pub fn provider_keys_hint(status: u16, body: &Value) -> Option<Value> {
    if status < 400 {
        return None;
    }
    let text = body.to_string();
    let (code, provider) = if text.contains("patent_provider_key_invalid") {
        let provider = if text.to_lowercase().contains("uspto") {
            "uspto"
        } else {
            "epo"
        };
        ("provider_keys_invalid", provider)
    } else if text.contains("EPO_CLIENT_ID") || text.contains("EPO_CLIENT_SECRET") {
        ("provider_keys_required", "epo")
    } else if text.contains("USPTO_ODP_API_KEY")
        || text.contains("USPTO ODP API key not configured")
    {
        ("provider_keys_required", "uspto")
    } else {
        return None;
    };

    Some(json!({
        "code": code,
        "provider": provider,
        "requiresHumanIntervention": true,
        "humanAction": "Run 'flowleap setup' (or 'flowleap keys set') in a terminal. Getting keys involves a browser signup, so an agent cannot complete this alone — ask the user.",
        "nonInteractive": {
            "command": if provider == "epo" {
                "flowleap keys set epo --key <consumer-key> --secret <consumer-secret>"
            } else {
                "flowleap keys set uspto --key <api-key>"
            },
            "env": if provider == "epo" {
                json!(["FLOWLEAP_EPO_KEY", "FLOWLEAP_EPO_SECRET"])
            } else {
                json!(["FLOWLEAP_USPTO_KEY"])
            },
        },
        "signup": if provider == "epo" {
            "https://developers.epo.org (free, 'My apps' → create app)"
        } else {
            "https://data.uspto.gov/apis/getting-started (free API key)"
        },
        "verify": "flowleap keys test",
    }))
}

/// Human-readable info box for a provider-keys hint, printed to stderr so it
/// never corrupts parseable stdout.
pub fn print_keys_hint_box(hint: &Value) {
    use colored::Colorize;
    let provider = hint["provider"].as_str().unwrap_or("provider");
    let invalid = hint["code"].as_str() == Some("provider_keys_invalid");
    let title = if invalid {
        format!("{} keys were rejected", provider.to_uppercase())
    } else {
        format!("{} keys required", provider.to_uppercase())
    };
    let signup = hint["signup"].as_str().unwrap_or("");
    let command = hint["nonInteractive"]["command"].as_str().unwrap_or("");

    eprintln!();
    eprintln!(
        "┌─ {} {}",
        title.yellow().bold(),
        "─".repeat(50_usize.saturating_sub(title.len()))
    );
    if invalid {
        eprintln!(
            "│ The backend rejected the configured {} credentials.",
            provider.to_uppercase()
        );
    } else {
        eprintln!(
            "│ This command needs {} credentials and none are configured",
            provider.to_uppercase()
        );
        eprintln!("│ (neither on this machine nor on the server).");
    }
    eprintln!("│");
    eprintln!("│ Fix it (requires a human — keys come from a browser signup):");
    eprintln!("│   {}   guided setup", "flowleap setup".cyan().bold());
    eprintln!("│   {}", command.cyan());
    eprintln!("│");
    eprintln!("│ Get keys: {}", signup);
    eprintln!("│ Verify:   {}", "flowleap keys test".cyan());
    eprintln!("└{}", "─".repeat(64));
}

pub fn encode_url_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }
    encoded
}

pub struct Context {
    pub config: Config,
    pub credentials: Credentials,
    pub output_format: String,
    pub dry_run: bool,
    pub verbose: bool,
    /// True when the session token came from --token / FLOWLEAP_TOKEN rather
    /// than the credential store. An explicit token expresses intent, so the
    /// 401 → api_key fallback is disabled.
    pub token_overridden: bool,
    pub http: Client,
}

impl Context {
    fn client(&self) -> &Client {
        &self.http
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url.trim_end_matches('/'), path)
    }

    fn apply_auth(&self, req: RequestBuilder) -> RequestBuilder {
        // The backend accepts exactly one credential shape: `Authorization:
        // Bearer <token>` — a Clerk JWT or a personal API token (fl_pat_…).
        // There is no X-API-Key path server-side.
        let req = if let Some(ref token) = self.credentials.token {
            req.header("Authorization", format!("Bearer {}", token))
        } else if let Some(ref key) = self.credentials.api_key {
            req.header("Authorization", format!("Bearer {}", key))
        } else {
            req
        };

        // BYOK provider keys, forwarded per-request. The EPO pair only travels
        // complete — the backend 400s on half a pair.
        let req = if let Some((key, secret)) = self.credentials.epo_pair() {
            req.header("x-epo-ops-key", key)
                .header("x-epo-ops-secret", secret)
        } else {
            req
        };
        if let Some(ref uspto) = self.credentials.uspto_key {
            req.header("x-uspto-odp-key", uspto)
        } else {
            req
        }
    }

    /// Build a GET request with auth
    pub fn get(&self, path: &str) -> RequestBuilder {
        let req = self.client().get(self.url(path));
        self.apply_auth(req)
    }

    /// Build a POST request with auth and JSON body
    pub fn post(&self, path: &str, body: &Value) -> RequestBuilder {
        let req = self.client().post(self.url(path)).json(body);
        self.apply_auth(req)
    }

    /// Build an arbitrary request with optional JSON body.
    pub fn request(&self, method: Method, path: &str, body: Option<&Value>) -> RequestBuilder {
        let req = self.client().request(method, self.url(path));
        let req = if let Some(body) = body {
            req.json(body)
        } else {
            req
        };
        self.apply_auth(req)
    }

    /// The API key to retry with when the stored session token is rejected
    /// with a 401. None when there is nothing to fall back to: no session
    /// token in play, no stored api_key, or the token was passed explicitly
    /// (--token / FLOWLEAP_TOKEN).
    pub fn auth_fallback_key(&self) -> Option<&str> {
        if self.token_overridden || self.credentials.token.is_none() {
            return None;
        }
        self.credentials.api_key.as_deref()
    }

    /// Send a request; on 401 with a stored session token, retry once with
    /// the stored API key. A Clerk session token expires quickly and would
    /// otherwise shadow a still-valid fl_pat_ key (see auth_header precedence).
    async fn send_with_auth_fallback(&self, req: Request) -> Result<Response> {
        let retry = match self.auth_fallback_key() {
            Some(key) if req.headers().contains_key(reqwest::header::AUTHORIZATION) => {
                req.try_clone().and_then(|mut r| {
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", key))
                        .ok()
                        .map(|v| {
                            r.headers_mut().insert(reqwest::header::AUTHORIZATION, v);
                            r
                        })
                })
            }
            _ => None,
        };

        let resp = self.client().execute(req).await?;
        if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
            return Ok(resp);
        }
        let Some(retry) = retry else { return Ok(resp) };

        if self.verbose {
            eprintln!("  401 with session token — retrying with stored API key");
        }
        let retry_resp = self.client().execute(retry).await?;
        if retry_resp.status().is_success() {
            eprintln!(
                "warning: the stored session token was rejected (401); the request succeeded with the stored API key. \
                 Clear the stale session with 'flowleap auth logout --session-only'."
            );
        }
        Ok(retry_resp)
    }

    /// Execute a request, handling dry-run and verbose modes
    pub async fn execute(&self, req: RequestBuilder) -> Result<Response> {
        let req = req.build()?;

        if self.verbose {
            eprintln!("{} {}", req.method(), req.url());
            for (key, val) in req.headers() {
                if !is_secret_header(key) {
                    eprintln!("  {}: {}", key, val.to_str().unwrap_or("(binary)"));
                }
            }
        }

        if self.dry_run {
            eprintln!("[dry-run] {} {}", req.method(), req.url());
            if let Some(body) = req.body() {
                if let Some(bytes) = body.as_bytes() {
                    if let Ok(json) = serde_json::from_slice::<Value>(bytes) {
                        eprintln!("[dry-run] Body: {}", serde_json::to_string_pretty(&json)?);
                    }
                }
            }
            bail!("Dry run — no request was sent");
        }

        let resp = self.send_with_auth_fallback(req).await?;

        if self.verbose {
            eprintln!("  Status: {}", resp.status());
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("API error ({}): {}", status, body);
        }

        Ok(resp)
    }

    /// Execute and parse JSON response
    pub async fn execute_json(&self, req: RequestBuilder) -> Result<Value> {
        let req = req.build()?;

        if self.verbose {
            self.print_request(&req);
        }

        if self.dry_run {
            return self.dry_run_response(&req);
        }

        let resp = self.send_with_auth_fallback(req).await?;

        if self.verbose {
            eprintln!("  Status: {}", resp.status());
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("API error ({}): {}", status, body);
        }

        let json: Value = resp.json().await?;
        Ok(json)
    }

    /// Execute and parse JSON body WITHOUT bailing on non-2xx status.
    /// Lets the caller inspect application-level error envelopes (e.g. ops
    /// endpoints return `{ success: false, error, code }` alongside a 4xx status).
    /// Still respects dry-run and verbose modes via execute_raw().
    pub async fn execute_json_allow_error(&self, req: RequestBuilder) -> Result<Value> {
        let req = req.build()?;

        if self.verbose {
            self.print_request(&req);
        }

        if self.dry_run {
            return self.dry_run_response(&req);
        }

        let resp = self.send_with_auth_fallback(req).await?;

        if self.verbose {
            eprintln!("  Status: {}", resp.status());
        }

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        // Try to parse as JSON; if not JSON and non-2xx, surface as generic error.
        match serde_json::from_str::<Value>(&body) {
            Ok(json) => Ok(json),
            Err(_) if !status.is_success() => bail!("API error ({}): {}", status, body),
            Err(e) => bail!("Failed to parse response: {}", e),
        }
    }

    /// Execute and return a stable response envelope, preserving non-2xx API bodies.
    pub async fn execute_json_envelope(&self, req: RequestBuilder) -> Result<Value> {
        let req = req.build()?;

        if self.verbose {
            self.print_request(&req);
        }

        if self.dry_run {
            return self.dry_run_response(&req);
        }

        let resp = self.send_with_auth_fallback(req).await?;
        let status = resp.status();
        let headers = resp.headers().clone();
        let text = resp.text().await.unwrap_or_default();
        let body = serde_json::from_str::<Value>(&text).unwrap_or_else(|_| json!(text));

        if self.verbose {
            eprintln!("  Status: {}", status);
        }

        let mut envelope = json!({
            "ok": status.is_success(),
            "status": status.as_u16(),
            "contentType": headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or(""),
            "body": body,
        });
        // Surface Retry-After so agents know how long to back off on 429.
        if let Some(retry_after) = headers.get("retry-after").and_then(|v| v.to_str().ok()) {
            envelope["retryAfterSeconds"] = retry_after
                .parse::<u64>()
                .map(|secs| json!(secs))
                .unwrap_or_else(|_| json!(retry_after));
        }
        // Provider-key problems get a structured hint so agents know this
        // needs human intervention rather than a retry.
        if !status.is_success() {
            if let Some(hint) = provider_keys_hint(status.as_u16(), &envelope["body"]) {
                envelope["providerKeysHint"] = hint;
            }
        }
        Ok(envelope)
    }

    pub async fn execute_json_body_or_error(&self, req: RequestBuilder) -> Result<Value> {
        let envelope = self.execute_json_envelope(req).await?;
        if envelope.get("dryRun").and_then(|v| v.as_bool()) == Some(true) {
            return Ok(envelope);
        }
        if envelope.get("ok").and_then(|v| v.as_bool()) == Some(true) {
            return Ok(envelope.get("body").cloned().unwrap_or(Value::Null));
        }
        self.print_error_envelope(&envelope);
        Err(PrintedError.into())
    }

    pub async fn execute_json_envelope_or_error(&self, req: RequestBuilder) -> Result<Value> {
        let envelope = self.execute_json_envelope(req).await?;
        if envelope.get("dryRun").and_then(|v| v.as_bool()) == Some(true)
            || envelope.get("ok").and_then(|v| v.as_bool()) == Some(true)
        {
            return Ok(envelope);
        }
        self.print_error_envelope(&envelope);
        Err(PrintedError.into())
    }

    /// Print an error envelope; in human/table formats also render the
    /// provider-keys info box (stderr) when the failure is key-related.
    fn print_error_envelope(&self, envelope: &Value) {
        crate::output::print_value(&self.output_format, envelope, &[]);
        if self.output_format != "json" {
            if let Some(hint) = envelope.get("providerKeysHint") {
                print_keys_hint_box(hint);
            }
        }
    }

    /// Require authentication, returning an error if not configured
    pub fn require_auth(&self) -> Result<()> {
        if self.dry_run {
            return Ok(());
        }
        if self.credentials.auth_header().is_none() {
            bail!(
                "Not authenticated. Run 'flowleap auth login' or set FLOWLEAP_API_KEY / FLOWLEAP_TOKEN."
            );
        }
        Ok(())
    }

    fn print_request(&self, req: &Request) {
        eprintln!("{} {}", req.method(), req.url());
        for (key, val) in req.headers() {
            if !is_secret_header(key) {
                eprintln!("  {}: {}", key, val.to_str().unwrap_or("(binary)"));
            }
        }
    }

    fn dry_run_response(&self, req: &Request) -> Result<Value> {
        let body = req
            .body()
            .and_then(|body| body.as_bytes())
            .and_then(|bytes| serde_json::from_slice::<Value>(bytes).ok());

        let dry_run = json!({
            "dryRun": true,
            "method": req.method().as_str(),
            "url": req.url().as_str(),
            "authenticated": req.headers().contains_key("authorization") || req.headers().contains_key("x-api-key"),
            // Presence only — the key material itself never appears in output.
            "providerKeys": {
                "epo": req.headers().contains_key("x-epo-ops-key"),
                "uspto": req.headers().contains_key("x-uspto-odp-key"),
            },
            "body": body,
        });

        Ok(dry_run)
    }
}
