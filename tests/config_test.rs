use std::fs;
use tempfile::TempDir;

/// Test config TOML serialization/deserialization roundtrip
#[test]
fn test_config_toml_roundtrip() {
    let toml_content = r#"
base_url = "https://custom.api.example.com"
website_url = "https://custom.example.com"
default_model = "patent-claude-sonnet"
output_format = "json"
"#;

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct Config {
        #[serde(default)]
        base_url: String,
        #[serde(default)]
        website_url: String,
        default_model: Option<String>,
        output_format: Option<String>,
    }

    let config: Config = toml::from_str(toml_content).unwrap();
    assert_eq!(config.base_url, "https://custom.api.example.com");
    assert_eq!(config.website_url, "https://custom.example.com");
    assert_eq!(
        config.default_model.as_deref(),
        Some("patent-claude-sonnet")
    );
    assert_eq!(config.output_format.as_deref(), Some("json"));

    // Roundtrip
    let serialized = toml::to_string_pretty(&config).unwrap();
    let deserialized: Config = toml::from_str(&serialized).unwrap();
    assert_eq!(config, deserialized);
}

/// Test config with missing optional fields
#[test]
fn test_config_toml_defaults() {
    let toml_content = r#"
base_url = "https://api.flowleap.co"
"#;

    #[derive(Debug, serde::Deserialize)]
    struct Config {
        base_url: String,
        #[serde(default = "default_website_url")]
        website_url: String,
        default_model: Option<String>,
        output_format: Option<String>,
    }

    fn default_website_url() -> String {
        "https://flowleap.co".to_string()
    }

    let config: Config = toml::from_str(toml_content).unwrap();
    assert_eq!(config.base_url, "https://api.flowleap.co");
    assert_eq!(config.website_url, "https://flowleap.co");
    assert!(config.default_model.is_none());
    assert!(config.output_format.is_none());
}

/// Test empty config file
#[test]
fn test_config_toml_empty() {
    #[derive(Debug, serde::Deserialize, Default)]
    struct Config {
        #[serde(default = "default_url")]
        base_url: String,
        default_model: Option<String>,
    }

    fn default_url() -> String {
        "https://api.flowleap.co".to_string()
    }

    let config: Config = toml::from_str("").unwrap();
    assert_eq!(config.base_url, "https://api.flowleap.co");
    assert!(config.default_model.is_none());
}

/// Test credentials TOML roundtrip
#[test]
fn test_credentials_toml_roundtrip() {
    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Default)]
    struct Credentials {
        api_key: Option<String>,
        token: Option<String>,
        refresh_token: Option<String>,
    }

    let creds = Credentials {
        api_key: Some("sk-test-key-123".to_string()),
        token: Some("eyJhbGciOiJIUzI1NiJ9.test".to_string()),
        refresh_token: Some("refresh-tok-456".to_string()),
    };

    let serialized = toml::to_string_pretty(&creds).unwrap();
    let deserialized: Credentials = toml::from_str(&serialized).unwrap();
    assert_eq!(creds, deserialized);
}

/// Test credentials auth header precedence (token > api_key)
#[test]
fn test_credentials_auth_header_precedence() {
    #[derive(Default)]
    struct Credentials {
        api_key: Option<String>,
        token: Option<String>,
    }

    impl Credentials {
        fn auth_header(&self) -> Option<String> {
            self.token
                .as_ref()
                .or(self.api_key.as_ref())
                .map(|v| format!("Bearer {}", v))
        }
    }

    // Token takes precedence
    let creds = Credentials {
        api_key: Some("api-key".to_string()),
        token: Some("my-token".to_string()),
    };
    assert_eq!(creds.auth_header(), Some("Bearer my-token".to_string()));

    // Falls back to api_key
    let creds = Credentials {
        api_key: Some("api-key".to_string()),
        token: None,
    };
    assert_eq!(creds.auth_header(), Some("Bearer api-key".to_string()));

    // No auth
    let creds = Credentials {
        api_key: None,
        token: None,
    };
    assert_eq!(creds.auth_header(), None);
}

/// Test config file write and read from disk
#[test]
fn test_config_file_persistence() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.toml");

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct Config {
        base_url: String,
        default_model: Option<String>,
    }

    let config = Config {
        base_url: "https://test.example.com".to_string(),
        default_model: Some("gpt-4".to_string()),
    };

    let contents = toml::to_string_pretty(&config).unwrap();
    fs::write(&path, &contents).unwrap();

    let read_contents = fs::read_to_string(&path).unwrap();
    let loaded: Config = toml::from_str(&read_contents).unwrap();
    assert_eq!(config, loaded);
}

/// Test credentials file write and read from disk
#[test]
fn test_credentials_file_persistence() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("credentials.toml");

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Default)]
    struct Credentials {
        api_key: Option<String>,
        token: Option<String>,
        refresh_token: Option<String>,
    }

    let creds = Credentials {
        api_key: None,
        token: Some("test-token".to_string()),
        refresh_token: None,
    };

    let contents = toml::to_string_pretty(&creds).unwrap();
    fs::write(&path, &contents).unwrap();

    let read_contents = fs::read_to_string(&path).unwrap();
    let loaded: Credentials = toml::from_str(&read_contents).unwrap();
    assert_eq!(creds, loaded);
}
