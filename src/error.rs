//! Unified error types for Switchboard.
//!
//! Defines [`SwitchboardError`] (the main crate error enum) and
//! [`ValidationError`] for config validation failures. Both use
//! `thiserror` for `Display` and `Error` derives. Error messages
//! include contextual hints to guide the user toward a fix.

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub route: String,
    pub field: String,
    pub message: String,
    pub suggestion: Option<String>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "  route {}: {} — {}",
            self.route, self.field, self.message
        )?;
        if let Some(ref suggestion) = self.suggestion {
            write!(f, " ({suggestion})")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationError {}

fn format_errors(errors: &[ValidationError]) -> String {
    use std::fmt::Write;
    let mut buf = String::new();
    for (i, e) in errors.iter().enumerate() {
        if i > 0 {
            buf.push('\n');
        }
        // write! to String is infallible (only fails on OOM which is unrecoverable)
        let _ = write!(buf, "{e}");
    }
    buf
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SwitchboardError {
    #[error("No config source found.\n\n  {hint}")]
    NoConfigSource { hint: String },

    #[error("Config file not found: {}", path.display())]
    ConfigFileNotFound { path: PathBuf },

    #[error("Config parse error in {path}:\n  {source}")]
    ConfigParse {
        path: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Config validation failed:\n{}", format_errors(.errors))]
    ConfigValidation { errors: Vec<ValidationError> },

    #[error("Unsupported config format: '{0}'")]
    UnsupportedFormat(String),

    #[error("Invalid address: {0}")]
    AddressParse(#[from] std::net::AddrParseError),

    #[error("Invalid URI: {source}")]
    UriParse {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("HTTP request failed: {source}")]
    HttpRequest {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("File already exists: {}", path.display())]
    FileExists { path: PathBuf },

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("Health check failed with status {0}")]
    HealthCheckFailed(hyper::StatusCode),

    #[error("Database error ({backend}): {source}")]
    #[cfg(any(
        feature = "dynamodb",
        feature = "redis",
        feature = "postgres",
        feature = "mongodb",
        feature = "sqlite"
    ))]
    Database {
        backend: &'static str,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}
