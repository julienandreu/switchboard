//! `GET /health` endpoint handler.
//!
//! Returns a [`HealthResponse`] JSON payload containing the server
//! version, uptime, config source metadata, loaded route/target counts,
//! and cumulative request statistics.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::server::AppState;

#[derive(Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub config: ConfigHealth,
    pub stats: StatsResponse,
}

#[derive(Serialize, Deserialize)]
pub struct ConfigHealth {
    pub source: String,
    pub version: String,
    pub loaded_ago_seconds: u64,
    pub namespace: String,
    pub routes: usize,
    pub targets: usize,
}

#[derive(Serialize, Deserialize)]
pub struct StatsResponse {
    pub requests_forwarded: u64,
    pub requests_failed: u64,
}

pub async fn health_handler(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    // Clone Arc<Config> (cheap refcount bump) to release the lock quickly
    let (config, source_name, version_str, loaded_ago) = {
        let loaded = state.config.read().await;
        let config = Arc::clone(&loaded.config);
        let version_str = match &loaded.version {
            crate::config::ConfigVersion::Hash(h) => h.get(..8).unwrap_or(h).to_string(),
        };
        (
            config,
            loaded.source_name.clone(),
            version_str,
            loaded.loaded_at.elapsed().as_secs(),
        )
    };

    let total_targets = config.total_targets();

    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
        config: ConfigHealth {
            source: source_name,
            version: version_str,
            loaded_ago_seconds: loaded_ago,
            namespace: state.namespace.clone(),
            routes: config.routes.len(),
            targets: total_targets,
        },
        stats: StatsResponse {
            requests_forwarded: state.stats.forwarded.load(Ordering::Relaxed),
            requests_failed: state.stats.failed.load(Ordering::Relaxed),
        },
    })
}
