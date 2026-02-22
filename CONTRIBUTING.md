# Contributing to Switchboard

Thank you for considering contributing to Switchboard! This document explains the development workflow, code standards, and how to submit changes.

Please note that this project follows the [Contributor Covenant Code of Conduct](https://www.contributor-covenant.org/version/2/1/code_of_conduct/). By participating, you agree to uphold it.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Setup](#setup)
- [Development Workflow](#development-workflow)
- [Code Style](#code-style)
- [Architecture](#architecture)
- [Adding a Config Backend](#adding-a-config-backend)
- [Testing](#testing)
- [Feature Flags](#feature-flags)
- [Commit Messages](#commit-messages)
- [Pull Requests](#pull-requests)
- [Release Process](#release-process)

## Prerequisites

- Rust 1.80+ (see `rust-version` in `Cargo.toml`)
- `cargo` and `rustup`

## Setup

```bash
git clone https://github.com/julienandreu/switchboard.git
cd switchboard
cargo build
```

## Development Workflow

```bash
# Build
cargo build

# Run tests
cargo test

# Run clippy
cargo clippy --all-features -- -D warnings

# Check formatting
cargo fmt --check

# Format code
cargo fmt

# Generate and open documentation
cargo doc --all-features --no-deps --open

# Run with a config
cargo run -- run -c example/switchboard.yaml --pretty

# Generate a config to test with
cargo run -- init
cargo run -- validate switchboard.yaml
```

## Code Style

- Follow standard Rust idioms (`cargo clippy` must pass with `-D warnings`)
- Run `cargo fmt` before committing — CI enforces `cargo fmt --check`
- No `unwrap()` in non-test code — use `?` or proper error handling
- Every public function that can fail returns `Result`
- Error messages include actionable hints for the user
- Use `#[cfg(feature = "...")]` to gate optional dependencies
- Unit tests go in `#[cfg(test)] mod tests` within the same file
- Integration tests go in `tests/`

## Architecture

```
src/
├── cli.rs          # Clap CLI structs (all args, env vars, subcommands)
├── cmd/            # Subcommand handlers (run, init, validate, health)
├── config/         # Config model, validation, source trait, file/DB backends
├── proxy/          # Core proxy: routing, header forwarding, fan-out
├── middleware/      # Placeholder for Tower middleware layers
├── server.rs       # Axum router, AppState, graceful shutdown
├── logging.rs      # Tracing setup (JSON/pretty, Targets filter)
├── health.rs       # GET /health endpoint
└── error.rs        # Error types with thiserror
```

### Adding a Config Backend

1. Create `src/config/sources/your_backend.rs`
2. Implement the `ConfigSource` trait (`name`, `load`, `has_changed`)
3. Gate it with `#[cfg(feature = "your_backend")]`
4. Add the feature and dependency to `Cargo.toml`
5. Add resolution logic in `cmd/run.rs` (`resolve_config_sources`)
6. Add CLI flag in `cli.rs` (`RunArgs`)

## Testing

- **Unit tests:** add to the `#[cfg(test)]` module in the relevant source file
- **Integration tests:** create a file in `tests/`, use `#[tokio::test]` for async tests
- **Run a specific test:** `cargo test test_name`
- **Run all tests with all features:** `cargo test --all-features`

## Feature Flags

When adding dependencies, always:

1. Make them optional with `optional = true` in `Cargo.toml`
2. Gate them behind a feature flag
3. Use `default-features = false` and enable only what you need
4. Verify the binary size impact with `cargo bloat --release`

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add Redis config backend
fix: handle empty X-Forwarded-For header
refactor: extract config resolver into separate module
test: add integration tests for route matching
docs: update README with Redis setup
chore: update dependencies
```

## Pull Requests

1. Fork and create a feature branch from `main`
2. Write tests for new functionality
3. Ensure all checks pass: `cargo test`, `cargo clippy --all-features -- -D warnings`, `cargo fmt --check`
4. Keep PRs focused — one feature or fix per PR
5. Update `CHANGELOG.md` under `[Unreleased]` if the change is user-facing

## Release Process

1. Update the version in `Cargo.toml`
2. Move `[Unreleased]` entries in `CHANGELOG.md` to a new `[x.y.z]` section with the release date
3. Add a fresh empty `[Unreleased]` section at the top
4. Update the comparison links at the bottom of `CHANGELOG.md`
5. Commit as `release: vx.y.z`
6. Tag with `git tag vx.y.z`
7. Push the tag: `git push origin vx.y.z`
