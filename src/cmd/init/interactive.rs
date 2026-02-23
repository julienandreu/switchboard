//! Interactive wizard for step-by-step config generation.

use std::path::{Path, PathBuf};

use console::style;
use dialoguer::{Confirm, Input, Password, Select};

use crate::cli::{ConfigFormat, InitArgs};
use crate::config::model::*;
use crate::config::validation::{validate, validate_method, validate_path, validate_target_url};
use crate::error::SwitchboardError;

use super::serialize::serialize_config;

/// Map a `dialoguer::Error` to a `SwitchboardError`.
fn map_prompt_err(e: dialoguer::Error) -> SwitchboardError {
    SwitchboardError::Io(std::io::Error::other(e.to_string()))
}

pub fn run(args: &InitArgs) -> Result<(), SwitchboardError> {
    // Ensure we're running in an interactive terminal
    if !console::Term::stdout().is_term() {
        return Err(SwitchboardError::Io(std::io::Error::other(
            "interactive mode requires a terminal (TTY). Use switchboard init without -i for non-interactive mode.",
        )));
    }

    println!(
        "\n  {} Config Wizard\n  {}\n",
        style("Switchboard").cyan().bold(),
        style("─────────────────────────").dim()
    );

    // Step 1: Output settings
    println!("  {}\n", style("Step 1: Output").bold());
    let format = prompt_format(args)?;
    let output = prompt_output(args, &format)?;

    // Step 2: Defaults
    println!("\n  {}\n", style("Step 2: Defaults").bold());
    let defaults = prompt_defaults()?;

    // Step 3: Routes
    println!("\n  {}\n", style("Step 3: Routes").bold());
    let routes = prompt_routes()?;

    // Step 4: Actuator
    println!("\n  {}\n", style("Step 4: Actuator").bold());
    let actuator = prompt_actuator()?;

    let config = Config {
        actuator,
        defaults,
        routes,
    };

    // Validate the assembled config
    if let Err(errors) = validate(&config) {
        eprintln!(
            "\n  {} Config has validation errors:",
            style("!").red().bold()
        );
        for e in &errors {
            eprintln!("    {e}");
        }
        return Err(SwitchboardError::ConfigValidation { errors });
    }

    // Step 5: Review
    println!("\n  {}\n", style("Step 5: Review").bold());
    print_summary(&config, &format, &output);

    let confirm = Confirm::new()
        .with_prompt(format!("Write config to {}?", output.display()))
        .default(true)
        .interact()
        .map_err(map_prompt_err)?;

    if !confirm {
        println!("  Aborted.");
        return Ok(());
    }

    // Handle existing file
    if output.exists() {
        let overwrite = Confirm::new()
            .with_prompt(format!("{} already exists. Overwrite?", output.display()))
            .default(false)
            .interact()
            .map_err(map_prompt_err)?;
        if !overwrite {
            println!("  Aborted.");
            return Ok(());
        }
    }

    let content = serialize_config(&config, &format)?;
    std::fs::write(&output, content)?;
    println!(
        "\n  {} Created {}",
        style("✓").green().bold(),
        output.display()
    );
    Ok(())
}

fn prompt_format(args: &InitArgs) -> Result<ConfigFormat, SwitchboardError> {
    let formats = &["yaml", "json", "toml"];
    let default_idx = match args.format {
        ConfigFormat::Yaml => 0,
        ConfigFormat::Json => 1,
        ConfigFormat::Toml => 2,
    };

    let selection = Select::new()
        .with_prompt("Config format")
        .items(formats)
        .default(default_idx)
        .interact()
        .map_err(map_prompt_err)?;

    Ok(match selection {
        0 => ConfigFormat::Yaml,
        1 => ConfigFormat::Json,
        2 => ConfigFormat::Toml,
        _ => unreachable!(),
    })
}

fn prompt_output(args: &InitArgs, format: &ConfigFormat) -> Result<PathBuf, SwitchboardError> {
    let default_path = args.output.as_ref().map_or_else(
        || format!("switchboard.{}", format.extension()),
        |p| p.display().to_string(),
    );

    let path_str: String = Input::new()
        .with_prompt("Output file path")
        .default(default_path)
        .interact_text()
        .map_err(map_prompt_err)?;

    Ok(PathBuf::from(path_str))
}

fn prompt_defaults() -> Result<Defaults, SwitchboardError> {
    let timeout: u64 = Input::new()
        .with_prompt("Default timeout (ms)")
        .default(5000)
        .validate_with(|input: &u64| -> Result<(), String> {
            if *input == 0 {
                Err("timeout must be greater than 0".into())
            } else {
                Ok(())
            }
        })
        .interact_text()
        .map_err(map_prompt_err)?;

    let forward_headers = Confirm::new()
        .with_prompt("Forward client headers to targets?")
        .default(true)
        .interact()
        .map_err(map_prompt_err)?;

    let proxy_headers = Confirm::new()
        .with_prompt("Add proxy headers (X-Forwarded-For, Via)?")
        .default(true)
        .interact()
        .map_err(map_prompt_err)?;

    let strip_hop_by_hop = Confirm::new()
        .with_prompt("Strip hop-by-hop headers?")
        .default(true)
        .interact()
        .map_err(map_prompt_err)?;

    Ok(Defaults {
        timeout,
        forward_headers,
        proxy_headers,
        strip_hop_by_hop,
        headers: HeaderRules::default(),
    })
}

fn prompt_routes() -> Result<Vec<Route>, SwitchboardError> {
    let mut routes = Vec::new();
    loop {
        if !routes.is_empty() {
            let add_another = Confirm::new()
                .with_prompt("Add another route?")
                .default(false)
                .interact()
                .map_err(map_prompt_err)?;
            if !add_another {
                break;
            }
        }
        let idx = routes.len() + 1;
        println!(
            "\n  {} Route {} {}",
            style("──").dim(),
            idx,
            style("──").dim()
        );
        routes.push(prompt_single_route()?);
    }
    Ok(routes)
}

fn prompt_single_route() -> Result<Route, SwitchboardError> {
    let path: String = Input::new()
        .with_prompt("Route path (e.g. /api/orders/:id)")
        .validate_with(|input: &String| -> Result<(), String> { validate_path(input) })
        .interact_text()
        .map_err(map_prompt_err)?;

    let methods_str: String = Input::new()
        .with_prompt("HTTP methods (comma-separated, or * for all)")
        .default("*".into())
        .validate_with(|input: &String| -> Result<(), String> {
            for m in input.split(',') {
                let trimmed = m.trim();
                if trimmed.is_empty() {
                    return Err("method cannot be empty".into());
                }
                validate_method(trimmed)?;
            }
            Ok(())
        })
        .interact_text()
        .map_err(map_prompt_err)?;

    let methods: Vec<String> = methods_str
        .split(',')
        .map(|m| m.trim().to_uppercase())
        .collect();

    let timeout_str: String = Input::new()
        .with_prompt("Route timeout override (ms, blank for default)")
        .default(String::new())
        .allow_empty(true)
        .validate_with(|input: &String| -> Result<(), String> {
            if input.is_empty() {
                return Ok(());
            }
            input
                .parse::<u64>()
                .map(|_| ())
                .map_err(|_| "must be a number".into())
        })
        .interact_text()
        .map_err(map_prompt_err)?;

    let timeout = if timeout_str.is_empty() {
        None
    } else {
        Some(timeout_str.parse::<u64>().unwrap())
    };

    let targets = prompt_targets()?;

    Ok(Route {
        path,
        methods,
        timeout,
        headers: HeaderRules::default(),
        targets,
    })
}

fn prompt_targets() -> Result<Vec<Target>, SwitchboardError> {
    let mut targets = Vec::new();
    loop {
        if !targets.is_empty() {
            let add_another = Confirm::new()
                .with_prompt("Add another target?")
                .default(false)
                .interact()
                .map_err(map_prompt_err)?;
            if !add_another {
                break;
            }
        }
        let idx = targets.len() + 1;
        let is_first = targets.is_empty();
        println!(
            "    {} Target {} {}",
            style("──").dim(),
            idx,
            style("──").dim()
        );
        targets.push(prompt_single_target(is_first)?);
    }
    Ok(targets)
}

fn prompt_single_target(is_first: bool) -> Result<Target, SwitchboardError> {
    let url: String = Input::new()
        .with_prompt("Target URL")
        .validate_with(|input: &String| -> Result<(), String> { validate_target_url(input) })
        .interact_text()
        .map_err(map_prompt_err)?;

    let primary = Confirm::new()
        .with_prompt("Primary target (response returned to caller)?")
        .default(is_first)
        .interact()
        .map_err(map_prompt_err)?;

    let timeout_str: String = Input::new()
        .with_prompt("Target timeout override (ms, blank for default)")
        .default(String::new())
        .allow_empty(true)
        .validate_with(|input: &String| -> Result<(), String> {
            if input.is_empty() {
                return Ok(());
            }
            input
                .parse::<u64>()
                .map(|_| ())
                .map_err(|_| "must be a number".into())
        })
        .interact_text()
        .map_err(map_prompt_err)?;

    let timeout = if timeout_str.is_empty() {
        None
    } else {
        Some(timeout_str.parse::<u64>().unwrap())
    };

    Ok(Target {
        url,
        primary,
        timeout,
    })
}

fn prompt_actuator() -> Result<ActuatorConfig, SwitchboardError> {
    let enabled = Confirm::new()
        .with_prompt("Enable actuator endpoints?")
        .default(false)
        .interact()
        .map_err(map_prompt_err)?;

    if !enabled {
        return Ok(ActuatorConfig::default());
    }

    let configure_auth = Confirm::new()
        .with_prompt("Configure actuator authentication?")
        .default(false)
        .interact()
        .map_err(map_prompt_err)?;

    if !configure_auth {
        return Ok(ActuatorConfig {
            enabled: true,
            auth: ActuatorAuth::default(),
        });
    }

    let username: String = Input::new()
        .with_prompt("Actuator username")
        .default("admin".into())
        .interact_text()
        .map_err(map_prompt_err)?;

    let password: String = Password::new()
        .with_prompt("Actuator password")
        .interact()
        .map_err(map_prompt_err)?;

    Ok(ActuatorConfig {
        enabled: true,
        auth: ActuatorAuth {
            username: Some(username),
            password: Some(password),
        },
    })
}

fn print_summary(config: &Config, format: &ConfigFormat, output: &Path) {
    println!(
        "  {}",
        style("┌─────────────────────────────────────────────┐").dim()
    );
    println!(
        "  {}  Format:   {:<35}{}",
        style("│").dim(),
        format.extension(),
        style("│").dim()
    );
    println!(
        "  {}  Output:   {:<35}{}",
        style("│").dim(),
        output.display(),
        style("│").dim()
    );
    println!(
        "  {}  Timeout:  {:<35}{}",
        style("│").dim(),
        format!("{}ms", config.defaults.timeout),
        style("│").dim()
    );
    println!(
        "  {}  Routes:   {:<35}{}",
        style("│").dim(),
        config.routes.len(),
        style("│").dim()
    );

    for route in &config.routes {
        let methods = route.methods.join(", ");
        let target_count = route.targets.len();
        println!(
            "  {}    {} [{}] \u{2192} {} target{}{}",
            style("│").dim(),
            route.path,
            methods,
            target_count,
            if target_count != 1 { "s" } else { "" },
            style("").dim()
        );

        for target in &route.targets {
            let marker = if target.primary {
                style("\u{2605}").yellow().to_string()
            } else {
                style("\u{25CB}").dim().to_string()
            };
            println!(
                "  {}      {} {}{}",
                style("│").dim(),
                marker,
                target.url,
                target
                    .timeout
                    .map_or(String::new(), |t| format!(" ({}ms)", t))
            );
        }
    }

    let actuator_status = if config.actuator.enabled {
        if config.actuator.auth.username.is_some() {
            "enabled (with auth)"
        } else {
            "enabled"
        }
    } else {
        "disabled"
    };
    println!(
        "  {}  Actuator: {:<35}{}",
        style("│").dim(),
        actuator_status,
        style("│").dim()
    );
    println!(
        "  {}\n",
        style("└─────────────────────────────────────────────┘").dim()
    );
}
