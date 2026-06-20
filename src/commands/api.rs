use anyhow::{Context as AnyhowContext, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

use crate::client::Context;
use crate::output;

#[derive(Parser)]
pub struct ApiArgs {
    #[command(subcommand)]
    command: ApiCommand,
}

#[derive(Subcommand)]
enum ApiCommand {
    /// Get the authenticated user profile
    Profile,
    /// Get authenticated usage information
    Usage,
    /// Raw API request using configured base URL and auth
    Request {
        /// HTTP method
        method: HttpMethod,

        /// API path, for example /v1/health
        path: String,

        /// JSON request body as an inline string
        #[arg(long, conflicts_with = "body_file")]
        body: Option<String>,

        /// File containing a JSON request body
        #[arg(long)]
        body_file: Option<PathBuf>,
    },
}

#[derive(Clone, ValueEnum)]
enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

pub async fn run(ctx: &Context, args: ApiArgs) -> Result<()> {
    match args.command {
        ApiCommand::Profile => get(ctx, "/api/profile").await,
        ApiCommand::Usage => get(ctx, "/api/usage").await,
        ApiCommand::Request {
            method,
            path,
            body,
            body_file,
        } => request(ctx, method, &path, body.as_deref(), body_file).await,
    }
}

async fn get(ctx: &Context, path: &str) -> Result<()> {
    ctx.require_auth()?;
    let result = ctx.execute_json_envelope_or_error(ctx.get(path)).await?;
    output::print_value(&ctx.output_format, &result, &[]);
    Ok(())
}

async fn request(
    ctx: &Context,
    method: HttpMethod,
    path: &str,
    body: Option<&str>,
    body_file: Option<PathBuf>,
) -> Result<()> {
    if !matches!(method, HttpMethod::Get) {
        ctx.require_auth()?;
    }

    let parsed_body = parse_body(body, body_file)?;
    let req = ctx.request(method.into(), &normalize_path(path), parsed_body.as_ref());
    let result = ctx.execute_json_envelope_or_error(req).await?;
    output::print_value(&ctx.output_format, &result, &[]);
    Ok(())
}

fn parse_body(body: Option<&str>, body_file: Option<PathBuf>) -> Result<Option<Value>> {
    let raw = if let Some(body) = body {
        Some(body.to_string())
    } else if let Some(path) = body_file {
        Some(fs::read_to_string(path).context("read body file")?)
    } else {
        None
    };

    let Some(raw) = raw else { return Ok(None) };

    let value = serde_json::from_str(&raw).context("request body must be valid JSON")?;
    Ok(Some(value))
}

fn normalize_path(path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{}", path)
    }
}

impl From<HttpMethod> for reqwest::Method {
    fn from(value: HttpMethod) -> Self {
        match value {
            HttpMethod::Get => reqwest::Method::GET,
            HttpMethod::Post => reqwest::Method::POST,
            HttpMethod::Put => reqwest::Method::PUT,
            HttpMethod::Patch => reqwest::Method::PATCH,
            HttpMethod::Delete => reqwest::Method::DELETE,
        }
    }
}
