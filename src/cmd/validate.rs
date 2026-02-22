//! `switchboard validate` — check a configuration file for errors.
//!
//! Parses and validates the config file, reporting results in either
//! human-readable text or machine-readable JSON format.

use crate::cli::{ValidateArgs, ValidateFormat};
use crate::config::sources::parse_config_str;
use crate::config::validation;
use crate::error::SwitchboardError;

pub fn execute(args: &ValidateArgs) -> Result<(), SwitchboardError> {
    let path = &args.config;

    if !path.exists() {
        return Err(SwitchboardError::ConfigFileNotFound { path: path.clone() });
    }

    let content = std::fs::read_to_string(path)?;

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let config = parse_config_str(ext, &content, &path.display().to_string())?;

    if let Err(errors) = validation::validate(&config) {
        match args.format {
            ValidateFormat::Text => {
                eprintln!("\u{2717} {} has {} errors\n", path.display(), errors.len());
                for error in &errors {
                    eprintln!("{error}");
                }
            }
            ValidateFormat::Json => {
                let json_errors: Vec<serde_json::Value> = errors
                    .iter()
                    .map(|e| {
                        serde_json::json!({
                            "route": e.route,
                            "field": e.field,
                            "message": e.message,
                            "suggestion": e.suggestion,
                        })
                    })
                    .collect();
                println!(
                    "{}",
                    serde_json::json!({
                        "valid": false,
                        "errors": json_errors,
                    })
                );
            }
        }
        return Err(SwitchboardError::ConfigValidation { errors });
    }

    match args.format {
        ValidateFormat::Text => {
            println!(
                "\u{2713} {}",
                validation::format_validation_report(&path.display().to_string(), &config)
            );
        }
        ValidateFormat::Json => {
            let total_targets = config.total_targets();
            println!(
                "{}",
                serde_json::json!({
                    "valid": true,
                    "routes": config.routes.len(),
                    "targets": total_targets,
                })
            );
        }
    }

    Ok(())
}
