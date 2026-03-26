use std::{
    fmt,
    sync::{Arc, Mutex},
};

use anyhow::{Result, anyhow};
use ic_agent::Identity;
#[cfg(test)]
use ic_agent::identity::AnonymousIdentity;

use crate::agent::load_identity_from_keyring;
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
use tui_kit_runtime::{
    CoreState, CreateCostState, CreateSubmitState, DataProvider, PaneFocus, apply_snapshot,
    kinic_tabs,
};

pub use provider::TuiConfig;

#[derive(Clone)]
pub enum TuiAuth {
    Mock,
    DeferredIdentity {
        identity_name: String,
        cached_identity: Arc<Mutex<Option<Arc<dyn Identity>>>>,
    },
    ResolvedIdentity(Arc<dyn Identity>),
}

impl fmt::Debug for TuiAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mock => f.write_str("Mock"),
            Self::DeferredIdentity { identity_name, .. } => {
                write!(f, "DeferredIdentity({identity_name})")
            }
            Self::ResolvedIdentity(_) => f.write_str("ResolvedIdentity(..)"),
        }
    }
}

impl TuiAuth {
    pub fn is_live(&self) -> bool {
        !matches!(self, Self::Mock)
    }

    pub(crate) fn agent_factory(&self, use_mainnet: bool) -> Result<crate::agent::AgentFactory> {
        match self {
            Self::Mock => anyhow::bail!("mock auth cannot be used for live TUI operations"),
            Self::DeferredIdentity {
                identity_name,
                cached_identity,
            } => {
                if let Some(identity) = cached_identity
                    .lock()
                    .map_err(|_| anyhow!("cached identity mutex poisoned"))?
                    .as_ref()
                    .cloned()
                {
                    return Ok(crate::agent::AgentFactory::new_with_arc_identity(
                        use_mainnet,
                        identity,
                    ));
                }

                let identity = load_identity_from_keyring(identity_name)?;
                let mut cached = cached_identity
                    .lock()
                    .map_err(|_| anyhow!("cached identity mutex poisoned"))?;
                let resolved = cached.get_or_insert_with(|| identity.clone()).clone();
                Ok(crate::agent::AgentFactory::new_with_arc_identity(
                    use_mainnet,
                    resolved,
                ))
            }
            Self::ResolvedIdentity(identity) => Ok(
                crate::agent::AgentFactory::new_with_arc_identity(use_mainnet, identity.clone()),
            ),
        }
    }

    #[cfg(test)]
    pub(crate) fn resolved_for_tests() -> Self {
        Self::ResolvedIdentity(Arc::new(AnonymousIdentity {}))
    }
}

#[derive(Debug, Clone)]
pub struct TuiLaunchConfig {
    pub auth: TuiAuth,
    pub use_mainnet: bool,
}

pub fn run(global: &GlobalOpts) -> Result<()> {
    run_with_config(build_launch_config_from_global(global)?)
}

pub fn build_launch_config(identity: Option<String>, use_mainnet: bool) -> Result<TuiLaunchConfig> {
    Ok(TuiLaunchConfig {
        auth: resolve_auth(identity)?,
        use_mainnet,
    })
}

pub fn build_launch_config_from_global(global: &GlobalOpts) -> Result<TuiLaunchConfig> {
    Ok(TuiLaunchConfig {
        auth: resolve_auth(resolve_tui_identity(global)?)?,
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
        if state.create_submit_state == CreateSubmitState::Submitting
            || matches!(state.create_cost_state, CreateCostState::Loading)
        {
            state.create_spinner_frame = state.create_spinner_frame.wrapping_add(1);
        } else {
            state.create_spinner_frame = 0;
        }
        if let Some(output) = provider.poll_background(state) {
            if let Some(snapshot) = output.snapshot {
                apply_snapshot(state, snapshot);
            }
            execute_effects_to_status(state, output.effects);
        }
    }
}

fn resolve_auth(identity: Option<String>) -> Result<TuiAuth> {
    match identity {
        Some(identity) => Ok(TuiAuth::DeferredIdentity {
            identity_name: identity,
            cached_identity: Arc::new(Mutex::new(None)),
        }),
        None => Ok(TuiAuth::Mock),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolved_auth_is_live() {
        let auth = TuiAuth::resolved_for_tests();

        assert!(auth.is_live());
    }

    #[test]
    fn resolve_auth_falls_back_to_mock_without_credentials() {
        let auth = resolve_auth(None).unwrap();

        assert!(matches!(auth, TuiAuth::Mock));
    }

    #[test]
    fn build_launch_config_sets_use_mainnet_and_resolved_auth() {
        let config = build_launch_config(Some("alice".to_string()), true).unwrap();

        assert!(config.auth.is_live());
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

        assert!(matches!(config.auth, TuiAuth::Mock));
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
