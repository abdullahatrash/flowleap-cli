use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default = "default_base_url")]
    pub base_url: String,
    pub api_key: Option<String>,
    pub token: Option<String>,
    pub default_model: Option<String>,
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
        Ok(Self::config_dir()?.join("config.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            let config: Config = serde_json::from_str(&contents)?;
            Ok(config)
        } else {
            Ok(Config {
                base_url: default_base_url(),
                ..Default::default()
            })
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }

    /// Get the authorization header value, preferring token > api_key
    pub fn auth_header(&self) -> Option<String> {
        if let Some(ref token) = self.token {
            Some(format!("Bearer {}", token))
        } else if let Some(ref key) = self.api_key {
            Some(format!("Bearer {}", key))
        } else {
            None
        }
    }
}
