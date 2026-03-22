use anyhow::{Result, anyhow};

use crate::{cli::GlobalOpts, tui_config::kinic_ui_config};

use tui_kit_host::{execute_effects_to_status, runtime_loop::{run_provider_app_with_hooks, RuntimeLoopConfig, RuntimeLoopHooks}};
use tui_kit_runtime::{apply_snapshot, CoreState, PaneFocus};

pub fn run(global: &GlobalOpts) -> Result<()> {
    let mut provider = kinic_tui::provider::KinicProvider::new(kinic_tui::provider::TuiConfig {
        identity: global.identity.clone(),
        use_mainnet: global.ic,
    });
    let mut hooks = KinicRuntimeHooks;

    run_provider_app_with_hooks(
        &mut provider,
        RuntimeLoopConfig {
            initial_tab_id: "",
            tab_ids: &[],
            initial_focus: PaneFocus::Search,
            ui_config: kinic_ui_config,
        },
        &mut hooks,
    )
    .map_err(|error| anyhow!(error.to_string()))
}

struct KinicRuntimeHooks;

impl RuntimeLoopHooks<kinic_tui::provider::KinicProvider> for KinicRuntimeHooks {
    fn on_tick(
        &mut self,
        provider: &mut kinic_tui::provider::KinicProvider,
        state: &mut CoreState,
    ) {
        if let Some(output) = provider.poll_background(state) {
            if let Some(snapshot) = output.snapshot {
                apply_snapshot(state, snapshot);
            }
            execute_effects_to_status(state, output.effects);
        }
    }
}

pub mod kinic_tui {
    pub use crate::app;

    pub mod adapter {
        include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tui/src/adapter.rs"));
    }

    pub mod provider {
        include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tui/src/provider.rs"));
    }
}
