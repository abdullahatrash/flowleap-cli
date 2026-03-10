use anyhow::Result;
use clap::{Parser, Subcommand};
use reqwest::multipart;
use std::path::PathBuf;

use crate::client::Context;
use crate::output;

#[derive(Parser)]
pub struct OcrArgs {
    #[command(subcommand)]
    command: OcrCommand,
}

#[derive(Subcommand)]
enum OcrCommand {
    /// Extract text from a PDF or image file
    Extract {
        /// Path to the file to process
        file: PathBuf,

        /// Output format (text, markdown)
        #[arg(long, default_value = "markdown")]
        format: String,
    },
}

pub async fn run(ctx: &Context, args: OcrArgs) -> Result<()> {
    ctx.require_auth()?;

    match args.command {
        OcrCommand::Extract { file, format } => extract(ctx, &file, &format).await,
    }
}

async fn extract(ctx: &Context, file: &PathBuf, format: &str) -> Result<()> {
    if !file.exists() {
        anyhow::bail!("File not found: {}", file.display());
    }

    let file_bytes = tokio::fs::read(file).await?;
    let file_name = file
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "document".to_string());

    let part = multipart::Part::bytes(file_bytes)
        .file_name(file_name)
        .mime_str("application/octet-stream")?;

    let form = multipart::Form::new()
        .part("file", part)
        .text("format", format.to_string());

    let req = ctx.post_multipart("/v1/ocr", form);
    let result = ctx.execute_json(req).await?;

    if ctx.output_format == "json" {
        output::print_json(&result);
    } else if let Some(text) = result.get("text").and_then(|t| t.as_str()) {
        println!("{}", text);
    } else {
        output::print_json(&result);
    }

    Ok(())
}
