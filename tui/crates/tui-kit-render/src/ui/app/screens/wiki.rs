//! Wiki browser screen composition.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget, Wrap},
};
use tui_kit_runtime::{
    CreateSubmitState, PaneRow, ThreePaneMode, ThreePaneSnapshot, WikiEditorFooterFocus,
};

use super::{multiline_cursor_x, multiline_visible_row, spinner_frame, visible_multiline_rows};
use crate::ui::app::{Focus, TuiKitUi, shared};

impl<'a> TuiKitUi<'a> {
    pub(crate) fn render_wiki_screen(&self, area: Rect, buf: &mut Buffer) {
        let body = shared::layout::body_rect_for_area_with_tabs(area, !self.tab_specs.is_empty());
        let default_snapshot = ThreePaneSnapshot::default();
        let snapshot = self.three_pane.unwrap_or(&default_snapshot);
        match snapshot.mode {
            ThreePaneMode::List => {
                self.render_wiki_databases(body, buf, snapshot);
                return;
            }
            ThreePaneMode::Diagnostic => {
                self.render_wiki_diagnostic(body, buf, snapshot);
                return;
            }
            _ => {}
        }
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(38),
                Constraint::Length(1),
                Constraint::Min(36),
            ])
            .split(body);

        self.render_wiki_browser(chunks[0], buf, snapshot);
        self.render_wiki_divider(chunks[1], buf);
        self.render_wiki_document(chunks[2], buf, snapshot);
    }

    fn render_wiki_databases(&self, area: Rect, buf: &mut Buffer, snapshot: &ThreePaneSnapshot) {
        let title = if snapshot.left.loading {
            format!(
                " Databases | {} Loading... ",
                spinner_frame(self.insert_spinner_frame)
            )
        } else {
            " Databases ".to_string()
        };
        let items = if snapshot.left.rows.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                if snapshot.left.loading {
                    format!("  {} Loading...", spinner_frame(self.insert_spinner_frame))
                } else {
                    format!("  {}", snapshot.left.empty_message)
                },
                self.theme.style_dim(),
            )))]
        } else {
            snapshot
                .left
                .rows
                .iter()
                .map(|row| self.render_wiki_database_row(row))
                .collect()
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(if self.focus == Focus::Items {
                self.theme.style_border_focused()
            } else {
                self.theme.style_border()
            })
            .style(Style::default().bg(self.theme.bg_panel))
            .title(title);
        List::new(items).block(block).render(area, buf);
    }

    fn render_wiki_database_row(&self, row: &PaneRow) -> ListItem<'static> {
        let marker = if row.selected { "▸" } else { " " };
        let style = if row.selected {
            self.theme.style_normal().add_modifier(Modifier::BOLD)
        } else {
            self.theme.style_dim()
        };
        ListItem::new(vec![
            Line::from(vec![
                Span::styled(format!("{marker} "), self.theme.style_accent()),
                Span::styled(row.label.clone(), style),
            ]),
            Line::from(vec![Span::styled(
                format!("  {}", row.detail),
                self.theme.style_muted(),
            )]),
        ])
    }

    fn render_wiki_browser(&self, area: Rect, buf: &mut Buffer, snapshot: &ThreePaneSnapshot) {
        let path = selected_browser_path(snapshot);
        let title = if snapshot.middle.loading {
            format!(
                " Browser | {} Loading... ",
                spinner_frame(self.insert_spinner_frame)
            )
        } else if snapshot.mode == ThreePaneMode::Search {
            " Browser (search) ".to_string()
        } else {
            format!(" Browser | {path} ")
        };
        let rows = if snapshot.middle.rows.is_empty() {
            vec![PaneRow {
                label: if snapshot.middle.loading {
                    format!("{} Loading...", spinner_frame(self.insert_spinner_frame))
                } else {
                    "No entries".to_string()
                },
                detail: snapshot.middle.empty_message.clone(),
                selected: false,
            }]
        } else {
            snapshot.middle.rows.clone()
        };
        let items = rows
            .iter()
            .map(|row| self.render_wiki_browser_row(row, snapshot.mode))
            .collect::<Vec<_>>();
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(if self.focus == Focus::Items {
                self.theme.style_border_focused()
            } else {
                self.theme.style_border()
            })
            .style(Style::default().bg(self.theme.bg_panel))
            .title(title);
        List::new(items).block(block).render(area, buf);
    }

    fn render_wiki_browser_row(&self, row: &PaneRow, mode: ThreePaneMode) -> ListItem<'static> {
        let marker = if row.selected { ">" } else { " " };
        let icon = wiki_browser_icon(row.detail.as_str());
        let style = if row.selected {
            self.theme.style_selected()
        } else {
            self.theme.style_dim()
        };
        let (indent, label) = split_tree_indent(row.label.as_str());
        let mut first_line = vec![
            Span::styled(format!("{marker} "), self.theme.style_accent()),
            Span::styled(indent, style),
            Span::styled(icon, style),
            Span::styled(label, style),
        ];

        if mode != ThreePaneMode::Search
            && let Some(meta) = wiki_browser_inline_meta(row.detail.as_str())
        {
            first_line.push(Span::styled(format!("  {meta}"), self.theme.style_muted()));
        }

        let mut lines = vec![Line::from(first_line)];
        if mode == ThreePaneMode::Search && !row.detail.trim().is_empty() {
            lines.push(Line::from(Span::styled(
                format!("    {}", row.detail),
                self.theme.style_muted(),
            )));
        }
        ListItem::new(lines)
    }

    fn render_wiki_divider(&self, area: Rect, buf: &mut Buffer) {
        let style = self.theme.style_border();
        for y in area.top()..area.bottom() {
            if area.width > 0
                && let Some(cell) = buf.cell_mut((area.x, y))
            {
                cell.set_symbol("│").set_style(style);
            }
        }
    }

    fn render_wiki_diagnostic(&self, area: Rect, buf: &mut Buffer, snapshot: &ThreePaneSnapshot) {
        let mut lines = Vec::new();
        if let Some(diagnostic) = &snapshot.diagnostic {
            for row in &diagnostic.rows {
                lines.push(Line::from(vec![
                    Span::styled(format!("{}: ", row.label), self.theme.style_muted()),
                    Span::raw(row.detail.clone()),
                ]));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(diagnostic.message.clone()));
        } else {
            lines.push(Line::from("No diagnostics."));
        }
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.style_border_focused())
            .style(Style::default().bg(self.theme.bg_panel))
            .title(" Wiki diagnostics ");
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }

    fn render_wiki_document(&self, area: Rect, buf: &mut Buffer, snapshot: &ThreePaneSnapshot) {
        if self.wiki_editor.open {
            self.render_wiki_editor(area, buf);
            return;
        }
        let mut lines = Vec::new();
        if let Some(diagnostic) = &snapshot.diagnostic {
            for row in &diagnostic.rows {
                lines.push(Line::from(vec![
                    Span::styled(format!("{}: ", row.label), self.theme.style_muted()),
                    Span::raw(row.detail.clone()),
                ]));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(diagnostic.message.clone()));
        } else {
            lines.extend(snapshot.document.lines.iter().cloned().map(Line::from));
        }
        let title = if snapshot.document.title.trim().is_empty() {
            " Document ".to_string()
        } else {
            format!(" Document: {} ", snapshot.document.title)
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(if self.focus == Focus::Content {
                self.theme.style_border_focused()
            } else {
                self.theme.style_border()
            })
            .style(Style::default().bg(self.theme.bg_panel))
            .title(title);
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.inspector_scroll as u16, 0))
            .render(area, buf);
    }

    fn render_wiki_editor(&self, area: Rect, buf: &mut Buffer) {
        let status = match self.wiki_editor.submit_state {
            CreateSubmitState::Submitting => "saving",
            CreateSubmitState::Idle | CreateSubmitState::Error if self.wiki_editor.dirty => "dirty",
            CreateSubmitState::Idle | CreateSubmitState::Error => "clean",
        };
        let title = format!(" Editing: {} [{status}] ", self.wiki_editor.path);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.wiki_editor_border_style())
            .style(Style::default().bg(self.theme.bg_panel))
            .title(title);
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 {
            return;
        }
        let message_height =
            u16::from(self.wiki_editor.discard_confirm || self.wiki_editor.error.is_some());
        let footer_height = 1;
        let body_height = inner
            .height
            .saturating_sub(footer_height)
            .saturating_sub(message_height)
            .max(1);
        let body_area = Rect::new(inner.x, inner.y, inner.width, body_height);
        let footer_y = body_area.y.saturating_add(body_area.height);
        let footer_area = Rect::new(inner.x, footer_y, inner.width, footer_height);
        let message_area = Rect::new(
            inner.x,
            footer_y.saturating_add(footer_height),
            inner.width,
            message_height,
        );

        let cursor_row = self
            .wiki_editor_cursor
            .map(|(row, _)| row)
            .unwrap_or_default();
        let visible = visible_multiline_rows(
            self.wiki_editor.draft_content.as_str(),
            "",
            body_height,
            cursor_row,
            body_area.width,
        );
        let lines = visible.rows.into_iter().map(Line::from).collect::<Vec<_>>();
        Paragraph::new(lines)
            .style(Style::default().bg(self.theme.bg_panel))
            .wrap(Wrap { trim: false })
            .render(body_area, buf);

        Paragraph::new(self.wiki_editor_footer_line())
            .style(Style::default().bg(self.theme.bg_panel))
            .render(footer_area, buf);

        if message_height > 0 {
            let message = if self.wiki_editor.discard_confirm {
                Line::from(vec![
                    Span::styled("Unsaved changes. ", self.theme.style_warning()),
                    Span::raw("Confirm before leaving edit mode."),
                ])
            } else {
                Line::from(Span::styled(
                    self.wiki_editor.error.as_deref().unwrap_or_default(),
                    self.theme.style_error(),
                ))
            };
            Paragraph::new(message)
                .style(Style::default().bg(self.theme.bg_panel))
                .render(message_area, buf);
        }
    }

    fn wiki_editor_border_style(&self) -> Style {
        if self.wiki_editor.submit_state == CreateSubmitState::Submitting {
            return self.theme.style_info().add_modifier(Modifier::BOLD);
        }
        if self.wiki_editor.error.is_some() {
            return self.theme.style_error().add_modifier(Modifier::BOLD);
        }
        if self.wiki_editor.dirty || self.wiki_editor.discard_confirm {
            return self.theme.style_warning().add_modifier(Modifier::BOLD);
        }
        if self.focus == Focus::Content {
            return self.theme.style_accent_bold();
        }
        self.theme.style_border()
    }

    fn wiki_editor_footer_line(&self) -> Line<'_> {
        if self.wiki_editor.discard_confirm {
            return Line::from(vec![
                Span::styled(
                    " Enter ",
                    self.theme.style_warning().add_modifier(Modifier::BOLD),
                ),
                Span::styled(" discard changes ", self.theme.style_muted()),
                Span::raw("  "),
                Span::styled(" Esc ", self.theme.style_accent_bold()),
                Span::styled(" continue editing ", self.theme.style_muted()),
            ]);
        }
        Line::from(vec![
            Span::styled(
                " Body ",
                self.wiki_footer_style(WikiEditorFooterFocus::Body),
            ),
            Span::raw(" "),
            Span::styled(
                " Save ",
                self.wiki_footer_style(WikiEditorFooterFocus::Save),
            ),
            Span::raw(" "),
            Span::styled(
                " Cancel ",
                self.wiki_footer_style(WikiEditorFooterFocus::Cancel),
            ),
            Span::styled("  Ctrl+S save", self.theme.style_muted()),
        ])
    }

    fn wiki_footer_style(&self, focus: WikiEditorFooterFocus) -> Style {
        if self.wiki_editor.footer_focus == focus && !self.wiki_editor.discard_confirm {
            self.theme.style_selected()
        } else {
            self.theme.style_muted()
        }
    }

    pub(crate) fn wiki_cursor_position_for_area(&self, area: Rect) -> Option<(u16, u16)> {
        if self.focus != Focus::Content || !self.wiki_editor.open {
            return None;
        }
        if self.wiki_editor.footer_focus != WikiEditorFooterFocus::Body
            || self.wiki_editor.discard_confirm
        {
            return None;
        }
        let body = shared::layout::body_rect_for_area_with_tabs(area, !self.tab_specs.is_empty());
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(38),
                Constraint::Length(1),
                Constraint::Min(36),
            ])
            .split(body);
        let document = chunks[2];
        let inner = Block::default().borders(Borders::ALL).inner(document);
        let (cursor_row, cursor_col) = self.wiki_editor_cursor?;
        let message_height =
            u16::from(self.wiki_editor.discard_confirm || self.wiki_editor.error.is_some());
        let body_height = inner
            .height
            .saturating_sub(1)
            .saturating_sub(message_height)
            .max(1);
        let visible = visible_multiline_rows(
            self.wiki_editor.draft_content.as_str(),
            "",
            body_height,
            cursor_row,
            inner.width,
        );
        let visible_row = multiline_visible_row(cursor_row, visible.scroll_row, body_height);
        let x = inner.x
            + multiline_cursor_x(
                visible.rows[visible_row as usize].as_str(),
                cursor_col,
                inner.width,
            );
        Some((
            x.min(inner.right().saturating_sub(1)),
            inner.y + visible_row,
        ))
    }
}

fn selected_browser_path(snapshot: &ThreePaneSnapshot) -> String {
    if snapshot.middle.title.trim().is_empty() {
        "/".to_string()
    } else {
        snapshot.middle.title.clone()
    }
}

fn wiki_browser_icon(detail: &str) -> &'static str {
    if detail.starts_with("directory expanded") {
        "▾ "
    } else if detail.starts_with("directory") {
        "› "
    } else if detail.starts_with("source") {
        "  S "
    } else if detail.starts_with("file") || detail.trim().is_empty() {
        "  "
    } else {
        "? "
    }
}

fn split_tree_indent(label: &str) -> (String, String) {
    let indent_width = label.len() - label.trim_start_matches(' ').len();
    (
        label[..indent_width].to_string(),
        label[indent_width..].to_string(),
    )
}

fn wiki_browser_inline_meta(detail: &str) -> Option<String> {
    if detail.starts_with("directory") || detail == "file" || detail == "source" {
        return None;
    }
    ["file ", "source "].into_iter().find_map(|prefix| {
        detail
            .strip_prefix(prefix)
            .and_then(|value| value.strip_suffix(" bytes"))
            .map(|value| format!("{value} bytes"))
    })
}

#[cfg(test)]
mod tests {
    use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
    use tui_kit_runtime::{
        DiagnosticSnapshot, DocumentSnapshot, PaneRow, PaneSnapshot, ThreePaneMode,
        ThreePaneSnapshot, kinic_tabs::KINIC_WIKI_TAB_ID,
    };

    use crate::ui::{app::TabId, theme::Theme};

    use super::*;

    fn rendered(buf: &Buffer) -> String {
        buf.content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn wiki_screen_renders_database_list_mode_as_single_pane() {
        let theme = Theme::default();
        let snapshot = ThreePaneSnapshot {
            left: PaneSnapshot {
                rows: vec![
                    PaneRow {
                        label: "db-a".to_string(),
                        detail: "Hot Owner 42 bytes".to_string(),
                        selected: false,
                    },
                    PaneRow {
                        label: "+ Create database".to_string(),
                        detail: "create new database".to_string(),
                        selected: true,
                    },
                ],
                ..PaneSnapshot::default()
            },
            mode: ThreePaneMode::List,
            ..ThreePaneSnapshot::default()
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 110, 30));
        TuiKitUi::new(&theme)
            .current_tab_id(TabId::new(KINIC_WIKI_TAB_ID))
            .three_pane_snapshot(&snapshot)
            .render(Rect::new(0, 0, 110, 30), &mut buf);
        let text = rendered(&buf);
        assert!(text.contains("Databases"));
        assert!(text.contains("db-a"));
        assert!(text.contains("+ Create database"));
        assert!(text.contains("create new database"));
        assert!(!text.contains("Browser"));
        assert!(!text.contains("Document"));
    }

    #[test]
    fn wiki_screen_renders_browser_mode_as_two_panes() {
        let theme = Theme::default();
        let snapshot = ThreePaneSnapshot {
            middle: PaneSnapshot {
                rows: vec![PaneRow {
                    label: "/Wiki/index.md".to_string(),
                    detail: "file".to_string(),
                    selected: true,
                }],
                ..PaneSnapshot::default()
            },
            document: DocumentSnapshot {
                title: "db-a /Wiki/index.md".to_string(),
                lines: vec!["hello wiki".to_string()],
            },
            mode: ThreePaneMode::Browse,
            ..ThreePaneSnapshot::default()
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 110, 30));
        TuiKitUi::new(&theme)
            .current_tab_id(TabId::new(KINIC_WIKI_TAB_ID))
            .three_pane_snapshot(&snapshot)
            .render(Rect::new(0, 0, 110, 30), &mut buf);
        let text = rendered(&buf);
        assert!(text.contains("Browser"));
        assert!(text.contains("Document"));
        assert!(text.contains("/Wiki/index.md"));
        assert!(text.contains("hello wiki"));
        assert!(!text.contains("Databases"));
    }

    #[test]
    fn wiki_screen_renders_editor_state_and_discard_confirm() {
        let theme = Theme::default();
        let mut editor = tui_kit_runtime::WikiEditorState::default();
        editor.open(
            "/Wiki/index.md".to_string(),
            "# Index\nbody".to_string(),
            "etag".to_string(),
            "{}".to_string(),
        );
        editor.dirty = true;
        editor.discard_confirm = true;
        let snapshot = ThreePaneSnapshot {
            middle: PaneSnapshot {
                title: "/".to_string(),
                ..PaneSnapshot::default()
            },
            document: DocumentSnapshot {
                title: "/Wiki/index.md".to_string(),
                lines: vec!["readonly".to_string()],
                ..DocumentSnapshot::default()
            },
            mode: ThreePaneMode::Browse,
            ..ThreePaneSnapshot::default()
        };
        let ui = TuiKitUi::new(&theme)
            .current_tab_id(TabId::new(KINIC_WIKI_TAB_ID))
            .focus(Focus::Content)
            .three_pane_snapshot(&snapshot)
            .wiki_editor(editor)
            .wiki_editor_cursor(Some((1, 4)));
        let mut buf = Buffer::empty(Rect::new(0, 0, 100, 24));

        ui.render(buf.area, &mut buf);
        let text = rendered(&buf);

        assert!(text.contains("Editing: /Wiki/index.md"));
        assert!(text.contains("body"));
        assert!(text.contains("Unsaved changes."));
        assert!(text.contains("continue editing"));
    }

    #[test]
    fn wiki_browser_directory_rows_stay_single_line() {
        let theme = Theme::default();
        let snapshot = ThreePaneSnapshot {
            middle: PaneSnapshot {
                rows: vec![
                    PaneRow {
                        label: "Wiki".to_string(),
                        detail: "directory expanded".to_string(),
                        selected: true,
                    },
                    PaneRow {
                        label: "Sources".to_string(),
                        detail: "directory collapsed".to_string(),
                        selected: false,
                    },
                ],
                ..PaneSnapshot::default()
            },
            document: DocumentSnapshot {
                title: "db-a /".to_string(),
                lines: vec!["Loading /Wiki and /Sources...".to_string()],
            },
            mode: ThreePaneMode::Browse,
            ..ThreePaneSnapshot::default()
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 110, 30));
        TuiKitUi::new(&theme)
            .current_tab_id(TabId::new(KINIC_WIKI_TAB_ID))
            .three_pane_snapshot(&snapshot)
            .render(Rect::new(0, 0, 110, 30), &mut buf);
        let text = rendered(&buf);
        assert!(text.contains("▾ Wiki"));
        assert!(text.contains("› Sources"));
        assert!(!text.contains("directory"));
    }

    #[test]
    fn wiki_screen_renders_database_loading_spinner() {
        let theme = Theme::default();
        let snapshot = ThreePaneSnapshot {
            left: PaneSnapshot {
                empty_message: "No databases".to_string(),
                loading: true,
                ..PaneSnapshot::default()
            },
            mode: ThreePaneMode::List,
            ..ThreePaneSnapshot::default()
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 110, 30));
        TuiKitUi::new(&theme)
            .current_tab_id(TabId::new(KINIC_WIKI_TAB_ID))
            .insert_spinner_frame(1)
            .three_pane_snapshot(&snapshot)
            .render(Rect::new(0, 0, 110, 30), &mut buf);
        let text = rendered(&buf);
        assert!(text.contains("Databases"));
        assert!(text.contains("Loading"));
        assert!(text.contains("/"));
    }

    #[test]
    fn wiki_screen_renders_browser_loading_spinner() {
        let theme = Theme::default();
        let snapshot = ThreePaneSnapshot {
            middle: PaneSnapshot {
                empty_message: "Loading /Wiki".to_string(),
                loading: true,
                ..PaneSnapshot::default()
            },
            document: DocumentSnapshot {
                title: "db-a /".to_string(),
                lines: Vec::new(),
            },
            mode: ThreePaneMode::Browse,
            ..ThreePaneSnapshot::default()
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 110, 30));
        TuiKitUi::new(&theme)
            .current_tab_id(TabId::new(KINIC_WIKI_TAB_ID))
            .insert_spinner_frame(1)
            .three_pane_snapshot(&snapshot)
            .render(Rect::new(0, 0, 110, 30), &mut buf);
        let text = rendered(&buf);
        assert!(text.contains("Browser"));
        assert!(text.contains("Loading"));
        assert!(text.contains("/"));
    }

    #[test]
    fn wiki_screen_renders_diagnostic_panel() {
        let theme = Theme::default();
        let snapshot = ThreePaneSnapshot {
            diagnostic: Some(DiagnosticSnapshot {
                rows: vec![
                    PaneRow {
                        label: "canister".to_string(),
                        detail: "aaaaa-aa".to_string(),
                        selected: false,
                    },
                    PaneRow {
                        label: "principal".to_string(),
                        detail: "2vxsx-fae".to_string(),
                        selected: false,
                    },
                ],
                message: "wiki query failed for list_databases".to_string(),
            }),
            mode: ThreePaneMode::Diagnostic,
            ..ThreePaneSnapshot::default()
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 110, 30));
        TuiKitUi::new(&theme)
            .current_tab_id(TabId::new(KINIC_WIKI_TAB_ID))
            .three_pane_snapshot(&snapshot)
            .render(Rect::new(0, 0, 110, 30), &mut buf);
        let text = rendered(&buf);
        assert!(text.contains("Wiki diagnostics"));
        assert!(text.contains("wiki query failed for list_databases"));
        assert!(text.contains("aaaaa-aa"));
        assert!(text.contains("2vxsx-fae"));
    }
}
