use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

use crate::client::Context;
use crate::config::{Config, Credentials};

#[derive(Parser)]
pub struct ConfigArgs {
    #[command(subcommand)]
    command: ConfigCommand,
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Set a config value
    Set {
        /// Config key (base-url, default-model, output-format)
        key: String,
        /// Config value
        value: String,
    },
    /// Get a config value
    Get {
        /// Config key
        key: String,
    },
    /// List all config values
    List,
    /// Reset config to defaults
    Reset,
}

pub async fn run(_ctx: &Context, args: ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCommand::Set { key, value } => set(&key, &value).await,
        ConfigCommand::Get { key } => get(&key).await,
        ConfigCommand::List => list().await,
        ConfigCommand::Reset => reset().await,
    }
}

async fn set(key: &str, value: &str) -> Result<()> {
    let mut config = Config::load()?;

    match key {
        "base-url" => {
            // Validate early so typos (missing scheme, htp://, etc.) surface
            // here instead of as a cryptic reqwest error on first API call.
            let parsed = reqwest::Url::parse(value)
                .map_err(|e| anyhow::anyhow!("Invalid URL '{}': {}", value, e))?;
            if !matches!(parsed.scheme(), "http" | "https") {
                anyhow::bail!(
                    "base-url must use http or https (got scheme '{}')",
                    parsed.scheme()
                );
            }
            if parsed.host_str().is_none() {
                anyhow::bail!("base-url must include a host (got '{}')", value);
            }
            config.base_url = value.to_string();
        }
        "default-model" => config.default_model = Some(value.to_string()),
        "output-format" => {
            if !matches!(value, "json" | "table" | "human") {
                anyhow::bail!(
                    "output-format must be one of: json, table, human (got '{}')",
                    value
                );
            }
            config.output_format = Some(value.to_string());
        }
        _ => anyhow::bail!(
            "Unknown config key: '{}'. Valid keys: base-url, default-model, output-format",
            key
        ),
    }

    config.save()?;
    println!("{} Set {} = {}", "✓".green(), key, value);
    Ok(())
}

async fn get(key: &str) -> Result<()> {
    let config = Config::load()?;

    let value = match key {
        "base-url" => Some(config.base_url),
        "default-model" => config.default_model,
        "output-format" => config.output_format,
        _ => anyhow::bail!(
            "Unknown config key: '{}'. Valid keys: base-url, default-model, output-format",
            key
        ),
    };

    match value {
        Some(v) => println!("{}", v),
        None => println!("(not set)"),
    }
    Ok(())
}

async fn list() -> Result<()> {
    let config = Config::load()?;
    let creds = Credentials::load()?;

    println!("base-url       = {}", config.base_url);
    println!(
        "default-model  = {}",
        config.default_model.as_deref().unwrap_or("(not set)")
    );
    println!(
        "output-format  = {}",
        config.output_format.as_deref().unwrap_or("(not set)")
    );
    println!(
        "api-key        = {}",
        if creds.api_key.is_some() {
            "***configured***"
        } else {
            "(not set)"
        }
    );
    println!(
        "token          = {}",
        if creds.token.is_some() {
            "***configured***"
        } else {
            "(not set)"
        }
    );
    println!("\nConfig file:       {:?}", Config::config_path()?);
    println!("Credentials file:  {:?}", Credentials::credentials_path()?);
    Ok(())
}

async fn reset() -> Result<()> {
    let config = Config::default();
    config.save()?;
    println!("{} Config reset to defaults.", "✓".green());
    Ok(())
}
