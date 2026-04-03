use super::*;
use tui_kit_model::{UiItemKind, UiItemSummary, UiVisibility};
use tui_kit_runtime::{CoreState, PaneFocus, dispatch_action, kinic_tabs};

fn two_stub_list_items() -> Vec<UiItemSummary> {
    let stub = |id: &str| UiItemSummary {
        id: id.to_string(),
        name: "stub".to_string(),
        leading_marker: None,
        kind: UiItemKind::Custom("x".to_string()),
        visibility: UiVisibility::Private,
        qualified_name: None,
        subtitle: None,
        tags: vec![],
    };
    vec![stub("stub-0"), stub("stub-1")]
}

fn provider_two_memories_active_last() -> (KinicProvider, CoreState) {
    let mut provider = KinicProvider::new(live_config());
    provider.tab_id = kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string();
    provider.memory_summaries = vec![
        running_memory_summary("aaaaa-aa", "alpha"),
        running_memory_summary("bbbbb-bb", "beta"),
    ];
    provider.memory_records = vec![
        live_memory("aaaaa-aa", "Alpha"),
        live_memory("bbbbb-bb", "Beta"),
    ];
    provider.all = provider.memory_records.clone();
    provider.active_memory_id = Some("bbbbb-bb".to_string());
    let state = CoreState {
        current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Items,
        selected_index: Some(1),
        list_items: two_stub_list_items(),
        ..CoreState::default()
    };
    (provider, state)
}

fn provider_two_memories_active_first() -> (KinicProvider, CoreState) {
    let mut provider = KinicProvider::new(live_config());
    provider.tab_id = kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string();
    provider.memory_summaries = vec![
        running_memory_summary("aaaaa-aa", "alpha"),
        running_memory_summary("bbbbb-bb", "beta"),
    ];
    provider.memory_records = vec![
        live_memory("aaaaa-aa", "Alpha"),
        live_memory("bbbbb-bb", "Beta"),
    ];
    provider.all = provider.memory_records.clone();
    provider.active_memory_id = Some("aaaaa-aa".to_string());
    let state = CoreState {
        current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Items,
        selected_index: Some(0),
        list_items: two_stub_list_items(),
        ..CoreState::default()
    };
    (provider, state)
}

#[test]
fn memories_browser_move_next_wraps_active_memory_to_first() {
    let (mut provider, mut state) = provider_two_memories_active_last();
    let _ = dispatch_action(&mut provider, &mut state, &CoreAction::MoveNext)
        .expect("move next should dispatch");
    assert_eq!(provider.active_memory_id.as_deref(), Some("aaaaa-aa"));
    assert_eq!(state.selected_index, Some(0));
}

#[test]
fn memories_browser_move_prev_wraps_active_memory_to_last() {
    let (mut provider, mut state) = provider_two_memories_active_first();
    let _ = dispatch_action(&mut provider, &mut state, &CoreAction::MovePrev)
        .expect("move prev should dispatch");
    assert_eq!(provider.active_memory_id.as_deref(), Some("bbbbb-bb"));
    assert_eq!(state.selected_index, Some(1));
}
