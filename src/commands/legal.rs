use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::json;

use crate::client::{encode_url_component, Context};
use crate::output;

#[derive(Parser)]
pub struct LegalArgs {
    #[command(subcommand)]
    command: LegalCommand,
}

#[derive(Subcommand)]
enum LegalCommand {
    /// Search patent law documents
    Search {
        /// Search query
        query: String,

        /// Jurisdiction filter
        #[arg(long)]
        jurisdiction: Option<Jurisdiction>,

        /// Maximum results to return
        #[arg(long, default_value = "10")]
        limit: u32,

        /// Search mode
        #[arg(long, default_value = "hybrid")]
        search_mode: SearchMode,

        /// Include neighboring context chunks
        #[arg(long)]
        include_context: bool,

        /// Return grouped comprehensive results
        #[arg(long)]
        comprehensive: bool,
    },
    /// Get legal search index statistics
    Stats,
    /// List available legal jurisdictions and sources
    Jurisdictions,
    /// Get legal-search route documentation
    Docs {
        /// Documentation format
        #[arg(long, default_value = "compact")]
        format: String,
    },
}

#[derive(Clone, ValueEnum)]
enum Jurisdiction {
    Epo,
    Uspto,
    Eu,
    Wipo,
    All,
}

#[derive(Clone, ValueEnum)]
enum SearchMode {
    Hybrid,
    Semantic,
    Keyword,
}

pub async fn run(ctx: &Context, args: LegalArgs) -> Result<()> {
    ctx.require_auth()?;

    match args.command {
        LegalCommand::Search {
            query,
            jurisdiction,
            limit,
            search_mode,
            include_context,
            comprehensive,
        } => {
            let mut body = json!({
                "query": query,
                "limit": limit,
                "search_mode": search_mode.as_backend_value(),
                "include_context": include_context,
                "comprehensive": comprehensive,
            });
            if let Some(jurisdiction) = jurisdiction {
                body["jurisdiction"] = json!(jurisdiction.as_backend_value());
            }
            post(ctx, "/v1/legal-search", &body).await
        }
        LegalCommand::Stats => get(ctx, "/v1/legal-search/stats").await,
        LegalCommand::Jurisdictions => get(ctx, "/v1/legal-search/jurisdictions").await,
        LegalCommand::Docs { format } => {
            get(
                ctx,
                &format!(
                    "/v1/legal-search/docs?format={}",
                    encode_url_component(&format)
                ),
            )
            .await
        }
    }
}

async fn get(ctx: &Context, path: &str) -> Result<()> {
    let result = ctx.execute_json_body_or_error(ctx.get(path)).await?;
    output::print_value(&ctx.output_format, &result, &[]);
    Ok(())
}

async fn post(ctx: &Context, path: &str, body: &serde_json::Value) -> Result<()> {
    let result = ctx.execute_json_body_or_error(ctx.post(path, body)).await?;
    output::print_value(&ctx.output_format, &result, &[]);
    Ok(())
}

impl Jurisdiction {
    fn as_backend_value(&self) -> &'static str {
        match self {
            Jurisdiction::Epo => "EPO",
            Jurisdiction::Uspto => "USPTO",
            Jurisdiction::Eu => "EU",
            Jurisdiction::Wipo => "WIPO",
            Jurisdiction::All => "all",
        }
    }
}

impl SearchMode {
    fn as_backend_value(&self) -> &'static str {
        match self {
            SearchMode::Hybrid => "hybrid",
            SearchMode::Semantic => "semantic",
            SearchMode::Keyword => "keyword",
        }
    }
}
