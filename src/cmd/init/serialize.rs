//! Serialize a [`Config`] struct to the chosen output format.

use crate::cli::ConfigFormat;
use crate::config::model::Config;
use crate::error::SwitchboardError;

/// Serialize a `Config` to a formatted string in the given format.
pub fn serialize_config(
    config: &Config,
    format: &ConfigFormat,
) -> Result<String, SwitchboardError> {
    match format {
        #[cfg(feature = "yaml")]
        ConfigFormat::Yaml => serde_yml::to_string(config)
            .map_err(|e| SwitchboardError::Io(std::io::Error::other(e.to_string()))),

        #[cfg(not(feature = "yaml"))]
        ConfigFormat::Yaml => Err(SwitchboardError::UnsupportedFormat("yaml".into())),

        ConfigFormat::Json => serde_json::to_string_pretty(config)
            .map_err(|e| SwitchboardError::Io(std::io::Error::other(e.to_string()))),

        #[cfg(feature = "toml")]
        ConfigFormat::Toml => toml::to_string_pretty(config)
            .map_err(|e| SwitchboardError::Io(std::io::Error::other(e.to_string()))),

        #[cfg(not(feature = "toml"))]
        ConfigFormat::Toml => Err(SwitchboardError::UnsupportedFormat("toml".into())),
    }
}
