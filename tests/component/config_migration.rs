//! Config Schema Migration Tests
//!
//! Validates that V1 (old top-level layout) configs are correctly migrated
//! to V2 (providers.models) layout via V1Compat deserialization.

use zeroclaw::config::migration::{self, V1Compat, CURRENT_SCHEMA_VERSION};

fn migrate(toml_str: &str) -> zeroclaw::config::Config {
    let compat: V1Compat = toml::from_str(toml_str).expect("failed to deserialize");
    compat.into_config()
}

// ─────────────────────────────────────────────────────────────────────────────
// Provider field migration
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn v1_top_level_fields_migrate_to_providers() {
    let config = migrate(r#"
api_key = "sk-test-123"
api_url = "https://api.example.com"
api_path = "/v2/generate"
default_provider = "openrouter"
default_model = "anthropic/claude-sonnet-4-6"
default_temperature = 0.9
provider_timeout_secs = 300
provider_max_tokens = 4096
"#);

    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(config.providers.fallback.as_deref(), Some("openrouter"));

    let entry = &config.providers.models["openrouter"];
    assert_eq!(entry.api_key.as_deref(), Some("sk-test-123"));
    assert_eq!(entry.base_url.as_deref(), Some("https://api.example.com"));
    assert_eq!(entry.api_path.as_deref(), Some("/v2/generate"));
    assert_eq!(entry.model.as_deref(), Some("anthropic/claude-sonnet-4-6"));
    assert_eq!(entry.temperature, Some(0.9));
    assert_eq!(entry.timeout_secs, Some(300));
    assert_eq!(entry.max_tokens, Some(4096));
}

#[test]
fn v1_extra_headers_migrate_to_fallback_entry() {
    let config = migrate(r#"
default_provider = "openrouter"

[extra_headers]
X-Title = "zeroclaw"
"#);

    let entry = &config.providers.models["openrouter"];
    assert_eq!(entry.extra_headers.get("X-Title").map(|s| s.as_str()), Some("zeroclaw"));
}

#[test]
fn v1_model_providers_migrate_to_providers_models() {
    let config = migrate(r#"
default_provider = "openrouter"

[model_providers.ollama]
base_url = "http://localhost:11434"
"#);

    assert!(config.providers.models.contains_key("ollama"));
    assert_eq!(
        config.providers.models["ollama"].base_url.as_deref(),
        Some("http://localhost:11434")
    );
}

#[test]
fn v1_top_level_fields_merge_with_existing_model_providers_entry() {
    let config = migrate(r#"
api_key = "sk-test"
default_provider = "openrouter"

[model_providers.openrouter]
base_url = "https://openrouter.ai/api"
"#);

    let entry = &config.providers.models["openrouter"];
    assert_eq!(entry.api_key.as_deref(), Some("sk-test"));
    assert_eq!(entry.base_url.as_deref(), Some("https://openrouter.ai/api"));
}

#[test]
fn v1_top_level_fields_do_not_overwrite_existing_entry_values() {
    let config = migrate(r#"
api_key = "sk-top-level"
default_provider = "openrouter"

[model_providers.openrouter]
api_key = "sk-from-profile"
"#);

    // Profile value takes precedence — top-level only fills gaps.
    let entry = &config.providers.models["openrouter"];
    assert_eq!(entry.api_key.as_deref(), Some("sk-from-profile"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Resolved cache
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn resolved_cache_populated_after_migration() {
    let config = migrate(r#"
api_key = "sk-test"
default_provider = "openrouter"
default_model = "claude"
default_temperature = 0.5
provider_timeout_secs = 60
"#);

    assert_eq!(config.api_key.as_deref(), Some("sk-test"));
    assert_eq!(config.default_provider.as_deref(), Some("openrouter"));
    assert_eq!(config.default_model.as_deref(), Some("claude"));
    assert!((config.default_temperature - 0.5).abs() < f64::EPSILON);
    assert_eq!(config.provider_timeout_secs, 60);
}

#[test]
fn resolved_cache_populated_for_v2_config() {
    let config = migrate(r#"
schema_version = 2

[providers]
fallback = "anthropic"

[providers.models.anthropic]
api_key = "sk-ant"
model = "claude-opus"
temperature = 0.3
"#);

    assert_eq!(config.api_key.as_deref(), Some("sk-ant"));
    assert_eq!(config.default_provider.as_deref(), Some("anthropic"));
    assert_eq!(config.default_model.as_deref(), Some("claude-opus"));
    assert!((config.default_temperature - 0.3).abs() < f64::EPSILON);
}

// ─────────────────────────────────────────────────────────────────────────────
// Matrix room_id migration
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn room_id_migrated_to_allowed_rooms() {
    let config = migrate(r#"
[channels_config.matrix]
homeserver = "https://matrix.org"
access_token = "tok"
room_id = "!abc:matrix.org"
allowed_users = ["@user:matrix.org"]
"#);

    let matrix = config.channels_config.matrix.as_ref().unwrap();
    assert!(matrix.room_id.is_none());
    assert_eq!(matrix.allowed_rooms, vec!["!abc:matrix.org"]);
}

#[test]
fn room_id_deduped_with_existing_allowed_rooms() {
    let config = migrate(r#"
[channels_config.matrix]
homeserver = "https://matrix.org"
access_token = "tok"
room_id = "!abc:matrix.org"
allowed_users = ["@user:matrix.org"]
allowed_rooms = ["!abc:matrix.org", "!other:matrix.org"]
"#);

    let matrix = config.channels_config.matrix.as_ref().unwrap();
    assert_eq!(matrix.allowed_rooms.len(), 2);
}

// ─────────────────────────────────────────────────────────────────────────────
// Edge cases
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn already_v2_config_unchanged() {
    let config = migrate(r#"
schema_version = 2

[providers]
fallback = "openrouter"

[providers.models.openrouter]
api_key = "sk-test"
model = "claude"
"#);

    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(config.providers.fallback.as_deref(), Some("openrouter"));
    assert_eq!(config.providers.models["openrouter"].api_key.as_deref(), Some("sk-test"));
}

#[test]
fn no_default_provider_uses_fallback_name_default() {
    let config = migrate(r#"
api_key = "sk-orphan"
"#);

    assert_eq!(config.providers.fallback.as_deref(), Some("default"));
    assert_eq!(config.providers.models["default"].api_key.as_deref(), Some("sk-orphan"));
}

#[test]
fn empty_config_produces_valid_v2() {
    let config = migrate("");
    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);
}

#[test]
fn model_provider_alias_works() {
    let config = migrate(r#"
model_provider = "ollama"
"#);

    assert_eq!(config.providers.fallback.as_deref(), Some("ollama"));
}

#[test]
fn migrate_file_preserves_comments() {
    let raw = r#"
# Global settings
schema_version = 0

api_key = "sk-test"          # my API key
default_provider = "openrouter"

# Agent tuning
[agent]
max_tool_iterations = 5  # keep it tight

# Matrix channel
[channels_config.matrix]
homeserver = "https://matrix.org"  # production server
access_token = "tok"
room_id = "!abc:matrix.org"
allowed_users = ["@user:matrix.org"]
"#;
    let migrated = migration::migrate_file(raw).unwrap().expect("should migrate");

    // Comments on unchanged sections are preserved.
    assert!(migrated.contains("# Agent tuning"), "section comment preserved");
    assert!(migrated.contains("# keep it tight"), "inline comment preserved");
    assert!(migrated.contains("# production server"), "matrix inline comment preserved");

    // Old top-level keys are gone, new structure is present.
    // (api_key now lives inside [providers.models.*], not at the top level)
    let lines: Vec<&str> = migrated.lines().collect();
    let top_level_keys: Vec<&str> = lines.iter()
        .take_while(|l| !l.starts_with('['))
        .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
        .copied()
        .collect();
    assert!(!top_level_keys.iter().any(|l| l.starts_with("api_key")), "old api_key removed from top level");
    assert!(!top_level_keys.iter().any(|l| l.starts_with("default_provider")), "old default_provider removed from top level");
    assert!(migrated.contains("[providers"), "providers section added");
    assert!(!migrated.contains("room_id"), "room_id removed");
}

#[test]
fn migrate_file_returns_none_when_current() {
    let raw = r#"
schema_version = 2

[providers]
fallback = "openrouter"

[providers.models.openrouter]
api_key = "sk-test"
"#;
    assert!(migration::migrate_file(raw).unwrap().is_none());
}

#[test]
fn migration_works_with_toml_comments() {
    let config = migrate(r#"
# Primary provider config
api_key = "sk-test"          # my API key
default_provider = "openrouter"  # main provider
default_model = "claude"     # preferred model

# Matrix channel
[channels_config.matrix]
homeserver = "https://matrix.org"  # production server
access_token = "tok"
room_id = "!abc:matrix.org"  # main room
allowed_users = ["@user:matrix.org"]
# enabled intentionally omitted
"#);

    // Provider fields migrated correctly despite comments
    let entry = &config.providers.models["openrouter"];
    assert_eq!(entry.api_key.as_deref(), Some("sk-test"));
    assert_eq!(entry.model.as_deref(), Some("claude"));

    // Matrix room_id migrated despite inline comment
    let matrix = config.channels_config.matrix.as_ref().unwrap();
    assert!(matrix.room_id.is_none());
    assert_eq!(matrix.allowed_rooms, vec!["!abc:matrix.org"]);
}

// ─────────────────────────────────────────────────────────────────────────────
// Exhaustive migration walk
// ─────────────────────────────────────────────────────────────────────────────

/// Verifies that migrating a V0 config with every migrated field populated
/// produces a Config where no prop was lost. Uses `prop_fields()` to compare
/// the migrated config against a natively-constructed V2 Config with the same values.
#[test]
fn exhaustive_walk_no_props_lost() {
    use zeroclaw::config::{Config, ModelProviderConfig};

    // Build a V0 config with every migrated field set to a distinct value.
    let v0 = migrate(r#"
api_key = "walk-key"
api_url = "https://walk.example.com"
api_path = "/walk/path"
default_provider = "walk-provider"
default_model = "walk-model"
default_temperature = 1.11
provider_timeout_secs = 222
provider_max_tokens = 333

[extra_headers]
X-Walk = "walk-header"

[model_providers.other-profile]
base_url = "https://other.example.com"
name = "other"

[channels_config.matrix]
homeserver = "https://walk-matrix.org"
access_token = "walk-token"
room_id = "!walk:matrix.org"
allowed_users = ["@walk:matrix.org"]
allowed_rooms = ["!existing:matrix.org"]
"#);

    // Build the equivalent V2 config natively.
    let mut expected = Config::default();
    expected.providers.fallback = Some("walk-provider".into());
    let mut entry = ModelProviderConfig::default();
    entry.api_key = Some("walk-key".into());
    entry.base_url = Some("https://walk.example.com".into());
    entry.api_path = Some("/walk/path".into());
    entry.model = Some("walk-model".into());
    entry.temperature = Some(1.11);
    entry.timeout_secs = Some(222);
    entry.max_tokens = Some(333);
    entry.extra_headers.insert("X-Walk".into(), "walk-header".into());
    expected.providers.models.insert("walk-provider".into(), entry);
    let mut other = ModelProviderConfig::default();
    other.base_url = Some("https://other.example.com".into());
    other.name = Some("other".into());
    expected.providers.models.insert("other-profile".into(), other);
    expected.resolve_provider_cache();

    // Compare provider fields.
    assert_eq!(v0.providers.fallback, expected.providers.fallback);
    assert_eq!(v0.providers.models.len(), expected.providers.models.len());
    for (key, v0_entry) in &v0.providers.models {
        let exp_entry = expected.providers.models.get(key)
            .unwrap_or_else(|| panic!("missing provider entry: {key}"));
        assert_eq!(v0_entry.api_key, exp_entry.api_key, "api_key mismatch for {key}");
        assert_eq!(v0_entry.base_url, exp_entry.base_url, "base_url mismatch for {key}");
        assert_eq!(v0_entry.api_path, exp_entry.api_path, "api_path mismatch for {key}");
        assert_eq!(v0_entry.model, exp_entry.model, "model mismatch for {key}");
        assert_eq!(v0_entry.temperature, exp_entry.temperature, "temperature mismatch for {key}");
        assert_eq!(v0_entry.timeout_secs, exp_entry.timeout_secs, "timeout_secs mismatch for {key}");
        assert_eq!(v0_entry.max_tokens, exp_entry.max_tokens, "max_tokens mismatch for {key}");
        assert_eq!(v0_entry.extra_headers, exp_entry.extra_headers, "extra_headers mismatch for {key}");
        assert_eq!(v0_entry.name, exp_entry.name, "name mismatch for {key}");
    }

    // Compare resolved cache.
    assert_eq!(v0.api_key, expected.api_key);
    assert_eq!(v0.api_url, expected.api_url);
    assert_eq!(v0.api_path, expected.api_path);
    assert_eq!(v0.default_provider, expected.default_provider);
    assert_eq!(v0.default_model, expected.default_model);
    assert!((v0.default_temperature - expected.default_temperature).abs() < f64::EPSILON);
    assert_eq!(v0.provider_timeout_secs, expected.provider_timeout_secs);
    assert_eq!(v0.provider_max_tokens, expected.provider_max_tokens);
    assert_eq!(v0.extra_headers, expected.extra_headers);

    // Verify matrix room_id was merged into allowed_rooms.
    let v0_matrix = v0.channels_config.matrix.as_ref().unwrap();
    assert!(v0_matrix.room_id.is_none(), "room_id should be cleared");
    assert!(v0_matrix.allowed_rooms.contains(&"!walk:matrix.org".to_string()));
    assert!(v0_matrix.allowed_rooms.contains(&"!existing:matrix.org".to_string()));

    // Use prop_fields() to verify no prop was lost: every non-secret prop
    // that has a value on the expected config should also have a value on
    // the migrated config.
    let v0_props = v0.prop_fields();
    let expected_props = expected.prop_fields();
    for exp in &expected_props {
        if exp.is_secret || exp.display_value == "<unset>" {
            continue;
        }
        let found = v0_props.iter().find(|p| p.name == exp.name)
            .unwrap_or_else(|| panic!("prop {} missing after migration", exp.name));
        assert_eq!(
            found.display_value, exp.display_value,
            "prop {} value mismatch: got {:?}, expected {:?}",
            exp.name, found.display_value, exp.display_value
        );
    }
}

/// Verifies migrate_file output round-trips back to the same Config.
#[test]
fn migrate_file_round_trips() {
    let raw = r#"
api_key = "rt-key"
default_provider = "openrouter"
default_model = "claude"
default_temperature = 0.5
provider_timeout_secs = 60

[model_providers.ollama]
base_url = "http://localhost:11434"

[channels_config.matrix]
homeserver = "https://matrix.org"
access_token = "tok"
room_id = "!rt:matrix.org"
allowed_users = ["@u:m"]
"#;
    let migrated_toml = migration::migrate_file(raw).unwrap().expect("should migrate");

    // The migrated file should deserialize into a valid V2 config.
    let config = migrate(&migrated_toml);
    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(config.providers.fallback.as_deref(), Some("openrouter"));
    assert_eq!(config.providers.models["openrouter"].api_key.as_deref(), Some("rt-key"));
    assert_eq!(config.providers.models["openrouter"].model.as_deref(), Some("claude"));
    assert!(config.providers.models.contains_key("ollama"));

    let matrix = config.channels_config.matrix.as_ref().unwrap();
    assert!(matrix.room_id.is_none());
    assert!(matrix.allowed_rooms.contains(&"!rt:matrix.org".to_string()));

    // Re-migrating should be a no-op.
    assert!(migration::migrate_file(&migrated_toml).unwrap().is_none());
}
