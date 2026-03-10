use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;

use crate::client::Context;
use crate::output;

#[derive(Parser)]
pub struct PatentArgs {
    #[command(subcommand)]
    command: PatentCommand,
}

#[derive(Subcommand)]
enum PatentCommand {
    /// Search patents across multiple databases
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,

        /// Source database (epo, uspto)
        #[arg(long, default_value = "epo")]
        source: String,

        /// Maximum results to return
        #[arg(long, default_value = "10")]
        limit: u32,
    },
    /// Build a CQL query from natural language
    BuildQuery {
        /// Natural language description
        description: String,

        /// Model to use for query building
        #[arg(long)]
        model: Option<String>,
    },
}

pub async fn run(ctx: &Context, args: PatentArgs) -> Result<()> {
    ctx.require_auth()?;

    match args.command {
        PatentCommand::Search {
            query,
            source,
            limit,
        } => search(ctx, &query, &source, limit).await,
        PatentCommand::BuildQuery { description, model } => {
            build_query(ctx, &description, model.as_deref()).await
        }
    }
}

async fn search(ctx: &Context, query: &str, source: &str, limit: u32) -> Result<()> {
    let body = json!({
        "query": query,
        "source": source,
        "limit": limit,
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

async fn build_query(ctx: &Context, description: &str, model: Option<&str>) -> Result<()> {
    let mut body = json!({
        "description": description,
    });

    if let Some(m) = model {
        body["model"] = json!(m);
    }

    let req = ctx.post("/v1/build-patent-query", &body);
    let result = ctx.execute_json(req).await?;

    if ctx.output_format == "json" {
        output::print_json(&result);
    } else if let Some(query) = result.get("query").and_then(|q| q.as_str()) {
        println!("Generated CQL query:\n");
        println!("  {}", query);
        if let Some(explanation) = result.get("explanation").and_then(|e| e.as_str()) {
            println!("\n{}", explanation);
        }
    } else {
        output::print_json(&result);
    }

    Ok(())
}
