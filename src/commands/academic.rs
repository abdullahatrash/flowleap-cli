use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;

use crate::client::Context;
use crate::output;

#[derive(Parser)]
pub struct AcademicArgs {
    #[command(subcommand)]
    command: AcademicCommand,
}

#[derive(Subcommand)]
enum AcademicCommand {
    /// Search academic literature
    Search {
        /// Search query
        query: String,

        /// Maximum results
        #[arg(long, default_value = "10")]
        limit: u32,
    },
}

pub async fn run(ctx: &Context, args: AcademicArgs) -> Result<()> {
    ctx.require_auth()?;

    match args.command {
        AcademicCommand::Search { query, limit } => search(ctx, &query, limit).await,
    }
}

async fn search(ctx: &Context, query: &str, limit: u32) -> Result<()> {
    let body = json!({
        "query": query,
        "limit": limit,
    });

    let req = ctx.post("/v1/academic-search", &body);
    let result = ctx.execute_json(req).await?;

    let columns = &[
        ("title", "Title"),
        ("authors", "Authors"),
        ("year", "Year"),
        ("source", "Source"),
    ];

    if let Some(results) = result.get("results") {
        output::print_value(&ctx.output_format, results, columns);
    } else {
        output::print_value(&ctx.output_format, &result, columns);
    }

    Ok(())
}
