//! Enhanced health endpoints with Kubernetes liveness/readiness probes.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use crate::server::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub components: HealthComponents,
}

#[derive(Serialize)]
pub struct HealthComponents {
    pub liveness: ComponentHealth,
    pub readiness: ComponentHealth,
}

#[derive(Serialize)]
pub struct ComponentHealth {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

pub async fn health_handler(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let liveness = build_liveness();
    let readiness = build_readiness(&state).await;

    let overall = if liveness.status == "UP" && readiness.status == "UP" {
        "UP"
    } else {
        "DOWN"
    };

    Json(HealthResponse {
        status: overall.to_string(),
        components: HealthComponents {
            liveness,
            readiness,
        },
    })
}

pub async fn liveness_handler() -> (StatusCode, Json<ComponentHealth>) {
    let health = build_liveness();
    (StatusCode::OK, Json(health))
}

pub async fn readiness_handler(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<ComponentHealth>) {
    let health = build_readiness(&state).await;
    let status = if health.status == "UP" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(health))
}

fn build_liveness() -> ComponentHealth {
    ComponentHealth {
        status: "UP".to_string(),
        details: None,
    }
}

async fn build_readiness(state: &AppState) -> ComponentHealth {
    let loaded = state.config.read().await;
    let route_count = loaded.config.routes.len();
    let is_ready = !loaded.config.routes.is_empty();

    ComponentHealth {
        status: if is_ready { "UP" } else { "DOWN" }.to_string(),
        details: Some(serde_json::json!({
            "config_source": loaded.source_name,
            "routes_loaded": route_count,
        })),
    }
}
