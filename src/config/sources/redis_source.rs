//! Redis-backed config source with SHA256 change detection.
//!
//! [`RedisSource`] implements [`ConfigSource`]
//! by storing the Switchboard configuration as a JSON string in Redis
//! under the key `switchboard:{namespace}:config`. It reads the value
//! asynchronously via a multiplexed Tokio connection, deserializes the
//! JSON into a [`Config`](crate::config::model::Config), validates the result, and computes a SHA256
//! hash for version tracking.

use async_trait::async_trait;
use redis::AsyncCommands;
use tokio::sync::Mutex;

use super::{parse_validate_hash, sha256_hex};
use crate::config::{ConfigSource, ConfigVersion};
use crate::error::SwitchboardError;

pub struct RedisSource {
    connection: Mutex<redis::aio::MultiplexedConnection>,
    key: String,
}

impl RedisSource {
    pub async fn new(url: &str, namespace: &str) -> Result<Self, SwitchboardError> {
        let client = redis::Client::open(url).map_err(|e| SwitchboardError::Database {
            backend: "redis",
            source: Box::new(e),
        })?;

        let connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| SwitchboardError::Database {
                backend: "redis",
                source: Box::new(e),
            })?;

        Ok(Self {
            connection: Mutex::new(connection),
            key: format!("switchboard:{namespace}:config"),
        })
    }

    #[allow(clippy::significant_drop_tightening)]
    async fn read_content(&self) -> Result<String, SwitchboardError> {
        let mut conn = self.connection.lock().await;

        let value: Option<String> =
            conn.get(&self.key)
                .await
                .map_err(|e| SwitchboardError::Database {
                    backend: "redis",
                    source: Box::new(e),
                })?;

        value.ok_or_else(|| SwitchboardError::ConfigParse {
            path: self.key.clone(),
            source: format!("key '{}' not found in Redis", self.key).into(),
        })
    }
}

#[async_trait]
impl ConfigSource for RedisSource {
    fn name(&self) -> &'static str {
        "redis"
    }

    async fn load(
        &self,
    ) -> Result<(crate::config::model::Config, ConfigVersion), SwitchboardError> {
        let content = self.read_content().await?;
        parse_validate_hash(&content, &self.key)
    }

    async fn has_changed(&self, current: &ConfigVersion) -> Result<bool, SwitchboardError> {
        let content = self.read_content().await?;
        Ok(*current != ConfigVersion::Hash(sha256_hex(content.as_bytes())))
    }
}
