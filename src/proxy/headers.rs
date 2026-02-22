//! Header construction, forwarding, and hop-by-hop stripping.
//!
//! [`build_forwarded_headers`] clones the original client headers (when
//! forwarding is enabled), strips hop-by-hop headers, rewrites `Host`,
//! adds proxy metadata (`X-Forwarded-For`, `X-Real-IP`, `Via`,
//! `X-Correlation-Id`), and applies per-route and per-defaults custom
//! header rules.

use std::sync::LazyLock;

use axum::http::{HeaderMap, HeaderName, HeaderValue};

use crate::config::model::{Defaults, Route};

static HOP_BY_HOP: LazyLock<Vec<HeaderName>> = LazyLock::new(|| {
    [
        "connection",
        "keep-alive",
        "transfer-encoding",
        "te",
        "trailer",
        "upgrade",
        "proxy-authorization",
        "proxy-authenticate",
    ]
    .iter()
    .filter_map(|name| name.parse::<HeaderName>().ok())
    .collect()
});

/// Strip hop-by-hop headers and `content-length` from an upstream response.
///
/// The body has already been fully collected by the fan-out engine, so
/// `transfer-encoding` and `content-length` from the origin are no longer
/// accurate. Axum will set the correct `content-length` based on the actual
/// body bytes.
pub fn strip_response_hop_by_hop(headers: &mut HeaderMap) {
    for name in HOP_BY_HOP.iter() {
        headers.remove(name);
    }
    headers.remove(hyper::header::CONTENT_LENGTH);
}

pub fn build_forwarded_headers(
    original: &HeaderMap,
    client_ip: &str,
    target_url: &url::Url,
    route: &Route,
    defaults: &Defaults,
    correlation_id: &str,
) -> HeaderMap {
    let mut headers = if defaults.forward_headers {
        original.clone()
    } else {
        HeaderMap::new()
    };

    // Strip hop-by-hop
    if defaults.strip_hop_by_hop {
        for header_name in HOP_BY_HOP.iter() {
            headers.remove(header_name);
        }
    }

    // Rewrite Host
    if let Some(host) = target_url.host_str() {
        let host_value = target_url
            .port()
            .map_or_else(|| host.to_string(), |port| format!("{host}:{port}"));
        if let Ok(val) = HeaderValue::from_str(&host_value) {
            headers.insert("host", val);
        }
    }

    // X-Forwarded-For: append to chain
    if defaults.proxy_headers {
        let xff = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .map_or_else(
                || client_ip.to_string(),
                |existing| format!("{existing}, {client_ip}"),
            );
        if let Ok(val) = HeaderValue::from_str(&xff) {
            headers.insert("x-forwarded-for", val);
        }

        // X-Real-IP (first IP in chain)
        let real_ip = xff.split(',').next().unwrap_or(client_ip).trim();
        if let Ok(val) = HeaderValue::from_str(real_ip) {
            headers.insert("x-real-ip", val);
        }

        // X-Forwarded-Proto
        let proto = if target_url.scheme() == "https" {
            "https"
        } else {
            "http"
        };
        if let Ok(val) = HeaderValue::from_str(proto) {
            headers.insert("x-forwarded-proto", val);
        }

        // X-Forwarded-Host (original Host the client targeted)
        if let Some(original_host) = original.get("host") {
            headers.insert("x-forwarded-host", original_host.clone());
        }

        // Via
        if let Ok(val) = HeaderValue::from_str("1.1 switchboard") {
            headers.insert("via", val);
        }

        // Correlation ID
        if let Ok(val) = HeaderValue::from_str(correlation_id) {
            headers.insert("x-correlation-id", val);
        }
    }

    // Apply defaults.headers.add
    for (key, value) in &defaults.headers.add {
        match (key.parse::<HeaderName>(), HeaderValue::from_str(value)) {
            (Ok(name), Ok(val)) => {
                headers.insert(name, val);
            }
            _ => {
                tracing::warn!(header = %key, "invalid header name or value in defaults.headers.add, skipping");
            }
        }
    }

    // Apply route.headers.add (overrides defaults)
    for (key, value) in &route.headers.add {
        match (key.parse::<HeaderName>(), HeaderValue::from_str(value)) {
            (Ok(name), Ok(val)) => {
                headers.insert(name, val);
            }
            _ => {
                tracing::warn!(header = %key, "invalid header name or value in route.headers.add, skipping");
            }
        }
    }

    // Apply defaults.headers.strip
    for key in &defaults.headers.strip {
        if let Ok(name) = key.parse::<HeaderName>() {
            headers.remove(&name);
        }
    }

    // Apply route.headers.strip
    for key in &route.headers.strip {
        if let Ok(name) = key.parse::<HeaderName>() {
            headers.remove(&name);
        }
    }

    headers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{Defaults, HeaderRules, Route, Target};

    fn default_route() -> Route {
        Route {
            path: "/test".into(),
            methods: vec!["*".into()],
            timeout: None,
            headers: HeaderRules::default(),
            targets: vec![Target {
                url: "http://target:8080/test".into(),
                primary: false,
                timeout: None,
            }],
        }
    }

    #[test]
    fn strips_hop_by_hop() {
        let mut original = HeaderMap::new();
        original.insert("connection", "keep-alive".parse().unwrap());
        original.insert("content-type", "application/json".parse().unwrap());

        let target = url::Url::parse("http://target:8080").unwrap();
        let result = build_forwarded_headers(
            &original,
            "10.0.0.1",
            &target,
            &default_route(),
            &Defaults::default(),
            "test-id",
        );

        assert!(result.get("connection").is_none());
        assert!(result.get("content-type").is_some());
    }

    #[test]
    fn rewrites_host() {
        let original = HeaderMap::new();
        let target = url::Url::parse("http://backend:9090/path").unwrap();
        let result = build_forwarded_headers(
            &original,
            "10.0.0.1",
            &target,
            &default_route(),
            &Defaults::default(),
            "test-id",
        );

        assert_eq!(result.get("host").unwrap(), "backend:9090");
    }

    #[test]
    fn appends_x_forwarded_for() {
        let mut original = HeaderMap::new();
        original.insert("x-forwarded-for", "1.2.3.4".parse().unwrap());

        let target = url::Url::parse("http://target:8080").unwrap();
        let result = build_forwarded_headers(
            &original,
            "10.0.0.1",
            &target,
            &default_route(),
            &Defaults::default(),
            "test-id",
        );

        assert_eq!(result.get("x-forwarded-for").unwrap(), "1.2.3.4, 10.0.0.1");
    }

    #[test]
    fn sets_correlation_id() {
        let original = HeaderMap::new();
        let target = url::Url::parse("http://target:8080").unwrap();
        let result = build_forwarded_headers(
            &original,
            "10.0.0.1",
            &target,
            &default_route(),
            &Defaults::default(),
            "my-correlation-id",
        );

        assert_eq!(result.get("x-correlation-id").unwrap(), "my-correlation-id");
    }

    #[test]
    fn applies_route_header_overrides() {
        let original = HeaderMap::new();
        let target = url::Url::parse("http://target:8080").unwrap();
        let mut route = default_route();
        route.headers.add.insert("x-custom".into(), "value".into());

        let result = build_forwarded_headers(
            &original,
            "10.0.0.1",
            &target,
            &route,
            &Defaults::default(),
            "test-id",
        );

        assert_eq!(result.get("x-custom").unwrap(), "value");
    }
}
