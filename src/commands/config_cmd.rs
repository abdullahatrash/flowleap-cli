use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

use crate::client::Context;
use crate::config::Config;

#[derive(Parser)]
pub struct ConfigArgs {
    #[command(subcommand)]
    command: ConfigCommand,
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Set a config value
    Set {
        /// Config key (base-url, default-model)
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
        "base-url" => config.base_url = value.to_string(),
        "default-model" => config.default_model = Some(value.to_string()),
        _ => anyhow::bail!("Unknown config key: '{}'. Valid keys: base-url, default-model", key),
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
        _ => anyhow::bail!("Unknown config key: '{}'. Valid keys: base-url, default-model", key),
    };

    match value {
        Some(v) => println!("{}", v),
        None => println!("(not set)"),
    }
    Ok(())
}

async fn list() -> Result<()> {
    let config = Config::load()?;
    println!("base-url       = {}", config.base_url);
    println!(
        "default-model  = {}",
        config.default_model.as_deref().unwrap_or("(not set)")
    );
    println!(
        "api-key        = {}",
        if config.api_key.is_some() {
            "***configured***"
        } else {
            "(not set)"
        }
    );
    println!(
        "token          = {}",
        if config.token.is_some() {
            "***configured***"
        } else {
            "(not set)"
        }
    );
    println!("\nConfig file: {:?}", Config::config_path()?);
    Ok(())
}

async fn reset() -> Result<()> {
    let config = Config::default();
    config.save()?;
    println!("{} Config reset to defaults.", "✓".green());
    Ok(())
}
