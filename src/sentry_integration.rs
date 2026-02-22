//! Optional Sentry error tracking integration.
//!
//! Initializes the Sentry SDK with the provided DSN and environment.
//! The returned guard must be held for the lifetime of the application
//! to ensure errors and panics are reported.

pub fn init(dsn: &str, environment: Option<&str>) -> sentry::ClientInitGuard {
    let parsed_dsn = match dsn.parse() {
        Ok(d) => Some(d),
        Err(e) => {
            tracing::warn!(error = %e, "invalid Sentry DSN, error tracking disabled");
            None
        }
    };

    sentry::init(sentry::ClientOptions {
        dsn: parsed_dsn,
        environment: environment.map(|e| e.to_string().into()),
        release: Some(env!("CARGO_PKG_VERSION").into()),
        ..Default::default()
    })
}
