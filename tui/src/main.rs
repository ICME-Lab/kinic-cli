mod adapter;
mod provider;

use clap::Parser;
use provider::{KinicProvider, TuiConfig};
pub use tui_kit_lib::app;
use tui_kit_host::runtime_loop::{run_provider_app, RuntimeLoopConfig};
use tui_kit_render::ui::{BrandingText, HeaderText, UiConfig};

#[derive(Debug, Parser)]
#[command(name = "kinic-tui", about = "Kinic terminal UI")]
struct TuiArgs {
    #[arg(long, help = "Dfx identity name used to load credentials from the system keyring")]
    identity: Option<String>,

    #[arg(long, help = "Use the Internet Computer mainnet instead of local replica")]
    ic: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = TuiArgs::parse();
    let mut provider = KinicProvider::new(TuiConfig {
        identity: args.identity,
        use_mainnet: args.ic,
    });
    run_provider_app(
        &mut provider,
        RuntimeLoopConfig {
            initial_tab_id: "",
            tab_ids: &[],
            ui_config: kinic_ui_config,
        },
    )
}

fn kinic_ui_config() -> UiConfig {
    UiConfig {
        branding: BrandingText {
            logo_lines: vec![
                "██╗  ██╗ ██╗ ███╗   ██╗ ██╗  ██████╗".to_string(),
                "██║ ██╔╝ ██║ ████╗  ██║ ██║ ██╔════╝".to_string(),
                "█████╔╝  ██║ ██╔██╗ ██║ ██║ ██║".to_string(),
                "██╔═██╗  ██║ ██║╚██╗██║ ██║ ██║".to_string(),
                "██║  ██╗ ██║ ██║ ╚████║ ██║ ╚██████╗".to_string(),
                "╚═╝  ╚═╝ ╚═╝ ╚═╝  ╚═══╝ ╚═╝  ╚═════╝".to_string(),
            ],
            attribution: "kinic pink".to_string(),
        },
        header: HeaderText {
            visible_icon: "◆".to_string(),
            visible_suffix: "items".to_string(),
            contexts_icon: "◈".to_string(),
            contexts_suffix: "groups".to_string(),
            data_label: "cache".to_string(),
        },
        tabs: vec![],
        ..UiConfig::default()
    }
}
