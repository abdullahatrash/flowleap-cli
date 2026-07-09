use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application configuration stored in ~/.config/flowleap/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_base_url")]
    pub base_url: String,
    pub default_model: Option<String>,
    pub output_format: Option<String>,
    /// Destinations `flowleap skills install` has written to, so
    /// `flowleap skills update` can refresh them after a CLI upgrade.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skill_installs: Vec<SkillInstall>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            default_model: None,
            output_format: None,
            skill_installs: Vec::new(),
        }
    }
}

/// One recorded `skills install` destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillInstall {
    /// Harness target: claude, claude-project, dir, codex, cursor, or gemini.
    pub target: String,
    /// Skills directory for copy targets; the rendered file otherwise.
    pub path: PathBuf,
    /// CLI version that produced the installed output.
    pub version: String,
    /// Skills selected at install time (empty = all bundled skills).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<String>,
}

/// Credentials stored separately in ~/.config/flowleap/credentials.toml
///
/// Older CLI versions wrote a `refresh_token` field that nothing ever
/// populated; serde ignores unknown fields, so files written by those
/// versions still load fine.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Credentials {
    pub api_key: Option<String>,
    pub token: Option<String>,
    /// BYOK patent-provider credentials, forwarded per-request as headers
    /// (x-epo-ops-key / x-epo-ops-secret / x-uspto-odp-key). EPO key and
    /// secret only work as a pair — the backend rejects half a pair.
    pub epo_key: Option<String>,
    pub epo_secret: Option<String>,
    pub uspto_key: Option<String>,
}

fn default_base_url() -> String {
    "https://api.flowleap.co".to_string()
}

impl Config {
    pub fn config_dir() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("flowleap");
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&contents)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let contents = toml::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }
}

impl Credentials {
    pub fn credentials_path() -> Result<PathBuf> {
        Ok(Config::config_dir()?.join("credentials.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::credentials_path()?;
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            let creds: Credentials = toml::from_str(&contents)?;
            Ok(creds)
        } else {
            Ok(Credentials::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::credentials_path()?;
        let contents = toml::to_string_pretty(self)?;
        // Credentials are secrets: create the file 0600 from the start —
        // write-then-chmod would leave a world-readable window on first save.
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&path)?;
            file.write_all(contents.as_bytes())?;
            // Pre-existing files keep their old mode; tighten those too.
            fs::set_permissions(&path, {
                use std::os::unix::fs::PermissionsExt;
                fs::Permissions::from_mode(0o600)
            })?;
        }
        #[cfg(not(unix))]
        fs::write(&path, contents)?;
        Ok(())
    }

    /// Get the authorization header value, preferring token > api_key
    pub fn auth_header(&self) -> Option<String> {
        self.token
            .as_ref()
            .or(self.api_key.as_ref())
            .map(|v| format!("Bearer {}", v))
    }

    pub fn clear(&mut self) {
        self.api_key = None;
        self.token = None;
        self.epo_key = None;
        self.epo_secret = None;
        self.uspto_key = None;
    }

    /// Clear only the OAuth session token, keeping the API key and BYOK
    /// provider keys. An expired session token would otherwise shadow a
    /// still-valid api_key in auth_header().
    pub fn clear_session(&mut self) {
        self.token = None;
    }

    /// EPO pair, only when complete (the backend rejects half a pair).
    pub fn epo_pair(&self) -> Option<(&str, &str)> {
        match (self.epo_key.as_deref(), self.epo_secret.as_deref()) {
            (Some(key), Some(secret)) => Some((key, secret)),
            _ => None,
        }
    }
}
