use anyhow::{bail, Result};

use crate::client::Context;

pub(super) fn require_external_processing_consent(ctx: &Context, allowed: bool) -> Result<()> {
    if ctx.dry_run {
        return Ok(());
    }

    if !allowed {
        bail!(
            "Query generation sends the description to the FlowLeap backend and then to Anthropic or OpenAI. \
             Review the data-handling implications and re-run with --allow-external-processing to consent."
        );
    }

    eprintln!(
        "notice: sending the query description to FlowLeap for processing by Anthropic or OpenAI"
    );
    Ok(())
}
