//! Workspace facade crate for running the standalone Kinic TUI.

use clap::Parser;
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
        help = "Dfx identity name used to load credentials from the system keyring"
    )]
    pub identity: Option<String>,

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
        let args = TuiArgs {
            identity: None,
            ic: true,
        };

        let config = build_launch_config_from_args(&args).unwrap();

        assert!(matches!(config.auth, tui::TuiAuth::Mock));
        assert!(config.use_mainnet);
    }

    #[test]
    fn build_launch_config_from_args_uses_mock_without_identity() {
        let args = TuiArgs {
            identity: None,
            ic: false,
        };

        let config = build_launch_config_from_args(&args).unwrap();
        assert!(matches!(config.auth, tui::TuiAuth::Mock));
    }
}
