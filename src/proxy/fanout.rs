//! Concurrent fan-out of a single request to multiple targets.
//!
//! Spawns requests to all targets in parallel. The primary target's
//! response is returned as soon as it arrives. Secondary targets run
//! as detached tasks — their results are logged but never block the
//! caller.
//!
//! **Shutdown behavior:** Secondary tasks are fire-and-forget. During
//! graceful shutdown they may be cancelled by the Tokio runtime before
//! completing. This is by design — secondary results are best-effort
//! and are not required for correctness.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use axum::http::{HeaderMap, Method};
use bytes::Bytes;
use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::StatusCode;

use crate::config::model::{Defaults, Route, Target};
use crate::error::SwitchboardError;
use crate::server::HttpClient;

use super::headers::build_forwarded_headers;

#[derive(Debug)]
pub struct FanOutResult {
    pub primary_response: Option<(StatusCode, HeaderMap, Bytes)>,
}

#[derive(Debug)]
pub struct TargetResult {
    pub url: String,
    pub status: Option<u16>,
    pub latency_ms: u64,
    pub error: Option<String>,
}

pub struct FanOutRequest<'a> {
    pub client: &'a HttpClient,
    pub targets: &'a [Target],
    pub method: &'a Method,
    pub original_headers: &'a HeaderMap,
    pub body: &'a Bytes,
    pub params: &'a HashMap<String, String>,
    pub defaults: &'a Defaults,
    pub route: &'a Route,
    pub client_ip: &'a str,
    pub correlation_id: &'a str,
}

#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
pub async fn fan_out(req: FanOutRequest<'_>) -> Result<FanOutResult, SwitchboardError> {
    let primary_idx = req.targets.iter().position(|t| t.primary).unwrap_or(0);

    let mut primary_handle = None;

    for (idx, target) in req.targets.iter().enumerate() {
        let resolved_url = substitute_params(&target.url, req.params);
        let timeout_ms = target
            .timeout
            .or(req.route.timeout)
            .unwrap_or(req.defaults.timeout);

        let parsed_url = match url::Url::parse(&resolved_url) {
            Ok(u) => u,
            Err(e) => {
                tracing::error!(target = %resolved_url, error = %e, "invalid target URL");
                continue;
            }
        };

        let forwarded_headers = build_forwarded_headers(
            req.original_headers,
            req.client_ip,
            &parsed_url,
            req.route,
            req.defaults,
            req.correlation_id,
        );

        let method = req.method.clone();
        let body = req.body.clone();
        let client = req.client.clone();
        let timeout = Duration::from_millis(timeout_ms);
        let correlation_id = req.correlation_id.to_string();

        let task = async move {
            let start = Instant::now();

            let mut req_builder = hyper::Request::builder()
                .method(method)
                .uri(resolved_url.clone());

            for (key, value) in &forwarded_headers {
                req_builder = req_builder.header(key, value);
            }

            let req = match req_builder.body(Full::new(body)) {
                Ok(r) => r,
                Err(e) => {
                    return (
                        TargetResult {
                            url: resolved_url,
                            status: None,
                            latency_ms: start.elapsed().as_millis() as u64,
                            error: Some(e.to_string()),
                        },
                        None,
                    );
                }
            };

            let result = tokio::time::timeout(timeout, client.request(req)).await;
            let latency_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok(Ok(response)) => {
                    let status = response.status();
                    let headers = response.headers().clone();
                    let body_result = response.into_body().collect().await;

                    match body_result {
                        Ok(collected) => {
                            let body_bytes = collected.to_bytes();
                            (
                                TargetResult {
                                    url: resolved_url,
                                    status: Some(status.as_u16()),
                                    latency_ms,
                                    error: None,
                                },
                                Some((status, headers, body_bytes)),
                            )
                        }
                        Err(e) => (
                            TargetResult {
                                url: resolved_url,
                                status: Some(status.as_u16()),
                                latency_ms,
                                error: Some(format!("body read error: {e}")),
                            },
                            None,
                        ),
                    }
                }
                Ok(Err(e)) => (
                    TargetResult {
                        url: resolved_url,
                        status: None,
                        latency_ms,
                        error: Some(e.to_string()),
                    },
                    None,
                ),
                Err(_) => (
                    TargetResult {
                        url: resolved_url,
                        status: None,
                        latency_ms,
                        error: Some("request timed out".into()),
                    },
                    None,
                ),
            }
        };

        if idx == primary_idx {
            // Primary: store handle so we can await it directly
            primary_handle = Some(tokio::spawn(task));
        } else {
            // Secondary: fire-and-forget with self-contained logging
            let cid = correlation_id.clone();
            tokio::spawn(async move {
                let (target_result, _) = task.await;
                if let Some(err) = &target_result.error {
                    tracing::warn!(
                        correlation_id = %cid,
                        target = %target_result.url,
                        error = %err,
                        latency_ms = target_result.latency_ms,
                        "secondary target failed"
                    );
                } else {
                    tracing::info!(
                        correlation_id = %cid,
                        target = %target_result.url,
                        status = target_result.status.unwrap_or(0),
                        latency_ms = target_result.latency_ms,
                        "secondary target responded"
                    );
                }
            });
        }
    }

    // Await only the primary target
    let primary_response = if let Some(handle) = primary_handle {
        match handle.await {
            Ok((target_result, response_data)) => {
                if let Some(err) = &target_result.error {
                    tracing::warn!(
                        target = %target_result.url,
                        error = %err,
                        latency_ms = target_result.latency_ms,
                        "primary target failed"
                    );
                } else {
                    tracing::info!(
                        target = %target_result.url,
                        status = target_result.status.unwrap_or(0),
                        latency_ms = target_result.latency_ms,
                        "primary target responded"
                    );
                }
                response_data
            }
            Err(join_err) => {
                tracing::error!(error = %join_err, "primary target task panicked");
                None
            }
        }
    } else {
        None
    };

    Ok(FanOutResult { primary_response })
}

/// Substitute `:param` placeholders in URL templates.
/// Sorts params by key length descending to prevent partial replacement
/// (e.g., `:userId` is replaced before `:user`).
fn substitute_params(url_template: &str, params: &HashMap<String, String>) -> String {
    let mut result = url_template.to_string();
    let mut sorted_entries: Vec<(&str, &str)> = params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    sorted_entries.sort_by_key(|(k, _)| std::cmp::Reverse(k.len()));

    for (key, value) in sorted_entries {
        result = result.replace(&format!(":{key}"), value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitute_single_param() {
        let mut params = HashMap::new();
        params.insert("id".into(), "42".into());
        assert_eq!(
            substitute_params("http://host/orders/:id", &params),
            "http://host/orders/42"
        );
    }

    #[test]
    fn substitute_multiple_params() {
        let mut params = HashMap::new();
        params.insert("user_id".into(), "1".into());
        params.insert("order_id".into(), "2".into());
        assert_eq!(
            substitute_params("http://host/users/:user_id/orders/:order_id", &params),
            "http://host/users/1/orders/2"
        );
    }

    #[test]
    fn no_params() {
        let params = HashMap::new();
        assert_eq!(
            substitute_params("http://host/orders", &params),
            "http://host/orders"
        );
    }

    #[test]
    fn longer_param_names_replaced_first() {
        let mut params = HashMap::new();
        params.insert("id".into(), "short".into());
        params.insert("item_id".into(), "long".into());
        assert_eq!(
            substitute_params("http://host/:item_id/:id", &params),
            "http://host/long/short"
        );
    }
}
