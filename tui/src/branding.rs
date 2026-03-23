use tui_kit_render::ui::{BrandingText, HeaderText};

pub fn kinic_branding() -> BrandingText {
    BrandingText {
        logo_lines: vec![
            "██╗  ██╗ ██╗ ███╗   ██╗ ██╗  ██████╗".to_string(),
            "██║ ██╔╝ ██║ ████╗  ██║ ██║ ██╔════╝".to_string(),
            "█████╔╝  ██║ ██╔██╗ ██║ ██║ ██║".to_string(),
            "██╔═██╗  ██║ ██║╚██╗██║ ██║ ██║".to_string(),
            "██║  ██╗ ██║ ██║ ╚████║ ██║ ╚██████╗".to_string(),
            "╚═╝  ╚═╝ ╚═╝ ╚═╝  ╚═══╝ ╚═╝  ╚═════╝".to_string(),
        ],
        attribution: String::new(),
    }
}

pub fn kinic_header() -> HeaderText {
    HeaderText {
        visible_icon: "◆".to_string(),
        visible_suffix: "items".to_string(),
        contexts_icon: "◈".to_string(),
        contexts_suffix: "groups".to_string(),
        data_label: "cache".to_string(),
    }
}
