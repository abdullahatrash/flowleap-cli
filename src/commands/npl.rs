use anyhow::Result;
use clap::{Parser, ValueEnum};
use serde_json::json;

use crate::client::Context;
use crate::output;

#[derive(Parser)]
pub struct NplArgs {
    /// Search query for scholarly works
    pub query: String,

    /// Maximum results to return
    #[arg(long, default_value = "10")]
    pub limit: u32,

    /// Page number
    #[arg(long, default_value = "1")]
    pub page: u32,

    /// Filter by publication year from
    #[arg(long)]
    pub from_year: Option<u32>,

    /// Filter by publication year to
    #[arg(long)]
    pub to_year: Option<u32>,

    /// Only return open-access works
    #[arg(long)]
    pub open_access: bool,

    /// Filter by publication type
    #[arg(long)]
    pub r#type: Option<NplType>,
}

#[derive(Clone, ValueEnum)]
pub enum NplType {
    JournalArticle,
    BookChapter,
    ProceedingsArticle,
    Preprint,
}

pub async fn run(ctx: &Context, args: NplArgs) -> Result<()> {
    ctx.require_auth()?;

    let mut filter = json!({});
    if let Some(year) = args.from_year {
        filter["fromYear"] = json!(year);
    }
    if let Some(year) = args.to_year {
        filter["toYear"] = json!(year);
    }
    if args.open_access {
        filter["openAccess"] = json!(true);
    }
    if let Some(kind) = args.r#type {
        filter["type"] = json!(kind.as_backend_value());
    }

    let mut body = json!({
        "query": args.query,
        "limit": args.limit,
        "page": args.page,
    });
    if filter
        .as_object()
        .map(|obj| !obj.is_empty())
        .unwrap_or(false)
    {
        body["filter"] = filter;
    }

    let result = ctx
        .execute_json_body_or_error(ctx.post("/v1/npl-search", &body))
        .await?;
    output::print_value(&ctx.output_format, &result, &[]);
    Ok(())
}

impl NplType {
    fn as_backend_value(&self) -> &'static str {
        match self {
            NplType::JournalArticle => "journal-article",
            NplType::BookChapter => "book-chapter",
            NplType::ProceedingsArticle => "proceedings-article",
            NplType::Preprint => "preprint",
        }
    }
}
