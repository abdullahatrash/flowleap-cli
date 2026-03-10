use crate::client::Context;
use crate::config::Credentials;
use anyhow::{bail, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use clap::{Parser, Subcommand};
use colored::Colorize;
use rand::Rng;
use sha2::{Digest, Sha256};

const CLIENT_ID: &str = "flowleap-cli";

#[derive(Parser)]
pub struct AuthArgs {
    #[command(subcommand)]
    command: AuthCommand,
}

#[derive(Subcommand)]
enum AuthCommand {
    /// Login via OAuth 2.0 (opens browser) or with API key/token
    Login {
        /// API key to store (skips OAuth flow)
        #[arg(long)]
        api_key: Option<String>,
        /// Bearer token to store (skips OAuth flow)
        #[arg(long)]
        token: Option<String>,
    },
    /// Clear stored credentials
    Logout,
    /// Show current authentication status
    Status,
}

pub async fn run(ctx: &Context, args: AuthArgs) -> Result<()> {
    match args.command {
        AuthCommand::Login { api_key, token } => login(ctx, api_key, token).await,
        AuthCommand::Logout => logout().await,
        AuthCommand::Status => status(ctx).await,
    }
}

/// Generate a cryptographically random string for PKCE verifier or CSRF state
fn generate_random_string() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen::<u8>()).collect();
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Derive the code challenge from the verifier using S256
fn code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(hash)
}

async fn login(ctx: &Context, api_key: Option<String>, token: Option<String>) -> Result<()> {
    // If credentials passed directly, store them
    if api_key.is_some() || token.is_some() {
        let mut creds = Credentials::load()?;

        if let Some(key) = api_key {
            creds.api_key = Some(key);
            println!("{} API key stored.", "✓".green());
        }

        if let Some(tok) = token {
            creds.token = Some(tok);
            println!("{} Token stored.", "✓".green());
        }

        creds.save()?;
        println!(
            "Credentials saved to {:?}",
            Credentials::credentials_path()?
        );
        return Ok(());
    }

    // OAuth 2.0 + PKCE flow (Clerk-based)
    println!("Starting OAuth login flow...");

    let code_verifier = generate_random_string();
    let challenge = code_challenge(&code_verifier);
    let state = generate_random_string();

    // Start local callback server on a random port
    let server = tiny_http::Server::http("127.0.0.1:0")
        .map_err(|e| anyhow::anyhow!("Failed to start callback server: {}", e))?;
    let port = server.server_addr().to_ip().unwrap().port();
    let redirect_uri = format!("http://127.0.0.1:{}/callback", port);

    let base_url = ctx.config.base_url.trim_end_matches('/');
    let auth_url = format!(
        "{}/oauth/authorize?client_id={}&redirect_uri={}&state={}&response_type=code&code_challenge={}&code_challenge_method=S256",
        base_url,
        CLIENT_ID,
        urlencoding(&redirect_uri),
        state,
        challenge
    );

    println!("Opening browser for authentication...");
    println!("If the browser doesn't open, visit:\n  {}\n", auth_url);

    if open::that(&auth_url).is_err() {
        eprintln!("Could not open browser automatically. Please visit the URL above.");
    }

    println!("Waiting for authorization callback on port {}...", port);

    // Wait for the callback
    let callback = wait_for_callback(server, &state)?;

    match callback {
        CallbackResult::Code(auth_code) => {
            println!("Authorization code received. Exchanging for token...");

            // Exchange auth code for token
            let token_url = format!("{}/oauth/token", base_url);
            let client = reqwest::Client::new();
            let resp = client
                .post(&token_url)
                .json(&serde_json::json!({
                    "grant_type": "authorization_code",
                    "client_id": CLIENT_ID,
                    "code": auth_code,
                    "redirect_uri": redirect_uri,
                    "code_verifier": code_verifier,
                }))
                .send()
                .await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                bail!("Token exchange failed ({}): {}", status, body);
            }

            let token_resp: serde_json::Value = resp.json().await?;

            let mut creds = Credentials::load()?;

            if let Some(access_token) = token_resp.get("access_token").and_then(|v| v.as_str()) {
                creds.token = Some(access_token.to_string());
            }
            if let Some(refresh) = token_resp.get("refresh_token").and_then(|v| v.as_str()) {
                creds.refresh_token = Some(refresh.to_string());
            }

            creds.save()?;
        }
        CallbackResult::Token(token_value) => {
            // Implicit flow — token returned directly (Clerk session token)
            let mut creds = Credentials::load()?;
            creds.token = Some(token_value);
            creds.save()?;
        }
    }

    println!("{} Successfully authenticated!", "✓".green());
    println!(
        "Credentials saved to {:?}",
        Credentials::credentials_path()?
    );

    Ok(())
}

enum CallbackResult {
    Code(String),
    Token(String),
}

/// Parse query string parameters from a URL path
fn parse_query_params(url: &str) -> Vec<(String, String)> {
    let query = url
        .split('?')
        .nth(1)
        .or_else(|| url.split('#').nth(1))
        .unwrap_or("");

    query
        .split('&')
        .filter_map(|param| {
            let (key, value) = param.split_once('=')?;
            Some((key.to_string(), value.to_string()))
        })
        .collect()
}

/// Wait for the OAuth callback and extract the authorization code or token
fn wait_for_callback(server: tiny_http::Server, expected_state: &str) -> Result<CallbackResult> {
    let request = server
        .recv_timeout(std::time::Duration::from_secs(120))
        .map_err(|e| anyhow::anyhow!("Callback server error: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("Timed out waiting for authorization callback"))?;

    let url = request.url().to_string();
    let params = parse_query_params(&url);

    // Check for error
    if let Some((_, error)) = params.iter().find(|(k, _)| k == "error") {
        let desc = params
            .iter()
            .find(|(k, _)| k == "error_description")
            .map(|(_, v)| v.as_str())
            .unwrap_or("Unknown error");

        let response = tiny_http::Response::from_string(
            "<html><body><h1>Authentication Failed</h1><p>You can close this window.</p></body></html>",
        )
        .with_header("Content-Type: text/html".parse::<tiny_http::Header>().unwrap());
        let _ = request.respond(response);
        bail!("OAuth error: {} — {}", error, desc);
    }

    // Validate state parameter (CSRF protection)
    if let Some((_, state)) = params.iter().find(|(k, _)| k == "state") {
        if state != expected_state {
            let response = tiny_http::Response::from_string(
                "<html><body><h1>Authentication Failed</h1><p>Invalid state parameter.</p></body></html>",
            )
            .with_header("Content-Type: text/html".parse::<tiny_http::Header>().unwrap());
            let _ = request.respond(response);
            bail!("OAuth state mismatch — possible CSRF attack");
        }
    }

    // Send success response to browser
    let response = tiny_http::Response::from_string(
        "<html><body><h1>Authentication Successful!</h1><p>You can close this window and return to the terminal.</p></body></html>",
    )
    .with_header("Content-Type: text/html".parse::<tiny_http::Header>().unwrap());
    let _ = request.respond(response);

    // Check for token (implicit flow) or code (authorization code flow)
    if let Some((_, token)) = params.iter().find(|(k, _)| k == "access_token") {
        return Ok(CallbackResult::Token(token.clone()));
    }

    if let Some((_, code)) = params.iter().find(|(k, _)| k == "code") {
        return Ok(CallbackResult::Code(code.clone()));
    }

    bail!("No authorization code or token in callback URL")
}

/// Simple URL encoding for query parameter values
fn urlencoding(s: &str) -> String {
    s.replace(':', "%3A").replace('/', "%2F")
}

async fn logout() -> Result<()> {
    let mut creds = Credentials::load()?;
    creds.clear();
    creds.save()?;
    println!("{} Credentials cleared.", "✓".green());
    Ok(())
}

async fn status(ctx: &Context) -> Result<()> {
    println!("Base URL:  {}", ctx.config.base_url);

    if ctx.credentials.token.is_some() {
        println!("Auth:      {} (token)", "Authenticated".green());
    } else if ctx.credentials.api_key.is_some() {
        println!("Auth:      {} (API key)", "Authenticated".green());
    } else {
        println!("Auth:      {}", "Not authenticated".red());
        println!("\nRun 'flowleap auth login' to authenticate via OAuth,");
        println!("or 'flowleap auth login --api-key <key>' to store an API key.");
    }

    if let Some(ref model) = ctx.config.default_model {
        println!("Model:     {}", model);
    }

    // Try to fetch profile if authenticated
    if ctx.credentials.auth_header().is_some() {
        let req = ctx.get("/api/profile");
        match ctx.execute_json(req).await {
            Ok(profile) => {
                if let Some(email) = profile.get("email").and_then(|e| e.as_str()) {
                    println!("Email:     {}", email);
                }
                if let Some(name) = profile.get("name").and_then(|n| n.as_str()) {
                    println!("Name:      {}", name);
                }
            }
            Err(_) => {
                // Profile fetch is best-effort
            }
        }
    }

    Ok(())
}
