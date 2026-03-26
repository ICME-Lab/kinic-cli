mod provider;

use provider::KinicProvider;
use tui_kit_host::runtime_loop::{run_provider_app, RuntimeLoopConfig};
use tui_kit_render::ui::{BrandingText, HeaderText, TabId, TabSpec, UiConfig};
use tui_kit_runtime::PaneFocus;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut provider = KinicProvider::sample();
    run_provider_app(
        &mut provider,
        RuntimeLoopConfig {
            initial_tab_id: "kinic-memories",
            tab_ids: &[
                "kinic-memories",
                "kinic-insert",
                "kinic-create",
                "kinic-market",
                "kinic-settings",
            ],
            initial_focus: PaneFocus::Items,
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
            attribution: "kinic demo".to_string(),
        },
        header: HeaderText {
            visible_icon: "◆".to_string(),
            visible_suffix: "items".to_string(),
            contexts_icon: "◈".to_string(),
            contexts_suffix: "groups".to_string(),
            data_label: "cache".to_string(),
        },
        tabs: vec![
            TabSpec {
                id: TabId::new("kinic-memories"),
                title: "Memories".to_string(),
                search_placeholder: "Search memories...".to_string(),
            },
            TabSpec {
                id: TabId::new("kinic-insert"),
                title: "Insert".to_string(),
                search_placeholder: "Insert content...".to_string(),
            },
            TabSpec {
                id: TabId::new("kinic-create"),
                title: "Create".to_string(),
                search_placeholder: "Search create...".to_string(),
            },
            TabSpec {
                id: TabId::new("kinic-market"),
                title: "Market".to_string(),
                search_placeholder: "Search market...".to_string(),
            },
            TabSpec {
                id: TabId::new("kinic-settings"),
                title: "Settings".to_string(),
                search_placeholder: "Search settings...".to_string(),
            },
        ],
        ..UiConfig::default()
    }
}
