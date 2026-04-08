//! Forward-only config schema migration.
//!
//! Old config layouts are typed structs. Migration deserializes into the legacy
//! struct, moves field values into the new layout, and returns a clean [`Config`].
//!
//! The on-disk file is never rewritten by migration.
//!
//! ## When to bump the schema version
//!
//! Only when props are **renamed, moved, or removed**. New props with `#[serde(default)]`
//! don't need a bump.

use serde::Deserialize;
use std::collections::HashMap;

use super::providers::ProvidersConfig;
use super::schema::ModelProviderConfig;

pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// Wraps the current Config with extra fields from V1 that no longer exist on Config.
/// `#[serde(flatten)]` lets Config consume its known fields; the old fields are
/// captured here.
#[derive(Deserialize)]
pub struct V1Compat {
    #[serde(flatten)]
    pub config: super::schema::Config,

    // ── Old top-level provider fields (removed in V2) ──
    #[serde(default)]
    api_key: Option<String>,
    #[serde(default)]
    api_url: Option<String>,
    #[serde(default)]
    api_path: Option<String>,
    #[serde(default, alias = "model_provider")]
    default_provider: Option<String>,
    #[serde(default, alias = "model")]
    default_model: Option<String>,
    #[serde(default)]
    model_providers: HashMap<String, ModelProviderConfig>,
    #[serde(default)]
    default_temperature: Option<f64>,
    #[serde(default)]
    provider_timeout_secs: Option<u64>,
    #[serde(default)]
    provider_max_tokens: Option<u32>,
    #[serde(default)]
    extra_headers: Option<HashMap<String, String>>,
}

impl V1Compat {
    /// Consume self, migrating old fields into the current Config layout.
    pub fn into_config(mut self) -> super::schema::Config {
        let from = self.config.schema_version;
        let needs_migration = from < CURRENT_SCHEMA_VERSION || self.has_legacy_fields();

        if !needs_migration {
            self.config.resolve_provider_cache();
            return self.config;
        }

        self.migrate_providers();
        self.migrate_matrix_room_id();
        self.config.schema_version = CURRENT_SCHEMA_VERSION;
        self.config.resolve_provider_cache();

        tracing::info!(
            from = from,
            to = CURRENT_SCHEMA_VERSION,
            "Config schema migrated in-memory from version {from} to {CURRENT_SCHEMA_VERSION}. \
             Run `zeroclaw props migrate` to update the file on disk.",
        );

        self.config
    }

    fn has_legacy_fields(&self) -> bool {
        self.api_key.is_some()
            || self.api_url.is_some()
            || self.api_path.is_some()
            || self.default_provider.is_some()
            || self.default_model.is_some()
            || !self.model_providers.is_empty()
            || self.default_temperature.is_some()
            || self.provider_timeout_secs.is_some()
            || self.provider_max_tokens.is_some()
            || self.extra_headers.as_ref().is_some_and(|h| !h.is_empty())
    }

    fn migrate_providers(&mut self) {
        let fallback = self
            .default_provider
            .take()
            .unwrap_or_else(|| "default".into());

        // First, move old model_providers entries into providers.models.
        // These take precedence over top-level fields (more specific).
        for (key, profile) in std::mem::take(&mut self.model_providers) {
            self.config.providers.models.entry(key).or_insert(profile);
        }

        // Then fill gaps in the fallback entry from top-level fields.
        let entry = self
            .config
            .providers
            .models
            .entry(fallback.clone())
            .or_default();

        if entry.api_key.is_none() {
            entry.api_key = self.api_key.take();
        }
        if entry.base_url.is_none() {
            entry.base_url = self.api_url.take();
        }
        if entry.api_path.is_none() {
            entry.api_path = self.api_path.take();
        }
        if entry.model.is_none() {
            entry.model = self.default_model.take();
        }
        if entry.temperature.is_none() {
            entry.temperature = self.default_temperature.take();
        }
        if entry.timeout_secs.is_none() {
            entry.timeout_secs = self.provider_timeout_secs.take();
        }
        if entry.max_tokens.is_none() {
            entry.max_tokens = self.provider_max_tokens.take();
        }
        if entry.extra_headers.is_empty() {
            if let Some(headers) = self.extra_headers.take() {
                entry.extra_headers = headers;
            }
        }

        if self.config.providers.fallback.is_none() {
            self.config.providers.fallback = Some(fallback);
        }
    }

    fn migrate_matrix_room_id(&mut self) {
        if let Some(ref mut matrix) = self.config.channels_config.matrix {
            if let Some(room_id) = matrix.room_id.take() {
                if !room_id.is_empty() && !matrix.allowed_rooms.contains(&room_id) {
                    matrix.allowed_rooms.push(room_id);
                }
            }
        }
    }
}
