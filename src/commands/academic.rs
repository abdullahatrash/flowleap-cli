use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
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
    /// Search academic literature (Semantic Scholar + arXiv)
    Search {
        /// Search query
        query: String,

        /// Maximum results
        #[arg(long, default_value = "10")]
        limit: u32,

        /// Sources to search (repeatable)
        #[arg(long, value_enum)]
        source: Vec<AcademicSource>,

        /// Only include papers published in or after this year
        #[arg(long)]
        from_year: Option<u32>,

        /// Only include papers published in or before this year
        #[arg(long)]
        to_year: Option<u32>,
    },
}

#[derive(Clone, ValueEnum)]
enum AcademicSource {
    Scholar,
    Arxiv,
}

impl AcademicSource {
    fn as_backend_value(&self) -> &'static str {
        match self {
            AcademicSource::Scholar => "scholar",
            AcademicSource::Arxiv => "arxiv",
        }
    }
}

pub async fn run(ctx: &Context, args: AcademicArgs) -> Result<()> {
    ctx.require_auth()?;

    match args.command {
        AcademicCommand::Search {
            query,
            limit,
            source,
            from_year,
            to_year,
        } => search(ctx, &query, limit, &source, from_year, to_year).await,
    }
}

async fn search(
    ctx: &Context,
    query: &str,
    limit: u32,
    sources: &[AcademicSource],
    from_year: Option<u32>,
    to_year: Option<u32>,
) -> Result<()> {
    let mut body = json!({
        "query": query,
        "maxResults": limit,
    });
    if !sources.is_empty() {
        body["sources"] = json!(sources
            .iter()
            .map(AcademicSource::as_backend_value)
            .collect::<Vec<_>>());
    }
    let mut filter = json!({});
    if let Some(year) = from_year {
        filter["fromYear"] = json!(year);
    }
    if let Some(year) = to_year {
        filter["toYear"] = json!(year);
    }
    if filter.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
        body["filter"] = filter;
    }

    let req = ctx.post("/v1/academic-search", &body);
    let result = ctx.execute_json_body_or_error(req).await?;

    let columns = &[
        ("title", "Title"),
        ("authors", "Authors"),
        ("year", "Year"),
        ("source", "Source"),
        ("citations", "Citations"),
    ];

    if let Some(papers) = result.get("papers") {
        output::print_value(&ctx.output_format, papers, columns);
    } else {
        output::print_value(&ctx.output_format, &result, columns);
    }

    Ok(())
}
