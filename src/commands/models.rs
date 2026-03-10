use anyhow::Result;
use clap::Parser;

use crate::client::Context;
use crate::output;

#[derive(Parser)]
pub struct ModelsArgs {
    /// Filter by provider (openai, anthropic, google)
    #[arg(long)]
    provider: Option<String>,
}

pub async fn run(ctx: &Context, args: ModelsArgs) -> Result<()> {
    let path = match args.provider {
        Some(ref p) => format!("/api/models?provider={}", p),
        None => "/api/models".to_string(),
    };

    let req = ctx.get(&path);
    let result = ctx.execute_json(req).await?;

    let columns = &[
        ("id", "Model ID"),
        ("provider", "Provider"),
        ("name", "Name"),
    ];

    if let Some(data) = result.get("data") {
        output::print_value(&ctx.output_format, data, columns);
    } else {
        output::print_value(&ctx.output_format, &result, columns);
    }

    Ok(())
}
