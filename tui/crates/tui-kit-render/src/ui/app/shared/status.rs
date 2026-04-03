//! Status bar (footer) block: context-specific commands and counts.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};
use tui_kit_runtime::{
    current_form_focus_for_tab, form_shows_horizontal_change_hint,
    kinic_tabs::{KINIC_MEMORIES_TAB_ID, KINIC_SETTINGS_TAB_ID, TabKind, tab_kind},
    settings_row_behavior_for_index,
};
use unicode_width::UnicodeWidthStr;

use crate::ui::app::{Focus, TuiKitUi};

impl<'a> TuiKitUi<'a> {
    pub(crate) fn render_status(&self, area: Rect, buf: &mut Buffer) {
        let tab_id = self.current_tab_id.0.as_str();
        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(self.theme.style_border())
            .style(Style::default().bg(self.theme.bg_panel));
        let inner = block.inner(area);

        let status_line = if matches!(tab_kind(tab_id), TabKind::InsertForm | TabKind::CreateForm) {
            self.form_status_line(tab_id)
        } else if matches!(
            tab_kind(tab_id),
            TabKind::PlaceholderMarket | TabKind::PlaceholderSettings
        ) {
            self.placeholder_status_line(tab_id)
        } else if self.show_context_panel && self.in_context_items_view {
            self.context_items_status_line()
        } else if self.show_context_panel {
            self.context_status_line()
        } else if !self.status_message.is_empty() {
            self.generic_status_line()
        } else {
            self.default_status_line()
        };

        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(self.theme.style_border())
            .style(Style::default().bg(self.theme.bg_panel));
        let inner = block.inner(area);
        block.render(area, buf);
        Paragraph::new(status_line)
            .alignment(Alignment::Left)
            .render(inner, buf);
    }

    fn generic_status_line(&self) -> Line<'_> {
        let cfg = &self.ui_config.status;
        let (icon, label) = self.focus_indicator();
        let tab_id = self.current_tab_id.0.as_str();
        let mut spans = vec![
            Span::styled(
                format!(" {} ", self.status_message),
                self.theme.style_string(),
            ),
            Span::styled(" │ ", self.theme.style_muted()),
        ];
        if show_memories_search_scope_hint(tab_id, self.focus) {
            spans.extend([
                Span::styled("←/→", self.theme.style_accent()),
                Span::styled(" scope ", self.theme.style_muted()),
                Span::styled("Enter", self.theme.style_accent()),
                Span::styled(" search ", self.theme.style_muted()),
                Span::styled("│ ", self.theme.style_dim()),
            ]);
        }
        spans.extend([
            Span::styled("Tab", self.theme.style_accent()),
            Span::styled(" focus ", self.theme.style_muted()),
            Span::styled("↑/↓ Enter / ", self.theme.style_accent()),
            Span::styled(
                format!("? help q {} ", cfg.quit_label),
                self.theme.style_muted(),
            ),
            Span::styled("│ ", self.theme.style_dim()),
            Span::styled(icon, self.theme.style_accent()),
            Span::styled(format!(" {}", label), self.theme.style_dim()),
        ]);
        let suffix_width = line_width(&suffix_spans);
        let message_width = max_width.saturating_sub(suffix_width);
        let mut spans = status_message_prefix(
            &self.status_message,
            message_width,
            self.theme.style_string(),
            self.theme.style_muted(),
        );
        spans.extend(suffix_spans);
        Line::from(spans)
    }

    fn form_status_line(&self, _tab_id: &str) -> Line<'_> {
        let cfg = &self.ui_config.status;
        let (icon, label) = self.focus_indicator();
        let tab_label = self
            .tab_specs
            .iter()
            .find(|tab| tab.id == self.current_tab_id)
            .map(|tab| tab.title.as_str())
            .unwrap_or("Form");
        let mut spans = vec![
            Span::styled(tab_label, self.theme.style_accent_bold()),
            Span::styled(" │ ", self.theme.style_dim()),
            Span::styled("↑/↓", self.theme.style_accent()),
            Span::styled(" or ", self.theme.style_muted()),
            Span::styled("Tab/Shift+Tab", self.theme.style_accent()),
            Span::styled(" fields ", self.theme.style_muted()),
        ];
        if show_form_change_shortcut(self) {
            spans.extend([
                Span::styled("←/→", self.theme.style_accent()),
                Span::styled(" change ", self.theme.style_muted()),
            ]);
        }
        spans.extend([
            Span::styled("Enter", self.theme.style_accent()),
            Span::styled(" action ", self.theme.style_muted()),
            Span::styled("1-5", self.theme.style_accent()),
            Span::styled(format!(" {} ", cfg.tabs_label), self.theme.style_muted()),
            Span::styled("│ ", self.theme.style_dim()),
            Span::styled(icon, self.theme.style_accent()),
            Span::styled(format!(" {}", label), self.theme.style_dim()),
        ]);
        if self.focus == Focus::Form {
            let insert_at = 6;
            spans.splice(
                insert_at..insert_at,
                [
                    Span::styled("Esc", self.theme.style_accent()),
                    Span::styled(" tabs ", self.theme.style_muted()),
                ],
            );
        }

        prepend_status_message(self, spans)
    }

    fn placeholder_status_line(&self, tab_id: &str) -> Line<'_> {
        if tab_id == KINIC_SETTINGS_TAB_ID && self.focus == Focus::Content {
            let enter_hint = self
                .list_selected
                .and_then(|index| {
                    self.settings_snapshot
                        .and_then(|snapshot| settings_row_behavior_for_index(snapshot, index))
                })
                .map(|behavior| behavior.status_hint)
                .unwrap_or(" open Default memory ");
            return prepend_status_message(
                self,
                vec![
                    Span::styled("↑/↓", self.theme.style_accent()),
                    Span::styled(" choose row ", self.theme.style_muted()),
                    Span::styled("Enter", self.theme.style_accent()),
                    Span::styled(enter_hint, self.theme.style_muted()),
                    Span::styled("Esc", self.theme.style_accent()),
                    Span::styled(" tabs ", self.theme.style_muted()),
                    Span::styled("│ ", self.theme.style_dim()),
                    Span::styled("Shift+D", self.theme.style_accent()),
                    Span::styled(
                        " saves current memory from Memories",
                        self.theme.style_muted(),
                    ),
                ],
            );
        }

        let cfg = &self.ui_config.status;
        let (icon, label) = self.focus_indicator();
        let mut spans = vec![
            Span::styled("Tab", self.theme.style_accent()),
            Span::styled(" switch ", self.theme.style_muted()),
            Span::styled("1-5", self.theme.style_accent()),
            Span::styled(format!(" {} ", cfg.tabs_label), self.theme.style_muted()),
            Span::styled("│ ", self.theme.style_dim()),
            Span::styled(icon, self.theme.style_accent()),
            Span::styled(format!(" {}", label), self.theme.style_dim()),
        ];
        if self.focus == Focus::Content {
            spans.splice(
                4..4,
                [
                    Span::styled("Esc", self.theme.style_accent()),
                    Span::styled(" tabs ", self.theme.style_muted()),
                ],
            );
        }

        prepend_status_message(self, spans)
    }

    fn context_items_status_line(&self) -> Line<'_> {
        let selection_info = self.selection_info();
        let context_name = self
            .ui_context_node
            .map(|n| n.name.clone())
            .unwrap_or_else(|| "context".to_string());
        let context_ver = self
            .ui_context_node
            .and_then(|n| n.version.clone())
            .unwrap_or_else(|| "?".to_string());

        prepend_status_message(
            self,
            vec![
                Span::styled(" ◇ ", self.theme.style_accent()),
                Span::styled(context_name, self.theme.style_normal()),
                Span::styled(format!(" v{}", context_ver), self.theme.style_dim()),
                Span::styled(" │ ", self.theme.style_muted()),
                Span::styled(selection_info, self.theme.style_muted()),
                Span::styled(" │ Tab ↑/↓ Enter / Esc back ", self.theme.style_muted()),
            ],
        )
    }

    fn context_status_line(&self) -> Line<'_> {
        let cfg = &self.ui_config.status;

        prepend_status_message(
            self,
            vec![
                Span::styled(format!("{}: ", cfg.commands_label), self.theme.style_dim()),
                Span::styled("Tab", self.theme.style_accent()),
                Span::styled(" focus ", self.theme.style_muted()),
                Span::styled("↑/↓", self.theme.style_accent()),
                Span::styled(" list ", self.theme.style_muted()),
                Span::styled("Enter →", self.theme.style_accent()),
                Span::styled(" open ", self.theme.style_muted()),
                Span::styled("/", self.theme.style_accent()),
                Span::styled(" search ", self.theme.style_muted()),
                Span::styled("│ ", self.theme.style_dim()),
                Span::styled(
                    format!("◇ Context ({}) ", self.filtered_context_indices.len()),
                    self.theme.style_normal(),
                ),
            ],
        )
    }

    fn default_status_line(&self) -> Line<'_> {
        let cfg = &self.ui_config.status;
        let selection_info = self.selection_info();
        let tab_id = self.current_tab_id.0.as_str();
        let mut spans = vec![Span::styled(
            format!("{}: ", cfg.commands_label),
            self.theme.style_dim(),
        )];
        if show_memories_search_scope_hint(tab_id, self.focus) {
            spans.extend([
                Span::styled("←/→", self.theme.style_accent()),
                Span::styled(" scope ", self.theme.style_muted()),
                Span::styled("Enter", self.theme.style_accent()),
                Span::styled(" search ", self.theme.style_muted()),
            ]);
        }
        spans.extend([
            Span::styled("Tab", self.theme.style_accent()),
            Span::styled(" focus ", self.theme.style_muted()),
            Span::styled("↑/↓", self.theme.style_accent()),
            Span::styled(" list ", self.theme.style_muted()),
            Span::styled("Enter →", self.theme.style_accent()),
            Span::styled(" open ", self.theme.style_muted()),
            Span::styled("/", self.theme.style_accent()),
            Span::styled(" search ", self.theme.style_muted()),
        ]);
        if !self.tab_specs.is_empty() {
            spans.push(Span::styled("1-5", self.theme.style_accent()));
            spans.push(Span::styled(
                format!(" {} ", cfg.tabs_label),
                self.theme.style_muted(),
            ));
        }
        spans.extend([
            Span::styled("? ", self.theme.style_accent()),
            Span::styled("help ", self.theme.style_muted()),
            Span::styled("q ", self.theme.style_accent()),
            Span::styled(format!("{} ", cfg.quit_label), self.theme.style_muted()),
            Span::styled("│ ", self.theme.style_dim()),
            Span::styled("visible ", self.theme.style_muted()),
            Span::styled(
                format!("{}/{} ", self.ui_summaries.len(), self.ui_total_count),
                self.theme.style_normal(),
            ),
            Span::styled("selection ", self.theme.style_muted()),
            Span::styled(selection_info, self.theme.style_dim()),
        ]);

        Line::from(spans)
    }

    fn focus_indicator(&self) -> (&'static str, &'static str) {
        match self.focus {
            Focus::Search => ("🔍", "Search"),
            Focus::Items => ("📋", "Items"),
            Focus::Tabs => ("🗂", "Tabs"),
            Focus::Content => ("🔬", "Content"),
            Focus::Form => ("✍", "Form"),
            Focus::Chat => ("💬", "Chat"),
        }
    }

    fn selection_info(&self) -> String {
        if let Some(selected) = self.list_selected {
            format!("[{}/{}]", selected + 1, self.ui_summaries.len())
        } else {
            format!("[0/{}]", self.ui_summaries.len())
        }
    }
}

fn show_memories_search_scope_hint(tab_id: &str, focus: Focus) -> bool {
    tab_id == KINIC_MEMORIES_TAB_ID && focus == Focus::Search
}

fn show_form_change_shortcut(ui: &TuiKitUi<'_>) -> bool {
    current_form_focus_for_tab(
        ui.current_tab_id.0.as_str(),
        ui.create_focus,
        ui.insert_focus,
    )
    .is_some_and(|focus| form_shows_horizontal_change_hint(ui.focus == Focus::Form, focus))
}

fn prepend_status_message<'a>(ui: &'a TuiKitUi<'a>, mut spans: Vec<Span<'a>>) -> Line<'a> {
    if ui.status_message.is_empty() {
        return Line::from(spans);
    }

    let mut prefixed = vec![
        Span::styled(format!(" {} ", ui.status_message), ui.theme.style_string()),
        Span::styled(" │ ", ui.theme.style_muted()),
    ];
    prefixed.append(&mut spans);
    Line::from(prefixed)
}

fn line_width(spans: &[Span<'_>]) -> u16 {
    spans
        .iter()
        .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
        .sum::<usize>()
        .min(u16::MAX as usize) as u16
}

fn status_message_prefix<'a>(
    message: &str,
    max_width: u16,
    message_style: Style,
    separator_style: Style,
) -> Vec<Span<'a>> {
    if message.is_empty() || max_width == 0 {
        return Vec::new();
    }

    let separator = " │ ";
    let separator_width = UnicodeWidthStr::width(separator).min(u16::MAX as usize) as u16;
    if max_width <= separator_width {
        return vec![Span::styled(
            trim_to_width(message, max_width),
            message_style,
        )];
    }

    vec![
        Span::styled(
            format!(
                " {} ",
                trim_to_width(message, max_width - separator_width - 2)
            ),
            message_style,
        ),
        Span::styled(separator, separator_style),
    ]
}

fn trim_to_width(value: &str, max_width: u16) -> String {
    if max_width == 0 || UnicodeWidthStr::width(value) as u16 <= max_width {
        return value.to_string();
    }

    let mut trimmed = String::new();
    for ch in value.chars() {
        let next = format!("{trimmed}{ch}");
        if UnicodeWidthStr::width(next.as_str()) as u16 >= max_width.saturating_sub(1) {
            break;
        }
        trimmed.push(ch);
    }
    format!("{trimmed}…")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::Theme;
    use tui_kit_runtime::InsertFormFocus;
    use tui_kit_runtime::kinic_tabs::{
        KINIC_CREATE_TAB_ID, KINIC_INSERT_TAB_ID, KINIC_SETTINGS_TAB_ID,
    };

    fn render_status_line(ui: &TuiKitUi<'_>) -> String {
        let area = Rect::new(0, 0, 160, 3);
        let mut buf = Buffer::empty(area);
        ui.render_status(area, &mut buf);
        (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn insert_tab_enter_hint_mentions_picker_and_submit() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .current_tab_id(crate::ui::TabId::new(KINIC_INSERT_TAB_ID))
            .focus(Focus::Form);
        let rendered = render_status_line(&ui);

        assert!(rendered.contains("Enter"));
        assert!(rendered.contains("action"));
    }

    #[test]
    fn change_shortcut_is_shown_for_insert_mode_only() {
        let theme = Theme::default();
        let insert_mode_ui = TuiKitUi::new(&theme)
            .current_tab_id(crate::ui::TabId::new(KINIC_INSERT_TAB_ID))
            .focus(Focus::Form)
            .insert_focus(InsertFormFocus::Mode);
        let create_ui = TuiKitUi::new(&theme)
            .current_tab_id(crate::ui::TabId::new(KINIC_CREATE_TAB_ID))
            .focus(Focus::Form);

        assert!(show_form_change_shortcut(&insert_mode_ui));
        assert!(!show_form_change_shortcut(&create_ui));
    }

    #[test]
    fn memories_scope_hint_is_search_focus_only() {
        assert!(show_memories_search_scope_hint(
            KINIC_MEMORIES_TAB_ID,
            Focus::Search
        ));
        assert!(!show_memories_search_scope_hint(
            KINIC_MEMORIES_TAB_ID,
            Focus::Items
        ));
        assert!(!show_memories_search_scope_hint(
            KINIC_INSERT_TAB_ID,
            Focus::Search
        ));
    }

    #[test]
    fn insert_form_status_keeps_hints_when_status_message_exists() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .current_tab_id(crate::ui::TabId::new(KINIC_INSERT_TAB_ID))
            .focus(Focus::Form)
            .status_message("Inserted 12 chunks");
        let rendered = render_status_line(&ui);

        assert!(rendered.contains("Inserted 12 chunks"));
        assert!(rendered.contains("Enter"));
        assert!(rendered.contains("action"));
        assert!(rendered.contains("change"));
        assert!(rendered.contains("Tab/Shift+Tab"));
        assert!(rendered.contains("Esc"));
    }

    #[test]
    fn create_form_status_keeps_hints_when_status_message_exists() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .current_tab_id(crate::ui::TabId::new(KINIC_CREATE_TAB_ID))
            .focus(Focus::Form)
            .status_message("Creating memory...");
        let rendered = render_status_line(&ui);

        assert!(rendered.contains("Creating memory..."));
        assert!(rendered.contains("↑/↓"));
        assert!(rendered.contains("Tab/Shift+Tab"));
        assert!(rendered.contains("action"));
        assert!(rendered.contains("Esc"));
    }

    #[test]
    fn settings_content_status_keeps_settings_hints_when_status_message_exists() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .current_tab_id(crate::ui::TabId::new(KINIC_SETTINGS_TAB_ID))
            .focus(Focus::Content)
            .status_message("Review session details");
        let rendered = render_status_line(&ui);

        assert!(rendered.contains("Review session details"));
        assert!(rendered.contains("open Default memory"));
        assert!(rendered.contains("Esc"));
        assert!(rendered.contains("Shift+D"));
    }

    #[test]
    fn non_form_tabs_render_generic_status_message() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .current_tab_id(crate::ui::TabId::new("tab-1"))
            .focus(Focus::Items)
            .status_message("Loaded memories.");
        let rendered = render_status_line(&ui);

        assert!(rendered.contains("Loaded memories."));
        assert!(rendered.contains("? help q"));
    }

    #[test]
    fn context_panel_status_keeps_context_hints_when_status_message_exists() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .show_context_panel(true)
            .filtered_context_indices(&[0, 1, 2])
            .status_message("Review dependency details");
        let rendered = render_status_line(&ui);

        assert!(rendered.contains("Review dependency details"));
        assert!(rendered.contains("open"));
        assert!(rendered.contains("search"));
        assert!(rendered.contains("Context (3)"));
    }

    #[test]
    fn context_items_status_keeps_back_hint_when_status_message_exists() {
        let theme = Theme::default();
        let context_node = crate::ui::model::UiContextNode {
            name: "serde".to_string(),
            version: Some("1.0.0".to_string()),
            ..Default::default()
        };
        let ui = TuiKitUi::new(&theme)
            .show_context_panel(true)
            .in_context_items_view(true)
            .ui_context_node(Some(&context_node))
            .ui_summaries(&[])
            .status_message("Inspecting package items");
        let rendered = render_status_line(&ui);

        assert!(rendered.contains("Inspecting package items"));
        assert!(rendered.contains("serde"));
        assert!(rendered.contains("Esc back"));
    }

    #[test]
    fn insert_form_without_status_message_still_shows_static_hints() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .current_tab_id(crate::ui::TabId::new(KINIC_INSERT_TAB_ID))
            .focus(Focus::Form);
        let rendered = render_status_line(&ui);

        assert!(rendered.contains("↑/↓"));
        assert!(rendered.contains("Tab/Shift+Tab"));
        assert!(rendered.contains("Enter"));
        assert!(rendered.contains("action"));
        assert!(!rendered.contains("│  │"));
    }
}
