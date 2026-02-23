//! `switchboard init` — generate a starter configuration file.
//!
//! Supports two modes:
//! - **Template mode** (default): writes a static template config file.
//! - **Interactive mode** (`--interactive`): walks through a step-by-step wizard.

mod interactive;
mod serialize;
mod template;

use crate::cli::InitArgs;
use crate::error::SwitchboardError;

pub fn execute(args: &InitArgs) -> Result<(), SwitchboardError> {
    if args.interactive {
        interactive::run(args)
    } else {
        template::run(args)
    }
}
