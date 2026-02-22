//! SQLite-backed [`ConfigSource`] implementation.
//!
//! Stores the Switchboard configuration as a JSON blob in a local `SQLite`
//! database, keyed by namespace. The table `switchboard_config` is
//! auto-created on first connection. Change detection uses SHA-256
//! hashing of the raw `config_json` column value.

use std::path::Path;

use async_trait::async_trait;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;

use super::{parse_validate_hash, sha256_hex};
use crate::config::{ConfigSource, ConfigVersion};
use crate::error::SwitchboardError;

pub struct SqliteSource {
    pool: SqlitePool,
    namespace: String,
}

impl SqliteSource {
    pub async fn new(path: &Path, namespace: &str) -> Result<Self, SwitchboardError> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);

        let pool =
            SqlitePool::connect_with(options)
                .await
                .map_err(|e| SwitchboardError::Database {
                    backend: "sqlite",
                    source: Box::new(e),
                })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS switchboard_config (\
                namespace TEXT PRIMARY KEY, \
                config_json TEXT NOT NULL\
            )",
        )
        .execute(&pool)
        .await
        .map_err(|e| SwitchboardError::Database {
            backend: "sqlite",
            source: Box::new(e),
        })?;

        Ok(Self {
            pool,
            namespace: namespace.to_string(),
        })
    }

    async fn fetch_config_json(&self) -> Result<String, SwitchboardError> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT config_json FROM switchboard_config WHERE namespace = ?")
                .bind(&self.namespace)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| SwitchboardError::Database {
                    backend: "sqlite",
                    source: Box::new(e),
                })?;

        row.map(|(json,)| json)
            .ok_or_else(|| SwitchboardError::Database {
                backend: "sqlite",
                source: format!("no config row found for namespace '{}'", self.namespace).into(),
            })
    }
}

#[async_trait]
impl ConfigSource for SqliteSource {
    fn name(&self) -> &'static str {
        "sqlite"
    }

    async fn load(
        &self,
    ) -> Result<(crate::config::model::Config, ConfigVersion), SwitchboardError> {
        let json = self.fetch_config_json().await?;
        parse_validate_hash(&json, &format!("sqlite::{}", self.namespace))
    }

    async fn has_changed(&self, current: &ConfigVersion) -> Result<bool, SwitchboardError> {
        let json = self.fetch_config_json().await?;
        Ok(*current != ConfigVersion::Hash(sha256_hex(json.as_bytes())))
    }
}
