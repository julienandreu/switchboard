//! Generic async file-based config source with SHA256 change detection.
//!
//! [`FileSource`] implements [`ConfigSource`]
//! for any file format by accepting a deserialization function at
//! construction time. It reads the file asynchronously via Tokio,
//! validates the result, and computes a SHA256 hash for version tracking.

use std::path::PathBuf;

use async_trait::async_trait;

use super::sha256_hex;
use crate::config::model::Config;
use crate::config::validation::validate;
use crate::config::{ConfigSource, ConfigVersion};
use crate::error::SwitchboardError;

pub struct FileSource {
    path: PathBuf,
    name: &'static str,
    deserialize: fn(&str) -> Result<Config, Box<dyn std::error::Error + Send + Sync>>,
}

impl FileSource {
    #[must_use]
    pub fn new(
        path: PathBuf,
        name: &'static str,
        deserialize: fn(&str) -> Result<Config, Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self {
            path,
            name,
            deserialize,
        }
    }

    async fn read_content(&self) -> Result<String, SwitchboardError> {
        tokio::fs::read_to_string(&self.path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SwitchboardError::ConfigFileNotFound {
                    path: self.path.clone(),
                }
            } else {
                SwitchboardError::Io(e)
            }
        })
    }
}

#[async_trait]
impl ConfigSource for FileSource {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn load(&self) -> Result<(Config, ConfigVersion), SwitchboardError> {
        let content = self.read_content().await?;

        let config = (self.deserialize)(&content).map_err(|e| SwitchboardError::ConfigParse {
            path: self.path.display().to_string(),
            source: e,
        })?;

        if let Err(errors) = validate(&config) {
            return Err(SwitchboardError::ConfigValidation { errors });
        }

        let hash = sha256_hex(content.as_bytes());
        Ok((config, ConfigVersion::Hash(hash)))
    }

    async fn has_changed(&self, current: &ConfigVersion) -> Result<bool, SwitchboardError> {
        let content = self.read_content().await?;
        let hash = sha256_hex(content.as_bytes());
        Ok(*current != ConfigVersion::Hash(hash))
    }
}
