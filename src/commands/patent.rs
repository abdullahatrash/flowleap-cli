use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
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
    /// Search patents via EPO OPS (worldwide coverage)
    Search {
        /// EPO CQL query (e.g. 'ti="battery separator" and pa=lg'). Use
        /// `flowleap patent build-query` to generate CQL from plain language.
        #[arg(long, short)]
        query: String,

        /// Maximum results to return (1-100)
        #[arg(long, default_value = "10")]
        limit: u32,

        /// Country filter, comma-separated (e.g. "EP,WO"); "all" disables
        #[arg(long)]
        countries: Option<String>,
    },
    /// Build a CQL query from natural language
    BuildQuery {
        /// Natural language description
        description: String,

        /// Query strategy focus
        #[arg(long, value_enum, default_value = "comprehensive")]
        focus: QueryFocus,
    },
}

#[derive(Clone, ValueEnum)]
enum QueryFocus {
    Broad,
    Precise,
    Comprehensive,
}

impl QueryFocus {
    fn as_backend_value(&self) -> &'static str {
        match self {
            QueryFocus::Broad => "broad",
            QueryFocus::Precise => "precise",
            QueryFocus::Comprehensive => "comprehensive",
        }
    }
}

pub async fn run(ctx: &Context, args: PatentArgs) -> Result<()> {
    ctx.require_auth()?;

    match args.command {
        PatentCommand::Search {
            query,
            limit,
            countries,
        } => search(ctx, &query, limit, countries.as_deref()).await,
        PatentCommand::BuildQuery { description, focus } => {
            build_query(ctx, &description, &focus).await
        }
    }
}

async fn search(ctx: &Context, query: &str, limit: u32, countries: Option<&str>) -> Result<()> {
    let mut body = json!({
        "query": query,
        "range": format!("1-{}", limit.clamp(1, 100)),
    });
    if let Some(countries) = countries {
        body["countries"] = json!(countries);
    }

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

async fn build_query(ctx: &Context, description: &str, focus: &QueryFocus) -> Result<()> {
    let body = json!({
        "description": description,
        "focus": focus.as_backend_value(),
    });

    let req = ctx.post("/v1/build-patent-query", &body);
    let result = ctx.execute_json_body_or_error(req).await?;

    if ctx.output_format == "json" || result.get("dryRun").is_some() {
        output::print_json(&result);
        return Ok(());
    }

    // Response shape: { success, cached, strategy: { recommended_cql,
    // explanation, alternatives: { broader, narrower }, tips } }
    let strategy = result.get("strategy").unwrap_or(&result);
    if let Some(query) = strategy.get("recommended_cql").and_then(|q| q.as_str()) {
        println!("Generated CQL query:\n");
        println!("  {}", query);
        if let Some(explanation) = strategy.get("explanation").and_then(|e| e.as_str()) {
            println!("\n{}", explanation);
        }
        if let Some(alternatives) = strategy.get("alternatives") {
            if let Some(broader) = alternatives.get("broader").and_then(|v| v.as_str()) {
                println!("\nBroader:  {}", broader);
            }
            if let Some(narrower) = alternatives.get("narrower").and_then(|v| v.as_str()) {
                println!("Narrower: {}", narrower);
            }
        }
        if let Some(tips) = strategy.get("tips").and_then(|t| t.as_array()) {
            if !tips.is_empty() {
                println!("\nTips:");
                for tip in tips.iter().filter_map(|t| t.as_str()) {
                    println!("  - {}", tip);
                }
            }
        }
    } else {
        output::print_json(&result);
    }

    Ok(())
}
