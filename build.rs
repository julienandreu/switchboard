//! Build script for embedding git and build metadata at compile time.
//!
//! Sets `cargo:rustc-env` variables consumed by the `actuator::info`
//! module via `env!()` macros. Falls back to `"unknown"` when git is
//! unavailable (e.g. Docker builds without `.git`).

use std::process::Command;

/// Try an override env var first (for Docker builds), then fall back to git.
fn git_or_env(env_key: &str, args: &[&str]) -> String {
    std::env::var(env_key)
        .ok()
        .filter(|s| !s.is_empty() && s != "unknown")
        .unwrap_or_else(|| {
            Command::new("git")
                .args(args)
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "unknown".into())
        })
}

fn main() {
    // Re-run when HEAD changes (branch switch, new commit)
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    println!(
        "cargo:rustc-env=SWITCHBOARD_GIT_HASH={}",
        git_or_env("SWITCHBOARD_GIT_HASH_OVERRIDE", &["rev-parse", "HEAD"])
    );
    println!(
        "cargo:rustc-env=SWITCHBOARD_GIT_SHORT={}",
        git_or_env(
            "SWITCHBOARD_GIT_SHORT_OVERRIDE",
            &["rev-parse", "--short", "HEAD"]
        )
    );
    println!(
        "cargo:rustc-env=SWITCHBOARD_GIT_BRANCH={}",
        git_or_env(
            "SWITCHBOARD_GIT_BRANCH_OVERRIDE",
            &["rev-parse", "--abbrev-ref", "HEAD"]
        )
    );

    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=SWITCHBOARD_BUILD_PROFILE={profile}");

    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=SWITCHBOARD_TARGET={target}");

    let rustc_version = Command::new("rustc")
        .args(["--version"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=SWITCHBOARD_RUSTC_VERSION={rustc_version}");

    let build_time = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=SWITCHBOARD_BUILD_TIME={build_time}");
}
