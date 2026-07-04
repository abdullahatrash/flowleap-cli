use anyhow::{bail, Result};
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password};
use serde_json::{json, Value};
use std::io::IsTerminal;

use crate::client::Context;
use crate::config::Credentials;
use crate::output;

pub const EPO_SIGNUP: &str = "https://developers.epo.org";
pub const USPTO_SIGNUP: &str = "https://data.uspto.gov/apis/getting-started";

#[derive(Parser)]
pub struct KeysArgs {
    #[command(subcommand)]
    command: KeysCommand,
}

#[derive(Clone, ValueEnum)]
pub enum Provider {
    Epo,
    Uspto,
}

#[derive(Subcommand)]
enum KeysCommand {
    /// Interactive wizard for all provider keys (human-only)
    Setup,
    /// Set keys for one provider (flags for non-interactive use)
    Set {
        provider: Provider,
        /// EPO consumer key or USPTO API key
        #[arg(long)]
        key: Option<String>,
        /// EPO consumer secret (EPO only)
        #[arg(long)]
        secret: Option<String>,
        /// Skip live validation against the backend
        #[arg(long)]
        no_verify: bool,
    },
    /// Show configured providers (secrets masked)
    List,
    /// Validate configured keys against the live providers
    Test,
    /// Remove stored keys for one provider
    Rm { provider: Provider },
}

pub async fn run(ctx: &Context, args: KeysArgs) -> Result<()> {
    match args.command {
        KeysCommand::Setup => setup_wizard(ctx).await,
        KeysCommand::Set {
            provider,
            key,
            secret,
            no_verify,
        } => set(ctx, provider, key, secret, no_verify).await,
        KeysCommand::List => list(ctx),
        KeysCommand::Test => test(ctx).await,
        KeysCommand::Rm { provider } => rm(ctx, provider),
    }
}

fn mask(value: &str) -> String {
    let visible: String = value.chars().take(4).collect();
    format!("{}…{}", visible, "•".repeat(4))
}

fn require_tty() -> Result<()> {
    if !std::io::stdin().is_terminal() {
        bail!(
            "Interactive setup needs a terminal (a human). Non-interactive alternatives:\n  \
             flowleap keys set epo --key <consumer-key> --secret <consumer-secret>\n  \
             flowleap keys set uspto --key <api-key>\n  \
             env: FLOWLEAP_EPO_KEY, FLOWLEAP_EPO_SECRET, FLOWLEAP_USPTO_KEY"
        );
    }
    Ok(())
}

/// Build a Context that authenticates like `ctx` but carries the candidate
/// provider keys, so validation exercises keys before anything is saved.
fn with_candidate_keys(ctx: &Context, creds: Credentials) -> Context {
    Context {
        config: ctx.config.clone(),
        credentials: creds,
        output_format: ctx.output_format.clone(),
        dry_run: false,
        verbose: ctx.verbose,
        http: ctx.http.clone(),
    }
}

/// POST /v1/keys/validate with the given credentials. Returns the per-provider
/// verdicts object, mapping the middleware's eager EPO rejection (400
/// patent_provider_key_invalid) into an epo-invalid verdict.
async fn validate(ctx: &Context, creds: Credentials) -> Result<Value> {
    let probe = with_candidate_keys(ctx, creds);
    let envelope = probe
        .execute_json_envelope(probe.post("/v1/keys/validate", &json!({})))
        .await?;

    if envelope["ok"].as_bool() == Some(true) {
        return Ok(envelope["body"]["providers"].clone());
    }
    let body_text = envelope["body"].to_string();
    if body_text.contains("patent_provider_key_invalid") {
        let message = envelope["body"]["error"]["message"]
            .as_str()
            .unwrap_or("Provider rejected the supplied keys.")
            .to_string();
        return Ok(json!({
            "epo": { "source": "user", "valid": false, "message": message },
            "uspto": { "source": "unknown", "valid": null, "message": "Not checked (EPO validation failed first)." },
        }));
    }
    bail!(
        "Key validation request failed (HTTP {}): {}",
        envelope["status"],
        body_text
    );
}

fn verdict_line(name: &str, verdict: &Value) -> String {
    let source = verdict["source"].as_str().unwrap_or("unknown");
    let message = verdict["message"].as_str().unwrap_or("");
    let symbol = match verdict["valid"] {
        Value::Bool(true) => "✓".green(),
        Value::Bool(false) => "✗".red(),
        _ => "•".yellow(),
    };
    format!("{} {:<6} [{}] {}", symbol, name, source, message)
}

// ---------------------------------------------------------------------------
// keys set
// ---------------------------------------------------------------------------

async fn set(
    ctx: &Context,
    provider: Provider,
    key: Option<String>,
    secret: Option<String>,
    no_verify: bool,
) -> Result<()> {
    let mut creds = Credentials::load()?;

    match provider {
        Provider::Epo => {
            let (key, secret) = match (key, secret) {
                (Some(k), Some(s)) => (k, s),
                (None, None) => {
                    require_tty()?;
                    prompt_epo_pair()?
                }
                _ => bail!("EPO needs both --key and --secret (they only work as a pair)."),
            };
            creds.epo_key = Some(key);
            creds.epo_secret = Some(secret);
        }
        Provider::Uspto => {
            let key = match key {
                Some(k) => k,
                None => {
                    require_tty()?;
                    prompt_uspto_key()?
                }
            };
            if secret.is_some() {
                bail!("USPTO takes only --key (no secret).");
            }
            creds.uspto_key = Some(key);
        }
    }

    if !no_verify {
        let verdicts = validate(ctx, creds.clone()).await?;
        let (name, verdict) = match provider {
            Provider::Epo => ("epo", &verdicts["epo"]),
            Provider::Uspto => ("uspto", &verdicts["uspto"]),
        };
        if verdict["valid"] == Value::Bool(false) {
            if ctx.output_format == "json" {
                output::print_json(&json!({ "ok": false, "provider": name, "verdict": verdict }));
            } else {
                eprintln!("{}", verdict_line(name, verdict));
                eprintln!("Keys NOT saved. Fix them and retry, or use --no-verify to save anyway.");
            }
            return Err(crate::client::PrintedError.into());
        }
    }

    creds.save()?;
    if ctx.output_format == "json" {
        output::print_json(&json!({ "ok": true, "saved": true, "verified": !no_verify }));
    } else {
        println!(
            "{} Keys saved to {:?} (0600){}",
            "✓".green(),
            Credentials::credentials_path()?,
            if no_verify { " — not verified" } else { "" }
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// keys list / test / rm
// ---------------------------------------------------------------------------

fn list(ctx: &Context) -> Result<()> {
    let creds = &ctx.credentials;
    if ctx.output_format == "json" {
        // Presence + masked previews only — never the key material.
        output::print_json(&json!({
            "epo": {
                "configured": creds.epo_pair().is_some(),
                "keyPreview": creds.epo_key.as_deref().map(mask),
            },
            "uspto": {
                "configured": creds.uspto_key.is_some(),
                "keyPreview": creds.uspto_key.as_deref().map(mask),
            },
            "note": "Sources include env overrides (FLOWLEAP_EPO_KEY/FLOWLEAP_EPO_SECRET/FLOWLEAP_USPTO_KEY).",
        }));
        return Ok(());
    }
    println!("Provider keys (this machine):");
    match creds.epo_pair() {
        Some((key, _)) => println!("  epo    {} {} / secret set", "✓".green(), mask(key)),
        None => {
            if creds.epo_key.is_some() || creds.epo_secret.is_some() {
                println!(
                    "  epo    {} incomplete pair — both key and secret are required",
                    "!".yellow()
                );
            } else {
                println!("  epo    {} not set   ({})", "✗".red(), EPO_SIGNUP);
            }
        }
    }
    match &creds.uspto_key {
        Some(key) => println!("  uspto  {} {}", "✓".green(), mask(key)),
        None => println!("  uspto  {} not set   ({})", "✗".red(), USPTO_SIGNUP),
    }
    println!(
        "\nVerify against live providers: {}",
        "flowleap keys test".cyan()
    );
    Ok(())
}

async fn test(ctx: &Context) -> Result<()> {
    ctx.require_auth()?;
    let verdicts = validate(ctx, ctx.credentials.clone()).await?;
    if ctx.output_format == "json" {
        output::print_json(&json!({ "ok": true, "providers": verdicts }));
    } else {
        println!("{}", verdict_line("epo", &verdicts["epo"]));
        println!("{}", verdict_line("uspto", &verdicts["uspto"]));
    }
    Ok(())
}

fn rm(ctx: &Context, provider: Provider) -> Result<()> {
    let mut creds = Credentials::load()?;
    let name = match provider {
        Provider::Epo => {
            creds.epo_key = None;
            creds.epo_secret = None;
            "epo"
        }
        Provider::Uspto => {
            creds.uspto_key = None;
            "uspto"
        }
    };
    creds.save()?;
    if ctx.output_format == "json" {
        output::print_json(&json!({ "ok": true, "removed": name }));
    } else {
        println!("{} Removed {} keys.", "✓".green(), name);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Wizard (flowleap setup / flowleap keys setup)
// ---------------------------------------------------------------------------

fn prompt_epo_pair() -> Result<(String, String)> {
    let theme = ColorfulTheme::default();
    let key: String = Input::with_theme(&theme)
        .with_prompt("EPO consumer key")
        .validate_with(|v: &String| {
            if v.trim().is_empty() || v.contains(char::is_whitespace) {
                Err("must be non-empty, without spaces")
            } else {
                Ok(())
            }
        })
        .interact_text()?;
    let secret = Password::with_theme(&theme)
        .with_prompt("EPO consumer secret (hidden)")
        .interact()?;
    if secret.trim().is_empty() {
        bail!("EPO consumer secret cannot be empty.");
    }
    Ok((key.trim().to_string(), secret.trim().to_string()))
}

fn prompt_uspto_key() -> Result<String> {
    let key = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("USPTO ODP API key (hidden)")
        .interact()?;
    if key.trim().is_empty() {
        bail!("USPTO API key cannot be empty.");
    }
    Ok(key.trim().to_string())
}

fn skip_warning(provider: &str, server_has_keys: bool, commands: &str) {
    if server_has_keys {
        println!(
            "  {} Skipped — the server has its own {} keys, so commands still work.",
            "•".yellow(),
            provider.to_uppercase()
        );
    } else {
        println!();
        println!(
            "  {} Skipped {} keys — {} will fail until they are added:",
            "!".yellow().bold(),
            provider.to_uppercase(),
            commands
        );
        println!(
            "    add later with {} or {}",
            format!("flowleap keys set {}", provider).cyan(),
            "flowleap setup".cyan()
        );
        println!();
    }
}

pub async fn setup_wizard(ctx: &Context) -> Result<()> {
    require_tty()?;
    let theme = ColorfulTheme::default();

    println!("{}", "FlowLeap CLI Setup".bold());
    println!("{}", "──────────────────".dimmed());

    // 1. Backend reachability
    let health = ctx
        .execute_json_envelope(ctx.request(reqwest::Method::GET, "/health", None))
        .await;
    match health {
        Ok(value) if value["ok"].as_bool() == Some(true) => {
            println!(
                "{} Backend reachable ({})",
                "✓".green(),
                ctx.config.base_url
            );
        }
        _ => {
            println!(
                "{} Backend not reachable at {} — check --base-url or start it, then re-run.",
                "✗".red(),
                ctx.config.base_url
            );
            bail!("Backend unreachable.");
        }
    }

    // 2. Auth
    if ctx.credentials.auth_header().is_none() {
        println!(
            "{} Not authenticated. Run {} first (or set FLOWLEAP_API_KEY), then re-run setup.",
            "✗".red(),
            "flowleap auth login".cyan()
        );
        bail!("Not authenticated.");
    }
    println!("{} Authenticated", "✓".green());

    // What does the server already have? Drives the skip warnings.
    let baseline = validate(ctx, ctx.credentials.clone())
        .await
        .unwrap_or(json!({}));
    let epo_on_server = baseline["epo"]["source"].as_str() == Some("server");
    let uspto_on_server = baseline["uspto"]["source"].as_str() == Some("server");

    println!();
    println!("{}", "Patent data providers — bring your own keys".bold());
    println!("Keys are stored only on this machine (~/.config/flowleap/credentials.toml, 0600)");
    println!("and forwarded per-request. Each step is skippable.");
    println!();

    let mut creds = Credentials::load()?;

    // 3. EPO
    let epo_prompt = match creds.epo_pair() {
        Some((key, _)) => format!("EPO OPS keys (currently {}) — replace?", mask(key)),
        None => "Add EPO OPS keys? (worldwide patent data)".to_string(),
    };
    println!("  Get free EPO keys: {} → My apps", EPO_SIGNUP.underline());
    if Confirm::with_theme(&theme)
        .with_prompt(epo_prompt)
        .default(creds.epo_pair().is_none())
        .interact()?
    {
        loop {
            let (key, secret) = prompt_epo_pair()?;
            let mut candidate = creds.clone();
            candidate.epo_key = Some(key);
            candidate.epo_secret = Some(secret);
            let spinner = indicatif::ProgressBar::new_spinner();
            spinner.set_message("Validating against EPO OPS…");
            spinner.enable_steady_tick(std::time::Duration::from_millis(90));
            let verdicts = validate(ctx, candidate.clone()).await?;
            spinner.finish_and_clear();
            println!("  {}", verdict_line("epo", &verdicts["epo"]));
            if verdicts["epo"]["valid"] == Value::Bool(false) {
                if Confirm::with_theme(&theme)
                    .with_prompt("EPO rejected those keys — try again?")
                    .default(true)
                    .interact()?
                {
                    continue;
                }
                skip_warning("epo", epo_on_server, "patent/ops commands");
                break;
            }
            creds = candidate;
            break;
        }
    } else {
        skip_warning(
            "epo",
            epo_on_server || creds.epo_pair().is_some(),
            "patent/ops commands",
        );
    }

    // 4. USPTO
    println!("  Get a free USPTO ODP key: {}", USPTO_SIGNUP.underline());
    let uspto_prompt = match &creds.uspto_key {
        Some(key) => format!("USPTO ODP key (currently {}) — replace?", mask(key)),
        None => "Add a USPTO ODP API key? (US prosecution data)".to_string(),
    };
    if Confirm::with_theme(&theme)
        .with_prompt(uspto_prompt)
        .default(creds.uspto_key.is_none())
        .interact()?
    {
        loop {
            let key = prompt_uspto_key()?;
            let mut candidate = creds.clone();
            candidate.uspto_key = Some(key);
            let spinner = indicatif::ProgressBar::new_spinner();
            spinner.set_message("Validating against USPTO ODP…");
            spinner.enable_steady_tick(std::time::Duration::from_millis(90));
            let verdicts = validate(ctx, candidate.clone()).await?;
            spinner.finish_and_clear();
            println!("  {}", verdict_line("uspto", &verdicts["uspto"]));
            if verdicts["uspto"]["valid"] == Value::Bool(false) {
                if Confirm::with_theme(&theme)
                    .with_prompt("USPTO rejected that key — try again?")
                    .default(true)
                    .interact()?
                {
                    continue;
                }
                skip_warning("uspto", uspto_on_server, "uspto/citation commands");
                break;
            }
            creds = candidate;
            break;
        }
    } else {
        skip_warning(
            "uspto",
            uspto_on_server || creds.uspto_key.is_some(),
            "uspto/citation commands",
        );
    }

    // 5. Save + summary
    creds.save()?;
    println!();
    println!(
        "{} Saved to {:?} (0600)",
        "✓".green(),
        Credentials::credentials_path()?
    );
    println!();
    println!("You're ready. Try:");
    println!(
        "  {}",
        r#"flowleap patent search --query 'ti="battery"' --limit 3"#.cyan()
    );
    println!("  {}", "flowleap keys test".cyan());
    Ok(())
}
