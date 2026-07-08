//! Once-a-day update notifier.
//!
//! Agent-safe by construction: never runs for --json / --dry-run / non-TTY
//! stderr / FLOWLEAP_NO_UPDATE_CHECK, the notice goes to stderr only, and the
//! registry fetch overlaps the command's own work (plus a short grace period)
//! so it adds no meaningful latency. All failures are silent — an update
//! notice is never worth breaking a command for.

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

use crate::config::Config;

const CHECK_INTERVAL_SECS: u64 = 60 * 60 * 24;
const REGISTRY_URL: &str = "https://registry.npmjs.org/flowleap/latest";
const FETCH_TIMEOUT: Duration = Duration::from_millis(1500);
/// Post-command wait ceiling for the spawned check; kept just above
/// FETCH_TIMEOUT so the task resolves rather than being cancelled mid-write.
pub const CHECK_GRACE: Duration = Duration::from_millis(1700);

#[derive(Debug, Default, Serialize, Deserialize)]
struct CheckState {
    #[serde(default)]
    last_checked_unix: u64,
    #[serde(default)]
    latest: String,
}

fn state_path() -> Option<PathBuf> {
    Config::config_dir()
        .ok()
        .map(|dir| dir.join("update-check.toml"))
}

fn load_state() -> CheckState {
    state_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| toml::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_state(state: &CheckState) {
    if let (Some(path), Ok(text)) = (state_path(), toml::to_string(state)) {
        let _ = std::fs::write(path, text);
    }
}

/// Strictly-newer x.y.z compare, tolerant of a leading `v` and trailing
/// pre-release/build suffixes on the patch component.
pub fn is_newer(latest: &str, current: &str) -> bool {
    fn parse(v: &str) -> Option<(u64, u64, u64)> {
        let mut parts = v.trim().trim_start_matches('v').splitn(3, '.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch_digits: String = parts
            .next()?
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        Some((major, minor, patch_digits.parse().ok()?))
    }
    match (parse(latest), parse(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

/// How this binary was installed, which decides the upgrade command shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallChannel {
    /// Installed via the npm wrapper package (`npm i -g flowleap`).
    Npm,
    /// Installed standalone (install.sh, cargo install, manual download).
    Standalone,
}

/// Heuristic channel detection from the running binary's path. The npm
/// wrapper downloads the native binary as `flowleap-native[.exe]`, either
/// inside the package's `node_modules/.../bin` dir or in the per-user
/// wrapper cache — so the file name or a `node_modules` path component
/// both mean npm; anything else is a standalone install.
fn detect_channel(exe: &Path) -> InstallChannel {
    let npm_wrapper_name = exe
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem == "flowleap-native");
    let in_node_modules = exe
        .components()
        .any(|component| component.as_os_str() == "node_modules");
    if npm_wrapper_name || in_node_modules {
        InstallChannel::Npm
    } else {
        InstallChannel::Standalone
    }
}

fn upgrade_command(channel: InstallChannel) -> &'static str {
    match channel {
        InstallChannel::Npm => "npm i -g flowleap@latest",
        InstallChannel::Standalone => {
            "curl -fsSL https://raw.githubusercontent.com/abdullahatrash/flowleap-cli/main/install.sh | sh"
        }
    }
}

fn notice(latest: &str, current: &str, channel: InstallChannel) -> Option<String> {
    if latest.is_empty() || !is_newer(latest, current) {
        return None;
    }
    let upgrade = upgrade_command(channel);
    Some(format!(
        "flowleap {latest} is available (you have {current}). Update: {upgrade}"
    ))
}

/// Spawn the update check, or None when gated off. The handle resolves to
/// the stderr notice to print after the command finishes, if any.
pub fn spawn_check(
    http: &reqwest::Client,
    json: bool,
    dry_run: bool,
) -> Option<JoinHandle<Option<String>>> {
    if json
        || dry_run
        || std::env::var_os("FLOWLEAP_NO_UPDATE_CHECK").is_some()
        || !std::io::stderr().is_terminal()
    {
        return None;
    }
    let http = http.clone();
    Some(tokio::spawn(check(http)))
}

async fn check(http: reqwest::Client) -> Option<String> {
    let current = env!("CARGO_PKG_VERSION");
    let channel = std::env::current_exe()
        .map(|exe| detect_channel(&exe))
        .unwrap_or(InstallChannel::Standalone);
    let mut state = load_state();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();

    // Inside the daily window: answer from the cached state, no network.
    if now.saturating_sub(state.last_checked_unix) < CHECK_INTERVAL_SECS {
        return notice(&state.latest, current, channel);
    }

    let resp = http
        .get(REGISTRY_URL)
        .timeout(FETCH_TIMEOUT)
        .send()
        .await
        .ok()?;
    let body: serde_json::Value = resp.json().await.ok()?;
    state.latest = body.get("version")?.as_str()?.to_string();
    state.last_checked_unix = now;
    save_state(&state);
    notice(&state.latest, current, channel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_detection() {
        assert!(is_newer("0.2.5", "0.2.4"));
        assert!(is_newer("0.3.0", "0.2.9"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(is_newer("v0.2.5", "0.2.4"));
        assert!(is_newer("0.2.5-rc.1", "0.2.4"));
        assert!(!is_newer("0.2.4", "0.2.4"));
        assert!(!is_newer("0.2.3", "0.2.4"));
        assert!(!is_newer("garbage", "0.2.4"));
        assert!(!is_newer("", "0.2.4"));
    }

    #[test]
    fn notice_only_when_newer() {
        assert!(notice("0.2.5", "0.2.4", InstallChannel::Standalone).is_some());
        assert!(notice("0.2.4", "0.2.4", InstallChannel::Standalone).is_none());
        assert!(notice("", "0.2.4", InstallChannel::Standalone).is_none());
    }

    #[test]
    fn channel_detection() {
        let cases: Vec<(&str, InstallChannel)> = vec![
            // npm: wrapper cache dir (binary is named flowleap-native)
            (
                "/home/u/.cache/flowleap/v0.3.0/flowleap-native",
                InstallChannel::Npm,
            ),
            // npm: binary inside the global package's bin dir
            (
                "/usr/local/lib/node_modules/flowleap/bin/flowleap-native",
                InstallChannel::Npm,
            ),
            // npm: node_modules path component even with an unexpected name
            (
                "/opt/node/node_modules/flowleap/bin/flowleap",
                InstallChannel::Npm,
            ),
            // npm: .exe suffix is stripped by file_stem (forward slashes so
            // the case parses on every host; Windows accepts both separators)
            (
                "C:/Users/u/.cache/flowleap/v0.3.0/flowleap-native.exe",
                InstallChannel::Npm,
            ),
            // standalone: install.sh default location
            ("/usr/local/bin/flowleap", InstallChannel::Standalone),
            // standalone: cargo install
            ("/home/u/.cargo/bin/flowleap", InstallChannel::Standalone),
        ];
        let detected: Vec<(&str, InstallChannel)> = cases
            .iter()
            .map(|(path, _)| (*path, detect_channel(Path::new(path))))
            .collect();
        assert_eq!(detected, cases);
    }

    #[test]
    fn notice_matches_channel() {
        let npm = notice("0.2.5", "0.2.4", InstallChannel::Npm).unwrap();
        assert!(npm.contains("npm i -g flowleap@latest"));
        let standalone = notice("0.2.5", "0.2.4", InstallChannel::Standalone).unwrap();
        assert!(standalone.contains("install.sh | sh"));
    }
}
