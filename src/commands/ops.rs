use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::{json, Value};

use crate::client::{encode_url_component, Context};
use crate::output;

#[derive(Parser)]
pub struct OpsArgs {
    #[command(subcommand)]
    command: OpsCommand,
}

#[derive(Subcommand)]
enum OpsCommand {
    /// Search patents using CQL query
    Search {
        /// CQL query string
        #[arg(long)]
        cql: String,

        /// Start position
        #[arg(long, default_value = "1")]
        start: u32,

        /// End position
        #[arg(long, default_value = "25")]
        end: u32,
    },
    /// Get bibliographic data for a patent
    Biblio {
        /// Patent document number (e.g., EP1234567)
        doc: String,
    },
    /// Get claims text for a patent
    Claims {
        /// Patent document number
        doc: String,
        /// Language code (e.g., en, de, fr)
        #[arg(long, default_value = "en")]
        lang: String,
    },
    /// Get full description text for a patent
    Description {
        /// Patent document number
        doc: String,
        /// Language code (e.g., en, de, fr)
        #[arg(long, default_value = "en")]
        lang: String,
    },
    /// Get patent family members
    Family {
        /// Patent document number
        doc: String,
    },
    /// Get legal status events
    Legal {
        /// Patent document number
        doc: String,
    },
    /// Get abstract text
    Abstract {
        /// Patent document number
        doc: String,
    },
}

pub async fn run(ctx: &Context, args: OpsArgs) -> Result<()> {
    ctx.require_auth()?;

    match args.command {
        OpsCommand::Search { cql, start, end } => search(ctx, &cql, start, end).await,
        OpsCommand::Biblio { doc } => fetch_doc(ctx, "biblio", &doc, None).await,
        OpsCommand::Claims { doc, lang } => {
            fetch_doc(ctx, "fulltext/claims", &doc, Some(&lang)).await
        }
        OpsCommand::Description { doc, lang } => {
            fetch_doc(ctx, "fulltext/description", &doc, Some(&lang)).await
        }
        OpsCommand::Family { doc } => fetch_doc(ctx, "family", &doc, None).await,
        OpsCommand::Legal { doc } => fetch_doc(ctx, "legal", &doc, None).await,
        OpsCommand::Abstract { doc } => fetch_doc(ctx, "abstract", &doc, None).await,
    }
}

async fn search(ctx: &Context, cql: &str, start: u32, end: u32) -> Result<()> {
    // The backend expects a "start-end" range string, not separate fields.
    let body = json!({
        "query": cql,
        "range": format!("{}-{}", start, end),
    });

    let req = ctx.post("/v1/patent-search", &body);
    let result = ctx.execute_json_body_or_error(req).await?;

    let columns = &[
        ("docId", "Patent ID"),
        ("title", "Title"),
        ("applicants", "Applicants"),
        ("publicationDate", "Date"),
    ];

    if let Some(docs) = result.get("docs") {
        output::print_value(&ctx.output_format, docs, columns);
    } else {
        output::print_value(&ctx.output_format, &result, columns);
    }

    Ok(())
}

async fn fetch_doc(ctx: &Context, endpoint: &str, doc: &str, lang: Option<&str>) -> Result<()> {
    let mut path = format!("/v1/ops/{}?doc={}", endpoint, encode_url_component(doc));
    if let Some(l) = lang {
        path.push_str(&format!("&lang={}", encode_url_component(l)));
    }

    let req = ctx.get(&path);
    let envelope = ctx.execute_json_allow_error(req).await?;

    // Ops endpoints wrap payloads in a success/data envelope:
    //   { "success": true,  "data": {...}, "cached": bool, "executionTimeMs": n }
    //   { "success": false, "error": "...", "code": "NOT_FOUND", "status": 404 }
    if envelope.get("success") == Some(&Value::Bool(false)) {
        let code = envelope
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or("ERROR");
        let message = envelope
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        let status = envelope
            .get("status")
            .and_then(|v| v.as_u64())
            .unwrap_or(500) as u16;
        let mut error = json!({
            "ok": false,
            "error": {
                "code": code,
                "message": message,
            }
        });
        let hint = crate::client::provider_keys_hint(status, &envelope);
        if let Some(ref hint) = hint {
            error["providerKeysHint"] = hint.clone();
        }
        output::print_value(&ctx.output_format, &error, &[]);
        if ctx.output_format != "json" {
            if let Some(ref hint) = hint {
                crate::client::print_keys_hint_box(hint);
            }
        }
        return Err(crate::client::PrintedError.into());
    }

    if ctx.verbose {
        if let Some(cached) = envelope.get("cached").and_then(|v| v.as_bool()) {
            eprintln!("  cached: {}", cached);
        }
        if let Some(ms) = envelope.get("executionTimeMs").and_then(|v| v.as_u64()) {
            eprintln!("  executionTimeMs: {}", ms);
        }
    }

    let data = envelope.get("data").unwrap_or(&envelope);
    output::print_json(data);

    Ok(())
}
