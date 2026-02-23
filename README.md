# Switchboard

[![CI](https://github.com/julienandreu/switchboard/actions/workflows/ci.yml/badge.svg)](https://github.com/julienandreu/switchboard/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust: 1.80+](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org)

HTTP request broadcasting proxy. Receives incoming requests and fans them out to multiple downstream targets.

Built in Rust for minimal binary size (~2-3 MiB) and maximum throughput. Runs as a single static binary from a `scratch` Docker image. Includes optional [Spring Boot-style actuator endpoints](#actuator-endpoints) for operational monitoring.

## Quick Start

```bash
# Install
cargo install --path .

# Generate a config
switchboard init

# Start the proxy
switchboard run
```

## How It Works

Define routing rules in a YAML (or JSON/TOML) config. Switchboard listens for HTTP requests, matches them against your routes, and forwards each request to all configured targets concurrently. The primary target's response is returned to the caller.

```
Client ──> Switchboard ──┬──> Target A (primary, response returned)
                         ├──> Target B (fire and continue)
                         └──> Target C (fire and continue)
```

## Config

```yaml
actuator:
  enabled: true
  auth:
    username: "admin"
    password: "changeme"

defaults:
  timeout: 5000

routes:
  - path: "/orders/:id"
    methods: ["GET", "POST"]
    targets:
      - url: "http://primary-service:8080/orders/:id"
        primary: true
      - url: "http://analytics:9090/ingest/orders/:id"
        timeout: 2000

  - path: "/health-check"
    targets:
      - url: "http://internal:8080/health"
```

See the [`example/`](example/) directory for complete config samples in YAML, JSON, and TOML.

### Route Patterns

| Pattern | Example | Matches |
|---------|---------|---------|
| Exact | `/orders` | `/orders` only |
| Parameterized | `/orders/:id` | `/orders/42`, `/orders/abc` |
| Wildcard | `/api/*` | `/api/anything/deep/nested` |
| Catch-all | `/*` | Everything (lowest priority) |

Parameters are substituted in target URLs: `:id` in the target URL gets replaced with the captured value.

### Defaults

| Field | Default | Description |
|-------|---------|-------------|
| `timeout` | `5000` | Target timeout in ms |
| `forward_headers` | `true` | Forward client headers to targets |
| `proxy_headers` | `true` | Add `X-Forwarded-*`, `Via`, `X-Real-IP` |
| `strip_hop_by_hop` | `true` | Strip `Connection`, `TE`, etc. |

## CLI

```
switchboard                          Show help
switchboard run                      Start proxy (auto-detects config in cwd)
switchboard run -c routes.yaml       Start with specific config
switchboard run -p 8080 --pretty     Local dev (port 8080, colored logs)
switchboard init                     Generate starter config
switchboard init --full              Generate fully documented config
switchboard init -f toml             Generate TOML config
switchboard validate                 Validate config file
switchboard validate --format json   Machine-readable validation output
switchboard health                   Check running instance health
switchboard health http://host:3000  Check remote instance
```

### Environment Variables

Every CLI flag has an env var equivalent for container deployments:

| Env Var | Flag | Default |
|---------|------|---------|
| `CONFIG_FILE` | `-c, --config` | Auto-detect `switchboard.{yaml,json,toml}` |
| `PORT` | `-p, --port` | `3000` |
| `HOST` | `--host` | `0.0.0.0` |
| `LOG_LEVEL` | `-l, --log-level` | `info` |
| `SWITCHBOARD_NAMESPACE` | `-n, --namespace` | `default` |
| `REQUEST_TIMEOUT_MS` | `--timeout` | `5000` |
| `MAX_BODY_SIZE` | `--max-body` | `1048576` |
| `POLL_INTERVAL_SECS` | `--poll-interval` | `30` |
| `SENTRY_DSN` | `--sentry-dsn` | _(disabled)_ |

## Cargo Features

Build only what you need:

```bash
# Default (YAML config only) — smallest binary
cargo build --release

# All file formats
cargo build --release --features file-backends

# With actuator endpoints
cargo build --release --features actuator

# Everything
cargo build --release --features full
```

| Feature | Description |
|---------|-------------|
| `yaml` | YAML config files _(default)_ |
| `json` | JSON config files |
| `toml` | TOML config files |
| `redis` | Redis config backend |
| `dynamodb` | AWS DynamoDB config backend |
| `postgres` | PostgreSQL config backend |
| `mongodb` | MongoDB config backend |
| `sqlite` | SQLite config backend |
| `actuator` | Spring Boot-style actuator endpoints (zero extra deps) |
| `sentry-integration` | Sentry error tracking |
| `file-backends` | All file formats |
| `db-backends` | All database backends |
| `full` | Everything |

## Logging

Structured JSON logs to stdout. Auto-detects format:

- **TTY** (local dev): colored, human-readable
- **Piped/Docker**: JSON with `timestamp`, `level`, `correlation_id`, `method`, `path`, `target`, `status`, `latency_ms`

Override with `--pretty` or `--json`.

Every request gets a correlation ID (`X-Correlation-Id`): forwarded from the client if present, generated as UUID v4 if not. Propagated to all downstream targets and included in all log entries.

## Header Forwarding

Switchboard forwards all client headers to targets with the following adjustments:

**Added** (proxy metadata):
- `X-Forwarded-For` (appended to existing chain)
- `X-Forwarded-Proto`, `X-Forwarded-Host`
- `X-Real-IP`
- `Via: 1.1 switchboard`
- `X-Correlation-Id`

**Stripped** (hop-by-hop):
- `Connection`, `Keep-Alive`, `Transfer-Encoding`, `TE`, `Trailer`, `Upgrade`

**Rewritten**:
- `Host` set to the target's host

Custom header rules can be configured per route:

```yaml
routes:
  - path: "/api/*"
    headers:
      add:
        X-Source: "switchboard"
      strip: ["Cookie"]
    targets:
      - url: "http://backend:8080"
```

## Health Check

`GET /health` returns:

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 3600,
  "config": {
    "source": "yaml",
    "version": "a3f2c1d8",
    "loaded_ago_seconds": 120,
    "namespace": "default",
    "routes": 5,
    "targets": 12
  },
  "stats": {
    "requests_forwarded": 14832,
    "requests_failed": 3
  }
}
```

This endpoint is always available regardless of the `actuator` feature flag.

## Actuator Endpoints

Requires `--features actuator` at build time and `actuator.enabled: true` in config. Provides Spring Boot-style endpoints for operational monitoring under `/actuator`.

```yaml
actuator:
  enabled: true   # can be toggled via config hot-reload
  auth:
    username: "admin"
    password: "changeme"
```

When `auth` is configured, all `/actuator/*` endpoints require HTTP Basic Authentication. Without `auth`, endpoints are open (backward-compatible).

Credentials can also be set via environment variables, which override config file values:

| Env Var | Description |
|---------|-------------|
| `ACTUATOR_AUTH_USERNAME` | Basic auth username |
| `ACTUATOR_AUTH_PASSWORD` | Basic auth password |

### Discovery

`GET /actuator` returns HATEOAS-style links to all available endpoints.

### Health Probes

| Endpoint | Purpose |
|----------|---------|
| `GET /actuator/health` | Overall health with component breakdown |
| `GET /actuator/health/liveness` | Kubernetes liveness probe (UP if running) |
| `GET /actuator/health/readiness` | Kubernetes readiness probe (UP if routes loaded, 503 otherwise) |

```json
{
  "status": "UP",
  "components": {
    "liveness": { "status": "UP" },
    "readiness": {
      "status": "UP",
      "details": { "config_source": "yaml", "routes_loaded": 5 }
    }
  }
}
```

### Info

`GET /actuator/info` returns build metadata (app version, git commit/branch, Rust version, enabled feature flags, build timestamp).

### Environment

`GET /actuator/env` returns environment variables. Sensitive values (passwords, secrets, tokens, keys, DSNs, credentials) are automatically masked.

### Metrics

| Endpoint | Description |
|----------|-------------|
| `GET /actuator/metrics` | List available metric names |
| `GET /actuator/metrics/{name}` | Get individual metric value |

Available metrics: `requests.forwarded`, `requests.failed`, `requests.active`, `config.reloads`, `uptime.seconds`.

### Configuration & Mappings

| Endpoint | Description |
|----------|-------------|
| `GET /actuator/configprops` | Current loaded configuration |
| `GET /actuator/mappings` | All route-to-target mappings |

### Loggers

| Endpoint | Description |
|----------|-------------|
| `GET /actuator/loggers` | Current log level |
| `POST /actuator/loggers` | Change log level at runtime |

```bash
# Change log level without restart
curl -X POST http://localhost:3000/actuator/loggers \
  -H "Content-Type: application/json" \
  -d '{"configuredLevel": "DEBUG"}'
```

Supported levels: `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`.

## Docker

```dockerfile
FROM rust:1.85-alpine AS builder
ARG FEATURES="full"
ARG GIT_HASH="unknown"
ARG GIT_SHORT="unknown"
ARG GIT_BRANCH="unknown"
RUN apk add --no-cache musl-dev build-base
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY src/ src/
ENV SWITCHBOARD_GIT_HASH_OVERRIDE=${GIT_HASH}
ENV SWITCHBOARD_GIT_SHORT_OVERRIDE=${GIT_SHORT}
ENV SWITCHBOARD_GIT_BRANCH_OVERRIDE=${GIT_BRANCH}
RUN cargo build --release --features "${FEATURES}"

FROM scratch
COPY --from=builder /build/target/release/switchboard /switchboard
ENTRYPOINT ["/switchboard"]
CMD ["run"]
```

```bash
# Minimal image (YAML only, ~2-3 MiB)
docker build --build-arg FEATURES="yaml" -t switchboard .

# Full features (default)
docker build -t switchboard .

# With actuator endpoints
docker build --build-arg FEATURES="yaml,actuator" -t switchboard .

# With git metadata (for /actuator/info)
docker build \
  --build-arg FEATURES="yaml,actuator" \
  --build-arg GIT_HASH="$(git rev-parse HEAD)" \
  --build-arg GIT_SHORT="$(git rev-parse --short HEAD)" \
  --build-arg GIT_BRANCH="$(git rev-parse --abbrev-ref HEAD)" \
  -t switchboard .

# Run
docker run -p 3000:3000 -v ./routes.yaml:/config.yaml switchboard run -c /config.yaml
```

## Minimum Supported Rust Version (MSRV)

The current MSRV is **1.80**. It is set in [`Cargo.toml`](Cargo.toml) via the `rust-version` field and will be bumped only in minor or major releases.

## Getting Help

- **Bug reports and feature requests:** [GitHub Issues](https://github.com/julienandreu/switchboard/issues)
- **Contributing:** See [CONTRIBUTING.md](CONTRIBUTING.md)
- **Changelog:** See [CHANGELOG.md](CHANGELOG.md)

## License

MIT
