use anyhow::{Result, anyhow};

use crate::cli::GlobalOpts;
use crate::resolve_tui_identity;

#[path = "../../tui/src/adapter.rs"]
mod adapter;
#[path = "../../tui/src/bridge.rs"]
mod bridge;
#[path = "../../tui/src/provider.rs"]
mod provider;
#[path = "../../tui/src/settings.rs"]
mod settings;
#[path = "../../tui/src/ui_config.rs"]
mod ui_config;

use tui_kit_host::{
    execute_effects_to_status,
    runtime_loop::{RuntimeLoopConfig, RuntimeLoopHooks, run_provider_app_with_hooks},
};
use tui_kit_runtime::{CoreState, PaneFocus, apply_snapshot, kinic_tabs};

pub use provider::TuiConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiAuth {
    Mock,
    KeyringIdentity(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiLaunchConfig {
    pub auth: TuiAuth,
    pub use_mainnet: bool,
}

pub fn run(global: &GlobalOpts) -> Result<()> {
    run_with_config(build_launch_config_from_global(global)?)
}

pub fn build_launch_config(identity: Option<String>, use_mainnet: bool) -> TuiLaunchConfig {
    TuiLaunchConfig {
        auth: resolve_auth(identity),
        use_mainnet,
    }
}

pub fn build_launch_config_from_global(global: &GlobalOpts) -> Result<TuiLaunchConfig> {
    let auth = match resolve_tui_identity(global)? {
        Some(identity) => TuiAuth::KeyringIdentity(identity),
        None => TuiAuth::Mock,
    };

    Ok(TuiLaunchConfig {
        auth,
        use_mainnet: global.ic,
    })
}

pub fn run_with_config(config: TuiLaunchConfig) -> Result<()> {
    let mut provider = provider::KinicProvider::new(TuiConfig {
        auth: config.auth,
        use_mainnet: config.use_mainnet,
    });
    let mut hooks = KinicRuntimeHooks;

    run_provider_app_with_hooks(&mut provider, kinic_runtime_loop_config(), &mut hooks)
        .map_err(|error| anyhow!(error.to_string()))
}

fn kinic_runtime_loop_config() -> RuntimeLoopConfig {
    RuntimeLoopConfig {
        initial_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID,
        tab_ids: &kinic_tabs::KINIC_TAB_IDS,
        initial_focus: PaneFocus::Search,
        ui_config: ui_config::kinic_ui_config,
    }
}

struct KinicRuntimeHooks;

impl RuntimeLoopHooks<provider::KinicProvider> for KinicRuntimeHooks {
    fn on_tick(&mut self, provider: &mut provider::KinicProvider, state: &mut CoreState) {
        if let Some(output) = provider.poll_background(state) {
            if let Some(snapshot) = output.snapshot {
                apply_snapshot(state, snapshot);
            }
            execute_effects_to_status(state, output.effects);
        }
    }
}

fn resolve_auth(identity: Option<String>) -> TuiAuth {
    match identity {
        Some(identity) => TuiAuth::KeyringIdentity(identity),
        None => TuiAuth::Mock,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_auth_uses_keyring_identity_when_present() {
        let auth = resolve_auth(Some("alice".to_string()));
        assert_eq!(auth, TuiAuth::KeyringIdentity("alice".to_string()));
    }

    #[test]
    fn resolve_auth_falls_back_to_mock_without_credentials() {
        let auth = resolve_auth(None);
        assert_eq!(auth, TuiAuth::Mock);
    }

    #[test]
    fn build_launch_config_sets_use_mainnet_and_resolved_auth() {
        let config = build_launch_config(Some("alice".to_string()), true);
        assert_eq!(config.auth, TuiAuth::KeyringIdentity("alice".to_string()));
        assert!(config.use_mainnet);
    }

    #[test]
    fn build_launch_config_from_global_uses_keyring_identity() {
        let global = GlobalOpts {
            verbose: 0,
            ic: true,
            identity: Some("alice".to_string()),
            ii: false,
            identity_path: None,
        };

        let config = build_launch_config_from_global(&global).unwrap();

        assert_eq!(config.auth, TuiAuth::KeyringIdentity("alice".to_string()));
        assert!(config.use_mainnet);
    }

    #[test]
    fn build_launch_config_from_global_uses_mock_without_identity() {
        let global = GlobalOpts {
            verbose: 0,
            ic: false,
            identity: None,
            ii: false,
            identity_path: None,
        };

        let config = build_launch_config_from_global(&global).unwrap();

        assert_eq!(config.auth, TuiAuth::Mock);
        assert!(!config.use_mainnet);
    }

    #[test]
    fn runtime_loop_config_uses_kinic_tabs() {
        let config = kinic_runtime_loop_config();
        assert_eq!(config.initial_tab_id, kinic_tabs::KINIC_MEMORIES_TAB_ID);
        assert_eq!(config.tab_ids, &kinic_tabs::KINIC_TAB_IDS);
        assert_eq!(config.initial_focus, PaneFocus::Search);
    }
}
