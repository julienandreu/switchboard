//! Switchboard is an HTTP request broadcasting proxy.
//!
//! It receives incoming HTTP requests, matches them against configured routes,
//! and fans each request out to multiple downstream targets concurrently.
//! The primary target's response is returned to the caller while secondary
//! targets receive the request on a fire-and-continue basis.
//!
//! # Architecture
//!
//! - [`cli`] -- Command-line argument parsing with clap derive macros.
//! - [`cmd`] -- Subcommand dispatch and execution (run, init, validate, health).
//! - [`config`] -- Configuration loading, validation, and hot-reloading via the
//!   [`ConfigSource`](config::ConfigSource) trait.
//! - [`error`] -- Unified error types using `thiserror`.
//! - [`health`] -- `GET /health` endpoint handler returning runtime diagnostics.
//! - [`logging`] -- Structured tracing setup with JSON and pretty-print output.
//! - [`middleware`] -- Placeholder for Tower middleware layers.
//! - [`proxy`] -- Core HTTP forwarding: route matching, header construction, and
//!   concurrent fan-out to multiple targets.
//! - [`server`] -- Axum server setup, shared application state, HTTP client, and
//!   graceful shutdown.
//!
//! # Feature Flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `yaml` | YAML config file support _(enabled by default)_ |
//! | `json` | JSON config file support |
//! | `toml` | TOML config file support |
//! | `redis` | Redis config backend |
//! | `dynamodb` | AWS DynamoDB config backend |
//! | `postgres` | PostgreSQL config backend |
//! | `mongodb` | MongoDB config backend |
//! | `sqlite` | SQLite config backend |
//! | `actuator` | Spring Boot-style actuator endpoints |
//! | `sentry-integration` | Sentry error tracking |
//! | `file-backends` | All file format backends |
//! | `db-backends` | All database backends |
//! | `full` | All features |

// Binary crate — public functions are internal, not consumed by external users.
#![allow(clippy::missing_errors_doc)]

#[cfg(feature = "actuator")]
pub mod actuator;
pub mod cli;
pub mod cmd;
pub mod config;
pub mod error;
pub mod health;
pub mod logging;
pub mod middleware;
pub mod proxy;
pub mod server;

#[cfg(feature = "sentry-integration")]
pub mod sentry_integration;
