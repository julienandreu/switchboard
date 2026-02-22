//! Core HTTP request forwarding handler.
//!
//! The [`forward_handler`] function is the Axum fallback that receives
//! every non-`/health` request, matches it against configured routes,
//! and delegates to the fan-out engine. Submodules handle route matching
//! ([`routing`]), header construction ([`headers`]), and concurrent
//! target dispatch ([`fanout`]).

pub mod fanout;
pub mod headers;
pub mod routing;

use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};

use crate::server::AppState;

#[allow(clippy::significant_drop_tightening)]
pub async fn forward_handler(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    method: Method,
    uri: Uri,
    req_headers: HeaderMap,
    body: Bytes,
) -> Response {
    let path = uri.path();
    let correlation_id = req_headers
        .get("x-correlation-id")
        .and_then(|v| v.to_str().ok())
        .map_or_else(|| uuid::Uuid::new_v4().to_string(), String::from);

    // Clone the Arc<Config> (cheap refcount bump) to release the RwLock before .await
    let (config, route_idx, params) = {
        let config_guard = state.config.read().await;
        let config = Arc::clone(&config_guard.config);

        let matched = routing::match_route(&config.routes, path, method.as_str());
        let Some((route_idx, params)) = matched else {
            tracing::warn!(
                correlation_id = %correlation_id,
                method = %method,
                path = %path,
                "no route matched"
            );
            return StatusCode::NOT_FOUND.into_response();
        };

        (config, route_idx, params)
    };

    let route = &config.routes[route_idx];
    let defaults = &config.defaults;

    tracing::info!(
        correlation_id = %correlation_id,
        method = %method,
        path = %path,
        targets = route.targets.len(),
        "request received"
    );

    let client_ip = addr.ip().to_string();
    let request = fanout::FanOutRequest {
        client: &state.http_client,
        targets: &route.targets,
        method: &method,
        original_headers: &req_headers,
        body: &body,
        params: &params,
        defaults,
        route,
        client_ip: &client_ip,
        correlation_id: &correlation_id,
    };

    match fanout::fan_out(request).await {
        Ok(fan_out_result) => {
            if let Some((status, mut resp_headers, body_bytes)) = fan_out_result.primary_response {
                state.stats.forwarded.fetch_add(1, Ordering::Relaxed);
                headers::strip_response_hop_by_hop(&mut resp_headers);
                let mut builder = Response::builder().status(status);
                for (key, value) in &resp_headers {
                    builder = builder.header(key, value);
                }
                builder
                    .header("x-correlation-id", &correlation_id)
                    .body(axum::body::Body::from(body_bytes))
                    .unwrap_or_else(|e| {
                        tracing::error!(
                            correlation_id = %correlation_id,
                            error = %e,
                            "failed to build response"
                        );
                        StatusCode::BAD_GATEWAY.into_response()
                    })
            } else {
                state.stats.failed.fetch_add(1, Ordering::Relaxed);
                StatusCode::BAD_GATEWAY.into_response()
            }
        }
        Err(e) => {
            tracing::error!(
                correlation_id = %correlation_id,
                error = %e,
                "fan-out failed"
            );
            state.stats.failed.fetch_add(1, Ordering::Relaxed);
            StatusCode::BAD_GATEWAY.into_response()
        }
    }
}
