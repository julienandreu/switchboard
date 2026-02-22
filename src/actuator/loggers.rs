//! Runtime log level inspection and mutation.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing_subscriber::filter::Targets;

use crate::server::AppState;

#[derive(Serialize)]
pub struct LoggersResponse {
    pub levels: Vec<&'static str>,
    pub loggers: LoggerConfig,
}

#[derive(Serialize)]
pub struct LoggerConfig {
    #[serde(rename = "ROOT")]
    pub root: LoggerLevel,
}

#[derive(Serialize)]
pub struct LoggerLevel {
    #[serde(rename = "configuredLevel")]
    pub configured_level: String,
    #[serde(rename = "effectiveLevel")]
    pub effective_level: String,
}

#[derive(Deserialize)]
pub struct SetLoggerRequest {
    #[serde(rename = "configuredLevel")]
    pub configured_level: String,
}

const AVAILABLE_LEVELS: &[&str] = &["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];

pub async fn get_loggers_handler(State(state): State<Arc<AppState>>) -> Json<LoggersResponse> {
    let current_level = state.current_log_level.read().await.clone();

    Json(LoggersResponse {
        levels: AVAILABLE_LEVELS.to_vec(),
        loggers: LoggerConfig {
            root: LoggerLevel {
                effective_level: current_level.clone(),
                configured_level: current_level,
            },
        },
    })
}

pub async fn set_loggers_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetLoggerRequest>,
) -> StatusCode {
    let Some(ref handle) = state.log_reload_handle else {
        return StatusCode::SERVICE_UNAVAILABLE;
    };

    let level_name = body.configured_level.to_uppercase();
    let level = match level_name.as_str() {
        "TRACE" => tracing::Level::TRACE,
        "DEBUG" => tracing::Level::DEBUG,
        "INFO" => tracing::Level::INFO,
        "WARN" => tracing::Level::WARN,
        "ERROR" => tracing::Level::ERROR,
        _ => return StatusCode::BAD_REQUEST,
    };

    let new_filter = Targets::new().with_default(level);

    match handle.reload(new_filter) {
        Ok(()) => {
            tracing::info!(level = %level_name, "log level changed via actuator");
            *state.current_log_level.write().await = level_name;
            StatusCode::OK
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to reload log level");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
