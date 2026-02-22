//! Concrete [`ConfigSource`](super::ConfigSource) implementations.
//!
//! Provides file-based sources (YAML, JSON, TOML) gated by feature flags,
//! database backend stubs (Redis, `DynamoDB`, `PostgreSQL`, `MongoDB`, `SQLite`),
//! and the [`parse_config_str`] helper for format-specific deserialization.

pub mod file_source;

#[cfg(feature = "yaml")]
pub mod yaml;

#[cfg(feature = "json")]
pub mod json;

#[cfg(feature = "toml")]
pub mod toml_source;

#[cfg(feature = "dynamodb")]
pub mod dynamodb;

#[cfg(feature = "redis")]
pub mod redis_source;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "mongodb")]
pub mod mongodb_source;

#[cfg(feature = "sqlite")]
pub mod sqlite;

use sha2::{Digest, Sha256};

use crate::config::model::Config;
use crate::config::validation::validate;
use crate::config::ConfigVersion;
use crate::error::SwitchboardError;

/// Parse a config string based on file extension.
pub fn parse_config_str(
    ext: &str,
    content: &str,
    path_display: &str,
) -> Result<Config, SwitchboardError> {
    match ext {
        #[cfg(feature = "yaml")]
        "yaml" | "yml" => serde_yml::from_str(content).map_err(|e| SwitchboardError::ConfigParse {
            path: path_display.to_string(),
            source: Box::new(e),
        }),

        #[cfg(feature = "json")]
        "json" => serde_json::from_str(content).map_err(|e| SwitchboardError::ConfigParse {
            path: path_display.to_string(),
            source: Box::new(e),
        }),

        #[cfg(feature = "toml")]
        "toml" => toml::from_str(content).map_err(|e| SwitchboardError::ConfigParse {
            path: path_display.to_string(),
            source: Box::new(e),
        }),

        other => Err(SwitchboardError::UnsupportedFormat(other.to_string())),
    }
}

/// Compute a lowercase hex-encoded SHA-256 digest.
#[must_use]
pub fn sha256_hex(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

/// Deserialize JSON into [`Config`], validate, and compute a SHA-256 version hash.
///
/// Shared by all database config sources to avoid duplicating the
/// parse-validate-hash pipeline.
pub fn parse_validate_hash(
    json: &str,
    source_label: &str,
) -> Result<(Config, ConfigVersion), SwitchboardError> {
    let config: Config = serde_json::from_str(json).map_err(|e| SwitchboardError::ConfigParse {
        path: source_label.to_string(),
        source: Box::new(e),
    })?;

    if let Err(errors) = validate(&config) {
        return Err(SwitchboardError::ConfigValidation { errors });
    }

    let hash = sha256_hex(json.as_bytes());
    Ok((config, ConfigVersion::Hash(hash)))
}
