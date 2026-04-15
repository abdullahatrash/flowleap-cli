use anyhow::{bail, Result};
use reqwest::{Client, RequestBuilder, Response};
use serde_json::Value;

use crate::config::{Config, Credentials};

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
        if let Some(auth) = self.credentials.auth_header() {
            req.header("Authorization", auth)
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

    /// Build a POST request with auth and multipart form
    pub fn post_multipart(&self, path: &str, form: reqwest::multipart::Form) -> RequestBuilder {
        let req = self.client().post(self.url(path)).multipart(form);
        self.apply_auth(req)
    }

    /// Execute a request, handling dry-run and verbose modes
    pub async fn execute(&self, req: RequestBuilder) -> Result<Response> {
        let req = req.build()?;

        if self.verbose {
            eprintln!("{} {}", req.method(), req.url());
            for (key, val) in req.headers() {
                if key != "authorization" {
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
        let resp = self.execute(req).await?;
        let json: Value = resp.json().await?;
        Ok(json)
    }

    /// Require authentication, returning an error if not configured
    pub fn require_auth(&self) -> Result<()> {
        if self.credentials.auth_header().is_none() {
            bail!(
                "Not authenticated. Run 'flowleap auth login' or set FLOWLEAP_API_KEY / FLOWLEAP_TOKEN."
            );
        }
        Ok(())
    }
}
