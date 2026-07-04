use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;

use crate::client::{encode_url_component, Context};
use crate::output;

#[derive(Parser)]
pub struct UsptoArgs {
    #[command(subcommand)]
    command: UsptoCommand,
}

#[derive(Subcommand)]
enum UsptoCommand {
    /// Search USPTO Open Data Portal records with an ODP Lucene query
    Search {
        /// USPTO ODP Lucene query string
        #[arg(long, short)]
        query: String,

        /// Maximum results to return
        #[arg(long, default_value = "10")]
        limit: u32,
    },
    /// Get a granted patent by patent number
    Grant {
        /// Patent number (for example, 11800000)
        patent_number: String,
    },
    /// Get a patent application by application number
    Application {
        /// Application number
        app_number: String,
    },
    /// Get application continuity data
    Continuity {
        /// Application number
        app_number: String,
    },
    /// Build a USPTO ODP Lucene query from natural language
    BuildQuery {
        /// Natural language description
        description: String,

        /// Query strategy focus
        #[arg(long, value_parser = ["broad", "precise", "comprehensive"], default_value = "comprehensive")]
        focus: String,
    },
}

pub async fn run(ctx: &Context, args: UsptoArgs) -> Result<()> {
    ctx.require_auth()?;

    match args.command {
        UsptoCommand::Search { query, limit } => search(ctx, &query, limit).await,
        UsptoCommand::Grant { patent_number } => grant(ctx, &patent_number).await,
        UsptoCommand::Application { app_number } => application(ctx, &app_number).await,
        UsptoCommand::Continuity { app_number } => continuity(ctx, &app_number).await,
        UsptoCommand::BuildQuery { description, focus } => {
            build_query(ctx, &description, &focus).await
        }
    }
}

async fn search(ctx: &Context, query: &str, limit: u32) -> Result<()> {
    let body = json!({
        "q": query,
        "pagination": {
            "limit": limit,
            "offset": 0,
        },
    });

    let req = ctx.post("/v1/patent-search-uspto/search", &body);
    let result = ctx.execute_json_body_or_error(req).await?;

    print_uspto_collection(ctx, &result);
    Ok(())
}

async fn grant(ctx: &Context, patent_number: &str) -> Result<()> {
    let path = format!(
        "/v1/patent-search-uspto/grants/{}",
        encode_url_component(patent_number)
    );
    let result = ctx.execute_json_body_or_error(ctx.get(&path)).await?;
    output::print_value(&ctx.output_format, &result, detail_columns());
    Ok(())
}

async fn application(ctx: &Context, app_number: &str) -> Result<()> {
    let path = format!(
        "/v1/patent-search-uspto/applications/{}",
        encode_url_component(app_number)
    );
    let result = ctx.execute_json_body_or_error(ctx.get(&path)).await?;
    output::print_value(&ctx.output_format, &result, detail_columns());
    Ok(())
}

async fn continuity(ctx: &Context, app_number: &str) -> Result<()> {
    let path = format!(
        "/v1/patent-search-uspto/applications/{}/continuity",
        encode_url_component(app_number)
    );
    let result = ctx.execute_json_body_or_error(ctx.get(&path)).await?;
    output::print_value(&ctx.output_format, &result, continuity_columns());
    Ok(())
}

async fn build_query(ctx: &Context, description: &str, focus: &str) -> Result<()> {
    let body = json!({
        "description": description,
        "focus": focus,
    });

    let result = ctx
        .execute_json_body_or_error(ctx.post("/v1/build-uspto-query", &body))
        .await?;

    if ctx.output_format == "json" || result.get("dryRun").is_some() {
        output::print_json(&result);
        return Ok(());
    }

    // Response shape: { success, strategy: { recommended_query, explanation, ... } }.
    // recommended_query is a full ODP search request body (JSON object), directly
    // submittable to `flowleap uspto search` / POST /v1/patent-search-uspto/search.
    let strategy = result.get("strategy").unwrap_or(&result);
    if let Some(query) = strategy.get("recommended_query") {
        println!("Generated USPTO search request body:\n");
        println!("{}", serde_json::to_string_pretty(query)?);
        if let Some(explanation) = strategy.get("explanation").and_then(|e| e.as_str()) {
            println!("\n{}", explanation);
        }
    } else {
        output::print_json(&result);
    }

    Ok(())
}

fn print_uspto_collection(ctx: &Context, result: &serde_json::Value) {
    let columns = search_columns();
    if let Some(results) = result.get("patentFileWrapperDataBag") {
        output::print_value(&ctx.output_format, results, columns);
    } else if let Some(results) = result.get("results") {
        output::print_value(&ctx.output_format, results, columns);
    } else if let Some(docs) = result.get("docs") {
        output::print_value(&ctx.output_format, docs, columns);
    } else if let Some(data) = result.get("data") {
        output::print_value(&ctx.output_format, data, columns);
    } else {
        output::print_value(&ctx.output_format, result, columns);
    }
}

fn search_columns() -> &'static [(&'static str, &'static str)] {
    &[
        ("patentNumber", "Patent #"),
        ("publicationNumber", "Publication #"),
        ("applicationNumber", "Application #"),
        ("title", "Title"),
        ("applicants", "Applicants"),
        ("publicationDate", "Published"),
    ]
}

fn detail_columns() -> &'static [(&'static str, &'static str)] {
    &[
        ("patentNumber", "Patent #"),
        ("applicationNumber", "Application #"),
        ("title", "Title"),
        ("applicants", "Applicants"),
        ("filingDate", "Filed"),
        ("grantDate", "Granted"),
        ("publicationDate", "Published"),
    ]
}

fn continuity_columns() -> &'static [(&'static str, &'static str)] {
    &[
        ("applicationNumber", "Application #"),
        ("parentApplicationNumber", "Parent Application #"),
        ("childApplicationNumber", "Child Application #"),
        ("continuityType", "Type"),
        ("filingDate", "Filed"),
        ("status", "Status"),
    ]
}
