//! Command-line interface definitions using clap derive macros.
//!
//! Contains the top-level [`Cli`] parser, the [`Commands`] enum for
//! subcommands (run, init, validate, health), and their associated
//! argument structs. Every flag has an environment variable equivalent
//! for container deployments.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "switchboard",
    version,
    about = "HTTP request broadcasting proxy",
    propagate_version = true,
    after_help = "\x1b[1mQuick start:\x1b[0m\n  \
        switchboard init                     Create a starter config\n  \
        switchboard run                      Start with ./switchboard.yaml\n  \
        switchboard run -c routes.yaml       Start with a specific config\n\n  \
        Docs: https://github.com/julienandreu/switchboard"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the proxy server
    Run(Box<RunArgs>),

    /// Generate a starter config file
    Init(InitArgs),

    /// Validate a config file without starting
    Validate(ValidateArgs),

    /// Check health of a running instance
    Health(HealthArgs),
}

#[derive(Args)]
#[command(after_help = "\x1b[1mExamples:\x1b[0m\n  \
        switchboard run                                    Auto-detect config\n  \
        switchboard run -c routes.yaml                     Specific config file\n  \
        switchboard run -c routes.yaml -p 8080 --pretty    Local dev mode\n  \
        switchboard run --redis-url redis://cache:6379      Redis config")]
pub struct RunArgs {
    /// Config file path (.yaml, .json, .toml)
    #[arg(short, long, env = "CONFIG_FILE")]
    pub config: Option<PathBuf>,

    /// Listen port
    #[arg(short, long, env = "PORT", default_value_t = 3000)]
    pub port: u16,

    /// Listen address
    #[arg(long, env = "HOST", default_value = "0.0.0.0")]
    pub host: String,

    /// Config namespace (for database backends)
    #[arg(short, long, env = "SWITCHBOARD_NAMESPACE", default_value = "default")]
    pub namespace: String,

    // -- Database Backends --
    /// `DynamoDB` table name
    #[cfg(feature = "dynamodb")]
    #[arg(long, env = "DYNAMODB_TABLE", help_heading = "Database Backends")]
    pub dynamodb_table: Option<String>,

    /// AWS region for `DynamoDB`
    #[cfg(feature = "dynamodb")]
    #[arg(
        long,
        env = "DYNAMODB_REGION",
        default_value = "us-east-1",
        help_heading = "Database Backends"
    )]
    pub dynamodb_region: String,

    /// Redis connection URL
    #[cfg(feature = "redis")]
    #[arg(long, env = "REDIS_URL", help_heading = "Database Backends")]
    pub redis_url: Option<String>,

    /// `PostgreSQL` connection URL
    #[cfg(feature = "postgres")]
    #[arg(long, env = "POSTGRES_URL", help_heading = "Database Backends")]
    pub postgres_url: Option<String>,

    /// `MongoDB` connection URL
    #[cfg(feature = "mongodb")]
    #[arg(long, env = "MONGODB_URL", help_heading = "Database Backends")]
    pub mongodb_url: Option<String>,

    /// `SQLite` database path
    #[cfg(feature = "sqlite")]
    #[arg(long, env = "SQLITE_PATH", help_heading = "Database Backends")]
    pub sqlite_path: Option<PathBuf>,

    // -- Logging --
    /// Log level
    #[arg(short, long, env = "LOG_LEVEL", default_value = "info")]
    pub log_level: LogLevel,

    /// Force pretty (human-readable) log output
    #[arg(long)]
    pub pretty: bool,

    /// Force JSON log output (overrides TTY detection)
    #[arg(long, conflicts_with = "pretty")]
    pub json: bool,

    // -- Observability --
    /// Sentry DSN (enables error tracking)
    #[cfg(feature = "sentry-integration")]
    #[arg(long, env = "SENTRY_DSN", help_heading = "Observability")]
    pub sentry_dsn: Option<String>,

    /// Sentry environment tag
    #[cfg(feature = "sentry-integration")]
    #[arg(long, env = "SENTRY_ENVIRONMENT", help_heading = "Observability")]
    pub sentry_environment: Option<String>,

    // -- Tuning --
    /// Default target timeout in milliseconds
    #[arg(
        long,
        env = "REQUEST_TIMEOUT_MS",
        default_value_t = 5000,
        help_heading = "Tuning"
    )]
    pub timeout: u64,

    /// Max request body size in bytes
    #[arg(
        long,
        env = "MAX_BODY_SIZE",
        default_value_t = 1_048_576,
        help_heading = "Tuning"
    )]
    pub max_body: usize,

    /// Config refresh interval in seconds (for database backends)
    #[arg(
        long,
        env = "POLL_INTERVAL_SECS",
        default_value_t = 30,
        help_heading = "Tuning"
    )]
    pub poll_interval: u64,
}

#[derive(Args)]
#[command(after_help = "\x1b[1mExamples:\x1b[0m\n  \
        switchboard init                          Quick start config (yaml)\n  \
        switchboard init -i                       Interactive wizard\n  \
        switchboard init -f toml -o config.toml   Non-interactive, TOML format")]
pub struct InitArgs {
    /// Output format
    #[arg(short, long, default_value = "yaml")]
    pub format: ConfigFormat,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Include full documentation as comments (non-interactive only)
    #[arg(long, conflicts_with = "interactive")]
    pub full: bool,

    /// Launch interactive wizard to build config step by step
    #[arg(short, long)]
    pub interactive: bool,
}

#[derive(Args)]
pub struct ValidateArgs {
    /// Config file to validate
    #[arg(default_value = "switchboard.yaml")]
    pub config: PathBuf,

    /// Output format
    #[arg(long, default_value = "text")]
    pub format: ValidateFormat,
}

#[derive(Args)]
pub struct HealthArgs {
    /// URL of the running instance
    #[arg(default_value = "http://localhost:3000")]
    pub url: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    #[must_use]
    pub const fn to_tracing_level(&self) -> tracing::Level {
        match self {
            Self::Trace => tracing::Level::TRACE,
            Self::Debug => tracing::Level::DEBUG,
            Self::Info => tracing::Level::INFO,
            Self::Warn => tracing::Level::WARN,
            Self::Error => tracing::Level::ERROR,
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ConfigFormat {
    Yaml,
    Json,
    Toml,
}

impl ConfigFormat {
    #[must_use]
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Yaml => "yaml",
            Self::Json => "json",
            Self::Toml => "toml",
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ValidateFormat {
    Text,
    Json,
}
