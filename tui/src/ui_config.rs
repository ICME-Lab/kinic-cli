use tui_kit_render::ui::{BrandingText, HeaderText, UiConfig};

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
        tabs: vec![],
        ..UiConfig::default()
    }
}
