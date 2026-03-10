use anyhow::Result;
use clap::Parser;
use futures::StreamExt;
use serde_json::{json, Value};
use std::io::{self, IsTerminal, Read};

use crate::client::Context;

#[derive(Parser)]
pub struct ChatArgs {
    /// Message to send
    message: Option<String>,

    /// Model to use (e.g., patent-gemini-3-flash, patent-claude-sonnet)
    #[arg(long, short)]
    model: Option<String>,

    /// System prompt
    #[arg(long)]
    system: Option<String>,

    /// Disable streaming (wait for full response)
    #[arg(long)]
    no_stream: bool,
}

pub async fn run(ctx: &Context, args: ChatArgs) -> Result<()> {
    ctx.require_auth()?;

    // Get message from arg or stdin
    let message = match args.message {
        Some(msg) => msg,
        None => {
            if !io::stdin().is_terminal() {
                let mut buf = String::new();
                io::stdin().read_to_string(&mut buf)?;
                buf
            } else {
                anyhow::bail!("Provide a message as argument or pipe via stdin.\n\nExamples:\n  flowleap chat \"What is claim 1 of EP1234567?\"\n  echo \"Summarize this\" | flowleap chat");
            }
        }
    };

    let model = args
        .model
        .or(ctx.config.default_model.clone())
        .unwrap_or_else(|| "patent-gemini-3-flash".to_string());

    let mut messages = Vec::new();

    if let Some(ref system) = args.system {
        messages.push(json!({"role": "system", "content": system}));
    }
    messages.push(json!({"role": "user", "content": message}));

    let body = json!({
        "model": model,
        "messages": messages,
        "stream": !args.no_stream,
    });

    if args.no_stream {
        run_complete(ctx, &body).await
    } else {
        run_stream(ctx, &body).await
    }
}

async fn run_complete(ctx: &Context, body: &Value) -> Result<()> {
    let req = ctx.post("/v1/chat/completions", body);
    let result = ctx.execute_json(req).await?;

    if ctx.output_format == "json" {
        crate::output::print_json(&result);
    } else {
        // Extract the assistant message content
        if let Some(content) = result
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
        {
            println!("{}", content);
        } else {
            crate::output::print_json(&result);
        }
    }

    Ok(())
}

async fn run_stream(ctx: &Context, body: &Value) -> Result<()> {
    let req = ctx.post("/v1/chat/completions", body);

    if ctx.dry_run {
        ctx.execute(req).await?;
        return Ok(());
    }

    let resp = ctx.execute(req).await?;
    let stream = resp.bytes_stream();

    let mut stream = stream;
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // Process complete SSE lines
        while let Some(pos) = buffer.find("\n\n") {
            let event = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            for line in event.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data.trim() == "[DONE]" {
                        println!();
                        return Ok(());
                    }

                    if let Ok(json) = serde_json::from_str::<Value>(data) {
                        if ctx.output_format == "json" {
                            crate::output::print_json(&json);
                        } else if let Some(content) = json
                            .get("choices")
                            .and_then(|c| c.get(0))
                            .and_then(|c| c.get("delta"))
                            .and_then(|d| d.get("content"))
                            .and_then(|c| c.as_str())
                        {
                            print!("{}", content);
                        }
                    }
                }
            }
        }
    }

    println!();
    Ok(())
}
