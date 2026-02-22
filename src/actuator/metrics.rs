//! Metrics index and individual metric endpoints.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use crate::server::AppState;

#[derive(Serialize)]
pub struct MetricsIndexResponse {
    pub names: Vec<&'static str>,
}

const METRIC_NAMES: &[&str] = &[
    "requests.forwarded",
    "requests.failed",
    "requests.active",
    "config.reloads",
    "uptime.seconds",
];

pub async fn metrics_index_handler() -> Json<MetricsIndexResponse> {
    Json(MetricsIndexResponse {
        names: METRIC_NAMES.to_vec(),
    })
}

#[derive(Serialize)]
pub struct MetricDetailResponse {
    pub name: String,
    pub measurement: MetricMeasurement,
}

#[derive(Serialize)]
pub struct MetricMeasurement {
    pub statistic: String,
    pub value: f64,
}

pub async fn metric_detail_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<MetricDetailResponse>, StatusCode> {
    let (statistic, value) = match name.as_str() {
        "requests.forwarded" => (
            "COUNT",
            state.stats.forwarded.load(Ordering::Relaxed) as f64,
        ),
        "requests.failed" => ("COUNT", state.stats.failed.load(Ordering::Relaxed) as f64),
        "requests.active" => (
            "VALUE",
            state.stats.active_requests.load(Ordering::Relaxed) as f64,
        ),
        "config.reloads" => (
            "COUNT",
            state.stats.config_reloads.load(Ordering::Relaxed) as f64,
        ),
        "uptime.seconds" => ("VALUE", state.start_time.elapsed().as_secs_f64()),
        _ => return Err(StatusCode::NOT_FOUND),
    };

    Ok(Json(MetricDetailResponse {
        name,
        measurement: MetricMeasurement {
            statistic: statistic.to_string(),
            value,
        },
    }))
}
