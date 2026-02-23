//! Configuration validation with detailed error reporting.
//!
//! The [`validate`] function checks a parsed [`Config`]
//! for structural errors such as empty routes, invalid paths, duplicate
//! entries, bad HTTP methods, multiple primaries, and malformed target URLs.
//! Returns a list of [`ValidationError`]
//! values with per-field suggestions.

use url::Url;

use super::model::Config;
use crate::error::ValidationError;

pub const VALID_METHODS: &[&str] = &[
    "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "*",
];

/// Validate a single route path. Returns `Ok(())` or a human-readable error.
pub fn validate_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("path cannot be empty".into());
    }
    if !path.starts_with('/') && path != "*" {
        return Err(format!(
            "path must start with '/' or be '*' (did you mean '/{path}'?)"
        ));
    }
    Ok(())
}

/// Validate a single target URL. Returns `Ok(())` or a human-readable error.
pub fn validate_target_url(url: &str) -> Result<(), String> {
    let test_url = replace_params_for_validation(url);
    match Url::parse(&test_url) {
        Ok(parsed) => {
            let scheme = parsed.scheme();
            if scheme != "http" && scheme != "https" {
                Err(format!(
                    "unsupported scheme '{scheme}' (expected http or https)"
                ))
            } else {
                Ok(())
            }
        }
        Err(_) => Err(format!("'{url}' is not a valid URL")),
    }
}

/// Validate an HTTP method string. Returns `Ok(())` or a human-readable error.
pub fn validate_method(method: &str) -> Result<(), String> {
    let upper = method.to_uppercase();
    if VALID_METHODS.contains(&upper.as_str()) {
        Ok(())
    } else {
        Err(format!("'{method}' is not a valid HTTP method"))
    }
}

pub fn validate(config: &Config) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    // Validate actuator auth: both username and password must be set together
    let auth = &config.actuator.auth;
    match (&auth.username, &auth.password) {
        (Some(u), Some(p)) => {
            if u.is_empty() {
                errors.push(ValidationError {
                    route: "(root)".into(),
                    field: "actuator.auth.username".into(),
                    message: "username cannot be empty when auth is configured".into(),
                    suggestion: None,
                });
            }
            if p.is_empty() {
                errors.push(ValidationError {
                    route: "(root)".into(),
                    field: "actuator.auth.password".into(),
                    message: "password cannot be empty when auth is configured".into(),
                    suggestion: None,
                });
            }
        }
        (Some(_), None) => {
            errors.push(ValidationError {
                route: "(root)".into(),
                field: "actuator.auth.password".into(),
                message: "password is required when username is set".into(),
                suggestion: None,
            });
        }
        (None, Some(_)) => {
            errors.push(ValidationError {
                route: "(root)".into(),
                field: "actuator.auth.username".into(),
                message: "username is required when password is set".into(),
                suggestion: None,
            });
        }
        (None, None) => {}
    }

    if config.routes.is_empty() {
        errors.push(ValidationError {
            route: "(root)".into(),
            field: "routes".into(),
            message: "at least one route must be defined".into(),
            suggestion: None,
        });
        return Err(errors);
    }

    let mut seen_paths = std::collections::HashSet::new();

    for (i, route) in config.routes.iter().enumerate() {
        let route_id = if route.path.is_empty() {
            format!("routes[{i}]")
        } else {
            route.path.clone()
        };

        if let Err(msg) = validate_path(&route.path) {
            errors.push(ValidationError {
                route: route_id.clone(),
                field: "path".into(),
                message: msg,
                suggestion: if !route.path.is_empty() && !route.path.starts_with('/') {
                    Some(format!("did you mean '/{}'?", route.path))
                } else {
                    None
                },
            });
        }

        if !seen_paths.insert(&route.path) {
            errors.push(ValidationError {
                route: route_id.clone(),
                field: "path".into(),
                message: "duplicate route path".into(),
                suggestion: None,
            });
        }

        for method in &route.methods {
            if let Err(msg) = validate_method(method) {
                errors.push(ValidationError {
                    route: route_id.clone(),
                    field: "methods".into(),
                    message: msg,
                    suggestion: None,
                });
            }
        }

        if route.targets.is_empty() {
            errors.push(ValidationError {
                route: route_id.clone(),
                field: "targets".into(),
                message: "at least one target must be defined".into(),
                suggestion: None,
            });
        }

        let primary_count = route.targets.iter().filter(|t| t.primary).count();
        if primary_count > 1 {
            errors.push(ValidationError {
                route: route_id.clone(),
                field: "targets".into(),
                message: format!("{primary_count} targets marked as primary, at most 1 allowed"),
                suggestion: None,
            });
        }

        for target in &route.targets {
            if let Err(msg) = validate_target_url(&target.url) {
                errors.push(ValidationError {
                    route: route_id.clone(),
                    field: "targets.url".into(),
                    message: msg,
                    suggestion: None,
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Replace `:param` patterns with a valid placeholder for URL validation.
fn replace_params_for_validation(url: &str) -> String {
    let mut result = String::with_capacity(url.len());
    let mut chars = url.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == ':' && chars.peek().is_some_and(|c| c.is_alphabetic() || *c == '_') {
            result.push_str("_p");
            while chars
                .peek()
                .is_some_and(|c| c.is_alphanumeric() || *c == '_')
            {
                chars.next();
            }
        } else {
            result.push(ch);
        }
    }
    result
}

#[must_use]
pub fn format_validation_report(path: &str, config: &Config) -> String {
    let total_targets = config.total_targets();
    let mut lines = vec![format!(
        "  {} routes, {} targets\n",
        config.routes.len(),
        total_targets
    )];

    for route in &config.routes {
        let primary = route
            .targets
            .iter()
            .find(|t| t.primary)
            .or_else(|| route.targets.first());

        let primary_url = primary.map_or("none", |t| t.url.as_str());
        let methods = route.methods.join(", ");
        let timeout = route.timeout.map_or_else(
            || format!("{}ms (default)", config.defaults.timeout),
            |t| format!("{t}ms"),
        );

        lines.push(format!(
            "  {}  -> {} targets (primary: {})",
            route.path,
            route.targets.len(),
            primary_url,
        ));
        lines.push(format!("    methods: {methods}"));
        lines.push(format!("    timeout: {timeout}"));
    }

    format!("{} is valid\n{}", path, lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{Config, Defaults, Route, Target};

    fn minimal_config() -> Config {
        Config {
            actuator: Default::default(),
            defaults: Defaults::default(),
            routes: vec![Route {
                path: "/test".into(),
                methods: vec!["*".into()],
                timeout: None,
                headers: Default::default(),
                targets: vec![Target {
                    url: "http://localhost:8080/test".into(),
                    primary: false,
                    timeout: None,
                }],
            }],
        }
    }

    #[test]
    fn valid_config_passes() {
        assert!(validate(&minimal_config()).is_ok());
    }

    #[test]
    fn empty_routes_fails() {
        let config = Config {
            actuator: Default::default(),
            defaults: Defaults::default(),
            routes: vec![],
        };
        let errors = validate(&config).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("at least one route"));
    }

    #[test]
    fn empty_targets_fails() {
        let config = Config {
            actuator: Default::default(),
            defaults: Defaults::default(),
            routes: vec![Route {
                path: "/test".into(),
                methods: vec!["*".into()],
                timeout: None,
                headers: Default::default(),
                targets: vec![],
            }],
        };
        let errors = validate(&config).unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.message.contains("at least one target")));
    }

    #[test]
    fn multiple_primaries_fails() {
        let config = Config {
            actuator: Default::default(),
            defaults: Defaults::default(),
            routes: vec![Route {
                path: "/test".into(),
                methods: vec!["*".into()],
                timeout: None,
                headers: Default::default(),
                targets: vec![
                    Target {
                        url: "http://a:80".into(),
                        primary: true,
                        timeout: None,
                    },
                    Target {
                        url: "http://b:80".into(),
                        primary: true,
                        timeout: None,
                    },
                ],
            }],
        };
        let errors = validate(&config).unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("primary")));
    }

    #[test]
    fn invalid_url_fails() {
        let config = Config {
            actuator: Default::default(),
            defaults: Defaults::default(),
            routes: vec![Route {
                path: "/test".into(),
                methods: vec!["*".into()],
                timeout: None,
                headers: Default::default(),
                targets: vec![Target {
                    url: "not a url".into(),
                    primary: false,
                    timeout: None,
                }],
            }],
        };
        let errors = validate(&config).unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("not a valid URL")));
    }

    #[test]
    fn path_without_slash_fails() {
        let config = Config {
            actuator: Default::default(),
            defaults: Defaults::default(),
            routes: vec![Route {
                path: "test".into(),
                methods: vec!["*".into()],
                timeout: None,
                headers: Default::default(),
                targets: vec![Target {
                    url: "http://localhost:8080".into(),
                    primary: false,
                    timeout: None,
                }],
            }],
        };
        let errors = validate(&config).unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.suggestion.as_deref() == Some("did you mean '/test'?")));
    }

    #[test]
    fn invalid_method_fails() {
        let config = Config {
            actuator: Default::default(),
            defaults: Defaults::default(),
            routes: vec![Route {
                path: "/test".into(),
                methods: vec!["INVALID".into()],
                timeout: None,
                headers: Default::default(),
                targets: vec![Target {
                    url: "http://localhost:8080".into(),
                    primary: false,
                    timeout: None,
                }],
            }],
        };
        let errors = validate(&config).unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.message.contains("not a valid HTTP method")));
    }
}
