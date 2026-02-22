//! PostgreSQL-backed [`ConfigSource`] implementation.
//!
//! Stores and retrieves Switchboard configuration from a `switchboard_config`
//! table keyed by namespace. The table is auto-created on first connection.
//! Change detection uses SHA-256 hashing of the raw JSON payload.

use async_trait::async_trait;
use sqlx::PgPool;

use super::{parse_validate_hash, sha256_hex};
use crate::config::{ConfigSource, ConfigVersion};
use crate::error::SwitchboardError;

pub struct PostgresSource {
    pool: PgPool,
    namespace: String,
}

impl PostgresSource {
    pub async fn new(url: &str, namespace: &str) -> Result<Self, SwitchboardError> {
        let pool = PgPool::connect(url)
            .await
            .map_err(|e| SwitchboardError::Database {
                backend: "postgres",
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
            backend: "postgres",
            source: Box::new(e),
        })?;

        Ok(Self {
            pool,
            namespace: namespace.to_string(),
        })
    }

    async fn fetch_config_json(&self) -> Result<String, SwitchboardError> {
        sqlx::query_scalar::<_, String>(
            "SELECT config_json FROM switchboard_config WHERE namespace = $1",
        )
        .bind(&self.namespace)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| SwitchboardError::Database {
            backend: "postgres",
            source: Box::new(e),
        })
    }
}

#[async_trait]
impl ConfigSource for PostgresSource {
    fn name(&self) -> &'static str {
        "postgres"
    }

    async fn load(
        &self,
    ) -> Result<(crate::config::model::Config, ConfigVersion), SwitchboardError> {
        let json = self.fetch_config_json().await?;
        parse_validate_hash(&json, &format!("postgres::{}", self.namespace))
    }

    async fn has_changed(&self, current: &ConfigVersion) -> Result<bool, SwitchboardError> {
        let json = self.fetch_config_json().await?;
        Ok(*current != ConfigVersion::Hash(sha256_hex(json.as_bytes())))
    }
}
