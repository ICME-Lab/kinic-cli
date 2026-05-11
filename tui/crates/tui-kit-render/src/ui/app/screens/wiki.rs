//! Wiki browser screen composition.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget, Wrap},
};
use tui_kit_runtime::{PaneRow, ThreePaneMode, ThreePaneSnapshot};

use super::spinner_frame;
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
            .constraints([Constraint::Length(38), Constraint::Min(36)])
            .split(body);

        self.render_wiki_browser(chunks[0], buf, snapshot);
        self.render_wiki_document(chunks[1], buf, snapshot);
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
            .border_style(
                if self.focus == Focus::Items || self.focus == Focus::Content {
                    self.theme.style_border_focused()
                } else {
                    self.theme.style_border()
                },
            )
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
            .map(|row| {
                let marker = if row.selected { ">" } else { " " };
                let icon = wiki_browser_icon(row.detail.as_str());
                let style = if row.selected {
                    self.theme.style_selected()
                } else {
                    self.theme.style_dim()
                };
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(format!("{marker} "), self.theme.style_accent()),
                        Span::styled(format!("{icon} "), style),
                        Span::styled(row.label.clone(), style),
                    ]),
                    Line::from(Span::styled(
                        format!("    {}", row.detail),
                        self.theme.style_muted(),
                    )),
                ])
            })
            .collect::<Vec<_>>();
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.style_border())
            .style(Style::default().bg(self.theme.bg_panel))
            .title(title);
        List::new(items).block(block).render(area, buf);
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
        } else if snapshot.document.lines.is_empty() {
            lines.push(Line::from(
                "Select a wiki database to browse /Wiki and /Sources.",
            ));
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
}

fn selected_browser_path(snapshot: &ThreePaneSnapshot) -> String {
    snapshot
        .document
        .title
        .split_once(' ')
        .map(|(_, path)| path)
        .filter(|path| path.starts_with('/'))
        .unwrap_or("/")
        .to_string()
}

fn wiki_browser_icon(detail: &str) -> &'static str {
    if detail.starts_with("directory") {
        "[D]"
    } else if detail.starts_with("source") {
        "[S]"
    } else if detail.starts_with("file") {
        "[F]"
    } else {
        "[ ]"
    }
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
