use anyhow::Result;
use clap::{Parser, Subcommand};
use flowleap_cli::commands::{academic, auth, config_cmd, ops, patent};
use flowleap_cli::{client, config};

/// One CLI for FlowLeap Patent AI — built for humans and AI agents.
#[derive(Parser)]
#[command(name = "flowleap", version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// API base URL
    #[arg(long, env = "FLOWLEAP_BASE_URL", global = true)]
    base_url: Option<String>,

    /// API key (overrides stored credentials)
    #[arg(long, env = "FLOWLEAP_API_KEY", global = true)]
    api_key: Option<String>,

    /// Bearer token (overrides stored credentials)
    #[arg(long, env = "FLOWLEAP_TOKEN", global = true)]
    token: Option<String>,

    /// Output format
    #[arg(long, default_value = "human", value_parser = ["json", "table", "human"], global = true)]
    output: String,

    /// Show request details without executing
    #[arg(long, global = true)]
    dry_run: bool,

    /// Show verbose request/response details
    #[arg(long, short, global = true)]
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
    /// Search academic literature
    Academic(academic::AcademicArgs),
    /// Manage CLI configuration
    Config(config_cmd::ConfigArgs),
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
        http: reqwest::Client::new(),
    };

    match cli.command {
        Commands::Auth(args) => auth::run(&ctx, args).await,
        Commands::Patent(args) => patent::run(&ctx, args).await,
        Commands::Ops(args) => ops::run(&ctx, args).await,
        Commands::Academic(args) => academic::run(&ctx, args).await,
        Commands::Config(args) => config_cmd::run(&ctx, args).await,
    }
}
