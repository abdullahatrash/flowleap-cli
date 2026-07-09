//! Integration tests for the real Config and Credentials types exported
//! from the `flowleap_cli` crate, so schema drift (adding/removing/renaming
//! fields in src/config.rs) triggers test failures instead of going silent.

use std::fs;
use tempfile::TempDir;

use flowleap_cli::config::{Config, Credentials};

/// Test config TOML serialization/deserialization roundtrip
#[test]
fn test_config_toml_roundtrip() {
    let toml_content = r#"
base_url = "https://custom.api.example.com"
default_model = "patent-claude-sonnet"
output_format = "json"
"#;

    let config: Config = toml::from_str(toml_content).unwrap();
    assert_eq!(config.base_url, "https://custom.api.example.com");
    assert_eq!(
        config.default_model.as_deref(),
        Some("patent-claude-sonnet")
    );
    assert_eq!(config.output_format.as_deref(), Some("json"));

    // Roundtrip
    let serialized = toml::to_string_pretty(&config).unwrap();
    let deserialized: Config = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.base_url, config.base_url);
    assert_eq!(deserialized.default_model, config.default_model);
    assert_eq!(deserialized.output_format, config.output_format);
}

/// Test config with missing optional fields
#[test]
fn test_config_toml_defaults() {
    let toml_content = r#"
base_url = "https://api.flowleap.co"
"#;

    let config: Config = toml::from_str(toml_content).unwrap();
    assert_eq!(config.base_url, "https://api.flowleap.co");
    assert!(config.default_model.is_none());
    assert!(config.output_format.is_none());
}

/// Test empty config file uses the #[serde(default)] base_url
#[test]
fn test_config_toml_empty() {
    let config: Config = toml::from_str("").unwrap();
    assert_eq!(config.base_url, "https://api.flowleap.co");
    assert!(config.default_model.is_none());
    assert!(config.output_format.is_none());
    assert!(config.skill_installs.is_empty());
}

/// Recorded skill installs survive a TOML roundtrip, and configs written
/// before the field existed still parse (serde default).
#[test]
fn test_config_skill_installs_roundtrip() {
    let toml_content = r#"
base_url = "https://api.flowleap.co"

[[skill_installs]]
target = "codex"
path = "/work/project/AGENTS.md"
version = "0.2.5"

[[skill_installs]]
target = "dir"
path = "/work/agent-skills"
version = "0.2.4"
skills = ["flowleap", "flowleap-patent"]
"#;

    let config: Config = toml::from_str(toml_content).unwrap();
    assert_eq!(config.skill_installs.len(), 2);
    assert_eq!(config.skill_installs[0].target, "codex");
    assert_eq!(config.skill_installs[0].version, "0.2.5");
    assert!(config.skill_installs[0].skills.is_empty());
    assert_eq!(
        config.skill_installs[1].skills,
        vec!["flowleap".to_string(), "flowleap-patent".to_string()]
    );

    let serialized = toml::to_string_pretty(&config).unwrap();
    let deserialized: Config = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.skill_installs, config.skill_installs);
}

/// Test credentials TOML roundtrip against the real Credentials type
#[test]
fn test_credentials_toml_roundtrip() {
    let creds = Credentials {
        api_key: Some("sk-test-key-123".to_string()),
        token: Some("eyJhbGciOiJIUzI1NiJ9.test".to_string()),
        refresh_token: Some("refresh-tok-456".to_string()),
        ..Default::default()
    };

    let serialized = toml::to_string_pretty(&creds).unwrap();
    let deserialized: Credentials = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.api_key, creds.api_key);
    assert_eq!(deserialized.token, creds.token);
    assert_eq!(deserialized.refresh_token, creds.refresh_token);
}

/// Test credentials auth header precedence (token > api_key) against real impl
#[test]
fn test_credentials_auth_header_precedence() {
    // Token takes precedence
    let creds = Credentials {
        api_key: Some("api-key".to_string()),
        token: Some("my-token".to_string()),
        refresh_token: None,
        ..Default::default()
    };
    assert_eq!(creds.auth_header(), Some("Bearer my-token".to_string()));

    // Falls back to api_key
    let creds = Credentials {
        api_key: Some("api-key".to_string()),
        token: None,
        refresh_token: None,
        ..Default::default()
    };
    assert_eq!(creds.auth_header(), Some("Bearer api-key".to_string()));

    // No auth
    let creds = Credentials::default();
    assert_eq!(creds.auth_header(), None);
}

/// Test clearing credentials zeroes all three fields
#[test]
fn test_credentials_clear() {
    let mut creds = Credentials {
        api_key: Some("key".to_string()),
        token: Some("tok".to_string()),
        refresh_token: Some("refresh".to_string()),
        ..Default::default()
    };
    creds.clear();
    assert!(creds.api_key.is_none());
    assert!(creds.token.is_none());
    assert!(creds.refresh_token.is_none());
}

/// Test that clear_session removes only the OAuth session, so a stored
/// api_key becomes the auth_header credential again
#[test]
fn test_credentials_clear_session() {
    let mut creds = Credentials {
        api_key: Some("fl_pat_key".to_string()),
        token: Some("expired-jwt".to_string()),
        refresh_token: Some("refresh".to_string()),
        epo_key: Some("epo".to_string()),
        epo_secret: Some("secret".to_string()),
        uspto_key: Some("uspto".to_string()),
    };
    creds.clear_session();
    assert!(creds.token.is_none());
    assert!(creds.refresh_token.is_none());
    assert_eq!(creds.api_key.as_deref(), Some("fl_pat_key"));
    assert_eq!(creds.epo_key.as_deref(), Some("epo"));
    assert_eq!(creds.epo_secret.as_deref(), Some("secret"));
    assert_eq!(creds.uspto_key.as_deref(), Some("uspto"));
    assert_eq!(creds.auth_header(), Some("Bearer fl_pat_key".to_string()));
}

/// Test that a manually-written Config TOML can be parsed and round-tripped
/// on disk. Uses a tempdir for the raw file; doesn't exercise Config::save()/
/// Config::load() because those target ~/.config/flowleap and aren't
/// path-injectable today.
#[test]
fn test_config_file_persistence() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.toml");

    let config = Config {
        base_url: "https://test.example.com".to_string(),
        default_model: Some("gpt-4".to_string()),
        output_format: Some("table".to_string()),
        ..Default::default()
    };

    let contents = toml::to_string_pretty(&config).unwrap();
    fs::write(&path, &contents).unwrap();

    let read_contents = fs::read_to_string(&path).unwrap();
    let loaded: Config = toml::from_str(&read_contents).unwrap();
    assert_eq!(loaded.base_url, config.base_url);
    assert_eq!(loaded.default_model, config.default_model);
    assert_eq!(loaded.output_format, config.output_format);
}

/// Test credentials file write and read from disk via the real type
#[test]
fn test_credentials_file_persistence() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("credentials.toml");

    let creds = Credentials {
        api_key: None,
        token: Some("test-token".to_string()),
        refresh_token: None,
        ..Default::default()
    };

    let contents = toml::to_string_pretty(&creds).unwrap();
    fs::write(&path, &contents).unwrap();

    let read_contents = fs::read_to_string(&path).unwrap();
    let loaded: Credentials = toml::from_str(&read_contents).unwrap();
    assert_eq!(loaded.api_key, creds.api_key);
    assert_eq!(loaded.token, creds.token);
    assert_eq!(loaded.refresh_token, creds.refresh_token);
}

/// Provider keys round-trip through TOML and the EPO pair is all-or-nothing
#[test]
fn test_provider_keys_roundtrip_and_pairing() {
    let creds = Credentials {
        epo_key: Some("consumer-key".to_string()),
        epo_secret: Some("consumer-secret".to_string()),
        uspto_key: Some("odp-key".to_string()),
        ..Default::default()
    };

    let contents = toml::to_string_pretty(&creds).unwrap();
    let parsed: Credentials = toml::from_str(&contents).unwrap();
    assert_eq!(parsed.epo_pair(), Some(("consumer-key", "consumer-secret")));
    assert_eq!(parsed.uspto_key.as_deref(), Some("odp-key"));

    // Half a pair is not a pair — the backend rejects one-without-the-other.
    let half = Credentials {
        epo_key: Some("only-key".to_string()),
        ..Default::default()
    };
    assert_eq!(half.epo_pair(), None);

    // clear() wipes provider keys too.
    let mut full = creds;
    full.clear();
    assert!(full.epo_key.is_none());
    assert!(full.epo_secret.is_none());
    assert!(full.uspto_key.is_none());
}

/// Old credentials files without provider-key fields still parse
#[test]
fn test_credentials_backwards_compatible_parse() {
    let legacy = "api_key = \"fl_pat_abc\"\n";
    let parsed: Credentials = toml::from_str(legacy).unwrap();
    assert_eq!(parsed.api_key.as_deref(), Some("fl_pat_abc"));
    assert_eq!(parsed.epo_pair(), None);
    assert!(parsed.uspto_key.is_none());
}

/// The 401 → api_key fallback only applies to a session token that came from
/// the credential store, alongside a stored api_key. An explicit --token /
/// FLOWLEAP_TOKEN (token_overridden) disables it.
#[test]
fn test_auth_fallback_key_gating() {
    use flowleap_cli::client::Context;

    let ctx = |token: Option<&str>, api_key: Option<&str>, overridden: bool| Context {
        config: Config::default(),
        credentials: Credentials {
            token: token.map(String::from),
            api_key: api_key.map(String::from),
            ..Default::default()
        },
        output_format: "json".to_string(),
        dry_run: false,
        verbose: false,
        token_overridden: overridden,
        assume_yes: false,
        http: reqwest::Client::new(),
    };

    // Stored session token + stored api_key → fallback available
    assert_eq!(
        ctx(Some("jwt"), Some("fl_pat_x"), false).auth_fallback_key(),
        Some("fl_pat_x")
    );
    // Explicit --token / FLOWLEAP_TOKEN → no fallback
    assert_eq!(
        ctx(Some("jwt"), Some("fl_pat_x"), true).auth_fallback_key(),
        None
    );
    // No session token in play → nothing to fall back from
    assert_eq!(ctx(None, Some("fl_pat_x"), false).auth_fallback_key(), None);
    // No api_key → nothing to fall back to
    assert_eq!(ctx(Some("jwt"), None, false).auth_fallback_key(), None);
}
