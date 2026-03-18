use anyhow::{Result, anyhow};

use crate::{cli::GlobalOpts, tui_config::kinic_ui_config};

use tui_kit_host::runtime_loop::{run_provider_app, RuntimeLoopConfig};

pub fn run(global: &GlobalOpts) -> Result<()> {
    let mut provider = kinic_tui::provider::KinicProvider::new(kinic_tui::provider::TuiConfig {
        identity: global.identity.clone(),
        use_mainnet: global.ic,
    });

    run_provider_app(
        &mut provider,
        RuntimeLoopConfig {
            initial_tab_id: "",
            tab_ids: &[],
            ui_config: kinic_ui_config,
        },
    )
    .map_err(|error| anyhow!(error.to_string()))
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
