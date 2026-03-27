use tui_kit_model::{UiItemKind, UiItemSummary, UiVisibility};
use tui_kit_render::theme::Theme;
use tui_kit_render::ui::{Focus, TuiKitUi, TabId, UiConfig};
use ratatui::{backend::TestBackend, Terminal};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend)?;
    let theme = Theme::default();

    let items = vec![UiItemSummary {
        id: "example:item:1".to_string(),
        name: "Inbox Item".to_string(),
        leading_marker: None,
        kind: UiItemKind::Custom("mail".to_string()),
        visibility: UiVisibility::Private,
        qualified_name: None,
        subtitle: Some("Sample item from external app".to_string()),
        tags: vec!["unread".to_string()],
    }];

    terminal.draw(|frame| {
        let ui = TuiKitUi::new(&theme)
            .ui_config(UiConfig::default())
            .ui_summaries(&items)
            .ui_total_count(items.len())
            .list_selected(Some(0))
            .search_input("inbox")
            .current_tab_id(TabId::new("tab-1"))
            .focus(Focus::List)
            .status_message("Rendered from tui-kit-render example")
            .show_help(false)
            .show_settings(false)
            .show_completion(false)
            .context_details_loading(false)
            .context_details_failed(false)
            .context_tree(&[])
            .filtered_context_indices(&[])
            .candidates(&[])
            .chat_messages(&[])
            .chat_input("")
            .chat_loading(false)
            .chat_scroll(0)
            .completion_selected(0)
            .in_context_items_view(false);

        frame.render_widget(ui, frame.area());
    })?;

    Ok(())
}
