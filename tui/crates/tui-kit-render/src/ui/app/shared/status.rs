//! Status bar (footer) block: context-specific commands and counts.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};
use tui_kit_runtime::kinic_tabs::{KINIC_INSERT_TAB_ID, KINIC_SETTINGS_TAB_ID, TabKind, tab_kind};

use crate::ui::app::{Focus, TuiKitUi, types::insert_form_copy};

impl<'a> TuiKitUi<'a> {
    pub(crate) fn render_status(&self, area: Rect, buf: &mut Buffer) {
        let tab_id = self.current_tab_id.0.as_str();

        let status_line = if matches!(tab_kind(tab_id), TabKind::InsertForm | TabKind::CreateForm) {
            self.form_status_line(tab_id)
        } else if matches!(
            tab_kind(tab_id),
            TabKind::PlaceholderMarket | TabKind::PlaceholderSettings
        ) {
            self.placeholder_status_line(tab_id)
        } else if !self.status_message.is_empty() {
            self.generic_status_line()
        } else if self.show_context_panel && self.in_context_items_view {
            self.context_items_status_line()
        } else if self.show_context_panel {
            self.context_status_line()
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

        Line::from(vec![
            Span::styled(
                format!(" {} ", self.status_message),
                self.theme.style_string(),
            ),
            Span::styled(" │ ", self.theme.style_muted()),
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
        ])
    }

    fn form_status_line(&self, tab_id: &str) -> Line<'_> {
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
        ];
        if show_form_mode_shortcut(tab_id) {
            spans.extend([
                Span::styled("←/→", self.theme.style_accent()),
                Span::styled(" mode ", self.theme.style_muted()),
            ]);
        }
        spans.extend([
            Span::styled("Tab/Shift+Tab", self.theme.style_accent()),
            Span::styled(" fields ", self.theme.style_muted()),
            Span::styled("Enter", self.theme.style_accent()),
            Span::styled(form_enter_hint(tab_id), self.theme.style_muted()),
            Span::styled("1-5", self.theme.style_accent()),
            Span::styled(format!(" {} ", cfg.tabs_label), self.theme.style_muted()),
            Span::styled("│ ", self.theme.style_dim()),
            Span::styled(icon, self.theme.style_accent()),
            Span::styled(format!(" {}", label), self.theme.style_dim()),
        ]);
        if self.focus == Focus::Form {
            let insert_at = if show_form_mode_shortcut(tab_id) {
                8
            } else {
                4
            };
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
            return prepend_status_message(
                self,
                vec![
                    Span::styled("↑/↓", self.theme.style_accent()),
                    Span::styled(" choose row ", self.theme.style_muted()),
                    Span::styled("Enter", self.theme.style_accent()),
                    Span::styled(" open Default memory ", self.theme.style_muted()),
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

        Line::from(vec![
            Span::styled(" ◇ ", self.theme.style_accent()),
            Span::styled(context_name, self.theme.style_normal()),
            Span::styled(format!(" v{}", context_ver), self.theme.style_dim()),
            Span::styled(" │ ", self.theme.style_muted()),
            Span::styled(selection_info, self.theme.style_muted()),
            Span::styled(" │ Tab ↑/↓ Enter / Esc back ", self.theme.style_muted()),
        ])
    }

    fn context_status_line(&self) -> Line<'_> {
        let cfg = &self.ui_config.status;

        Line::from(vec![
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
        ])
    }

    fn default_status_line(&self) -> Line<'_> {
        let cfg = &self.ui_config.status;
        let selection_info = self.selection_info();
        let mut spans = vec![
            Span::styled(format!("{}: ", cfg.commands_label), self.theme.style_dim()),
            Span::styled("Tab", self.theme.style_accent()),
            Span::styled(" focus ", self.theme.style_muted()),
            Span::styled("↑/↓", self.theme.style_accent()),
            Span::styled(" list ", self.theme.style_muted()),
            Span::styled("Enter →", self.theme.style_accent()),
            Span::styled(" open ", self.theme.style_muted()),
            Span::styled("/", self.theme.style_accent()),
            Span::styled(" search ", self.theme.style_muted()),
        ];
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

fn show_form_mode_shortcut(tab_id: &str) -> bool {
    tab_kind(tab_id) == TabKind::InsertForm
}

fn form_enter_hint(tab_id: &str) -> &'static str {
    if tab_id == KINIC_INSERT_TAB_ID {
        return insert_form_copy().status_enter_hint;
    }

    " submit "
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::Theme;
    use tui_kit_runtime::kinic_tabs::{KINIC_CREATE_TAB_ID, KINIC_SETTINGS_TAB_ID};

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
        assert_eq!(
            form_enter_hint(KINIC_INSERT_TAB_ID),
            " cycle/picker/submit "
        );
    }

    #[test]
    fn mode_shortcut_is_insert_only() {
        assert!(show_form_mode_shortcut(KINIC_INSERT_TAB_ID));
        assert!(!show_form_mode_shortcut(KINIC_CREATE_TAB_ID));
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
        assert!(rendered.contains("cycle/picker/submit"));
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
        assert!(rendered.contains("Tab/Shift+Tab"));
        assert!(rendered.contains("submit"));
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
    fn insert_form_without_status_message_still_shows_static_hints() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .current_tab_id(crate::ui::TabId::new(KINIC_INSERT_TAB_ID))
            .focus(Focus::Form);
        let rendered = render_status_line(&ui);

        assert!(rendered.contains("cycle/picker/submit"));
        assert!(!rendered.contains("│  │"));
    }
}
