//! Route mappings endpoint showing all configured routes.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::server::AppState;

#[derive(Serialize)]
pub struct MappingsResponse {
    pub contexts: MappingContext,
}

#[derive(Serialize)]
pub struct MappingContext {
    pub switchboard: SwitchboardMappings,
}

#[derive(Serialize)]
pub struct SwitchboardMappings {
    pub mappings: Vec<RouteMapping>,
}

#[derive(Serialize)]
pub struct RouteMapping {
    pub path: String,
    pub methods: Vec<String>,
    pub timeout_ms: u64,
    pub targets: Vec<TargetMapping>,
    pub headers: HeaderMapping,
}

#[derive(Serialize)]
pub struct TargetMapping {
    pub url: String,
    pub primary: bool,
    pub timeout_ms: Option<u64>,
}

#[derive(Serialize)]
pub struct HeaderMapping {
    pub add: HashMap<String, String>,
    pub strip: Vec<String>,
}

pub async fn mappings_handler(State(state): State<Arc<AppState>>) -> Json<MappingsResponse> {
    let config = Arc::clone(&state.config.read().await.config);

    let default_timeout = config.defaults.timeout;

    let mappings = config
        .routes
        .iter()
        .map(|route| RouteMapping {
            path: route.path.clone(),
            methods: route.methods.clone(),
            timeout_ms: route.timeout.unwrap_or(default_timeout),
            targets: route
                .targets
                .iter()
                .map(|t| TargetMapping {
                    url: t.url.clone(),
                    primary: t.primary,
                    timeout_ms: t.timeout,
                })
                .collect(),
            headers: HeaderMapping {
                add: route.headers.add.clone(),
                strip: route.headers.strip.clone(),
            },
        })
        .collect();

    Json(MappingsResponse {
        contexts: MappingContext {
            switchboard: SwitchboardMappings { mappings },
        },
    })
}
