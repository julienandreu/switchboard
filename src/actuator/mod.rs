//! Spring Boot-style actuator endpoints for operational monitoring.
//!
//! Provides health checks, build info, metrics, configuration inspection,
//! environment variables, route mappings, and runtime log level management
//! under the `/actuator` prefix.

mod configprops;
mod env;
mod health;
mod info;
mod loggers;
mod mappings;
mod metrics;

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::server::AppState;

#[derive(Serialize)]
struct ActuatorIndex {
    #[serde(rename = "_links")]
    links: BTreeMap<String, ActuatorLink>,
}

#[derive(Serialize)]
struct ActuatorLink {
    href: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    templated: Option<bool>,
}

/// Build the actuator sub-router (nested under `/actuator`).
pub fn actuator_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(index_handler))
        .route("/health", get(health::health_handler))
        .route("/health/liveness", get(health::liveness_handler))
        .route("/health/readiness", get(health::readiness_handler))
        .route("/info", get(info::info_handler))
        .route("/env", get(env::env_handler))
        .route("/metrics", get(metrics::metrics_index_handler))
        .route("/metrics/{name}", get(metrics::metric_detail_handler))
        .route("/configprops", get(configprops::configprops_handler))
        .route("/mappings", get(mappings::mappings_handler))
        .route(
            "/loggers",
            get(loggers::get_loggers_handler).post(loggers::set_loggers_handler),
        )
}

/// Middleware that returns 404 when actuator is disabled in config.
/// Respects hot-reload — toggling `actuator.enabled` takes effect immediately.
pub async fn actuator_enabled_guard(
    State(state): State<Arc<AppState>>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let enabled = state.config.read().await.config.actuator.enabled;
    if enabled {
        next.run(request).await
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

/// Middleware that enforces HTTP Basic Auth when credentials are configured.
/// When `actuator.auth.username` and `actuator.auth.password` are both set,
/// requests must include a valid `Authorization: Basic` header.
/// When no auth is configured, all requests pass through.
pub async fn basic_auth_guard(
    State(state): State<Arc<AppState>>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let config = state.config.read().await;
    let auth = &config.config.actuator.auth;

    let (expected_user, expected_pass) = match (&auth.username, &auth.password) {
        (Some(u), Some(p)) => (u.clone(), p.clone()),
        _ => return next.run(request).await,
    };
    drop(config);

    let unauthorized = || {
        (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Basic realm=\"switchboard\"")],
        )
            .into_response()
    };

    let header_value = match request.headers().get(header::AUTHORIZATION) {
        Some(v) => v,
        None => return unauthorized(),
    };

    let header_str = match header_value.to_str() {
        Ok(s) => s,
        Err(_) => return unauthorized(),
    };

    let encoded = match header_str.strip_prefix("Basic ") {
        Some(e) => e,
        None => return unauthorized(),
    };

    let decoded = match base64_decode(encoded) {
        Some(d) => d,
        None => return unauthorized(),
    };

    let (user, pass) = match decoded.split_once(':') {
        Some(pair) => pair,
        None => return unauthorized(),
    };

    if user == expected_user && pass == expected_pass {
        next.run(request).await
    } else {
        unauthorized()
    }
}

/// Minimal base64 decoder for Basic auth (RFC 7617).
/// Avoids pulling in the `base64` crate for a single use.
fn base64_decode(input: &str) -> Option<String> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let input = input.trim_end_matches('=');
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    let mut out = Vec::with_capacity(input.len() * 3 / 4);

    for byte in input.bytes() {
        let val = TABLE.iter().position(|&b| b == byte)? as u32;
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }

    String::from_utf8(out).ok()
}

async fn index_handler() -> Json<ActuatorIndex> {
    let endpoints = [
        ("self", "/actuator", false),
        ("health", "/actuator/health", false),
        ("health-liveness", "/actuator/health/liveness", false),
        ("health-readiness", "/actuator/health/readiness", false),
        ("info", "/actuator/info", false),
        ("env", "/actuator/env", false),
        ("metrics", "/actuator/metrics", false),
        ("metrics-name", "/actuator/metrics/{name}", true),
        ("configprops", "/actuator/configprops", false),
        ("mappings", "/actuator/mappings", false),
        ("loggers", "/actuator/loggers", false),
    ];

    let links = endpoints
        .into_iter()
        .map(|(name, href, templated)| {
            (
                name.to_string(),
                ActuatorLink {
                    href: href.to_string(),
                    templated: templated.then_some(true),
                },
            )
        })
        .collect();

    Json(ActuatorIndex { links })
}
