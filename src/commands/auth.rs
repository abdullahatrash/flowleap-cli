use crate::client::Context;
use crate::config::Credentials;
use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct DeviceAuthResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: String,
    expires_in: u64,
    interval: u64,
}

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

    // OAuth 2.0 Device Authorization flow
    println!("Starting device authorization flow...");

    let base_url = ctx.config.base_url.trim_end_matches('/');

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/oauth/device", base_url))
        .json(&serde_json::json!({"client_id": "flowleap-cli"}))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        bail!("Device authorization request failed ({}): {}", status, body);
    }

    let response: DeviceAuthResponse = resp.json().await?;

    println!(
        "\n  {} {}\n  {} {}\n",
        "▸ Visit:".bold(),
        response.verification_uri.cyan(),
        "▸ Enter code:".bold(),
        response.user_code.bold().yellow(),
    );

    let _ = open::that(&response.verification_uri_complete);

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Waiting for authorization...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let mut interval = response.interval;
    let deadline = std::time::Instant::now() + Duration::from_secs(response.expires_in);

    loop {
        tokio::time::sleep(Duration::from_secs(interval)).await;

        if std::time::Instant::now() > deadline {
            spinner.finish_and_clear();
            bail!("Device authorization expired. Please try again.");
        }

        let poll_resp = client
            .post(format!("{}/oauth/device/token", base_url))
            .json(&serde_json::json!({
                "device_code": response.device_code,
                "client_id": "flowleap-cli",
                "grant_type": "urn:ietf:params:oauth:grant-type:device_code"
            }))
            .send()
            .await?;

        let body: serde_json::Value = poll_resp.json().await?;

        if let Some(access_token) = body.get("access_token").and_then(|v| v.as_str()) {
            spinner.finish_and_clear();
            // Store token
            let mut creds = Credentials::load()?;
            creds.token = Some(access_token.to_string());
            creds.save()?;
            println!("{} Successfully authenticated!", "✓".green());
            println!(
                "Credentials saved to {:?}",
                Credentials::credentials_path()?
            );
            return Ok(());
        }

        if let Some(error) = body.get("error").and_then(|v| v.as_str()) {
            match error {
                "authorization_pending" => continue,
                "slow_down" => {
                    interval += 5;
                    continue;
                }
                "expired_token" => {
                    spinner.finish_and_clear();
                    bail!("Device authorization expired. Please try again.");
                }
                "access_denied" => {
                    spinner.finish_and_clear();
                    bail!("Authorization was denied.");
                }
                other => {
                    spinner.finish_and_clear();
                    let desc = body
                        .get("error_description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    bail!("Authorization failed: {} — {}", other, desc);
                }
            }
        }
    }
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
