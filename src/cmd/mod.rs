//! Subcommand dispatch and execution.
//!
//! The [`dispatch`] function routes the parsed CLI to the appropriate
//! subcommand handler: [`run`], [`init`], [`validate`], or [`health`].
//! Each handler lives in its own submodule.

pub mod health;
pub mod init;
pub mod run;
pub mod validate;

use crate::cli::{Cli, Commands};
use crate::error::SwitchboardError;

pub async fn dispatch(cli: Cli) -> Result<(), SwitchboardError> {
    match cli.command {
        Some(Commands::Run(args)) => run::execute(*args).await,
        Some(Commands::Init(ref args)) => init::execute(args),
        Some(Commands::Validate(ref args)) => validate::execute(args),
        Some(Commands::Health(args)) => health::execute(args).await,
        None => {
            print_welcome();
            Ok(())
        }
    }
}

fn print_welcome() {
    let version = env!("CARGO_PKG_VERSION");
    println!(
        "\n  switchboard v{version} \u{2014} HTTP request broadcasting proxy\n\n  \
         No command provided. To get started:\n\n    \
         switchboard init                  Generate a starter config\n    \
         switchboard run                   Start the proxy (auto-detects ./switchboard.yaml)\n    \
         switchboard run -c routes.yaml    Start with a specific config file\n    \
         switchboard --help                See all commands and options\n"
    );
}
