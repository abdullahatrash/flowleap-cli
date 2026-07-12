use std::io::Read;
use std::path::PathBuf;

use anyhow::{bail, Context as AnyhowContext, Result};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};

use crate::client::{encode_url_component, Context};
use crate::output;

#[derive(Parser)]
pub struct UsptoArgs {
    #[command(subcommand)]
    command: UsptoCommand,
}

#[derive(Subcommand)]
enum UsptoCommand {
    /// Search USPTO Open Data Portal records with an ODP Lucene query
    ///
    /// Provide either a `--query` Lucene string (wrapped as `{"q": ...}`) or a
    /// full request `--body` / `--body-file` — the JSON object that
    /// `uspto build-query` emits under `strategy.recommended_query` is a
    /// complete ODP request body and can be submitted directly.
    Search {
        /// USPTO ODP Lucene query string (wrapped as `{"q": ...}`)
        #[arg(long, short, conflicts_with_all = ["body", "body_file"])]
        query: Option<String>,

        /// Full ODP request body as inline JSON, e.g. the object emitted by
        /// `uspto build-query`. Pass `-` to read the body from stdin.
        #[arg(long, conflicts_with = "body_file")]
        body: Option<String>,

        /// File containing a full ODP request body as JSON
        #[arg(long)]
        body_file: Option<PathBuf>,

        /// Maximum results to return (ignored when the body already sets pagination)
        #[arg(long, default_value = "10")]
        limit: u32,
    },
    /// Get a granted patent by patent number
    Grant {
        /// Patent number (for example, 11800000)
        patent_number: String,
    },
    /// Get a patent application by application number
    Application {
        /// Application number
        app_number: String,
    },
    /// Get application continuity data
    Continuity {
        /// Application number
        app_number: String,
    },
    /// Build a USPTO ODP Lucene query from natural language
    BuildQuery {
        /// Natural language description
        description: String,

        /// Query strategy focus
        #[arg(long, value_parser = ["broad", "precise", "comprehensive"], default_value = "comprehensive")]
        focus: String,
    },
}

pub async fn run(ctx: &Context, args: UsptoArgs) -> Result<()> {
    ctx.require_auth()?;

    match args.command {
        UsptoCommand::Search {
            query,
            body,
            body_file,
            limit,
        } => search(ctx, query.as_deref(), body.as_deref(), body_file, limit).await,
        UsptoCommand::Grant { patent_number } => grant(ctx, &patent_number).await,
        UsptoCommand::Application { app_number } => application(ctx, &app_number).await,
        UsptoCommand::Continuity { app_number } => continuity(ctx, &app_number).await,
        UsptoCommand::BuildQuery { description, focus } => {
            build_query(ctx, &description, &focus).await
        }
    }
}

const SEARCH_PATH: &str = "/v1/patent-search-uspto/search";

/// The ODP field the backend's `build-uspto-query` guesses a CPC class into.
/// USPTO ODP search only indexes `inventionTitle` plus a handful of metadata
/// fields — there is no abstract/claims full-text — so a mis-guessed CPC class
/// (the backend has picked H01M batteries for a UV-C sterilization case) drops
/// recall to zero. The zero-recall fallback strips this constraint and retries.
const CPC_FIELD: &str = "applicationMetaData.cpcClassificationBag:";

async fn search(
    ctx: &Context,
    query: Option<&str>,
    body: Option<&str>,
    body_file: Option<PathBuf>,
    limit: u32,
) -> Result<()> {
    let request = build_search_request(query, body, body_file, limit)?;

    let mut result = ctx
        .execute_json_body_or_error(ctx.post(SEARCH_PATH, &request))
        .await?;

    // Zero-recall fallback. The backend query generator guesses a CPC class and
    // ANDs it into a title-only search; when that guess is wrong the search
    // returns nothing. Rather than silently handing back an empty set, drop the
    // CPC constraint and retry once so an over-narrow classification can never
    // blind the USPTO leg on its own.
    if count_results(&result) == 0 {
        if let Some(retried) = cpc_fallback(ctx, &request).await? {
            result = retried;
        }
    }

    // Whatever the query shape, an empty result set is never returned silently:
    // ODP has no abstract/claims full-text, so a feature that lives only in the
    // abstract cannot be matched — the recall pass has to key on the title.
    if count_results(&result) == 0 {
        eprintln!(
            "note: USPTO ODP search returned 0 results. ODP indexes the invention title and \
             metadata only (no abstract/claims full-text), so a distinguishing feature that lives \
             in the abstract cannot be matched here. Broaden to a title search on the core device \
             noun (e.g. --query 'applicationMetaData.inventionTitle:\"charging case\"') and triage \
             abstracts with 'flowleap ops abstract <number>'."
        );
    }

    print_uspto_collection(ctx, &result);
    Ok(())
}

/// Build the ODP request body from `--query` or `--body`/`--body-file`.
/// A `--query` string is wrapped as `{q, pagination}`; a full body is submitted
/// as-is, except that `limit` is injected as the pagination default when the
/// body does not already carry one.
fn build_search_request(
    query: Option<&str>,
    body: Option<&str>,
    body_file: Option<PathBuf>,
    limit: u32,
) -> Result<Value> {
    match (query, body, body_file) {
        (Some(query), None, None) => Ok(json!({
            "q": query,
            "pagination": { "limit": limit, "offset": 0 },
        })),
        (None, Some(body), None) => normalize_body(&read_body_arg(body)?, limit),
        (None, None, Some(path)) => {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("read body file {}", path.display()))?;
            normalize_body(&raw, limit)
        }
        (None, None, None) => bail!(
            "provide a query: --query \"<lucene>\", or a full request body via --body / --body-file \
             (submit the object from 'uspto build-query')"
        ),
        _ => bail!("--query, --body and --body-file are mutually exclusive"),
    }
}

/// Read a `--body` argument, treating a lone `-` as "read the body from stdin"
/// so a build-query pipeline can stream the recommended query in.
fn read_body_arg(body: &str) -> Result<String> {
    if body == "-" {
        let mut buffer = String::new();
        std::io::stdin()
            .read_to_string(&mut buffer)
            .context("read request body from stdin")?;
        Ok(buffer)
    } else {
        Ok(body.to_string())
    }
}

/// Keys the `/v1/patent-search-uspto/search` endpoint accepts in a request
/// body. `uspto build-query` (comprehensive/precise focus) also emits a
/// `filters` field that this endpoint rejects with 400, so a build-query body
/// has to be trimmed to this set before it can be submitted.
const SUPPORTED_BODY_KEYS: &[&str] = &["q", "pagination", "fields", "enrich"];

/// Parse a full ODP request body, drop keys the search endpoint does not accept
/// (so a `uspto build-query` body submits without a 400), and default its
/// pagination limit when absent.
fn normalize_body(raw: &str, limit: u32) -> Result<Value> {
    let mut value: Value = serde_json::from_str(raw).context("request body must be valid JSON")?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("request body must be a JSON object"))?;
    if !object.contains_key("q") {
        bail!("request body must contain a \"q\" field (an ODP Lucene query)");
    }

    let dropped: Vec<String> = object
        .keys()
        .filter(|key| !SUPPORTED_BODY_KEYS.contains(&key.as_str()))
        .cloned()
        .collect();
    for key in &dropped {
        object.remove(key);
    }
    if !dropped.is_empty() {
        eprintln!(
            "note: dropped unsupported request-body field(s) [{}] — the USPTO search endpoint \
             accepts only {:?}. (build-query emits these; they are not forwarded.)",
            dropped.join(", "),
            SUPPORTED_BODY_KEYS
        );
    }

    object
        .entry("pagination")
        .or_insert_with(|| json!({ "limit": limit, "offset": 0 }));
    Ok(value)
}

/// Retry a zero-recall search with the CPC-class constraint stripped. Returns
/// the retried result when a CPC clause was present and removable (even if the
/// retry itself is empty — the caller then falls through to the guidance note),
/// or None when there was no CPC constraint to strip.
async fn cpc_fallback(ctx: &Context, request: &Value) -> Result<Option<Value>> {
    let Some(q) = request.get("q").and_then(Value::as_str) else {
        return Ok(None);
    };
    let Some(stripped) = strip_cpc_constraint(q) else {
        return Ok(None);
    };

    eprintln!(
        "note: the CPC-constrained USPTO query returned 0 results; retrying without the \
         CPC filter ({CPC_FIELD}…)."
    );

    let mut retry = request.clone();
    retry["q"] = Value::String(stripped);
    let result = ctx
        .execute_json_body_or_error(ctx.post(SEARCH_PATH, &retry))
        .await?;
    Ok(Some(result))
}

/// Remove the `cpcClassificationBag:` constraint from an ODP Lucene `q`,
/// together with the boolean operator that joins it, so a zero-recall query can
/// be retried without the (often mis-guessed) CPC filter. Splits on top-level
/// ` AND `, respecting parentheses and quotes. Returns None when there is no CPC
/// clause or removing it would leave nothing to search.
fn strip_cpc_constraint(q: &str) -> Option<String> {
    if !q.contains(CPC_FIELD) {
        return None;
    }
    let clauses = split_top_level_and(q);
    let kept: Vec<&str> = clauses
        .iter()
        .copied()
        .filter(|clause| !clause.contains(CPC_FIELD))
        .collect();
    if kept.len() == clauses.len() || kept.is_empty() {
        return None;
    }
    let rebuilt = kept.join(" AND ");
    (rebuilt != q).then_some(rebuilt)
}

/// Split a Lucene query on top-level ` AND ` separators (case-insensitive),
/// ignoring any ` AND ` that sits inside parentheses or a quoted phrase.
fn split_top_level_and(q: &str) -> Vec<&str> {
    let bytes = q.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth: i32 = 0;
    let mut in_quote = false;
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => in_quote = !in_quote,
            b'(' if !in_quote => depth += 1,
            b')' if !in_quote => depth -= 1,
            _ => {}
        }
        if depth == 0 && !in_quote && is_and_separator(bytes, i) {
            parts.push(q[start..i].trim());
            i += 5; // len(" AND ")
            start = i;
            continue;
        }
        i += 1;
    }
    parts.push(q[start..].trim());
    parts
}

/// True when a case-insensitive ` AND ` separator begins at `i`.
fn is_and_separator(bytes: &[u8], i: usize) -> bool {
    let window = b" AND ";
    bytes.len() >= i + window.len()
        && bytes[i..i + window.len()]
            .iter()
            .zip(window)
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

/// Count the records in an ODP search response across the shapes the backend
/// returns them under.
fn count_results(result: &Value) -> usize {
    for key in ["patentFileWrapperDataBag", "results", "docs", "data"] {
        if let Some(array) = result.get(key).and_then(Value::as_array) {
            return array.len();
        }
    }
    // Some shapes return the collection at the top level.
    result.as_array().map(|array| array.len()).unwrap_or(0)
}

async fn grant(ctx: &Context, patent_number: &str) -> Result<()> {
    let path = format!(
        "/v1/patent-search-uspto/grants/{}",
        encode_url_component(patent_number)
    );
    let result = ctx.execute_json_body_or_error(ctx.get(&path)).await?;
    output::print_value(&ctx.output_format, &result, detail_columns());
    Ok(())
}

async fn application(ctx: &Context, app_number: &str) -> Result<()> {
    let path = format!(
        "/v1/patent-search-uspto/applications/{}",
        encode_url_component(app_number)
    );
    let result = ctx.execute_json_body_or_error(ctx.get(&path)).await?;
    output::print_value(&ctx.output_format, &result, detail_columns());
    Ok(())
}

async fn continuity(ctx: &Context, app_number: &str) -> Result<()> {
    let path = format!(
        "/v1/patent-search-uspto/applications/{}/continuity",
        encode_url_component(app_number)
    );
    let result = ctx.execute_json_body_or_error(ctx.get(&path)).await?;
    output::print_value(&ctx.output_format, &result, continuity_columns());
    Ok(())
}

async fn build_query(ctx: &Context, description: &str, focus: &str) -> Result<()> {
    let body = json!({
        "description": description,
        "focus": focus,
    });

    let result = ctx
        .execute_json_body_or_error(ctx.post("/v1/build-uspto-query", &body))
        .await?;

    if ctx.output_format == "json" || result.get("dryRun").is_some() {
        output::print_json(&result);
        return Ok(());
    }

    // Response shape: { success, strategy: { recommended_query, explanation, ... } }.
    // recommended_query is a full ODP search request body (JSON object), directly
    // submittable to `flowleap uspto search` / POST /v1/patent-search-uspto/search.
    let strategy = result.get("strategy").unwrap_or(&result);
    if let Some(query) = strategy.get("recommended_query") {
        println!("Generated USPTO search request body:\n");
        println!("{}", serde_json::to_string_pretty(query)?);
        if let Some(explanation) = strategy.get("explanation").and_then(|e| e.as_str()) {
            println!("\n{}", explanation);
        }
        println!(
            "\nSubmit the body above directly:\n  \
             flowleap --json uspto search --body '<the JSON object above>'\n  \
             # or save it to a file and: flowleap --json uspto search --body-file query.json\n  \
             # or pipe with jq: flowleap --json uspto build-query \"…\" \\\n  \
             #   | jq .strategy.recommended_query | flowleap --json uspto search --body -"
        );
    } else {
        output::print_json(&result);
    }

    Ok(())
}

fn print_uspto_collection(ctx: &Context, result: &serde_json::Value) {
    let columns = search_columns();
    if let Some(results) = result.get("patentFileWrapperDataBag") {
        output::print_value(&ctx.output_format, results, columns);
    } else if let Some(results) = result.get("results") {
        output::print_value(&ctx.output_format, results, columns);
    } else if let Some(docs) = result.get("docs") {
        output::print_value(&ctx.output_format, docs, columns);
    } else if let Some(data) = result.get("data") {
        output::print_value(&ctx.output_format, data, columns);
    } else {
        output::print_value(&ctx.output_format, result, columns);
    }
}

fn search_columns() -> &'static [(&'static str, &'static str)] {
    &[
        ("patentNumber", "Patent #"),
        ("publicationNumber", "Publication #"),
        ("applicationNumber", "Application #"),
        ("title", "Title"),
        ("applicants", "Applicants"),
        ("publicationDate", "Published"),
    ]
}

fn detail_columns() -> &'static [(&'static str, &'static str)] {
    &[
        ("patentNumber", "Patent #"),
        ("applicationNumber", "Application #"),
        ("title", "Title"),
        ("applicants", "Applicants"),
        ("filingDate", "Filed"),
        ("grantDate", "Granted"),
        ("publicationDate", "Published"),
    ]
}

fn continuity_columns() -> &'static [(&'static str, &'static str)] {
    &[
        ("applicationNumber", "Application #"),
        ("parentApplicationNumber", "Parent Application #"),
        ("childApplicationNumber", "Child Application #"),
        ("continuityType", "Type"),
        ("filingDate", "Filed"),
        ("status", "Status"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression for issue #152: the evaluator's Phase-2 dead-end. For the UV-C
    /// earbud sterilizing case, `uspto build-query --focus comprehensive`
    /// emitted this exact body — a CPC guess of H01M (batteries) ANDed onto
    /// title-only ODP terms — and `uspto search` returned 0, dead-ending the
    /// USPTO leg. The body must now be (a) accepted directly via --body, and
    /// (b) recoverable: the zero-recall fallback strips the wrong CPC class so
    /// the search is retried on the recall terms instead of silently empty.
    #[test]
    fn issue_152_uvc_earbud_build_query_body_is_accepted_and_recoverable() {
        let recommended_query = r#"{
            "q": "applicationMetaData.cpcClassificationBag:H01M* AND (\"UV-C\" OR \"ultraviolet\" OR \"steriliz*\" OR \"disinfect*\") AND \"earbud\"",
            "fields": ["applicationMetaData.inventionTitle"],
            "pagination": { "limit": 25, "offset": 0 }
        }"#;

        // (a) build-query's body submits directly through --body.
        let request = build_search_request(None, Some(recommended_query), None, 10).unwrap();
        let q = request["q"].as_str().unwrap();
        assert_eq!(request["pagination"]["limit"], 25); // body pagination preserved

        // (b) the H01M guess is stripped so the search can be retried.
        let recovered = strip_cpc_constraint(q).expect("CPC clause must be strippable");
        assert!(!recovered.contains(CPC_FIELD));
        assert!(recovered.contains("earbud"));
    }

    #[test]
    fn strip_cpc_drops_the_class_clause_and_one_operator() {
        // The evaluator's exact Phase-2 dead-end: build-query guessed H01M
        // (batteries) for a UV-C sterilization case, so the CPC-constrained
        // query returned 0. Stripping the CPC clause leaves the recall terms.
        let cases = [
            (
                "applicationMetaData.cpcClassificationBag:H01M* AND (\"UV-C\" OR \"steriliz*\") AND \"earbud\"",
                Some("(\"UV-C\" OR \"steriliz*\") AND \"earbud\""),
            ),
            // Leading CPC clause.
            (
                "applicationMetaData.cpcClassificationBag:H04* AND (\"UV-C\" OR \"ultraviolet\")",
                Some("(\"UV-C\" OR \"ultraviolet\")"),
            ),
            // Trailing CPC clause.
            (
                "(\"earbud\") AND applicationMetaData.cpcClassificationBag:A61L*",
                Some("(\"earbud\")"),
            ),
            // No CPC clause — nothing to strip.
            ("applicationMetaData.inventionTitle:\"charging case\"", None),
            // CPC is the only clause — stripping would leave nothing.
            ("applicationMetaData.cpcClassificationBag:H01M*", None),
        ];
        for (input, expected) in cases {
            assert_eq!(
                strip_cpc_constraint(input).as_deref(),
                expected,
                "input: {input}"
            );
        }
    }

    #[test]
    fn split_top_level_and_ignores_nested_and() {
        // A parenthesized " AND " must not be treated as a top-level separator.
        let q = "applicationMetaData.cpcClassificationBag:H04* AND (\"a\" AND \"b\") AND \"c\"";
        assert_eq!(
            split_top_level_and(q),
            vec![
                "applicationMetaData.cpcClassificationBag:H04*",
                "(\"a\" AND \"b\")",
                "\"c\"",
            ]
        );
    }

    #[test]
    fn build_request_wraps_query_and_normalizes_body() {
        // --query wraps into {q, pagination}.
        let wrapped = build_search_request(Some("ti:battery"), None, None, 7).unwrap();
        assert_eq!(wrapped["q"], "ti:battery");
        assert_eq!(wrapped["pagination"]["limit"], 7);

        // --body is submitted as-is; pagination defaults to --limit when absent.
        let body = build_search_request(None, Some(r#"{"q":"ti:x"}"#), None, 25).unwrap();
        assert_eq!(body["q"], "ti:x");
        assert_eq!(body["pagination"]["limit"], 25);

        // A body that already sets pagination is left untouched.
        let paged = build_search_request(
            None,
            Some(r#"{"q":"ti:x","pagination":{"limit":3}}"#),
            None,
            25,
        )
        .unwrap();
        assert_eq!(paged["pagination"]["limit"], 3);

        // Unsupported keys the search endpoint 400s on (build-query emits
        // `filters`) are dropped; supported keys survive.
        let trimmed = build_search_request(
            None,
            Some(r#"{"q":"ti:x","fields":["a"],"enrich":["abstract"],"filters":"typeCode UTL"}"#),
            None,
            10,
        )
        .unwrap();
        assert!(trimmed.get("filters").is_none());
        assert_eq!(trimmed["fields"], json!(["a"]));
        assert_eq!(trimmed["enrich"], json!(["abstract"]));

        // A body without a q field is rejected.
        assert!(build_search_request(None, Some(r#"{"fields":[]}"#), None, 10).is_err());
        // Nothing provided is a usage error.
        assert!(build_search_request(None, None, None, 10).is_err());
    }

    #[test]
    fn count_results_reads_every_collection_shape() {
        let bag = json!({ "patentFileWrapperDataBag": [1, 2, 3] });
        assert_eq!(count_results(&bag), 3);
        assert_eq!(count_results(&json!({ "results": [] })), 0);
        assert_eq!(count_results(&json!([1, 2])), 2);
        assert_eq!(count_results(&json!({ "other": true })), 0);
    }
}
