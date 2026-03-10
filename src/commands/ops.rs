use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;

use crate::client::Context;
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
    },
    /// Get full description text for a patent
    Description {
        /// Patent document number
        doc: String,
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
        OpsCommand::Biblio { doc } => fetch_doc(ctx, "biblio", &doc).await,
        OpsCommand::Claims { doc } => fetch_doc(ctx, "claims", &doc).await,
        OpsCommand::Description { doc } => fetch_doc(ctx, "description", &doc).await,
        OpsCommand::Family { doc } => fetch_doc(ctx, "family", &doc).await,
        OpsCommand::Legal { doc } => fetch_doc(ctx, "legal", &doc).await,
        OpsCommand::Abstract { doc } => fetch_doc(ctx, "abstract", &doc).await,
    }
}

async fn search(ctx: &Context, cql: &str, start: u32, end: u32) -> Result<()> {
    let body = json!({
        "query": cql,
        "start": start,
        "end": end,
    });

    let req = ctx.post("/v1/patent-search", &body);
    let result = ctx.execute_json(req).await?;

    let columns = &[
        ("publicationNumber", "Patent ID"),
        ("title", "Title"),
        ("applicant", "Applicant"),
        ("publicationDate", "Date"),
    ];

    if let Some(results) = result.get("results") {
        output::print_value(&ctx.output_format, results, columns);
    } else {
        output::print_value(&ctx.output_format, &result, columns);
    }

    Ok(())
}

async fn fetch_doc(ctx: &Context, endpoint: &str, doc: &str) -> Result<()> {
    let path = format!("/v1/patent-search/{}?doc={}", endpoint, doc);
    let req = ctx.get(&path);
    let result = ctx.execute_json(req).await?;

    if ctx.output_format == "json" {
        output::print_json(&result);
    } else {
        // Human-readable: print the relevant text content
        output::print_json(&result);
    }

    Ok(())
}
