//! Shared Kinic TUI tab identifiers used across host, runtime, render, and app glue.

pub const KINIC_MEMORIES_TAB_ID: &str = "kinic-memories";
pub const KINIC_CREATE_TAB_ID: &str = "kinic-create";
pub const KINIC_MARKET_TAB_ID: &str = "kinic-market";
pub const KINIC_SETTINGS_TAB_ID: &str = "kinic-settings";

pub const KINIC_TAB_IDS: [&str; 4] = [
    KINIC_MEMORIES_TAB_ID,
    KINIC_CREATE_TAB_ID,
    KINIC_MARKET_TAB_ID,
    KINIC_SETTINGS_TAB_ID,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabKind {
    Memories,
    Form,
    PlaceholderMarket,
    PlaceholderSettings,
    Unknown,
}

pub fn tab_kind(tab_id: &str) -> TabKind {
    match tab_id {
        KINIC_MEMORIES_TAB_ID => TabKind::Memories,
        KINIC_CREATE_TAB_ID => TabKind::Form,
        KINIC_MARKET_TAB_ID => TabKind::PlaceholderMarket,
        KINIC_SETTINGS_TAB_ID => TabKind::PlaceholderSettings,
        _ => TabKind::Unknown,
    }
}

pub fn is_form_tab(tab_id: &str) -> bool {
    matches!(tab_kind(tab_id), TabKind::Form)
}

pub fn is_kinic_memories_tab(tab_id: &str) -> bool {
    matches!(tab_kind(tab_id), TabKind::Memories)
}

pub fn is_kinic_market_tab(tab_id: &str) -> bool {
    matches!(tab_kind(tab_id), TabKind::PlaceholderMarket)
}

pub fn is_kinic_settings_tab(tab_id: &str) -> bool {
    matches!(tab_kind(tab_id), TabKind::PlaceholderSettings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinic_tab_helpers_match_expected_ids() {
        assert_eq!(tab_kind(KINIC_MEMORIES_TAB_ID), TabKind::Memories);
        assert_eq!(tab_kind(KINIC_CREATE_TAB_ID), TabKind::Form);
        assert_eq!(tab_kind(KINIC_MARKET_TAB_ID), TabKind::PlaceholderMarket);
        assert_eq!(
            tab_kind(KINIC_SETTINGS_TAB_ID),
            TabKind::PlaceholderSettings
        );
        assert_eq!(tab_kind("unknown"), TabKind::Unknown);
        assert!(is_form_tab(KINIC_CREATE_TAB_ID));
        assert!(is_kinic_memories_tab(KINIC_MEMORIES_TAB_ID));
        assert!(is_kinic_market_tab(KINIC_MARKET_TAB_ID));
        assert!(is_kinic_settings_tab(KINIC_SETTINGS_TAB_ID));
        assert!(!is_form_tab(KINIC_MEMORIES_TAB_ID));
    }
}
