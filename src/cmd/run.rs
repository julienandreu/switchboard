//! `switchboard run` — start the proxy server.
//!
//! Loads configuration from file or database sources, starts the Axum
//! HTTP server with graceful shutdown, and spawns a background config
//! refresh loop for hot-reloading.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::cli::RunArgs;
use crate::config::sources;
use crate::config::{ConfigResolver, ConfigSource};
use crate::error::SwitchboardError;
use crate::logging;
use crate::server::{self, AppState, LoadedConfig, Stats};

pub async fn execute(args: RunArgs) -> Result<(), SwitchboardError> {
    let log_format = logging::resolve_format(args.pretty, args.json);

    #[cfg(feature = "actuator")]
    let log_reload_handle = logging::init(&args.log_level, log_format);
    #[cfg(not(feature = "actuator"))]
    logging::init(&args.log_level, log_format);

    #[cfg(feature = "sentry-integration")]
    let _sentry_guard = args
        .sentry_dsn
        .as_ref()
        .map(|dsn| crate::sentry_integration::init(dsn, args.sentry_environment.as_deref()));

    let resolver = resolve_config_sources(&args).await?;
    let (mut config, version) = resolver.load_with_fallback().await?;

    // Apply CLI timeout override if it differs from the config default
    if args.timeout != config.defaults.timeout {
        config.defaults.timeout = args.timeout;
    }

    // Apply env var overrides for actuator auth
    if let Ok(username) = std::env::var("ACTUATOR_AUTH_USERNAME") {
        config.actuator.auth.username = Some(username);
    }
    if let Ok(password) = std::env::var("ACTUATOR_AUTH_PASSWORD") {
        config.actuator.auth.password = Some(password);
    }

    let route_count = config.routes.len();
    let target_count = config.total_targets();

    let loaded_config = tokio::sync::RwLock::new(LoadedConfig {
        config: Arc::new(config),
        version,
        source_name: resolver.primary_name().to_string(),
        loaded_at: Instant::now(),
    });

    #[cfg(feature = "actuator")]
    let state = Arc::new(AppState {
        config: loaded_config,
        http_client: server::build_http_client(),
        start_time: Instant::now(),
        namespace: args.namespace.clone(),
        stats: Stats::new(),
        log_reload_handle: Some(log_reload_handle),
        current_log_level: tokio::sync::RwLock::new(
            format!("{}", args.log_level.to_tracing_level()).to_uppercase(),
        ),
    });

    #[cfg(not(feature = "actuator"))]
    let state = Arc::new(AppState {
        config: loaded_config,
        http_client: server::build_http_client(),
        start_time: Instant::now(),
        namespace: args.namespace.clone(),
        stats: Stats::new(),
    });

    // Shutdown signal: dropping shutdown_tx closes the channel and stops the refresh loop
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // Spawn config refresh loop with cancellation
    let refresh_state = state.clone();
    let poll_interval = args.poll_interval;
    let refresh_handle = tokio::spawn(async move {
        config_refresh_loop(refresh_state, resolver, poll_interval, shutdown_rx).await;
    });

    let router = server::build_router(state, args.max_body);

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;

    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!(
        addr = %addr,
        routes = route_count,
        targets = target_count,
        namespace = %args.namespace,
        "switchboard started"
    );

    // Wrap the shutdown signal to also stop the config refresh loop immediately
    let graceful_shutdown = async move {
        server::shutdown_signal().await;
        let _ = shutdown_tx.send(true);
    };

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(graceful_shutdown)
    .await?;

    // Wait for the config refresh task to finish (catches panics)
    if let Err(e) = refresh_handle.await {
        tracing::error!(error = %e, "config refresh task failed");
    }

    tracing::info!("switchboard stopped");
    Ok(())
}

async fn resolve_config_sources(args: &RunArgs) -> Result<ConfigResolver, SwitchboardError> {
    let mut primary: Option<Box<dyn ConfigSource>> = None;

    // Check DB backends in priority order
    #[cfg(feature = "dynamodb")]
    if primary.is_none() {
        if let Some(ref table) = args.dynamodb_table {
            let source = sources::dynamodb::DynamoDbSource::new(
                table,
                &args.dynamodb_region,
                &args.namespace,
            )
            .await?;
            primary = Some(Box::new(source));
        }
    }

    #[cfg(feature = "redis")]
    if primary.is_none() {
        if let Some(ref url) = args.redis_url {
            let source = sources::redis_source::RedisSource::new(url, &args.namespace).await?;
            primary = Some(Box::new(source));
        }
    }

    #[cfg(feature = "postgres")]
    if primary.is_none() {
        if let Some(ref url) = args.postgres_url {
            let source = sources::postgres::PostgresSource::new(url, &args.namespace).await?;
            primary = Some(Box::new(source));
        }
    }

    #[cfg(feature = "mongodb")]
    if primary.is_none() {
        if let Some(ref url) = args.mongodb_url {
            let source = sources::mongodb_source::MongoDbSource::new(url, &args.namespace).await?;
            primary = Some(Box::new(source));
        }
    }

    #[cfg(feature = "sqlite")]
    if primary.is_none() {
        if let Some(ref path) = args.sqlite_path {
            let source = sources::sqlite::SqliteSource::new(path, &args.namespace).await?;
            primary = Some(Box::new(source));
        }
    }

    // File-based source
    let file_source = resolve_file_source(args.config.as_deref()).await?;

    if let Some(source) = file_source {
        if let Some(db_primary) = primary {
            // DB is primary, file is fallback
            return Ok(ConfigResolver::new(db_primary, Some(source)));
        }
        primary = Some(source);
    }

    primary.map_or_else(
        || {
            Err(SwitchboardError::NoConfigSource {
                hint: "Provide --config <file> or a database backend flag.\n  \
                       Run 'switchboard init' to create a config file."
                    .into(),
            })
        },
        |p| Ok(ConfigResolver::new(p, None)),
    )
}

async fn resolve_file_source(
    explicit: Option<&std::path::Path>,
) -> Result<Option<Box<dyn ConfigSource>>, SwitchboardError> {
    if let Some(path) = explicit {
        return create_file_source(path).map(Some);
    }

    // Auto-detect in current directory
    let candidates = [
        "switchboard.yaml",
        "switchboard.yml",
        "switchboard.json",
        "switchboard.toml",
    ];

    for name in &candidates {
        let path = PathBuf::from(name);
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            tracing::info!(path = %path.display(), "auto-detected config file");
            return create_file_source(&path).map(Some);
        }
    }

    Ok(None)
}

fn create_file_source(path: &std::path::Path) -> Result<Box<dyn ConfigSource>, SwitchboardError> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext {
        #[cfg(feature = "yaml")]
        "yaml" | "yml" => Ok(Box::new(sources::yaml::new(path.to_path_buf()))),

        #[cfg(feature = "json")]
        "json" => Ok(Box::new(sources::json::new(path.to_path_buf()))),

        #[cfg(feature = "toml")]
        "toml" => Ok(Box::new(sources::toml_source::new(path.to_path_buf()))),

        other => Err(SwitchboardError::UnsupportedFormat(other.to_string())),
    }
}

async fn config_refresh_loop(
    state: Arc<AppState>,
    resolver: ConfigResolver,
    interval_secs: u64,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    interval.tick().await; // Skip first immediate tick

    loop {
        tokio::select! {
            _ = interval.tick() => {}
            _ = shutdown.changed() => {
                tracing::debug!("config refresh loop shutting down");
                return;
            }
        }

        let current_version = {
            let config = state.config.read().await;
            config.version.clone()
        };

        match resolver.primary().has_changed(&current_version).await {
            Ok(true) => {
                tracing::info!("config change detected, reloading");
                match resolver.load_with_fallback().await {
                    Ok((config, version)) => {
                        let route_count = config.routes.len();
                        let mut loaded = state.config.write().await;
                        loaded.config = Arc::new(config);
                        loaded.version = version;
                        loaded.loaded_at = std::time::Instant::now();
                        drop(loaded);
                        state
                            .stats
                            .config_reloads
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        tracing::info!(routes = route_count, "config reloaded");
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "config reload failed, keeping current config");
                    }
                }
            }
            Ok(false) => {}
            Err(e) => {
                tracing::warn!(error = %e, "config change check failed");
            }
        }
    }
}
