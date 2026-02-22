//! Structured logging setup using the `tracing` ecosystem.
//!
//! Configures a `tracing-subscriber` with either JSON output (for
//! production) or pretty-printed output (for TTY / local dev). Format
//! is auto-detected from the terminal but can be forced via `--json`
//! or `--pretty`.

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::cli::LogLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Pretty,
}

#[must_use]
pub fn resolve_format(pretty: bool, json: bool) -> LogFormat {
    if json {
        return LogFormat::Json;
    }
    if pretty {
        return LogFormat::Pretty;
    }
    if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        LogFormat::Pretty
    } else {
        LogFormat::Json
    }
}

pub fn init(level: &LogLevel, format: LogFormat) {
    let tracing_level = level.to_tracing_level();
    let filter = tracing_subscriber::filter::Targets::new().with_default(tracing_level);

    match format {
        LogFormat::Json => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().json().with_target(false))
                .init();
        }
        LogFormat::Pretty => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().pretty())
                .init();
        }
    }
}
