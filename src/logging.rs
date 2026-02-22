//! Structured logging setup using the `tracing` ecosystem.
//!
//! Configures a `tracing-subscriber` with either JSON output (for
//! production) or pretty-printed output (for TTY / local dev). Format
//! is auto-detected from the terminal but can be forced via `--json`
//! or `--pretty`.
//!
//! When the `actuator` feature is enabled, returns a
//! [`LogReloadHandle`](crate::server::LogReloadHandle) for runtime
//! log level changes.

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
        LogFormat::Json
    } else if pretty || std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        LogFormat::Pretty
    } else {
        LogFormat::Json
    }
}

#[cfg(feature = "actuator")]
pub fn init(level: &LogLevel, format: LogFormat) -> crate::server::LogReloadHandle {
    use tracing_subscriber::filter::Targets;
    use tracing_subscriber::reload;

    let tracing_level = level.to_tracing_level();
    let filter = Targets::new().with_default(tracing_level);
    let (reload_filter, reload_handle) = reload::Layer::new(filter);

    match format {
        LogFormat::Json => {
            tracing_subscriber::registry()
                .with(reload_filter)
                .with(fmt::layer().json().with_target(false))
                .init();
        }
        LogFormat::Pretty => {
            tracing_subscriber::registry()
                .with(reload_filter)
                .with(fmt::layer().pretty())
                .init();
        }
    }

    reload_handle
}

#[cfg(not(feature = "actuator"))]
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
