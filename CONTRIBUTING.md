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
5. `CHANGELOG.md` is automatically generated — no manual updates needed

## Release Process

Releases are fully automated via [release-plz](https://release-plz.dev/) and driven by Conventional Commits.

**How it works:**

1. Push commits to `main` (directly or by merging a PR)
2. release-plz automatically opens (or updates) a **Release PR** containing:
   - Bumped version in `Cargo.toml` (based on commit types: `fix:` = patch, `feat:` = minor, `BREAKING CHANGE` = major)
   - Updated `CHANGELOG.md` with grouped, linked entries
3. Review and merge the Release PR
4. release-plz automatically creates:
   - A git tag (`v1.2.3`)
   - A GitHub Release with the changelog as the release body

**Configuration files:**

- `release-plz.toml` — release-plz settings (publishing, tagging, PR options)
- `cliff.toml` — changelog template and commit parsing rules (used by git-cliff under the hood)

**Manual override:** To force a specific version bump, include `BREAKING CHANGE` in a commit footer for major, or use `feat:` / `fix:` prefixes as usual.
