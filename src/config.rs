use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application configuration stored in ~/.config/flowleap/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_website_url")]
    pub website_url: String,
    pub default_model: Option<String>,
    pub output_format: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            website_url: default_website_url(),
            default_model: None,
            output_format: None,
        }
    }
}

/// Credentials stored separately in ~/.config/flowleap/credentials.toml
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Credentials {
    pub api_key: Option<String>,
    pub token: Option<String>,
    pub refresh_token: Option<String>,
}

fn default_base_url() -> String {
    "https://api.flowleap.co".to_string()
}

fn default_website_url() -> String {
    "https://www.flowleap.co".to_string()
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
        fs::write(path, contents)?;
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
        self.refresh_token = None;
    }
}
