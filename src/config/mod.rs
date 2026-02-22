//! Configuration loading, validation, and hot-reloading.
//!
//! Defines the [`ConfigSource`] trait for pluggable config backends,
//! the [`ConfigResolver`] for primary/fallback source resolution, and
//! the [`ConfigVersion`] enum for change detection. Submodules provide
//! the data model, validation logic, and concrete source implementations.

pub mod model;
pub mod sources;
pub mod validation;

use async_trait::async_trait;

use crate::error::SwitchboardError;
use model::Config;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConfigVersion {
    Hash(String),
}

// async_trait is required here because ConfigSource is used as Box<dyn ConfigSource>
// and native async fn in traits (Rust 1.75+) does not support dyn dispatch.
#[async_trait]
pub trait ConfigSource: Send + Sync {
    fn name(&self) -> &'static str;
    async fn load(&self) -> Result<(Config, ConfigVersion), SwitchboardError>;
    async fn has_changed(&self, current: &ConfigVersion) -> Result<bool, SwitchboardError>;
}

pub struct ConfigResolver {
    primary: Box<dyn ConfigSource>,
    fallback: Option<Box<dyn ConfigSource>>,
}

impl ConfigResolver {
    #[must_use]
    pub fn new(primary: Box<dyn ConfigSource>, fallback: Option<Box<dyn ConfigSource>>) -> Self {
        Self { primary, fallback }
    }

    pub async fn load_with_fallback(&self) -> Result<(Config, ConfigVersion), SwitchboardError> {
        match self.primary.load().await {
            Ok(result) => Ok(result),
            Err(primary_err) => {
                if let Some(ref fallback) = self.fallback {
                    tracing::warn!(
                        primary = self.primary.name(),
                        fallback = fallback.name(),
                        error = %primary_err,
                        "primary config source failed, using fallback"
                    );
                    fallback.load().await
                } else {
                    Err(primary_err)
                }
            }
        }
    }

    #[must_use]
    pub fn primary_name(&self) -> &str {
        self.primary.name()
    }

    #[must_use]
    pub fn primary(&self) -> &dyn ConfigSource {
        &*self.primary
    }
}
