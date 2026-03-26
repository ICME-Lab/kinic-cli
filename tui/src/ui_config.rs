use tui_kit_render::ui::{BrandingText, HeaderText, TabId, TabSpec, UiConfig};
pub use tui_kit_runtime::kinic_tabs::{
    KINIC_CREATE_TAB_ID, KINIC_INSERT_TAB_ID, KINIC_MARKET_TAB_ID, KINIC_MEMORIES_TAB_ID,
    KINIC_SETTINGS_TAB_ID,
};

pub fn kinic_ui_config() -> UiConfig {
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
            attribution: String::new(),
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
                id: TabId::new(KINIC_MEMORIES_TAB_ID),
                title: "Memories".to_string(),
                search_placeholder: "Search memories...".to_string(),
            },
            TabSpec {
                id: TabId::new(KINIC_INSERT_TAB_ID),
                title: "Insert".to_string(),
                search_placeholder: "Insert text, embeddings, or PDFs...".to_string(),
            },
            TabSpec {
                id: TabId::new(KINIC_CREATE_TAB_ID),
                title: "Create".to_string(),
                search_placeholder: "Create a memory...".to_string(),
            },
            TabSpec {
                id: TabId::new(KINIC_MARKET_TAB_ID),
                title: "Market".to_string(),
                search_placeholder: "Market is coming soon...".to_string(),
            },
            TabSpec {
                id: TabId::new(KINIC_SETTINGS_TAB_ID),
                title: "Settings".to_string(),
                search_placeholder: "Adjust TUI settings...".to_string(),
            },
        ],
        ..UiConfig::default()
    }
}
