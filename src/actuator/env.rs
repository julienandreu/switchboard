//! Environment variables endpoint with secret masking.

use std::collections::BTreeMap;

use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct EnvResponse {
    #[serde(rename = "activeProfiles")]
    pub active_profiles: Vec<String>,
    #[serde(rename = "propertySources")]
    pub property_sources: Vec<PropertySource>,
}

#[derive(Serialize)]
pub struct PropertySource {
    pub name: String,
    pub properties: BTreeMap<String, PropertyValue>,
}

#[derive(Serialize)]
pub struct PropertyValue {
    pub value: String,
}

const SENSITIVE_PATTERNS: &[&str] = &["PASSWORD", "SECRET", "TOKEN", "KEY", "DSN", "CREDENTIALS"];

fn is_sensitive(key: &str) -> bool {
    let upper = key.to_uppercase();
    SENSITIVE_PATTERNS.iter().any(|pat| upper.contains(pat))
}

pub async fn env_handler() -> Json<EnvResponse> {
    let mut properties = BTreeMap::new();

    for (key, value) in std::env::vars() {
        let masked = if is_sensitive(&key) {
            "******".to_string()
        } else {
            value
        };
        properties.insert(key, PropertyValue { value: masked });
    }

    Json(EnvResponse {
        active_profiles: vec![],
        property_sources: vec![PropertySource {
            name: "systemEnvironment".to_string(),
            properties,
        }],
    })
}
