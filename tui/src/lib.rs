//! Workspace facade crate for running the standalone Kinic TUI.

use clap::Parser;
use kinic_core::cli::parse_identity_arg;
use kinic_core::tui::{TuiLaunchConfig, build_launch_config};

pub use kinic_core::tui;
pub use tui_kit_host as host;
pub use tui_kit_model as model;
pub use tui_kit_render as render;
pub use tui_kit_runtime as runtime;

#[derive(Debug, Parser)]
#[command(name = "kinic-tui", about = "Kinic terminal UI")]
pub struct TuiArgs {
    #[arg(
        long,
        required = true,
        value_parser = parse_identity_arg,
        help = "Required dfx identity name used to load credentials from the system keyring"
    )]
    pub identity: String,

    #[arg(
        long,
        help = "Use the Internet Computer mainnet instead of local replica"
    )]
    pub ic: bool,
}

pub fn build_launch_config_from_args(args: &TuiArgs) -> anyhow::Result<TuiLaunchConfig> {
    build_launch_config(args.identity.clone(), args.ic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_launch_config_from_args_maps_ic_flag_without_identity() {
        let args = TuiArgs::try_parse_from(["kinic-tui"]).unwrap_err();

        assert_eq!(args.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn build_launch_config_from_args_rejects_empty_identity() {
        let args = TuiArgs::try_parse_from(["kinic-tui", "--identity", ""]).unwrap_err();

        assert_eq!(args.kind(), clap::error::ErrorKind::ValueValidation);
    }

    #[test]
    fn build_launch_config_from_args_rejects_whitespace_only_identity() {
        let args = TuiArgs::try_parse_from(["kinic-tui", "--identity", "   "]).unwrap_err();

        assert_eq!(args.kind(), clap::error::ErrorKind::ValueValidation);
    }

    #[test]
    fn build_launch_config_from_args_uses_provided_identity() {
        let args = TuiArgs {
            identity: "alice".to_string(),
            ic: false,
        };

        let config = build_launch_config_from_args(&args).unwrap();
        assert!(matches!(config.auth, tui::TuiAuth::DeferredIdentity { .. }));
    }
}
