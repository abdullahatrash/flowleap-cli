mod client;
mod commands;
mod config;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::{academic, auth, config_cmd, ocr, ops, patent};

/// One CLI for FlowLeap Patent AI — built for humans and AI agents.
#[derive(Parser)]
#[command(name = "flowleap", version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// API base URL
    #[arg(long, env = "FLOWLEAP_BASE_URL")]
    base_url: Option<String>,

    /// API key (overrides stored credentials)
    #[arg(long, env = "FLOWLEAP_API_KEY")]
    api_key: Option<String>,

    /// Bearer token (overrides stored credentials)
    #[arg(long, env = "FLOWLEAP_TOKEN")]
    token: Option<String>,

    /// Output format
    #[arg(long, default_value = "human", value_parser = ["json", "table", "human"])]
    output: String,

    /// Show request details without executing
    #[arg(long)]
    dry_run: bool,

    /// Show verbose request/response details
    #[arg(long, short)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with FlowLeap API
    Auth(auth::AuthArgs),
    /// Search and analyze patents
    Patent(patent::PatentArgs),
    /// Direct EPO OPS API commands
    Ops(ops::OpsArgs),
    /// OCR document processing
    Ocr(ocr::OcrArgs),
    /// Search academic literature
    Academic(academic::AcademicArgs),
    /// Manage CLI configuration
    Config(config_cmd::ConfigArgs),
    /// Discover available commands and API schema
    Schema(SchemaArgs),
}

#[derive(Parser)]
struct SchemaArgs {
    /// Service or service.resource.method to inspect
    path: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load config and credentials
    let mut cfg = config::Config::load()?;
    let mut creds = config::Credentials::load()?;

    // CLI flags > env vars > config file
    if let Some(ref url) = cli.base_url {
        cfg.base_url = url.clone();
    }
    if let Some(ref key) = cli.api_key {
        creds.api_key = Some(key.clone());
    }
    if let Some(ref tok) = cli.token {
        creds.token = Some(tok.clone());
    }

    let ctx = client::Context {
        config: cfg,
        credentials: creds,
        output_format: cli.output.clone(),
        dry_run: cli.dry_run,
        verbose: cli.verbose,
    };

    match cli.command {
        Commands::Auth(args) => auth::run(&ctx, args).await,
        Commands::Patent(args) => patent::run(&ctx, args).await,
        Commands::Ops(args) => ops::run(&ctx, args).await,
        Commands::Ocr(args) => ocr::run(&ctx, args).await,
        Commands::Academic(args) => academic::run(&ctx, args).await,
        Commands::Config(args) => config_cmd::run(&ctx, args).await,
        Commands::Schema(args) => run_schema(&ctx, args).await,
    }
}

async fn run_schema(_ctx: &client::Context, args: SchemaArgs) -> Result<()> {
    let services = vec![
        ("auth", "Authentication commands (login, logout, status)"),
        ("patent", "Patent search, query building, claim analysis"),
        (
            "ops",
            "Direct EPO OPS API (biblio, claims, family, legal, ...)",
        ),
        ("ocr", "Document OCR processing via Mistral"),
        ("academic", "Academic literature search"),
        ("config", "CLI configuration management"),
    ];

    match args.path {
        None => {
            println!("Available services:\n");
            for (name, desc) in &services {
                println!("  {:<12} {}", name, desc);
            }
            println!("\nUse 'flowleap schema <service>' for details.");
        }
        Some(path) => {
            println!(
                "Schema for '{}' — run 'flowleap {} --help' for full details.",
                path, path
            );
        }
    }
    Ok(())
}
