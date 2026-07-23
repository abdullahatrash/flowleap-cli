use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::{json, Value};

use crate::client::Context;
use crate::output;

#[derive(Parser)]
#[command(after_help = "Examples:
  flowleap patstat portfolio Siemens
  flowleap patstat portfolio \"Kia Motors\" --from-year 2015 --to-year 2024

Note: an ambiguous applicant name (matching several distinct corporate
entities) is never merged or auto-picked — re-run with one exact candidate
name from the list the command prints.")]
pub struct PatstatArgs {
    #[command(subcommand)]
    command: PatstatCommand,
}

#[derive(Subcommand)]
enum PatstatCommand {
    /// Aggregate patent portfolio for one applicant: filings by year, office,
    /// and grant status. An ambiguous applicant name prints its candidates
    /// instead of guessing — re-run with one exact name from that list.
    Portfolio {
        /// Applicant name or harmonized PSN name prefix (e.g. "Siemens")
        applicant: String,

        /// Earliest filing year, inclusive (default: to-year - 9)
        #[arg(long)]
        from_year: Option<i32>,

        /// Latest filing year, inclusive (default: current year)
        #[arg(long)]
        to_year: Option<i32>,
    },
}

pub async fn run(ctx: &Context, args: PatstatArgs) -> Result<()> {
    ctx.require_auth()?;

    match args.command {
        PatstatCommand::Portfolio {
            applicant,
            from_year,
            to_year,
        } => portfolio(ctx, &applicant, from_year, to_year).await,
    }
}

async fn portfolio(
    ctx: &Context,
    applicant: &str,
    from_year: Option<i32>,
    to_year: Option<i32>,
) -> Result<()> {
    let mut body = json!({ "applicant": applicant });
    if let Some(from_year) = from_year {
        body["fromYear"] = json!(from_year);
    }
    if let Some(to_year) = to_year {
        body["toYear"] = json!(to_year);
    }

    let envelope = ctx
        .execute_json_envelope(ctx.post("/v1/patstat/portfolio", &body))
        .await?;
    if envelope.get("dryRun").and_then(Value::as_bool) == Some(true) {
        output::print_json(&envelope);
        return Ok(());
    }

    let http_ok = envelope.get("ok").and_then(Value::as_bool) == Some(true);
    let resp_body = envelope.get("body").cloned().unwrap_or(Value::Null);

    if !http_ok {
        return Err(render_error(ctx, &envelope, &resp_body));
    }

    if ctx.output_format == "json" {
        output::print_json(&resp_body);
        return Ok(());
    }

    print_portfolio(&resp_body);
    Ok(())
}

/// Render a failed portfolio call and return the [`PrintedError`] the
/// top-level handler maps to the documented exit code.
///
/// Two typed PATSTAT error codes get dedicated rendering in both output
/// modes (never a raw envelope dump): an ambiguous applicant prints its
/// candidate list as an interaction step (never auto-picked), and an
/// unconfigured deployment states plainly that PATSTAT is unavailable.
/// Anything else (auth, rate limit, generic upstream failure, …) falls back
/// to the shared envelope + hint-box rendering every other command uses.
///
/// [`PrintedError`]: crate::client::PrintedError
fn render_error(ctx: &Context, envelope: &Value, body: &Value) -> anyhow::Error {
    let code = body
        .pointer("/error/code")
        .and_then(Value::as_str)
        .unwrap_or("");

    match code {
        "patstat_applicant_ambiguous" => render_ambiguous(ctx, body),
        "patstat_unavailable" => render_unavailable(ctx, body),
        _ => render_generic_error(ctx, envelope),
    }

    match envelope.get("status").and_then(Value::as_u64) {
        Some(status) => crate::client::PrintedError::with_status(status as u16).into(),
        None => crate::client::PrintedError::new().into(),
    }
}

/// The candidate-list rendering the ambiguity flow needs in both output
/// modes: the backend never merges distinct applicant entities, so this
/// command never auto-picks one either — it prints the candidates and tells
/// the caller to re-run with one exact name.
fn render_ambiguous(ctx: &Context, body: &Value) {
    let message = body
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("The applicant name matches several distinct entities.");
    let candidates = body
        .pointer("/error/candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if ctx.output_format == "json" {
        output::print_json(&json!({
            "ok": false,
            "error": {
                "code": "patstat_applicant_ambiguous",
                "message": message,
                "candidates": candidates,
            },
        }));
        return;
    }

    println!("Ambiguous applicant — {message}");
    println!();
    println!("Candidates:");
    for candidate in &candidates {
        let name = candidate.get("name").and_then(Value::as_str).unwrap_or("?");
        let applications = candidate
            .get("applications")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        println!("  - {name} ({applications} applications)");
    }
    println!();
    println!(
        "These may be separate companies, so none is picked automatically. Re-run with one \
         exact candidate name from the list above."
    );
}

/// The PATSTAT analytics layer is not configured on this deployment — a
/// typed unavailability, not a retryable error, so it is stated plainly
/// rather than rendered as a generic upstream failure.
fn render_unavailable(ctx: &Context, body: &Value) {
    let message = body
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or(
            "The PATSTAT analytics layer is not configured on this deployment. Aggregate \
             portfolio analytics are unavailable.",
        );

    if ctx.output_format == "json" {
        output::print_json(&json!({
            "ok": false,
            "error": {
                "code": "patstat_unavailable",
                "message": message,
            },
        }));
        return;
    }

    println!("PATSTAT analytics unavailable: backend has no PATSTAT dataset configured.");
    println!("{message}");
}

/// The shared envelope + hint-box rendering every other command uses
/// (mirrors `Context::print_error_envelope`, which is private to the client
/// module) — kept for error shapes this command has no dedicated rendering
/// for (auth failure, rate limit, generic upstream failure, …).
fn render_generic_error(ctx: &Context, envelope: &Value) {
    output::print_value(&ctx.output_format, envelope, &[]);
    if ctx.output_format != "json" {
        if let Some(hint) = envelope.get("providerKeysHint") {
            crate::client::print_keys_hint_box(hint);
        }
        if let Some(hint) = envelope.get("subscriptionHint") {
            crate::client::print_subscription_hint_box(hint);
        }
        if let Some(hint) = envelope.get("rateLimitHint") {
            crate::client::print_rate_limit_hint_box(hint);
        }
    }
}

/// Render a successful portfolio result for human/table output: the
/// backend's quotable summary first, then the year and office aggregates as
/// tables, any grant-status/data caveats, and a provenance line naming the
/// loaded PATSTAT edition.
fn print_portfolio(result: &Value) {
    if let Some(summary) = result.get("summary").and_then(Value::as_str) {
        println!("{summary}");
    }

    print_aggregate(
        result,
        "by_year",
        "Filings by Year",
        &[
            ("year", "Year"),
            ("applications", "Applications"),
            ("granted", "Granted"),
        ],
    );
    print_aggregate(
        result,
        "by_office",
        "Filings by Office",
        &[
            ("office", "Office"),
            ("applications", "Applications"),
            ("granted", "Granted"),
        ],
    );

    print_notes(result, "grant_status_caveats", "Grant status caveats");
    print_notes(result, "notes", "Notes");

    if let Some(edition) = result.get("data_edition").and_then(Value::as_str) {
        println!("\nSource: PATSTAT data edition {edition}");
    }
}

fn print_aggregate(result: &Value, key: &str, label: &str, columns: &[(&str, &str)]) {
    println!("\n{label}");
    match result.get(key).and_then(Value::as_array) {
        Some(rows) if !rows.is_empty() => output::print_table(rows, columns),
        _ => println!("  (no data)"),
    }
}

fn print_notes(result: &Value, key: &str, label: &str) {
    let Some(notes) = result.get(key).and_then(Value::as_array) else {
        return;
    };
    if notes.is_empty() {
        return;
    }
    println!("\n{label}:");
    for note in notes {
        if let Some(text) = note.as_str() {
            println!("  - {text}");
        }
    }
}
