use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::client::Context;
use crate::output;

#[derive(Parser)]
pub struct HealthArgs {
    #[command(subcommand)]
    command: Option<HealthCommand>,
}

#[derive(Subcommand)]
enum HealthCommand {
    /// Detailed API health endpoint
    Api,
    /// Combined local and distributed cache health
    Cache,
    /// Redis cache health
    Redis,
    /// HTTP agent pool status
    Agents,
}

pub async fn run(ctx: &Context, args: HealthArgs) -> Result<()> {
    let path = match args.command {
        None => "/health",
        Some(HealthCommand::Api) => "/v1/health",
        Some(HealthCommand::Cache) => "/health/cache",
        Some(HealthCommand::Redis) => "/health/redis",
        Some(HealthCommand::Agents) => "/health/agents",
    };

    let result = ctx.execute_json_envelope_or_error(ctx.get(path)).await?;
    output::print_value(&ctx.output_format, &result, &[]);
    Ok(())
}
