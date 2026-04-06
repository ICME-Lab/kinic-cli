//! App module entrypoint: types, shared helpers, screens, and root render orchestration.

mod overlays;
mod render_root;
mod screens;
mod shared;
mod special_tabs;
mod types;
mod ui_builder;
mod ui_state;

pub use shared::layout::{
    list_viewport_height_for_area, list_viewport_height_for_area_with_tabs, tabs_rect_for_area,
    tabs_rect_for_area_with_tabs,
};
pub use types::{
    BrandingText, ChatPanelText, Focus, HeaderText, TabId, TabSpec, UiConfig, default_tab_specs,
};
pub use ui_state::TuiKitUi;
