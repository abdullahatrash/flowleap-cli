use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

use crate::client::Context;
use crate::config::Config;

#[derive(Parser)]
pub struct AuthArgs {
    #[command(subcommand)]
    command: AuthCommand,
}

#[derive(Subcommand)]
enum AuthCommand {
    /// Login with an API key
    Login {
        /// API key to store
        #[arg(long)]
        api_key: Option<String>,
        /// Bearer token to store
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
        AuthCommand::Login { api_key, token } => login(api_key, token).await,
        AuthCommand::Logout => logout().await,
        AuthCommand::Status => status(ctx).await,
    }
}

async fn login(api_key: Option<String>, token: Option<String>) -> Result<()> {
    if api_key.is_none() && token.is_none() {
        eprintln!("Provide --api-key or --token to authenticate.");
        eprintln!("\nExamples:");
        eprintln!("  flowleap auth login --api-key sk-...");
        eprintln!("  flowleap auth login --token eyJ...");
        return Ok(());
    }

    let mut config = Config::load()?;

    if let Some(key) = api_key {
        config.api_key = Some(key);
        println!("{} API key stored.", "✓".green());
    }

    if let Some(tok) = token {
        config.token = Some(tok);
        println!("{} Token stored.", "✓".green());
    }

    config.save()?;
    println!("Credentials saved to {:?}", Config::config_path()?);
    Ok(())
}

async fn logout() -> Result<()> {
    let mut config = Config::load()?;
    config.api_key = None;
    config.token = None;
    config.save()?;
    println!("{} Credentials cleared.", "✓".green());
    Ok(())
}

async fn status(ctx: &Context) -> Result<()> {
    println!("Base URL:  {}", ctx.config.base_url);

    if ctx.config.token.is_some() {
        println!("Auth:      {} (token)", "Authenticated".green());
    } else if ctx.config.api_key.is_some() {
        println!("Auth:      {} (API key)", "Authenticated".green());
    } else {
        println!("Auth:      {}", "Not authenticated".red());
        println!("\nRun 'flowleap auth login --api-key <key>' to authenticate.");
    }

    if let Some(ref model) = ctx.config.default_model {
        println!("Model:     {}", model);
    }

    Ok(())
}
