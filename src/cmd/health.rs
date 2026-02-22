//! `switchboard health` — check the health of a running instance.
//!
//! Sends a `GET /health` request to the specified URL and displays
//! the response as formatted text or raw JSON.

use http_body_util::BodyExt;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;

use crate::cli::HealthArgs;
use crate::error::SwitchboardError;
use crate::health::HealthResponse;

pub async fn execute(args: HealthArgs) -> Result<(), SwitchboardError> {
    let url = format!("{}/health", args.url.trim_end_matches('/'));
    let uri: hyper::Uri =
        url.parse().map_err(
            |e: hyper::http::uri::InvalidUri| SwitchboardError::UriParse {
                source: Box::new(e),
            },
        )?;

    let connector = hyper_util::client::legacy::connect::HttpConnector::new();
    let client = Client::builder(TokioExecutor::new()).build(connector);

    let req = hyper::Request::builder()
        .uri(uri)
        .body(http_body_util::Full::new(bytes::Bytes::new()))
        .map_err(|e| SwitchboardError::HttpRequest {
            source: Box::new(e),
        })?;

    let response = tokio::time::timeout(std::time::Duration::from_secs(10), client.request(req))
        .await
        .map_err(|_| SwitchboardError::HttpRequest {
            source: "health check timed out after 10s".into(),
        })?
        .map_err(|e| SwitchboardError::HttpRequest {
            source: Box::new(e),
        })?;

    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .map_err(|e| SwitchboardError::HttpRequest {
            source: Box::new(e),
        })?
        .to_bytes();

    if !status.is_success() {
        return Err(SwitchboardError::HealthCheckFailed(status));
    }

    if args.json {
        println!("{}", String::from_utf8_lossy(&body));
        return Ok(());
    }

    let body_str = String::from_utf8_lossy(&body);
    match serde_json::from_str::<HealthResponse>(&body_str) {
        Ok(health) => {
            let uptime = format_uptime(health.uptime_seconds);
            println!("\u{2713} switchboard is healthy ({})", args.url);
            println!("  uptime:         {uptime}");
            println!("  config source:  {}", health.config.source);
            println!(
                "  config version: {} (loaded {}s ago)",
                health.config.version, health.config.loaded_ago_seconds
            );
            println!(
                "  routes:         {} routes, {} targets",
                health.config.routes, health.config.targets
            );
            println!("  namespace:      {}", health.config.namespace);
            println!(
                "  requests:       {} forwarded, {} failed",
                health.stats.requests_forwarded, health.stats.requests_failed
            );
        }
        Err(e) => {
            eprintln!("Failed to parse health response: {e}");
            println!("{}", String::from_utf8_lossy(&body));
        }
    }

    Ok(())
}

fn format_uptime(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    if hours > 0 {
        format!("{hours}h {minutes}m {secs}s")
    } else if minutes > 0 {
        format!("{minutes}m {secs}s")
    } else {
        format!("{secs}s")
    }
}
