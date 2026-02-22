//! MongoDB-backed [`ConfigSource`] implementation.
//!
//! Reads configuration from a `MongoDB` collection. The expected document
//! schema in the `switchboard.switchboard_config` collection is:
//!
//! ```json
//! { "namespace": "default", "config_json": "{...}" }
//! ```

use async_trait::async_trait;
use mongodb::bson::{doc, Document};
use mongodb::{Client, Collection};

use super::{parse_validate_hash, sha256_hex};
use crate::config::{ConfigSource, ConfigVersion};
use crate::error::SwitchboardError;

const DATABASE_NAME: &str = "switchboard";
const COLLECTION_NAME: &str = "switchboard_config";

pub struct MongoDbSource {
    collection: Collection<Document>,
    namespace: String,
}

impl MongoDbSource {
    pub async fn new(url: &str, namespace: &str) -> Result<Self, SwitchboardError> {
        let client = Client::with_uri_str(url)
            .await
            .map_err(|e| SwitchboardError::Database {
                backend: "mongodb",
                source: Box::new(e),
            })?;

        client
            .database(DATABASE_NAME)
            .run_command(doc! { "ping": 1 })
            .await
            .map_err(|e| SwitchboardError::Database {
                backend: "mongodb",
                source: Box::new(e),
            })?;

        let collection = client
            .database(DATABASE_NAME)
            .collection::<Document>(COLLECTION_NAME);

        Ok(Self {
            collection,
            namespace: namespace.to_owned(),
        })
    }

    async fn fetch_config_json(&self) -> Result<String, SwitchboardError> {
        let filter = doc! { "namespace": &self.namespace };

        let document = self
            .collection
            .find_one(filter)
            .await
            .map_err(|e| SwitchboardError::Database {
                backend: "mongodb",
                source: Box::new(e),
            })?
            .ok_or_else(|| SwitchboardError::Database {
                backend: "mongodb",
                source: format!("no document found for namespace '{}'", self.namespace).into(),
            })?;

        document
            .get_str("config_json")
            .map(std::borrow::ToOwned::to_owned)
            .map_err(|e| SwitchboardError::Database {
                backend: "mongodb",
                source: Box::new(e),
            })
    }
}

#[async_trait]
impl ConfigSource for MongoDbSource {
    fn name(&self) -> &'static str {
        "mongodb"
    }

    async fn load(
        &self,
    ) -> Result<(crate::config::model::Config, ConfigVersion), SwitchboardError> {
        let json = self.fetch_config_json().await?;
        parse_validate_hash(
            &json,
            &format!(
                "mongodb://.../{DATABASE_NAME}/{COLLECTION_NAME}?namespace={}",
                self.namespace
            ),
        )
    }

    async fn has_changed(&self, current: &ConfigVersion) -> Result<bool, SwitchboardError> {
        let json = self.fetch_config_json().await?;
        Ok(*current != ConfigVersion::Hash(sha256_hex(json.as_bytes())))
    }
}
