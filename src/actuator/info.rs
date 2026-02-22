//! Build and runtime information endpoint.

use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct InfoResponse {
    pub app: AppInfo,
    pub build: BuildInfo,
    pub git: GitInfo,
    pub rust: RustInfo,
    pub features: Vec<&'static str>,
}

#[derive(Serialize)]
pub struct AppInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub version: &'static str,
}

#[derive(Serialize)]
pub struct BuildInfo {
    pub profile: &'static str,
    pub target: &'static str,
    pub time: &'static str,
}

#[derive(Serialize)]
pub struct GitInfo {
    pub commit: &'static str,
    pub short_commit: &'static str,
    pub branch: &'static str,
}

#[derive(Serialize)]
pub struct RustInfo {
    pub version: &'static str,
}

pub async fn info_handler() -> Json<InfoResponse> {
    Json(InfoResponse {
        app: AppInfo {
            name: env!("CARGO_PKG_NAME"),
            description: env!("CARGO_PKG_DESCRIPTION"),
            version: env!("CARGO_PKG_VERSION"),
        },
        build: BuildInfo {
            profile: env!("SWITCHBOARD_BUILD_PROFILE"),
            target: env!("SWITCHBOARD_TARGET"),
            time: env!("SWITCHBOARD_BUILD_TIME"),
        },
        git: GitInfo {
            commit: env!("SWITCHBOARD_GIT_HASH"),
            short_commit: env!("SWITCHBOARD_GIT_SHORT"),
            branch: env!("SWITCHBOARD_GIT_BRANCH"),
        },
        rust: RustInfo {
            version: env!("SWITCHBOARD_RUSTC_VERSION"),
        },
        features: enabled_features(),
    })
}

fn enabled_features() -> Vec<&'static str> {
    let features: &[&str] = &[
        #[cfg(feature = "yaml")]
        "yaml",
        #[cfg(feature = "json")]
        "json",
        #[cfg(feature = "toml")]
        "toml",
        #[cfg(feature = "sqlite")]
        "sqlite",
        #[cfg(feature = "redis")]
        "redis",
        #[cfg(feature = "dynamodb")]
        "dynamodb",
        #[cfg(feature = "postgres")]
        "postgres",
        #[cfg(feature = "mongodb")]
        "mongodb",
        #[cfg(feature = "sentry-integration")]
        "sentry-integration",
    ];
    features.to_vec()
}
