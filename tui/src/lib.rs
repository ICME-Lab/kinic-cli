//! Workspace facade crate for running the standalone Kinic TUI.

use clap::Parser;
use kinic_core::tui::{build_launch_config, TuiLaunchConfig};

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
    Ok(build_launch_config(args.identity.clone(), args.ic))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_launch_config_from_args_maps_cli_fields() {
        let args = TuiArgs {
            identity: Some("alice".to_string()),
            ic: true,
        };

        let config = build_launch_config_from_args(&args).unwrap();
        assert_eq!(
            config.auth,
            tui::TuiAuth::KeyringIdentity("alice".to_string())
        );
        assert!(config.use_mainnet);
    }

    #[test]
    fn build_launch_config_from_args_uses_mock_without_identity() {
        let args = TuiArgs {
            identity: None,
            ic: false,
        };

        let config = build_launch_config_from_args(&args).unwrap();
        assert_eq!(config.auth, tui::TuiAuth::Mock);
    }
}
