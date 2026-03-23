use super::branding::{kinic_branding, kinic_header};
use tui_kit_render::ui::UiConfig;

pub fn kinic_ui_config() -> UiConfig {
    UiConfig {
        branding: kinic_branding(),
        header: kinic_header(),
        tabs: vec![],
        ..UiConfig::default()
    }
}
