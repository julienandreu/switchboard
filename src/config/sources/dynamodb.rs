//! `DynamoDB`-based [`ConfigSource`] implementation.
//!
//! Stores the configuration as a JSON blob in a `DynamoDB` table, keyed by
//! namespace. The table schema requires a partition key named `namespace`
//! (String) and a `config_json` attribute (String) containing the serialized
//! [`Config`](crate::config::model::Config).
//!
//! # CLI arguments
//!
//! | Flag                 | Env var            | Default       |
//! |----------------------|--------------------|---------------|
//! | `--dynamodb-table`   | `DYNAMODB_TABLE`   | *(required)*  |
//! | `--dynamodb-region`  | `DYNAMODB_REGION`  | `us-east-1`   |

use async_trait::async_trait;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client;

use super::{parse_validate_hash, sha256_hex};
use crate::config::{ConfigSource, ConfigVersion};
use crate::error::SwitchboardError;

pub struct DynamoDbSource {
    client: Client,
    table: String,
    namespace: String,
}

impl DynamoDbSource {
    pub async fn new(table: &str, region: &str, namespace: &str) -> Result<Self, SwitchboardError> {
        let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        let client = Client::new(&sdk_config);

        Ok(Self {
            client,
            table: table.to_string(),
            namespace: namespace.to_string(),
        })
    }

    async fn fetch_config_json(&self) -> Result<String, SwitchboardError> {
        let output = self
            .client
            .get_item()
            .table_name(&self.table)
            .key("namespace", AttributeValue::S(self.namespace.clone()))
            .send()
            .await
            .map_err(|e| SwitchboardError::Database {
                backend: "dynamodb",
                source: Box::new(e),
            })?;

        let item = output.item.ok_or_else(|| SwitchboardError::Database {
            backend: "dynamodb",
            source: format!(
                "no item found for namespace '{}' in table '{}'",
                self.namespace, self.table
            )
            .into(),
        })?;

        let attr = item
            .get("config_json")
            .ok_or_else(|| SwitchboardError::Database {
                backend: "dynamodb",
                source: format!(
                    "item for namespace '{}' is missing the 'config_json' attribute",
                    self.namespace
                )
                .into(),
            })?;

        attr.as_s().map_or_else(
            |_| {
                Err(SwitchboardError::Database {
                    backend: "dynamodb",
                    source: format!(
                        "'config_json' attribute for namespace '{}' is not a String",
                        self.namespace
                    )
                    .into(),
                })
            },
            |json| Ok(json.clone()),
        )
    }
}

#[async_trait]
impl ConfigSource for DynamoDbSource {
    fn name(&self) -> &'static str {
        "dynamodb"
    }

    async fn load(
        &self,
    ) -> Result<(crate::config::model::Config, ConfigVersion), SwitchboardError> {
        let json = self.fetch_config_json().await?;
        parse_validate_hash(
            &json,
            &format!("dynamodb://{}:{}", self.table, self.namespace),
        )
    }

    async fn has_changed(&self, current: &ConfigVersion) -> Result<bool, SwitchboardError> {
        let json = self.fetch_config_json().await?;
        Ok(*current != ConfigVersion::Hash(sha256_hex(json.as_bytes())))
    }
}
