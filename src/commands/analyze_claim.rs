use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use clap::{Parser, ValueEnum};
use serde_json::{json, Value};

use crate::client::Context;
use crate::output;

/// Analyze a patent claim: keywords, IPC codes, search queries, elements
///
/// Claim text comes from the positional argument, --file, or stdin when
/// neither is given (pipe-friendly for scripts and agents).
///
/// Examples:
///   flowleap analyze-claim "A method for training a neural network comprising..."
///   flowleap analyze-claim --file claim1.txt --focus search
///   pbpaste | flowleap analyze-claim --json
///   flowleap ops claims --doc EP1000000 --json | jq -r ... | flowleap analyze-claim
#[derive(Parser)]
#[command(after_help = "\
Claim text comes from the positional argument, --file, or stdin when neither
is given (pipe-friendly for scripts and agents).

Examples:
  flowleap analyze-claim \"A method for training a neural network comprising...\"
  flowleap analyze-claim --file claim1.txt --focus search
  pbpaste | flowleap analyze-claim --json
  cat claims.txt | flowleap analyze-claim --focus elements")]
pub struct AnalyzeClaimArgs {
    /// Claim text to analyze (omit to use --file or stdin)
    claim_text: Option<String>,

    /// Read claim text from a file
    #[arg(long, conflicts_with = "claim_text")]
    file: Option<PathBuf>,

    /// Analysis focus: search = keywords/queries, elements = claim breakdown, full = both
    #[arg(long, value_enum)]
    focus: Option<Focus>,
}

#[derive(Clone, ValueEnum)]
enum Focus {
    Search,
    Elements,
    Full,
}

impl Focus {
    fn as_backend_value(&self) -> &'static str {
        match self {
            Focus::Search => "search",
            Focus::Elements => "elements",
            Focus::Full => "full",
        }
    }
}

pub async fn run(ctx: &Context, args: AnalyzeClaimArgs) -> Result<()> {
    ctx.require_auth()?;

    let claim_text = resolve_claim_text(args.claim_text, args.file.as_deref())?;
    let mut body = json!({ "claimText": claim_text });
    if let Some(focus) = args.focus {
        body["focus"] = json!(focus.as_backend_value());
    }

    let req = ctx.post("/v1/analyze-claim", &body);

    if ctx.output_format == "json" {
        let envelope = ctx.execute_json_envelope_or_error(req).await?;
        output::print_json(&envelope);
        return Ok(());
    }

    let result = ctx.execute_json_body_or_error(req).await?;
    if result.get("dryRun").and_then(Value::as_bool) == Some(true) {
        output::print_value(&ctx.output_format, &result, &[]);
        return Ok(());
    }

    match result.get("analysis") {
        Some(analysis) => {
            print!("{}", render_analysis(analysis));
            if result.get("cached").and_then(Value::as_bool) == Some(true) {
                eprintln!("(cached result)");
            }
        }
        None => output::print_value(&ctx.output_format, &result, &[]),
    }

    Ok(())
}

/// Resolve claim text with precedence: positional argument > --file > stdin.
/// Reading stdin from an interactive terminal would hang, so that case errors
/// with guidance instead.
fn resolve_claim_text(arg: Option<String>, file: Option<&Path>) -> Result<String> {
    let text = if let Some(text) = arg {
        text
    } else if let Some(path) = file {
        match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) => bail!("Cannot read {}: {}", path.display(), err),
        }
    } else {
        if std::io::stdin().is_terminal() {
            bail!(
                "No claim text provided. Pass it as an argument, use --file <path>, or pipe it via stdin."
            );
        }
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        buffer
    };

    let text = text.trim().to_string();
    if text.is_empty() {
        bail!("Claim text is empty.");
    }
    Ok(text)
}

/// Human-readable rendering of the backend's `analysis` object.
fn render_analysis(analysis: &Value) -> String {
    let mut out = String::new();

    if let Some(keywords) = string_list(analysis.get("keywords")) {
        out.push_str(&format!("Keywords: {}\n", keywords.join(", ")));
    }
    if let Some(codes) = string_list(analysis.get("ipcCodes")) {
        out.push_str(&format!("IPC codes: {}\n", codes.join(", ")));
    }
    if let Some(queries) = string_list(analysis.get("suggestedQueries")) {
        out.push_str("Suggested queries:\n");
        for (i, query) in queries.iter().enumerate() {
            out.push_str(&format!("  {}. {}\n", i + 1, query));
        }
    }
    if let Some(elements) = analysis.get("claimElements").and_then(Value::as_array) {
        if !elements.is_empty() {
            out.push_str("Claim elements:\n");
            for element in elements {
                let kind = element
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("element");
                let text = element.get("element").and_then(Value::as_str).unwrap_or("");
                out.push_str(&format!("  [{}] {}\n", kind, text));
            }
        }
    }
    if let Some(synonyms) = analysis.get("synonyms").and_then(Value::as_object) {
        if !synonyms.is_empty() {
            out.push_str("Synonyms:\n");
            for (keyword, alternatives) in synonyms {
                if let Some(list) = string_list(Some(alternatives)) {
                    out.push_str(&format!("  {}: {}\n", keyword, list.join(", ")));
                }
            }
        }
    }

    if out.is_empty() {
        out = serde_json::to_string_pretty(analysis).unwrap_or_default();
        out.push('\n');
    }
    out
}

/// A JSON array of strings as Vec, or None when absent/empty.
fn string_list(value: Option<&Value>) -> Option<Vec<&str>> {
    let items: Vec<&str> = value?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

#[cfg(test)]
mod tests {
    use super::{render_analysis, resolve_claim_text};
    use serde_json::json;

    #[test]
    fn argument_takes_precedence() {
        let text = resolve_claim_text(Some("A method for...".to_string()), None).unwrap();
        assert_eq!(text, "A method for...");
    }

    #[test]
    fn file_input_is_read_and_trimmed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("claim.txt");
        std::fs::write(&path, "  A device comprising...\n").unwrap();

        let text = resolve_claim_text(None, Some(&path)).unwrap();
        assert_eq!(text, "A device comprising...");
    }

    #[test]
    fn missing_file_errors_clearly() {
        let err = resolve_claim_text(None, Some(std::path::Path::new("/nonexistent/claim.txt")))
            .unwrap_err();
        assert!(err.to_string().contains("Cannot read"));
    }

    #[test]
    fn empty_text_is_rejected() {
        let err = resolve_claim_text(Some("   ".to_string()), None).unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn renders_full_analysis() {
        let rendered = render_analysis(&json!({
            "keywords": ["battery", "lithium"],
            "synonyms": { "battery": ["cell", "accumulator"] },
            "ipcCodes": ["H01M10/052"],
            "suggestedQueries": ["ta=battery AND ic=H01M"],
            "claimElements": [
                { "element": "A battery pack", "type": "preamble" },
                { "element": "lithium cell", "type": "component" }
            ],
        }));

        assert_eq!(
            rendered,
            "Keywords: battery, lithium\n\
             IPC codes: H01M10/052\n\
             Suggested queries:\n  1. ta=battery AND ic=H01M\n\
             Claim elements:\n  [preamble] A battery pack\n  [component] lithium cell\n\
             Synonyms:\n  battery: cell, accumulator\n"
        );
    }
}
