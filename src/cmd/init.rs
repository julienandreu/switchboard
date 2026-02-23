//! `switchboard init` — generate a starter configuration file.
//!
//! Creates a YAML, JSON, or TOML config file with either minimal
//! or fully documented templates.

use std::path::PathBuf;

use crate::cli::{ConfigFormat, InitArgs};
use crate::error::SwitchboardError;

pub fn execute(args: &InitArgs) -> Result<(), SwitchboardError> {
    let output = args
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(format!("switchboard.{}", args.format.extension())));

    if output.exists() {
        return Err(SwitchboardError::FileExists { path: output });
    }

    let content = match (&args.format, args.full) {
        (ConfigFormat::Yaml, false) => YAML_MINIMAL,
        (ConfigFormat::Yaml, true) => YAML_FULL,
        (ConfigFormat::Json, false) => JSON_MINIMAL,
        (ConfigFormat::Json, true) => JSON_FULL,
        (ConfigFormat::Toml, false) => TOML_MINIMAL,
        (ConfigFormat::Toml, true) => TOML_FULL,
    };

    std::fs::write(&output, content)?;
    println!("Created {}", output.display());
    Ok(())
}

const YAML_MINIMAL: &str = r#"# Switchboard config — https://github.com/julienandreu/switchboard

routes:
  - path: "/example"
    targets:
      - url: "http://localhost:8080/example"
"#;

const YAML_FULL: &str = r#"# Switchboard config — https://github.com/julienandreu/switchboard
#
# All values shown are defaults. Uncomment and modify as needed.

# Actuator endpoints (requires --features actuator at build time)
# actuator:
#   enabled: false
#   auth:
#     username: "admin"
#     password: "changeme"

# Global defaults applied to all routes unless overridden
defaults:
  # timeout: 5000              # Target timeout in ms
  # forward_headers: true      # Forward client headers to targets
  # proxy_headers: true        # Add X-Forwarded-*, Via headers
  # strip_hop_by_hop: true     # Strip Connection, TE, etc.
  # headers:
  #   add: {}                  # Headers to add to all forwarded requests
  #   strip: []                # Headers to remove from all forwarded requests

routes:
  # Simple: one path, one target (first target is primary by default)
  - path: "/example"
    targets:
      - url: "http://localhost:8080/example"

  # Full: all options shown
  # - path: "/orders/:id"
  #   methods: ["GET", "POST"]        # Default: ["*"] (all methods)
  #   timeout: 10000                   # Override default for this route
  #   headers:
  #     add:
  #       X-Source: "switchboard"
  #     strip: ["Cookie"]
  #   targets:
  #     - url: "http://primary:8080/orders/:id"
  #       primary: true                # Response returned to caller
  #       timeout: 8000                # Override route timeout
  #     - url: "http://analytics:9090/ingest/:id"
  #       timeout: 2000

  # Wildcard: catch-all route
  # - path: "/*"
  #   targets:
  #     - url: "http://fallback:8080"
"#;

const JSON_MINIMAL: &str = r#"{
  "routes": [
    {
      "path": "/example",
      "targets": [
        { "url": "http://localhost:8080/example" }
      ]
    }
  ]
}
"#;

const JSON_FULL: &str = r#"{
  "actuator": {
    "enabled": false,
    "auth": {
      "username": "admin",
      "password": "changeme"
    }
  },
  "defaults": {
    "timeout": 5000,
    "forward_headers": true,
    "proxy_headers": true,
    "strip_hop_by_hop": true,
    "headers": {
      "add": {},
      "strip": []
    }
  },
  "routes": [
    {
      "path": "/example",
      "targets": [
        { "url": "http://localhost:8080/example" }
      ]
    }
  ]
}
"#;

const TOML_MINIMAL: &str = r#"# Switchboard config — https://github.com/julienandreu/switchboard

[[routes]]
path = "/example"

[[routes.targets]]
url = "http://localhost:8080/example"
"#;

const TOML_FULL: &str = r#"# Switchboard config — https://github.com/julienandreu/switchboard
#
# All values shown are defaults. Uncomment and modify as needed.

# Actuator endpoints (requires --features actuator at build time)
# [actuator]
# enabled = false
# [actuator.auth]
# username = "admin"
# password = "changeme"

[defaults]
# timeout = 5000
# forward_headers = true
# proxy_headers = true
# strip_hop_by_hop = true

# [defaults.headers]
# add = {}
# strip = []

[[routes]]
path = "/example"
# methods = ["*"]
# timeout = 5000

[[routes.targets]]
url = "http://localhost:8080/example"
# primary = true
# timeout = 5000
"#;
