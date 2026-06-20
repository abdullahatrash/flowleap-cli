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
        if let Some(ref token) = self.credentials.token {
            req.header("Authorization", format!("Bearer {}", token))
        } else if let Some(ref key) = self.credentials.api_key {
            if key.starts_with("fl_org_") {
                req.header("X-API-Key", key)
            } else {
                req.header("Authorization", format!("Bearer {}", key))
            }
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

    /// Execute a request, handling dry-run and verbose modes
    pub async fn execute(&self, req: RequestBuilder) -> Result<Response> {
        let req = req.build()?;

        if self.verbose {
            eprintln!("{} {}", req.method(), req.url());
            for (key, val) in req.headers() {
                if key != "authorization" && key != "x-api-key" {
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

        let resp = self.client().execute(req).await?;

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

        let resp = self.client().execute(req).await?;

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

        let resp = self.client().execute(req).await?;

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

        let resp = self.client().execute(req).await?;
        let status = resp.status();
        let headers = resp.headers().clone();
        let text = resp.text().await.unwrap_or_default();
        let body = serde_json::from_str::<Value>(&text).unwrap_or_else(|_| json!(text));

        if self.verbose {
            eprintln!("  Status: {}", status);
        }

        Ok(json!({
            "ok": status.is_success(),
            "status": status.as_u16(),
            "contentType": headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or(""),
            "body": body,
        }))
    }

    pub async fn execute_json_body_or_error(&self, req: RequestBuilder) -> Result<Value> {
        let envelope = self.execute_json_envelope(req).await?;
        if envelope.get("dryRun").and_then(|v| v.as_bool()) == Some(true) {
            return Ok(envelope);
        }
        if envelope.get("ok").and_then(|v| v.as_bool()) == Some(true) {
            return Ok(envelope.get("body").cloned().unwrap_or(Value::Null));
        }
        crate::output::print_value(&self.output_format, &envelope, &[]);
        Err(PrintedError.into())
    }

    pub async fn execute_json_envelope_or_error(&self, req: RequestBuilder) -> Result<Value> {
        let envelope = self.execute_json_envelope(req).await?;
        if envelope.get("dryRun").and_then(|v| v.as_bool()) == Some(true)
            || envelope.get("ok").and_then(|v| v.as_bool()) == Some(true)
        {
            return Ok(envelope);
        }
        crate::output::print_value(&self.output_format, &envelope, &[]);
        Err(PrintedError.into())
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
            if key != "authorization" && key != "x-api-key" {
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
            "body": body,
        });

        Ok(dry_run)
    }
}
