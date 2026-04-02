//! Overlay blocks: settings popup, help popup.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};
#[cfg(test)]
use tui_kit_runtime::MemorySelectorItem;
use tui_kit_runtime::{
    AccessControlAction, AccessControlFocus, AccessControlMode, AccessControlRole,
    CreateModalFocus, CreateSubmitState, SettingsSnapshot,
};

use super::TuiKitUi;
use super::screens::settings::{
    DefaultMemorySelectorLineKind, default_memory_selector_copy, default_memory_selector_lines,
};

impl<'a> TuiKitUi<'a> {
    pub(super) fn access_control_cursor_position_for_area(&self, area: Rect) -> Option<(u16, u16)> {
        if !self.access_control_open || self.access_control_mode != AccessControlMode::Add {
            return None;
        }
        let rect = access_control_area(area, self.access_control_mode)?;
        let inner = bordered_inner_area(rect)?;
        let principal_prefix = "  ";
        let x = inner.x
            + principal_prefix.len() as u16
            + visible_width(self.access_control_principal_id.to_string());
        let y = inner.y + 2;
        Some((x.min(inner.right().saturating_sub(1)), y))
    }

    pub(super) fn render_create_overlay(&self, area: Rect, buf: &mut Buffer) {
        if !self.show_create_modal {
            return;
        }
        let w = 64.min(area.width.saturating_sub(4));
        let h = 14.min(area.height.saturating_sub(4));
        let create_area = Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        };
        Clear.render(create_area, buf);
        let cfg = &self.ui_config.create;

        let name_style = if self.create_focus == CreateModalFocus::Name {
            self.theme.style_accent()
        } else {
            self.theme.style_normal()
        };
        let desc_style = if self.create_focus == CreateModalFocus::Description {
            self.theme.style_accent()
        } else {
            self.theme.style_normal()
        };
        let submit_style = if self.create_focus == CreateModalFocus::Submit {
            self.theme.style_accent_bold()
        } else {
            self.theme.style_normal()
        };

        let mut text = vec![
            Line::from(Span::styled(
                format!(" {} ", cfg.title),
                self.theme.style_accent_bold(),
            )),
            Line::from(""),
            Line::from(Span::styled(cfg.name_label.clone(), self.theme.style_dim())),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    if self.create_name.is_empty() {
                        "<enter a memory name>".to_string()
                    } else {
                        self.create_name.to_string()
                    },
                    name_style,
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                cfg.description_label.clone(),
                self.theme.style_dim(),
            )),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    if self.create_description.is_empty() {
                        "<enter a short description>".to_string()
                    } else {
                        self.create_description.to_string()
                    },
                    desc_style,
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    if self.create_submit_state == CreateSubmitState::Submitting {
                        cfg.submit_pending_label.as_str()
                    } else {
                        cfg.submit_label.as_str()
                    },
                    submit_style,
                ),
            ]),
        ];
        if let Some(error) = self.create_error {
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                format!("  {error}"),
                self.theme.style_error(),
            )));
        }
        text.push(Line::from(""));
        text.push(Line::from(Span::styled(
            cfg.close_hint.clone(),
            self.theme.style_muted(),
        )));

        let block = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.style_border_focused())
                .title(format!(" {} ", cfg.title))
                .style(Style::default().bg(self.theme.bg_panel)),
        );
        block.render(create_area, buf);
    }

    pub(super) fn render_settings_overlay(&self, area: Rect, buf: &mut Buffer) {
        if !self.show_settings {
            return;
        }
        let cfg = &self.ui_config.settings;
        let lines = settings_overlay_lines(self.settings_snapshot, cfg);
        let Some((w, h)) = fit_overlay_rect(area, 68, lines.len().saturating_add(2) as u16, 10)
        else {
            return;
        };
        let settings_area = Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        };
        Clear.render(settings_area, buf);
        let text = lines
            .into_iter()
            .map(|line| {
                if line == cfg.close_hint {
                    Line::from(Span::styled(line, self.theme.style_muted()))
                } else if line.starts_with(' ') {
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled(line.trim_start().to_string(), self.theme.style_normal()),
                    ])
                } else {
                    Line::from(Span::styled(line, self.theme.style_accent_bold()))
                }
            })
            .collect::<Vec<_>>();
        let block = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.style_border_focused())
                    .title(format!(" {} ", cfg.title))
                    .style(Style::default().bg(self.theme.bg_panel)),
            )
            .wrap(Wrap { trim: false });
        block.render(settings_area, buf);
    }

    pub(super) fn render_help_overlay(&self, area: Rect, buf: &mut Buffer) {
        if !self.show_help {
            return;
        }
        let help_width = 62.min(area.width.saturating_sub(4));
        let help_height = 30.min(area.height.saturating_sub(4));
        let help_area = Rect {
            x: area.x + (area.width - help_width) / 2,
            y: area.y + (area.height - help_height) / 2,
            width: help_width,
            height: help_height,
        };
        Clear.render(help_area, buf);
        let cfg = &self.ui_config.help;
        let mut help_text = vec![
            Line::from(Span::styled(
                format!("⌨️  {}", cfg.title),
                self.theme.style_accent_bold(),
            )),
            Line::from(""),
        ];
        for line in &cfg.lines {
            help_text.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(line.clone(), self.theme.style_normal()),
            ]));
        }
        help_text.push(Line::from(""));
        help_text.push(Line::from(Span::styled(
            cfg.close_hint.clone(),
            self.theme.style_muted(),
        )));
        let help = Paragraph::new(help_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.style_border_focused())
                .title(format!(" {} ", cfg.title))
                .style(Style::default().bg(self.theme.bg_panel)),
        );
        help.render(help_area, buf);
    }

    pub(super) fn render_default_memory_selector_overlay(&self, area: Rect, buf: &mut Buffer) {
        if !self.default_memory_selector_open {
            return;
        }

        let lines = default_memory_selector_lines(
            self.default_memory_selector_items,
            self.default_memory_selector_index,
            self.default_memory_selector_selected_id,
            default_memory_selector_copy(self.default_memory_selector_context),
        );
        let copy = default_memory_selector_copy(self.default_memory_selector_context);
        let Some((w, h)) = fit_overlay_rect(area, 56, lines.len().saturating_add(2) as u16, 8)
        else {
            return;
        };
        let picker_area = Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        };
        Clear.render(picker_area, buf);

        let text = visible_default_memory_selector_lines(lines, h)
            .into_iter()
            .map(|(line, kind)| match kind {
                DefaultMemorySelectorLineKind::Selected => {
                    Line::from(Span::styled(line, self.theme.style_accent_bold()))
                }
                DefaultMemorySelectorLineKind::CurrentDefault => {
                    Line::from(Span::styled(line, self.theme.style_accent()))
                }
                DefaultMemorySelectorLineKind::Hint => {
                    Line::from(Span::styled(line, self.theme.style_muted()))
                }
                DefaultMemorySelectorLineKind::Normal => {
                    Line::from(Span::styled(line, self.theme.style_normal()))
                }
                DefaultMemorySelectorLineKind::Title => {
                    Line::from(Span::styled(line, self.theme.style_accent_bold()))
                }
            })
            .collect::<Vec<_>>();

        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.style_border_focused())
                    .title(format!(" {} ", copy.title))
                    .style(Style::default().bg(self.theme.bg_panel)),
            )
            .wrap(Wrap { trim: false })
            .render(picker_area, buf);
    }

    pub(super) fn render_access_control_overlay(&self, area: Rect, buf: &mut Buffer) {
        if !self.access_control_open {
            return;
        }

        let Some(overlay_area) = access_control_area(area, self.access_control_mode) else {
            return;
        };
        Clear.render(overlay_area, buf);

        let (title, mut lines) = access_overlay_copy(self);

        if let Some(error) = self.access_control_error {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {error}"),
                self.theme.style_error(),
            )));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Tab: next field  Arrow: cycle selector  Enter: submit  Esc: close",
            self.theme.style_muted(),
        )));

        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.style_border_focused())
                    .title(format!(" {title} "))
                    .style(Style::default().bg(self.theme.bg_panel)),
            )
            .wrap(Wrap { trim: false })
            .render(overlay_area, buf);
    }
}

fn access_control_area(area: Rect, mode: AccessControlMode) -> Option<Rect> {
    let (desired_width, desired_height, min_height) = match mode {
        AccessControlMode::Action => (58, 11, 9),
        AccessControlMode::Add => (56, 13, 10),
        AccessControlMode::Confirm => (52, 9, 7),
        AccessControlMode::None => (48, 8, 6),
    };
    let (width, height) = fit_overlay_rect(area, desired_width, desired_height, min_height)?;
    Some(Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    })
}

fn access_principal_value(ui: &TuiKitUi<'_>) -> String {
    if ui.access_control_principal_id.is_empty() {
        "<principal or anonymous>".to_string()
    } else if ui.access_control_mode == AccessControlMode::Add {
        ui.access_control_principal_id.to_string()
    } else {
        shorten_principal(ui.access_control_principal_id)
    }
}

fn access_principal_span(ui: &TuiKitUi<'_>) -> Span<'static> {
    if ui.access_control_principal_id.is_empty() {
        Span::styled(access_principal_value(ui), ui.theme.style_muted())
    } else {
        Span::styled(
            access_principal_value(ui),
            access_field_style(ui, AccessControlFocus::Principal),
        )
    }
}

fn access_role_value(ui: &TuiKitUi<'_>) -> String {
    match ui.access_control_role {
        AccessControlRole::Admin => "[admin] / writer / reader".to_string(),
        AccessControlRole::Writer => " admin / [writer] / reader".to_string(),
        AccessControlRole::Reader => " admin / writer / [reader]".to_string(),
    }
}

fn access_field_style(ui: &TuiKitUi<'_>, focus: AccessControlFocus) -> ratatui::style::Style {
    if ui.access_control_focus == focus {
        ui.theme.style_accent_bold()
    } else {
        ui.theme.style_normal()
    }
}

fn access_overlay_copy(ui: &TuiKitUi<'_>) -> (&'static str, Vec<Line<'static>>) {
    match ui.access_control_mode {
        AccessControlMode::Action => (
            "Access Action",
            vec![
                Line::from(""),
                Line::from(Span::styled("Selected User", ui.theme.style_dim())),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(access_principal_value(ui), ui.theme.style_muted()),
                ]),
                Line::from(""),
                Line::from(Span::styled("Role / Remove", ui.theme.style_dim())),
                access_action_line(ui),
                Line::from(""),
                Line::from(Span::styled(
                    format!("Current role: {}", role_label(ui.access_control_current_role)),
                    ui.theme.style_muted(),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Enter: confirm  Esc: close",
                    ui.theme.style_muted(),
                )),
            ],
        ),
        AccessControlMode::Add => (
            "Add User",
            vec![
                Line::from(""),
                Line::from(Span::styled("Principal ID", ui.theme.style_dim())),
                add_user_principal_line(ui),
                Line::from(""),
                Line::from(Span::styled("Role", ui.theme.style_dim())),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(access_role_value(ui), ui.theme.style_accent_bold()),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Enter: confirm  Esc: close",
                    ui.theme.style_muted(),
                )),
            ],
        ),
        AccessControlMode::Confirm => (
            "Confirm Access",
            vec![
                Line::from(Span::styled(
                    confirm_message(ui),
                    ui.theme.style_accent_bold(),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(confirm_value(ui), ui.theme.style_accent_bold()),
                ]),
                Line::from(""),
                Line::from(Span::styled(confirm_hint(ui), ui.theme.style_muted())),
            ],
        ),
        AccessControlMode::None => ("Access", vec![]),
    }
}

fn add_user_principal_line(ui: &TuiKitUi<'_>) -> Line<'static> {
    let mut spans = vec![Span::raw("  "), access_principal_span(ui)];
    if ui.access_control_principal_id.is_empty() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            r#"type "anonymous""#,
            ui.theme.style_muted(),
        ));
    }
    Line::from(spans)
}

fn access_action_line(ui: &TuiKitUi<'_>) -> Line<'static> {
    let mut spans = vec![Span::raw("  ")];
    let mut first = true;
    for role in [
        AccessControlRole::Admin,
        AccessControlRole::Writer,
        AccessControlRole::Reader,
    ] {
        if !first {
            spans.push(Span::raw(" / "));
        }
        first = false;
        let label = role_label(role);
        let style = if role == ui.access_control_current_role {
            ui.theme.style_muted()
        } else if ui.access_control_action == AccessControlAction::Change
            && ui.access_control_role == role
        {
            ui.theme.style_accent_bold()
        } else {
            ui.theme.style_normal()
        };
        let text = if role == ui.access_control_current_role {
            label.to_string()
        } else if ui.access_control_action == AccessControlAction::Change
            && ui.access_control_role == role
        {
            format!("[{label}]")
        } else {
            label.to_string()
        };
        spans.push(Span::styled(text, style));
    }
    spans.push(Span::raw(" / "));
    let remove_text = if ui.access_control_action == AccessControlAction::Remove {
        "[remove]".to_string()
    } else {
        "remove".to_string()
    };
    let remove_style = if ui.access_control_action == AccessControlAction::Remove {
        ui.theme.style_accent_bold()
    } else {
        ui.theme.style_normal()
    };
    spans.push(Span::styled(remove_text, remove_style));
    Line::from(spans)
}

fn confirm_message(ui: &TuiKitUi<'_>) -> String {
    match ui.access_control_action {
        AccessControlAction::Remove => {
            format!("Remove access for {}?", access_principal_value(ui))
        }
        AccessControlAction::Change => {
            format!(
                "Change role for {} to {}?",
                access_principal_value(ui),
                role_label(ui.access_control_role)
            )
        }
        AccessControlAction::Add => {
            format!(
                "Add {} as {}?",
                access_principal_value(ui),
                role_label(ui.access_control_role)
            )
        }
    }
}

fn confirm_value(ui: &TuiKitUi<'_>) -> String {
    if ui.access_control_confirm_yes {
        "[yes] / no".to_string()
    } else {
        " yes / [no]".to_string()
    }
}

fn confirm_hint(ui: &TuiKitUi<'_>) -> &'static str {
    match ui.access_control_submit_state {
        CreateSubmitState::Submitting => "Applying access change...",
        CreateSubmitState::Idle | CreateSubmitState::Error => "Enter: continue  Esc: close",
    }
}

fn role_label(role: AccessControlRole) -> &'static str {
    match role {
        AccessControlRole::Admin => "admin",
        AccessControlRole::Writer => "writer",
        AccessControlRole::Reader => "reader",
    }
}


fn shorten_principal(value: &str) -> String {
    const EDGE_CHARS: usize = 5;
    const MAX_CHARS: usize = EDGE_CHARS * 2 + 5;

    let char_count = value.chars().count();
    if char_count <= MAX_CHARS {
        return value.to_string();
    }

    let prefix_end = nth_char_boundary(value, EDGE_CHARS);
    let suffix_start = nth_char_boundary(value, char_count - EDGE_CHARS);
    format!("{}...{}", &value[..prefix_end], &value[suffix_start..])
}

fn nth_char_boundary(value: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }

    value
        .char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(value.len())
}


fn visible_width(value: String) -> u16 {
    unicode_width::UnicodeWidthStr::width(value.as_str()) as u16
}

fn fit_overlay_rect(
    area: Rect,
    desired_width: u16,
    desired_height: u16,
    min_height: u16,
) -> Option<(u16, u16)> {
    let width = desired_width.min(area.width.saturating_sub(4));
    let max_height = area.height.saturating_sub(4);
    if width == 0 || max_height == 0 {
        return None;
    }

    let height = if max_height < min_height {
        max_height
    } else {
        desired_height.clamp(min_height, max_height)
    };

    Some((width, height))
}

fn bordered_inner_area(area: Rect) -> Option<Rect> {
    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    (inner.width > 0 && inner.height > 0).then_some(inner)
}

fn visible_default_memory_selector_lines(
    lines: Vec<(String, DefaultMemorySelectorLineKind)>,
    overlay_height: u16,
) -> Vec<(String, DefaultMemorySelectorLineKind)> {
    let fixed_prefix_len = 2usize;
    let fixed_suffix_len = 2usize;
    if lines.len() <= fixed_prefix_len + fixed_suffix_len {
        return lines;
    }

    let candidate_rows = overlay_height.saturating_sub(2) as usize;
    let visible_body_len = candidate_rows.saturating_sub(fixed_prefix_len + fixed_suffix_len);
    if visible_body_len == 0 {
        return lines;
    }

    let body_start = fixed_prefix_len;
    let body_end = lines.len().saturating_sub(fixed_suffix_len);
    let body = &lines[body_start..body_end];
    if body.len() <= visible_body_len {
        return lines;
    }

    let selected_in_body = body
        .iter()
        .position(|(_, kind)| *kind == DefaultMemorySelectorLineKind::Selected)
        .unwrap_or(0);
    let max_start = body.len().saturating_sub(visible_body_len);
    let start = selected_in_body
        .saturating_sub(visible_body_len / 2)
        .min(max_start);
    let end = start + visible_body_len;

    let mut visible = Vec::with_capacity(fixed_prefix_len + visible_body_len + fixed_suffix_len);
    visible.extend_from_slice(&lines[..fixed_prefix_len]);
    visible.extend_from_slice(&body[start..end]);
    visible.extend_from_slice(&lines[body_end..]);
    visible
}

fn settings_overlay_lines(
    snapshot: Option<&SettingsSnapshot>,
    cfg: &super::types::SettingsOverlayText,
) -> Vec<String> {
    let mut lines = vec![format!(" {} ", cfg.title), String::new()];
    if let Some(snapshot) = snapshot {
        if snapshot.quick_entries.is_empty() {
            lines.push(" No settings available yet.".to_string());
        } else {
            let visible_entries = snapshot.quick_entries.iter().take(7).collect::<Vec<_>>();
            let label_width = visible_entries
                .iter()
                .map(|entry| entry.label.chars().count())
                .max()
                .unwrap_or(0);
            for entry in visible_entries {
                lines.push(format!(
                    " {label:<width$}: {value}",
                    label = entry.label,
                    width = label_width,
                    value = entry.value,
                ));
            }
        }
    } else {
        lines.push(" No settings available yet.".to_string());
    }
    lines.push(String::new());
    lines.push(cfg.close_hint.clone());
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::app::UiConfig;
    use tui_kit_runtime::{
        MemorySelectorContext, SettingsEntry, SettingsSection, SettingsSnapshot,
    };

    #[test]
    fn settings_overlay_lines_show_up_to_seven_entries() {
        let cfg = UiConfig::default().settings;
        let snapshot = SettingsSnapshot {
            quick_entries: (1..=8)
                .map(|index| SettingsEntry {
                    id: format!("item_{index}"),
                    label: format!("Item {index}"),
                    value: format!("Value {index}"),
                    note: None,
                })
                .collect(),
            sections: vec![SettingsSection::default()],
        };

        let joined = settings_overlay_lines(Some(&snapshot), &cfg).join("\n");

        assert!(!joined.contains("Open the Settings tab for detailed view."));
        assert!(joined.contains("Item 7: Value 7"));
        assert!(!joined.contains("Item 8: Value 8"));
    }

    #[test]
    fn settings_overlay_lines_align_quick_entry_columns() {
        let cfg = UiConfig::default().settings;
        let snapshot = SettingsSnapshot {
            quick_entries: vec![
                SettingsEntry {
                    id: "auth".to_string(),
                    label: "Auth".to_string(),
                    value: "mock".to_string(),
                    note: None,
                },
                SettingsEntry {
                    id: "embedding_api_endpoint".to_string(),
                    label: "Embedding".to_string(),
                    value: "https://api.kinic.io".to_string(),
                    note: None,
                },
            ],
            sections: vec![SettingsSection::default()],
        };

        let lines = settings_overlay_lines(Some(&snapshot), &cfg);
        let auth_line = lines
            .iter()
            .find(|line| line.contains("Auth"))
            .expect("auth line");
        let embedding_line = lines
            .iter()
            .find(|line| line.contains("Embedding"))
            .expect("embedding line");

        assert_eq!(auth_line.find(':'), embedding_line.find(':'));
    }

    #[test]
    fn fit_overlay_rect_uses_available_height_when_terminal_is_short() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 11,
        };

        let rect = fit_overlay_rect(area, 56, 12, 8);

        assert_eq!(rect, Some((56, 7)));
    }

    #[test]
    fn fit_overlay_rect_returns_none_when_inner_area_is_unavailable() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 3,
            height: 3,
        };

        let rect = fit_overlay_rect(area, 56, 12, 8);

        assert_eq!(rect, None);
    }

    #[test]
    fn default_memory_selector_window_keeps_selected_row_visible_near_end() {
        let lines = default_memory_selector_lines(
            &(0..12)
                .map(|index| MemorySelectorItem {
                    id: format!("id-{index}"),
                    title: Some(format!("Memory {index}")),
                })
                .collect::<Vec<_>>(),
            10,
            Some("id-2"),
            default_memory_selector_copy(MemorySelectorContext::DefaultPreference),
        );

        let visible = visible_default_memory_selector_lines(lines, 8);
        let visible_lines = visible
            .iter()
            .map(|(line, _)| line.as_str())
            .collect::<Vec<_>>();
        let joined = visible_lines.join("\n");

        assert!(joined.contains("Memory 10"));
        assert!(!visible_lines.iter().any(|line| *line == "   Memory 1"));
        assert!(joined.contains("Enter: save"));
    }

    #[test]
    fn default_memory_selector_window_keeps_selected_row_visible_near_start() {
        let lines = default_memory_selector_lines(
            &(0..12)
                .map(|index| MemorySelectorItem {
                    id: format!("id-{index}"),
                    title: Some(format!("Memory {index}")),
                })
                .collect::<Vec<_>>(),
            1,
            Some("id-9"),
            default_memory_selector_copy(MemorySelectorContext::DefaultPreference),
        );
        let near_start = visible_default_memory_selector_lines(lines, 8);
        let near_start_joined = near_start
            .iter()
            .map(|(line, _)| line.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(near_start_joined.contains("Memory 1"));
        assert!(!near_start_joined.contains("Memory 10"));
        assert!(near_start_joined.contains("Select Default Memory"));
    }

    #[test]
    fn insert_target_selector_window_uses_insert_specific_copy() {
        let lines = default_memory_selector_lines(
            &[MemorySelectorItem {
                id: "id-0".to_string(),
                title: Some("Memory 0".to_string()),
            }],
            0,
            Some("id-0"),
            default_memory_selector_copy(MemorySelectorContext::InsertTarget),
        );

        let visible = visible_default_memory_selector_lines(lines, 8);
        let joined = visible
            .iter()
            .map(|(line, _)| line.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(joined.contains("Select Target Memory"));
        assert!(joined.contains("Enter: use target"));
        assert!(!joined.contains('★'));
    }
}
