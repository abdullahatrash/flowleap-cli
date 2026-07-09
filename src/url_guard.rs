//! Base-URL credential guard.
//!
//! Every authenticated request attaches the session/API token and all BYOK
//! provider keys to whatever base URL is in effect — so a typo'd or malicious
//! `--base-url` / `FLOWLEAP_BASE_URL` / config value would exfiltrate every
//! stored credential silently. Before the first request of an invocation
//! leaves the process, this module classifies the destination host and, for
//! non-FlowLeap hosts, prints one prominent stderr warning naming the host and
//! the credential kinds that will travel (presence only, never values). In an
//! interactive terminal it additionally requires a y/N confirmation; in
//! non-interactive, `--json`, or `--dry-run` runs it warns and proceeds, so
//! agents are never blocked and stdout stays clean.

use std::io::{BufRead, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{bail, Result};
use colored::Colorize;

use crate::config::Credentials;

/// Whether the guard already fired for this invocation — warn once per
/// process, not once per request.
static WARNED: AtomicBool = AtomicBool::new(false);

/// What the guard does for one destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardAction {
    /// Trusted host or nothing secret to send — stay silent.
    Allow,
    /// Warn on stderr and proceed (non-interactive, --json, --dry-run, --yes).
    WarnOnly,
    /// Warn and require a y/N confirmation before anything is sent.
    Confirm,
}

/// Hosts credentials may be sent to silently: the FlowLeap domain (and its
/// subdomains) plus local-development loopback names.
pub fn is_trusted_host(host: &str) -> bool {
    // The url crate serializes IPv6 hosts in brackets ("[::1]").
    let host = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase();
    host == "flowleap.co"
        || host.ends_with(".flowleap.co")
        || host == "localhost"
        || host == "127.0.0.1"
        || host == "::1"
}

/// The credential kinds this invocation would attach to requests — presence
/// only, never values. Mirrors exactly what `Context::apply_auth` sends (the
/// EPO pair only travels complete, so half a pair does not count).
pub fn credential_kinds(creds: &Credentials) -> Vec<&'static str> {
    let mut kinds = Vec::new();
    if creds.token.is_some() {
        kinds.push("session token");
    }
    if creds.api_key.is_some() {
        kinds.push("personal API token");
    }
    if creds.epo_pair().is_some() {
        kinds.push("EPO OPS keys");
    }
    if creds.uspto_key.is_some() {
        kinds.push("USPTO ODP key");
    }
    kinds
}

/// Pure decision: what to do for a destination, given host trust, credential
/// presence, interactivity, and an explicit --yes / FLOWLEAP_ASSUME_YES.
pub fn guard_action(
    trusted: bool,
    has_credentials: bool,
    interactive: bool,
    assume_yes: bool,
) -> GuardAction {
    if trusted || !has_credentials {
        GuardAction::Allow
    } else if interactive && !assume_yes {
        GuardAction::Confirm
    } else {
        GuardAction::WarnOnly
    }
}

/// True when FLOWLEAP_ASSUME_YES is set to anything but empty/"0"/"false".
pub fn env_assume_yes() -> bool {
    std::env::var("FLOWLEAP_ASSUME_YES").is_ok_and(|value| {
        let value = value.trim();
        !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
    })
}

/// A y/N answer counts as consent only when it is explicitly "y"/"yes".
pub fn confirmation_accepts(input: &str) -> bool {
    matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

/// Run the guard for one outbound request host. Warns (and, interactively,
/// confirms) at most once per invocation; declining aborts with an error
/// before anything is sent.
pub fn enforce(
    host: &str,
    creds: &Credentials,
    json_output: bool,
    dry_run: bool,
    assume_yes: bool,
) -> Result<()> {
    let kinds = credential_kinds(creds);
    // Confirmation needs a human on both ends and must never block or corrupt
    // agent-facing runs: --json keeps stdout parseable, --dry-run never sends.
    let interactive = !json_output
        && !dry_run
        && std::io::stdin().is_terminal()
        && std::io::stderr().is_terminal();
    let action = guard_action(
        is_trusted_host(host),
        !kinds.is_empty(),
        interactive,
        assume_yes || env_assume_yes(),
    );
    if action == GuardAction::Allow || WARNED.swap(true, Ordering::Relaxed) {
        return Ok(());
    }
    print_warning(host, &kinds);
    if action == GuardAction::Confirm && !confirm(host)? {
        bail!(
            "aborted: credentials were NOT sent to \"{host}\". Fix --base-url / \
             FLOWLEAP_BASE_URL / config.toml, or pass --yes to proceed."
        );
    }
    Ok(())
}

/// One prominent, stderr-only warning naming the host and the credential
/// kinds about to travel (presence only — values never appear anywhere).
fn print_warning(host: &str, kinds: &[&'static str]) {
    eprintln!();
    eprintln!(
        "{} {}",
        "warning:".yellow().bold(),
        format!("sending credentials to non-FlowLeap host \"{host}\"").bold()
    );
    eprintln!("  This invocation attaches: {}.", kinds.join(", "));
    eprintln!("  If this host is wrong, check --base-url, FLOWLEAP_BASE_URL, and config.toml.");
}

/// Interactive y/N prompt on stderr (stdout stays reserved for results).
fn confirm(host: &str) -> Result<bool> {
    eprint!("Send credentials to \"{host}\" anyway? [y/N] ");
    std::io::stderr().flush().ok();
    let mut answer = String::new();
    std::io::stdin().lock().read_line(&mut answer)?;
    Ok(confirmation_accepts(&answer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_classification() {
        let cases = vec![
            ("flowleap.co", true),
            ("api.flowleap.co", true),
            ("API.FLOWLEAP.CO", true),
            ("deep.sub.flowleap.co", true),
            ("localhost", true),
            ("127.0.0.1", true),
            ("::1", true),
            ("[::1]", true),
            ("evil-flowleap.co", false),
            ("notflowleap.co", false),
            ("flowleap.co.evil.example", false),
            ("example.com", false),
            ("192.168.1.10", false),
        ];
        assert_eq!(
            cases
                .iter()
                .map(|&(host, _)| (host, is_trusted_host(host)))
                .collect::<Vec<_>>(),
            cases
        );
    }

    #[test]
    fn action_decision() {
        // (trusted, has_credentials, interactive, assume_yes) → action
        let cases = vec![
            ((true, true, true, false), GuardAction::Allow),
            ((true, true, false, false), GuardAction::Allow),
            ((false, false, true, false), GuardAction::Allow),
            ((false, false, false, false), GuardAction::Allow),
            ((false, true, true, false), GuardAction::Confirm),
            ((false, true, true, true), GuardAction::WarnOnly),
            ((false, true, false, false), GuardAction::WarnOnly),
            ((false, true, false, true), GuardAction::WarnOnly),
        ];
        assert_eq!(
            cases
                .iter()
                .map(|&(input, _)| {
                    let (trusted, has_credentials, interactive, assume_yes) = input;
                    (
                        input,
                        guard_action(trusted, has_credentials, interactive, assume_yes),
                    )
                })
                .collect::<Vec<_>>(),
            cases
        );
    }

    #[test]
    fn confirmation_parsing() {
        let cases = vec![
            ("y", true),
            ("Y", true),
            ("yes", true),
            (" YES \n", true),
            ("", false),
            ("\n", false),
            ("n", false),
            ("no", false),
            ("yep", false),
            ("y please", false),
        ];
        assert_eq!(
            cases
                .iter()
                .map(|&(input, _)| (input, confirmation_accepts(input)))
                .collect::<Vec<_>>(),
            cases
        );
    }

    #[test]
    fn credential_kind_names() {
        let creds = Credentials {
            token: Some("jwt".into()),
            uspto_key: Some("k".into()),
            // Half an EPO pair never travels, so it must not be named.
            epo_key: Some("half-a-pair".into()),
            ..Default::default()
        };
        assert_eq!(
            credential_kinds(&creds),
            vec!["session token", "USPTO ODP key"]
        );
        assert_eq!(
            credential_kinds(&Credentials::default()),
            Vec::<&str>::new()
        );
    }
}
