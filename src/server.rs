//! Axum server setup, shared application state, and graceful shutdown.
//!
//! Contains [`AppState`] (the `Arc`-shared state holding config, HTTP
//! client, stats, and uptime), [`build_router`] for constructing the
//! Axum router with middleware layers, [`build_http_client`] for the
//! connection-pooled hyper client, and [`shutdown_signal`] for
//! SIGTERM / Ctrl+C handling.

use std::sync::atomic::AtomicU64;
use std::time::{Duration, Instant};

use axum::routing::get;
use axum::Router;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

use crate::config::model::Config;
use crate::config::ConfigVersion;
use crate::health::health_handler;
use crate::proxy;

#[derive(Debug)]
pub struct LoadedConfig {
    pub config: Arc<Config>,
    pub version: ConfigVersion,
    pub source_name: String,
    pub loaded_at: Instant,
}

#[derive(Debug)]
pub struct Stats {
    pub forwarded: AtomicU64,
    pub failed: AtomicU64,
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}

impl Stats {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            forwarded: AtomicU64::new(0),
            failed: AtomicU64::new(0),
        }
    }
}

pub type HttpsConnector =
    hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>;
pub type HttpClient = Client<HttpsConnector, http_body_util::Full<bytes::Bytes>>;

pub struct AppState {
    pub config: RwLock<LoadedConfig>,
    pub http_client: HttpClient,
    pub start_time: Instant,
    pub namespace: String,
    pub stats: Stats,
}

#[must_use]
pub fn build_http_client() -> HttpClient {
    // When multiple rustls crypto providers are compiled in (e.g. `--all-features`
    // enables both `ring` and `aws-lc-rs`), rustls cannot auto-detect which one
    // to use. Explicitly install `ring` as the default provider.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_webpki_roots()
        .https_or_http()
        .enable_http1()
        .build();
    Client::builder(TokioExecutor::new())
        .pool_idle_timeout(Duration::from_secs(30))
        .build(https)
}

pub fn build_router(state: Arc<AppState>, max_body: usize) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .fallback(proxy::forward_handler)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(RequestBodyLimitLayer::new(max_body)),
        )
        .with_state(state)
}

pub async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "failed to install Ctrl+C handler");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(e) => {
                tracing::error!(error = %e, "failed to install SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => tracing::info!("received Ctrl+C"),
        () = terminate => tracing::info!("received SIGTERM"),
    }
}
