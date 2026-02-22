//! Integration tests for the HTTP server, health endpoint, and graceful shutdown.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use switchboard::config::model::{Config, Defaults, HeaderRules, Route, Target};
use switchboard::config::ConfigVersion;
use switchboard::health::HealthResponse;
use switchboard::server::{self, AppState, LoadedConfig, Stats};

fn test_config() -> Config {
    Config {
        defaults: Defaults::default(),
        routes: vec![Route {
            path: "/test".into(),
            methods: vec!["*".into()],
            timeout: None,
            headers: HeaderRules::default(),
            targets: vec![Target {
                url: "http://localhost:19999/echo".into(),
                primary: true,
                timeout: None,
            }],
        }],
    }
}

async fn start_test_server() -> (SocketAddr, tokio::sync::oneshot::Sender<()>) {
    let config = test_config();
    let state = Arc::new(AppState {
        config: tokio::sync::RwLock::new(LoadedConfig {
            config: Arc::new(config),
            version: ConfigVersion::Hash("test-hash".into()),
            source_name: "test".into(),
            loaded_at: Instant::now(),
        }),
        http_client: server::build_http_client(),
        start_time: Instant::now(),
        namespace: "test".into(),
        stats: Stats::new(),
    });

    let router = server::build_router(state, 1_048_576);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        })
        .await
        .unwrap();
    });

    (addr, shutdown_tx)
}

#[tokio::test]
async fn health_endpoint_returns_healthy() {
    let (addr, shutdown) = start_test_server().await;

    let url = format!("http://{addr}/health");
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);

    let health: HealthResponse = resp.json().await.unwrap();
    assert_eq!(health.status, "healthy");
    assert_eq!(health.config.source, "test");
    assert_eq!(health.config.routes, 1);
    assert_eq!(health.config.targets, 1);
    assert_eq!(health.stats.requests_forwarded, 0);
    assert_eq!(health.stats.requests_failed, 0);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn unmatched_route_returns_404() {
    let (addr, shutdown) = start_test_server().await;

    let url = format!("http://{addr}/nonexistent");
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 404);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn health_version_matches_crate() {
    let (addr, shutdown) = start_test_server().await;

    let url = format!("http://{addr}/health");
    let health: HealthResponse = reqwest::get(&url).await.unwrap().json().await.unwrap();
    assert_eq!(health.version, env!("CARGO_PKG_VERSION"));

    let _ = shutdown.send(());
}

#[tokio::test]
async fn graceful_shutdown_works() {
    let (addr, shutdown) = start_test_server().await;

    // Verify server is running
    let url = format!("http://{addr}/health");
    assert!(reqwest::get(&url).await.is_ok());

    // Send shutdown
    let _ = shutdown.send(());

    // Give it a moment to shut down
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Server should no longer accept connections
    let result = reqwest::get(&url).await;
    assert!(result.is_err());
}
