//! Current resolved configuration endpoint.

use std::sync::Arc;

use axum::extract::State;
use axum::Json;

use crate::config::model::Config;
use crate::server::AppState;

pub async fn configprops_handler(State(state): State<Arc<AppState>>) -> Json<Config> {
    let config = Arc::clone(&state.config.read().await.config);
    Json((*config).clone())
}
