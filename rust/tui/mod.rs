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
use crate::{TUI_IDENTITY_REQUIRED_MESSAGE, resolve_tui_identity};

mod adapter;
mod bridge;
mod chat_prompt;
mod provider;
mod settings;
mod ui_config;

use tui_kit_host::{
    execute_effects_to_status,
    picker::default_picker_backend,
    runtime_loop::{RuntimeLoopConfig, RuntimeLoopHooks, run_provider_app_with_hooks},
};
use tui_kit_runtime::{
    CoreState, CreateCostState, CreateSubmitState, DataProvider, PaneFocus, apply_snapshot,
    kinic_tabs,
};

pub use provider::TuiConfig;

#[derive(Clone)]
pub enum TuiAuth {
    DeferredIdentity {
        identity_name: String,
        cached_identity: Arc<Mutex<Option<Arc<dyn Identity>>>>,
    },
    ResolvedIdentity(Arc<dyn Identity>),
}

impl fmt::Debug for TuiAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeferredIdentity { identity_name, .. } => {
                write!(f, "DeferredIdentity({identity_name})")
            }
            Self::ResolvedIdentity(_) => f.write_str("ResolvedIdentity(..)"),
        }
    }
}

impl TuiAuth {
    fn resolved_identity(&self) -> Result<Arc<dyn Identity>> {
        match self {
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
                    return Ok(identity);
                }

                let identity = load_identity_from_keyring(identity_name)?;
                let mut cached = cached_identity
                    .lock()
                    .map_err(|_| anyhow!("cached identity mutex poisoned"))?;
                Ok(cached.get_or_insert_with(|| identity.clone()).clone())
            }
            Self::ResolvedIdentity(identity) => Ok(identity.clone()),
        }
    }

    pub(crate) fn principal_text(&self) -> Result<String> {
        self.resolved_identity()?
            .sender()
            .map(|principal| principal.to_text())
            .map_err(anyhow::Error::msg)
    }

    pub(crate) fn agent_factory(&self, use_mainnet: bool) -> Result<crate::agent::AgentFactory> {
        Ok(crate::agent::AgentFactory::new_with_arc_identity(
            use_mainnet,
            self.resolved_identity()?,
        ))
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

pub fn build_launch_config(identity: String, use_mainnet: bool) -> Result<TuiLaunchConfig> {
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
        file_picker: default_picker_backend(),
    }
}

struct KinicRuntimeHooks;

impl RuntimeLoopHooks<provider::KinicProvider> for KinicRuntimeHooks {
    fn on_tick(&mut self, provider: &mut provider::KinicProvider, state: &mut CoreState) {
        if state.create_submit_state == CreateSubmitState::Submitting
            || matches!(state.create_cost_state, CreateCostState::Loading)
            || state.insert_submit_state == CreateSubmitState::Submitting
        {
            state.create_spinner_frame = state.create_spinner_frame.wrapping_add(1);
            state.insert_spinner_frame = state.insert_spinner_frame.wrapping_add(1);
        } else {
            state.create_spinner_frame = 0;
            state.insert_spinner_frame = 0;
        }
        if let Some(output) = provider.poll_background(state) {
            if let Some(snapshot) = output.snapshot {
                apply_snapshot(state, snapshot);
            }
            execute_effects_to_status(state, output.effects);
        }
    }
}

fn resolve_auth(identity: String) -> Result<TuiAuth> {
    if identity.trim().is_empty() {
        return Err(anyhow!(TUI_IDENTITY_REQUIRED_MESSAGE));
    }

    Ok(TuiAuth::DeferredIdentity {
        identity_name: identity,
        cached_identity: Arc::new(Mutex::new(None)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_auth_requires_identity() {
        let error = resolve_auth(String::new()).unwrap_err();

        assert_eq!(error.to_string(), TUI_IDENTITY_REQUIRED_MESSAGE);
    }

    #[test]
    fn build_launch_config_sets_use_mainnet_and_resolved_auth() {
        let config = build_launch_config("alice".to_string(), true).unwrap();

        assert!(matches!(config.auth, TuiAuth::DeferredIdentity { .. }));
        assert!(config.use_mainnet);
    }

    #[test]
    fn build_launch_config_from_global_requires_identity() {
        let global = GlobalOpts {
            verbose: 0,
            ic: false,
            identity: None,
            ii: false,
            identity_path: None,
        };

        let error = build_launch_config_from_global(&global).unwrap_err();

        assert_eq!(error.to_string(), TUI_IDENTITY_REQUIRED_MESSAGE);
    }

    #[test]
    fn resolved_auth_principal_text_uses_identity_sender() {
        let auth = TuiAuth::resolved_for_tests();

        let principal = auth.principal_text().unwrap();

        assert_eq!(principal, "2vxsx-fae");
    }

    #[test]
    fn runtime_loop_config_uses_kinic_tabs() {
        let config = kinic_runtime_loop_config();
        assert_eq!(config.initial_tab_id, kinic_tabs::KINIC_MEMORIES_TAB_ID);
        assert_eq!(config.tab_ids, &kinic_tabs::KINIC_TAB_IDS);
        assert_eq!(config.initial_focus, PaneFocus::Search);
    }
}
