#![cfg(feature = "actuator")]
//! Integration tests for actuator endpoints.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use switchboard::config::model::{
    ActuatorAuth, ActuatorConfig, Config, Defaults, HeaderRules, Route, Target,
};
use switchboard::config::ConfigVersion;
use switchboard::server::{self, AppState, LoadedConfig, Stats};

fn test_config(actuator_enabled: bool) -> Config {
    test_config_with_auth(actuator_enabled, None, None)
}

fn test_config_with_auth(
    actuator_enabled: bool,
    username: Option<String>,
    password: Option<String>,
) -> Config {
    Config {
        actuator: ActuatorConfig {
            enabled: actuator_enabled,
            auth: ActuatorAuth { username, password },
        },
        defaults: Defaults::default(),
        routes: vec![Route {
            path: "/test".into(),
            methods: vec!["GET".into(), "POST".into()],
            timeout: Some(10_000),
            headers: HeaderRules::default(),
            targets: vec![
                Target {
                    url: "http://primary:8080/test".into(),
                    primary: true,
                    timeout: None,
                },
                Target {
                    url: "http://secondary:9090/test".into(),
                    primary: false,
                    timeout: Some(2000),
                },
            ],
        }],
    }
}

async fn start_test_server() -> (SocketAddr, tokio::sync::oneshot::Sender<()>) {
    start_test_server_with(true).await
}

async fn start_test_server_with_auth(
    username: &str,
    password: &str,
) -> (SocketAddr, tokio::sync::oneshot::Sender<()>) {
    let config = test_config_with_auth(true, Some(username.into()), Some(password.into()));
    let state = Arc::new(AppState {
        config: tokio::sync::RwLock::new(LoadedConfig {
            config: Arc::new(config),
            version: ConfigVersion::Hash("abcdef1234567890".into()),
            source_name: "test".into(),
            loaded_at: Instant::now(),
        }),
        http_client: server::build_http_client(),
        start_time: Instant::now(),
        namespace: "test".into(),
        stats: Stats::new(),
        log_reload_handle: None,
        current_log_level: tokio::sync::RwLock::new("INFO".into()),
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

async fn start_test_server_with(
    actuator_enabled: bool,
) -> (SocketAddr, tokio::sync::oneshot::Sender<()>) {
    let config = test_config(actuator_enabled);
    let state = Arc::new(AppState {
        config: tokio::sync::RwLock::new(LoadedConfig {
            config: Arc::new(config),
            version: ConfigVersion::Hash("abcdef1234567890".into()),
            source_name: "test".into(),
            loaded_at: Instant::now(),
        }),
        http_client: server::build_http_client(),
        start_time: Instant::now(),
        namespace: "test".into(),
        stats: Stats::new(),
        log_reload_handle: None,
        current_log_level: tokio::sync::RwLock::new("INFO".into()),
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
async fn actuator_index_returns_links() {
    let (addr, shutdown) = start_test_server().await;

    let resp = reqwest::get(format!("http://{addr}/actuator"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    let links = body.get("_links").expect("missing _links");
    assert!(links.get("self").is_some());
    assert!(links.get("health").is_some());
    assert!(links.get("info").is_some());
    assert!(links.get("env").is_some());
    assert!(links.get("metrics").is_some());
    assert!(links.get("configprops").is_some());
    assert!(links.get("mappings").is_some());
    assert!(links.get("loggers").is_some());

    let _ = shutdown.send(());
}

#[tokio::test]
async fn actuator_health_returns_up() {
    let (addr, shutdown) = start_test_server().await;

    let resp = reqwest::get(format!("http://{addr}/actuator/health"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "UP");
    assert_eq!(body["components"]["liveness"]["status"], "UP");
    assert_eq!(body["components"]["readiness"]["status"], "UP");

    let _ = shutdown.send(());
}

#[tokio::test]
async fn actuator_liveness_returns_200() {
    let (addr, shutdown) = start_test_server().await;

    let resp = reqwest::get(format!("http://{addr}/actuator/health/liveness"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "UP");

    let _ = shutdown.send(());
}

#[tokio::test]
async fn actuator_readiness_returns_200_with_details() {
    let (addr, shutdown) = start_test_server().await;

    let resp = reqwest::get(format!("http://{addr}/actuator/health/readiness"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "UP");
    assert_eq!(body["details"]["config_source"], "test");
    assert_eq!(body["details"]["routes_loaded"], 1);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn actuator_info_returns_build_metadata() {
    let (addr, shutdown) = start_test_server().await;

    let resp = reqwest::get(format!("http://{addr}/actuator/info"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["app"]["name"], "switchboard");
    assert_eq!(body["app"]["version"], env!("CARGO_PKG_VERSION"));
    assert!(body["git"]["commit"].as_str().is_some());
    assert!(body["rust"]["version"].as_str().is_some());
    assert!(body["features"].as_array().is_some());

    let _ = shutdown.send(());
}

#[tokio::test]
async fn actuator_env_masks_sensitive_values() {
    // Safety: env var mutation is unsafe since Rust 1.66 due to thread-safety
    // concerns. Acceptable here because this test var is not read by other threads.
    unsafe { std::env::set_var("TEST_SECRET_KEY", "super-secret-value") };

    let (addr, shutdown) = start_test_server().await;

    let resp = reqwest::get(format!("http://{addr}/actuator/env"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    let sources = body["propertySources"].as_array().unwrap();
    assert!(!sources.is_empty());

    let props = &sources[0]["properties"];
    let secret = props.get("TEST_SECRET_KEY").unwrap();
    assert_eq!(secret["value"], "******");

    // Safety: see set_var above
    unsafe { std::env::remove_var("TEST_SECRET_KEY") };
    let _ = shutdown.send(());
}

#[tokio::test]
async fn actuator_metrics_index_returns_names() {
    let (addr, shutdown) = start_test_server().await;

    let resp = reqwest::get(format!("http://{addr}/actuator/metrics"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    let names = body["names"].as_array().unwrap();
    assert!(names.contains(&serde_json::json!("requests.forwarded")));
    assert!(names.contains(&serde_json::json!("requests.failed")));
    assert!(names.contains(&serde_json::json!("requests.active")));
    assert!(names.contains(&serde_json::json!("config.reloads")));
    assert!(names.contains(&serde_json::json!("uptime.seconds")));

    let _ = shutdown.send(());
}

#[tokio::test]
async fn actuator_metrics_detail_returns_value() {
    let (addr, shutdown) = start_test_server().await;

    let resp = reqwest::get(format!("http://{addr}/actuator/metrics/requests.forwarded"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "requests.forwarded");
    assert_eq!(body["measurement"]["statistic"], "COUNT");
    assert_eq!(body["measurement"]["value"], 0.0);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn actuator_metrics_unknown_returns_404() {
    let (addr, shutdown) = start_test_server().await;

    let resp = reqwest::get(format!("http://{addr}/actuator/metrics/unknown.metric"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn actuator_configprops_returns_config() {
    let (addr, shutdown) = start_test_server().await;

    let resp = reqwest::get(format!("http://{addr}/actuator/configprops"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    let routes = body["routes"].as_array().unwrap();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0]["path"], "/test");

    let _ = shutdown.send(());
}

#[tokio::test]
async fn actuator_mappings_returns_routes() {
    let (addr, shutdown) = start_test_server().await;

    let resp = reqwest::get(format!("http://{addr}/actuator/mappings"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    let mappings = body["contexts"]["switchboard"]["mappings"]
        .as_array()
        .unwrap();
    assert_eq!(mappings.len(), 1);
    assert_eq!(mappings[0]["path"], "/test");
    assert_eq!(mappings[0]["timeout_ms"], 10_000);
    assert_eq!(mappings[0]["targets"].as_array().unwrap().len(), 2);
    assert!(mappings[0]["targets"][0]["primary"].as_bool().unwrap());

    let _ = shutdown.send(());
}

#[tokio::test]
async fn actuator_loggers_returns_current_level() {
    let (addr, shutdown) = start_test_server().await;

    let resp = reqwest::get(format!("http://{addr}/actuator/loggers"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["loggers"]["ROOT"]["configuredLevel"], "INFO");
    assert!(body["levels"].as_array().unwrap().len() >= 5);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn actuator_loggers_post_without_handle_returns_503() {
    let (addr, shutdown) = start_test_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/actuator/loggers"))
        .json(&serde_json::json!({"configuredLevel": "DEBUG"}))
        .send()
        .await
        .unwrap();
    // No reload handle in test → SERVICE_UNAVAILABLE
    assert_eq!(resp.status(), 503);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn existing_health_endpoint_still_works() {
    let (addr, shutdown) = start_test_server().await;

    let resp = reqwest::get(format!("http://{addr}/health")).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "healthy");

    let _ = shutdown.send(());
}

#[tokio::test]
async fn actuator_disabled_returns_404() {
    let (addr, shutdown) = start_test_server_with(false).await;

    let resp = reqwest::get(format!("http://{addr}/actuator"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    let resp = reqwest::get(format!("http://{addr}/actuator/health"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    let resp = reqwest::get(format!("http://{addr}/actuator/info"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    // /health (non-actuator) should still work
    let resp = reqwest::get(format!("http://{addr}/health")).await.unwrap();
    assert_eq!(resp.status(), 200);

    let _ = shutdown.send(());
}

// -- Basic Auth tests --

#[tokio::test]
async fn actuator_auth_returns_401_without_credentials() {
    let (addr, shutdown) = start_test_server_with_auth("admin", "secret").await;

    let resp = reqwest::get(format!("http://{addr}/actuator/health"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
    assert!(resp
        .headers()
        .get("www-authenticate")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("Basic"));

    let _ = shutdown.send(());
}

#[tokio::test]
async fn actuator_auth_returns_200_with_valid_credentials() {
    let (addr, shutdown) = start_test_server_with_auth("admin", "secret").await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{addr}/actuator/health"))
        .basic_auth("admin", Some("secret"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn actuator_auth_returns_401_with_invalid_credentials() {
    let (addr, shutdown) = start_test_server_with_auth("admin", "secret").await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{addr}/actuator/health"))
        .basic_auth("admin", Some("wrong"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn actuator_without_auth_remains_open() {
    let (addr, shutdown) = start_test_server().await;

    // No auth configured — should work without credentials
    let resp = reqwest::get(format!("http://{addr}/actuator/health"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let _ = shutdown.send(());
}
