use anyhow::Result;
use clap::{error::ErrorKind, Parser, Subcommand};
use flowleap_cli::commands::{
    academic, api, auth, citation, config_cmd, doctor, facade, health, keys, legal, npl, ops,
    patent, skills, tools, uspto,
};
use flowleap_cli::{client, config, update};
use serde_json::json;

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

    /// Emit stable JSON output
    #[arg(long, global = true)]
    json: bool,

    /// Output format
    #[arg(long, value_parser = ["json", "table", "human"], global = true)]
    output: Option<String>,

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
    /// Check CLI config, auth, and backend reachability
    Doctor,
    /// Interactive onboarding: backend check, auth, provider keys (human-only)
    Setup,
    /// Manage patent-provider keys (EPO OPS, USPTO ODP)
    Keys(keys::KeysArgs),
    /// Store initial CLI configuration
    Init {
        /// API base URL to store
        #[arg(long, default_value = "https://api.flowleap.co")]
        base_url: String,
    },
    /// Authenticate with FlowLeap API
    Auth(auth::AuthArgs),
    /// Raw and user API helpers
    Api(api::ApiArgs),
    /// Public backend health probes
    Health(health::HealthArgs),
    /// Search and analyze patents
    Patent(patent::PatentArgs),
    /// Direct EPO OPS API commands
    Ops(ops::OpsArgs),
    /// USPTO Open Data Portal commands
    Uspto(uspto::UsptoArgs),
    /// Search academic literature
    Academic(academic::AcademicArgs),
    /// Search non-patent literature
    Npl(npl::NplArgs),
    /// Search patent-law documents
    Legal(legal::LegalArgs),
    /// Search USPTO citation/prior-art data
    Citation(citation::CitationArgs),
    /// Compare 2-10 patents side by side (bibliography)
    Compare(facade::CompareArgs),
    /// List a patent's drawings/figures; save image data with --out
    Figures(facade::FiguresArgs),
    /// One-call patent snapshot: bibliography, legal status, family, term
    Summary(facade::SummaryArgs),
    /// Chronological prosecution timeline for a patent
    Timeline(facade::TimelineArgs),
    /// Convert a patent number between formats (epodoc, docdb, original)
    ConvertNumber(facade::ConvertNumberArgs),
    /// Discover and run backend tools (agent-first /v1/tools facade)
    Tools(tools::ToolsArgs),
    /// Install FlowLeap agent skills into an agent's skills directory
    Skills(skills::SkillsArgs),
    /// Manage CLI configuration
    Config(config_cmd::ConfigArgs),
}

#[tokio::main]
async fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            if args_want_json()
                && !matches!(
                    err.kind(),
                    ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
                )
            {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "ok": false,
                        "error": {
                            "message": err.to_string(),
                            "kind": format!("{:?}", err.kind()),
                        }
                    }))
                    .unwrap_or_default()
                );
                std::process::exit(err.exit_code());
            }
            err.exit();
        }
    };
    let wants_json = cli.json || cli.output.as_deref() == Some("json");

    if let Err(err) = run(cli).await {
        if err
            .downcast_ref::<flowleap_cli::client::PrintedError>()
            .is_some()
        {
            std::process::exit(1);
        }
        if wants_json {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": false,
                    "error": {
                        "message": err.to_string(),
                    }
                }))
                .unwrap_or_default()
            );
        } else {
            eprintln!("Error: {err}");
        }
        std::process::exit(1);
    }
}

fn args_want_json() -> bool {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--json" {
            return true;
        }
        if arg == "--output=json" {
            return true;
        }
        if arg == "--output" && args.next().as_deref() == Some("json") {
            return true;
        }
    }
    false
}

async fn run(cli: Cli) -> Result<()> {
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

    // fl_org_ keys were a v0.1.x concept with no working backend path; they now
    // travel as Bearer and always 401. Warn instead of failing mysteriously.
    if creds
        .api_key
        .as_deref()
        .is_some_and(|k| k.starts_with("fl_org_"))
    {
        eprintln!(
            "warning: fl_org_ organization keys are not supported (the backend never accepted them). \
             Mint a personal token instead: flowleap auth login && flowleap auth create-token --name <name> --store"
        );
    }

    // Provider-key env overrides (headless/agent path; humans use `flowleap setup`).
    if let Ok(value) = std::env::var("FLOWLEAP_EPO_KEY") {
        creds.epo_key = Some(value);
    }
    if let Ok(value) = std::env::var("FLOWLEAP_EPO_SECRET") {
        creds.epo_secret = Some(value);
    }
    if let Ok(value) = std::env::var("FLOWLEAP_USPTO_KEY") {
        creds.uspto_key = Some(value);
    }

    let output_format = if cli.json {
        "json".to_string()
    } else {
        cli.output
            .clone()
            .or_else(|| cfg.output_format.clone())
            .unwrap_or_else(|| "human".to_string())
    };

    let ctx = client::Context {
        config: cfg,
        credentials: creds,
        output_format,
        dry_run: cli.dry_run,
        verbose: cli.verbose,
        token_overridden: cli.token.is_some(),
        http: reqwest::Client::new(),
    };

    // First-run auto-onboarding: an authenticated command with no credentials,
    // in an interactive terminal, offers setup instead of just erroring. Skips
    // for --json/--dry-run and non-TTY (agents get the structured error), and
    // for the commands that manage their own auth or don't need it.
    let wants_first_run = command_needs_auth(&cli.command)
        && ctx.credentials.auth_header().is_none()
        && !cli.dry_run
        && !cli.json
        && std::io::IsTerminal::is_terminal(&std::io::stdin())
        && std::io::IsTerminal::is_terminal(&std::io::stderr());
    // Once-a-day update notice. Spawned before the command so the registry
    // fetch overlaps its work; printed to stderr after it finishes.
    let update_check = update::spawn_check(&ctx.http, cli.json, cli.dry_run);

    let result = if wants_first_run && offer_first_run_setup(&ctx).await? {
        // Reload credentials the wizard just wrote and continue the command.
        let creds = config::Credentials::load()?;
        let ctx = client::Context {
            credentials: creds,
            ..ctx
        };
        dispatch(cli.command, &ctx).await
    } else {
        dispatch(cli.command, &ctx).await
    };

    if let Some(handle) = update_check {
        // Grace period sized just above the check's own fetch timeout so the
        // task always resolves (answer or timeout) instead of being cancelled
        // mid-flight. Only the one fetch run per day can wait here at all;
        // cached runs resolve instantly.
        if let Ok(Ok(Some(notice))) = tokio::time::timeout(update::CHECK_GRACE, handle).await {
            eprintln!("{}", notice);
        }
    }

    result
}

/// Commands that hit an authenticated endpoint (everything except local/auth-
/// managing ones). Used to decide whether to offer first-run setup.
fn command_needs_auth(command: &Commands) -> bool {
    !matches!(
        command,
        Commands::Doctor
            | Commands::Setup
            | Commands::Init { .. }
            | Commands::Auth(_)
            | Commands::Health(_)
            | Commands::Skills(_)
            | Commands::Config(_)
    )
}

/// Prompt to run the setup wizard on first use. Returns true if setup ran and
/// the caller should retry the original command.
async fn offer_first_run_setup(ctx: &client::Context) -> Result<bool> {
    use dialoguer::{theme::ColorfulTheme, Confirm};
    eprintln!("You're not set up yet — FlowLeap needs a quick one-time sign-in.");
    let run = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Run setup now?")
        .default(true)
        .interact()
        .unwrap_or(false);
    if !run {
        return Ok(false);
    }
    keys::setup_wizard(ctx).await?;
    eprintln!();
    Ok(true)
}

async fn dispatch(command: Commands, ctx: &client::Context) -> Result<()> {
    match command {
        Commands::Doctor => doctor::run(ctx).await,
        Commands::Setup => keys::setup_wizard(ctx).await,
        Commands::Keys(args) => keys::run(ctx, args).await,
        Commands::Init { base_url } => init(ctx, &base_url).await,
        Commands::Auth(args) => auth::run(ctx, args).await,
        Commands::Api(args) => api::run(ctx, args).await,
        Commands::Health(args) => health::run(ctx, args).await,
        Commands::Patent(args) => patent::run(ctx, args).await,
        Commands::Ops(args) => ops::run(ctx, args).await,
        Commands::Uspto(args) => uspto::run(ctx, args).await,
        Commands::Academic(args) => academic::run(ctx, args).await,
        Commands::Npl(args) => npl::run(ctx, args).await,
        Commands::Legal(args) => legal::run(ctx, args).await,
        Commands::Citation(args) => citation::run(ctx, args).await,
        Commands::Compare(args) => facade::compare(ctx, args).await,
        Commands::Figures(args) => facade::figures(ctx, args).await,
        Commands::Summary(args) => facade::summary(ctx, args).await,
        Commands::Timeline(args) => facade::timeline(ctx, args).await,
        Commands::ConvertNumber(args) => facade::convert_number(ctx, args).await,
        Commands::Tools(args) => tools::run(ctx, args).await,
        Commands::Skills(args) => skills::run(ctx, args),
        Commands::Config(args) => config_cmd::run(ctx, args).await,
    }
}

async fn init(ctx: &client::Context, base_url: &str) -> Result<()> {
    let parsed =
        reqwest::Url::parse(base_url).map_err(|e| anyhow::anyhow!("Invalid URL: {}", e))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        anyhow::bail!("base-url must use http or https");
    }
    if parsed.host_str().is_none() {
        anyhow::bail!("base-url must include a host");
    }

    let mut cfg = config::Config::load()?;
    cfg.base_url = base_url.to_string();
    cfg.save()?;
    let value = json!({
        "ok": true,
        "baseUrl": base_url,
        "configPath": config::Config::config_path()?,
    });
    if ctx.output_format == "json" {
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        println!("Configured FlowLeap base URL: {}", base_url);
    }
    Ok(())
}
