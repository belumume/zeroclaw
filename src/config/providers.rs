use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zeroclaw_macros::Configurable;

use super::schema::ModelProviderConfig;

/// Top-level `[providers]` section. Wraps model provider profiles and an optional
/// fallback reference.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Configurable, Default)]
#[prefix = "providers"]
pub struct ProvidersConfig {
    /// Key of the provider entry to use when no route matches.
    /// Optional — if unset, requests without a matching route fail at runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<String>,

    /// Named model provider profiles keyed by id.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[nested]
    pub models: HashMap<String, ModelProviderConfig>,
}
