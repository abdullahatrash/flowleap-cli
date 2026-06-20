use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::json;

use crate::client::{encode_url_component, Context};
use crate::output;

#[derive(Parser)]
pub struct CitationArgs {
    #[command(subcommand)]
    command: CitationCommand,
}

#[derive(Subcommand)]
enum CitationCommand {
    /// Search citations by USPTO application number
    Search {
        /// USPTO application number
        application_number: String,

        /// Number of results to return
        #[arg(long, default_value = "100")]
        size: u32,

        /// Pagination offset
        #[arg(long, default_value = "0")]
        offset: u32,

        /// Citation category filter
        #[arg(long)]
        category: Option<CitationCategory>,

        /// Only return examiner-cited references
        #[arg(long)]
        examiner_cited_only: bool,
    },
    /// Find patents that cite a document
    Forward {
        /// Cited patent or publication document
        cited_document: String,

        /// Number of results to return
        #[arg(long, default_value = "100")]
        size: u32,

        /// Pagination offset
        #[arg(long, default_value = "0")]
        offset: u32,

        /// Citation category filter
        #[arg(long)]
        category: Option<CitationCategory>,

        /// Only return examiner-cited references
        #[arg(long)]
        examiner_cited_only: bool,
    },
    /// Get citation statistics for an application
    Stats {
        /// USPTO application number
        application_number: String,
    },
    /// Get X-rated novelty-destroying citations
    Novelty {
        /// USPTO application number
        application_number: String,

        /// Number of results to return
        #[arg(long, default_value = "100")]
        size: u32,
    },
}

#[derive(Clone, ValueEnum)]
enum CitationCategory {
    X,
    Y,
    A,
    All,
}

pub async fn run(ctx: &Context, args: CitationArgs) -> Result<()> {
    ctx.require_auth()?;

    match args.command {
        CitationCommand::Search {
            application_number,
            size,
            offset,
            category,
            examiner_cited_only,
        } => {
            let mut body = json!({
                "applicationNumber": application_number,
                "size": size,
                "offset": offset,
                "examinerCitedOnly": examiner_cited_only,
            });
            if let Some(category) = category {
                body["category"] = json!(category.as_backend_value());
            }
            post(ctx, "/v1/citation-search", &body).await
        }
        CitationCommand::Forward {
            cited_document,
            size,
            offset,
            category,
            examiner_cited_only,
        } => {
            let mut body = json!({
                "citedDocument": cited_document,
                "size": size,
                "offset": offset,
                "examinerCitedOnly": examiner_cited_only,
            });
            if let Some(category) = category {
                body["category"] = json!(category.as_backend_value());
            }
            post(ctx, "/v1/citation-search/forward", &body).await
        }
        CitationCommand::Stats { application_number } => {
            get(
                ctx,
                &format!(
                    "/v1/citation-search/stats/{}",
                    encode_url_component(&application_number)
                ),
            )
            .await
        }
        CitationCommand::Novelty {
            application_number,
            size,
        } => {
            get(
                ctx,
                &format!(
                    "/v1/citation-search/novelty/{}?size={size}",
                    encode_url_component(&application_number)
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

impl CitationCategory {
    fn as_backend_value(&self) -> &'static str {
        match self {
            CitationCategory::X => "X",
            CitationCategory::Y => "Y",
            CitationCategory::A => "A",
            CitationCategory::All => "all",
        }
    }
}
