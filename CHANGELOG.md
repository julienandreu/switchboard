# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/julienandreu/switchboard/compare/v0.1.0...v0.1.1) - 2026-02-23

### Added

- setup changelog and dynamic version

## [0.1.0] - 2026-02-22

### Added

- HTTP request broadcasting proxy with concurrent fan-out to multiple targets.
- Specificity-based route matching: exact, parameterized (`:param`), wildcard (`/*`), and catch-all.
- Primary target response forwarding with fire-and-continue for secondary targets.
- YAML config support (default), with optional JSON and TOML via feature flags.
- File-based config hot-reloading with SHA256 change detection.
- Database config backend stubs: Redis, DynamoDB, PostgreSQL, MongoDB, SQLite (behind feature flags).
- Full proxy header enrichment: `X-Forwarded-For`, `X-Forwarded-Proto`, `X-Forwarded-Host`, `X-Real-IP`, `Via`.
- Hop-by-hop header stripping (`Connection`, `Keep-Alive`, `Transfer-Encoding`, `TE`, `Trailer`, `Upgrade`).
- Configurable per-route and per-target timeouts with global defaults.
- Custom header rules: add and strip headers at defaults and route level.
- Correlation ID propagation (`X-Correlation-Id`): forwarded or generated as UUID v4.
- CLI subcommands: `run`, `init`, `validate`, `health`.
- Structured logging via `tracing`: JSON for production, pretty-printed for TTY.
- `GET /health` endpoint with version, uptime, config metadata, and request statistics.
- Environment variable support for all CLI flags.
- Optional Sentry integration for error tracking (behind `sentry-integration` feature flag).
- Graceful shutdown on SIGTERM and Ctrl+C.
- Minimal release binary via `scratch` Docker image with LTO and `panic = "abort"`.
- CI pipeline: check, test, clippy, fmt, and dual build (minimal + full features).

[Unreleased]: https://github.com/julienandreu/switchboard/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/julienandreu/switchboard/releases/tag/v0.1.0
