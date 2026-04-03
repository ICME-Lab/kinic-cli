//! Overlay blocks: settings, help, picker, and create.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};
use tui_kit_runtime::{
    AccessControlAction, AccessControlFocus, AccessControlMode, AccessControlRole,
    CreateModalFocus, CreateSubmitState, RenameModalFocus, SettingsSnapshot, TransferModalFocus,
    TransferModalMode, format_e8s_to_kinic_string_u128,
};

use super::TuiKitUi;
use super::screens::settings::{
    PickerLineKind, picker_lines, picker_presentation, picker_requires_vertical_centering,
    picker_title,
};

impl<'a> TuiKitUi<'a> {
    pub(super) fn transfer_cursor_position_for_area(&self, area: Rect) -> Option<(u16, u16)> {
        if !self.transfer_modal.open || self.transfer_modal.mode != TransferModalMode::Edit {
            return None;
        }
        let rect = transfer_modal_area(area, self.transfer_modal.mode)?;
        let inner = bordered_inner_area(rect)?;
        let (prefix, value, y_offset) = match self.transfer_modal.focus {
            TransferModalFocus::Principal => ("  ", self.transfer_modal.principal_id.as_str(), 2),
            TransferModalFocus::Amount => ("  ", self.transfer_modal.amount.as_str(), 5),
            TransferModalFocus::Max | TransferModalFocus::Submit => return None,
        };
        let x = inner.x + prefix.len() as u16 + visible_width(value.to_string());
        let y = inner.y + y_offset;
        Some((x.min(inner.right().saturating_sub(1)), y))
    }

    pub(super) fn access_control_cursor_position_for_area(&self, area: Rect) -> Option<(u16, u16)> {
        if !self.access_control.open || self.access_control.mode != AccessControlMode::Add {
            return None;
        }
        let rect = access_control_area(area, self.access_control.mode)?;
        let inner = bordered_inner_area(rect)?;
        let principal_prefix = "  ";
        let x = inner.x
            + principal_prefix.len() as u16
            + visible_width(self.access_control.principal_id.to_string());
        let y = inner.y + 2;
        Some((x.min(inner.right().saturating_sub(1)), y))
    }

    pub(super) fn add_memory_cursor_position_for_area(&self, area: Rect) -> Option<(u16, u16)> {
        if !self.add_memory.open {
            return None;
        }
        let rect = add_memory_area(area)?;
        let inner = bordered_inner_area(rect)?;
        let x = inner.x + 2 + visible_width(self.add_memory.value.to_string());
        let y = inner.y + 2;
        Some((x.min(inner.right().saturating_sub(1)), y))
    }

    pub(super) fn rename_memory_cursor_position_for_area(&self, area: Rect) -> Option<(u16, u16)> {
        if !self.rename_memory.form.open || self.rename_memory.focus != RenameModalFocus::Name {
            return None;
        }
        let rect = rename_memory_area(area)?;
        let inner = bordered_inner_area(rect)?;
        let x = inner.x + 2 + visible_width(self.rename_memory.form.value.to_string());
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

        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.style_border_focused())
                    .title(format!(" {} ", cfg.title))
                    .style(Style::default().bg(self.theme.bg_panel)),
            )
            .render(create_area, buf);
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
        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.style_border_focused())
                    .title(format!(" {} ", cfg.title))
                    .style(Style::default().bg(self.theme.bg_panel)),
            )
            .wrap(Wrap { trim: false })
            .render(settings_area, buf);
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
        Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.style_border_focused())
                    .title(format!(" {} ", cfg.title))
                    .style(Style::default().bg(self.theme.bg_panel)),
            )
            .render(help_area, buf);
    }

    pub(super) fn render_picker_overlay(&self, area: Rect, buf: &mut Buffer) {
        let Some(presentation) = picker_presentation(self.picker) else {
            return;
        };
        let lines = picker_lines(&presentation);
        let Some((w, h)) = fit_overlay_rect(area, 58, lines.len().saturating_add(2) as u16, 8)
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
        let mut visible = visible_picker_lines(lines, h);
        if picker_requires_vertical_centering(&presentation) {
            let content_height = h.saturating_sub(2) as usize;
            if content_height > visible.len() {
                let top_padding = (content_height - visible.len()) / 2;
                let mut padded = Vec::with_capacity(top_padding + visible.len());
                padded.extend((0..top_padding).map(|_| (String::new(), PickerLineKind::Normal)));
                padded.append(&mut visible);
                visible = padded;
            }
        }
        let text = visible
            .into_iter()
            .map(render_picker_line(self.theme))
            .collect::<Vec<_>>();
        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.style_border_focused())
                    .title(format!(" {} ", picker_title(&presentation)))
                    .style(Style::default().bg(self.theme.bg_panel)),
            )
            .wrap(Wrap { trim: false })
            .render(picker_area, buf);
    }

    pub(super) fn render_access_control_overlay(&self, area: Rect, buf: &mut Buffer) {
        if !self.access_control.open {
            return;
        }

        let Some(overlay_area) = access_control_area(area, self.access_control.mode) else {
            return;
        };
        if self.access_control.mode == AccessControlMode::Confirm {
            render_confirm_choice_modal(
                self,
                overlay_area,
                buf,
                "Confirm Access",
                confirm_message(self).as_str(),
                Vec::new(),
                self.access_control.confirm_yes,
                self.access_control.submit_state.clone(),
                self.access_control.error.as_deref(),
                "Applying access change...",
            );
            return;
        }
        let (title, mut lines) = access_overlay_copy(self);

        push_error_line(self, &mut lines, self.access_control.error.as_deref());
        push_hint_line(
            self,
            &mut lines,
            "Tab: next field  Arrow: cycle selector  Enter: submit  Esc: close",
        );
        render_modal_overlay(self, overlay_area, buf, title, lines);
    }

    pub(super) fn render_transfer_overlay(&self, area: Rect, buf: &mut Buffer) {
        if !self.transfer_modal.open {
            return;
        }

        let Some(overlay_area) = transfer_modal_area(area, self.transfer_modal.mode) else {
            return;
        };
        if self.transfer_modal.mode == TransferModalMode::Confirm {
            render_confirm_choice_modal(
                self,
                overlay_area,
                buf,
                "Confirm Transfer",
                "Send this transfer?",
                transfer_confirm_summary(self),
                self.transfer_modal.confirm_yes,
                self.transfer_modal.submit_state.clone(),
                self.transfer_modal.error.as_deref(),
                "Submitting transfer...",
            );
            return;
        }
        let (title, mut lines) = transfer_overlay_copy(self);
        push_error_line(self, &mut lines, self.transfer_modal.error.as_deref());
        render_modal_overlay(self, overlay_area, buf, title, lines);
    }

    pub(super) fn render_add_memory_overlay(&self, area: Rect, buf: &mut Buffer) {
        if !self.add_memory.open {
            return;
        }

        let Some(overlay_area) = add_memory_area(area) else {
            return;
        };

        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Existing Memory Canister ID",
                self.theme.style_dim(),
            )),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    if self.add_memory.value.is_empty() {
                        "<enter existing memory canister id>".to_string()
                    } else {
                        self.add_memory.value.to_string()
                    },
                    if self.add_memory.value.is_empty() {
                        self.theme.style_muted()
                    } else {
                        self.theme.style_accent_bold()
                    },
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                match self.add_memory.submit_state {
                    CreateSubmitState::Submitting => {
                        "Checking access to existing memory via get_users()..."
                    }
                    CreateSubmitState::Idle | CreateSubmitState::Error => {
                        "Enter: submit  Backspace: delete  Esc: close"
                    }
                },
                self.theme.style_muted(),
            )),
        ];
        push_error_line(self, &mut lines, self.add_memory.error.as_deref());
        render_modal_overlay(
            self,
            overlay_area,
            buf,
            "Add Existing Memory Canister",
            lines,
        );
    }

    pub(super) fn render_remove_memory_overlay(&self, area: Rect, buf: &mut Buffer) {
        if !self.remove_memory_open {
            return;
        }

        let Some(overlay_area) = remove_memory_area(area) else {
            return;
        };
        render_confirm_choice_modal(
            self,
            overlay_area,
            buf,
            "Remove Memory",
            "Remove this manually added memory from the list?",
            Vec::new(),
            self.remove_memory_confirm_yes,
            self.remove_memory_submit_state.clone(),
            self.remove_memory_error.as_deref(),
            "Removing memory from local list...",
        );
    }

    pub(super) fn render_rename_memory_overlay(&self, area: Rect, buf: &mut Buffer) {
        if !self.rename_memory.form.open {
            return;
        }

        let Some(overlay_area) = rename_memory_area(area) else {
            return;
        };

        let name_style = if self.rename_memory.focus == RenameModalFocus::Name {
            self.theme.style_accent_bold()
        } else if self.rename_memory.form.value.is_empty() {
            self.theme.style_muted()
        } else {
            self.theme.style_normal()
        };
        let submit_style = if self.rename_memory.focus == RenameModalFocus::Submit {
            self.theme.style_accent_bold()
        } else {
            self.theme.style_normal()
        };
        let submit_label = match self.rename_memory.form.submit_state {
            CreateSubmitState::Submitting => "Renaming...",
            CreateSubmitState::Idle | CreateSubmitState::Error => "Rename",
        };

        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled("Memory Name", self.theme.style_dim())),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    if self.rename_memory.form.value.is_empty() {
                        "<enter memory name>".to_string()
                    } else {
                        self.rename_memory.form.value.to_string()
                    },
                    name_style,
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    if self.rename_memory.focus == RenameModalFocus::Submit {
                        format!("[{submit_label}]")
                    } else {
                        submit_label.to_string()
                    },
                    submit_style,
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                match self.rename_memory.form.submit_state {
                    CreateSubmitState::Submitting => "Updating memory name...",
                    CreateSubmitState::Idle | CreateSubmitState::Error => {
                        "Tab: next field  Shift+Tab: previous field  Enter: action  Esc: close"
                    }
                },
                self.theme.style_muted(),
            )),
        ];
        push_error_line(self, &mut lines, self.rename_memory.form.error.as_deref());
        render_modal_overlay(self, overlay_area, buf, "Rename Memory", lines);
    }
}

fn add_memory_area(area: Rect) -> Option<Rect> {
    centered_overlay_area(area, 58, 10, 8)
}

fn remove_memory_area(area: Rect) -> Option<Rect> {
    centered_overlay_area(area, 62, 11, 9)
}

fn rename_memory_area(area: Rect) -> Option<Rect> {
    centered_overlay_area(area, 58, 11, 8)
}

fn access_control_area(area: Rect, mode: AccessControlMode) -> Option<Rect> {
    let (desired_width, desired_height, min_height) = match mode {
        AccessControlMode::Action => (58, 11, 9),
        AccessControlMode::Add => (56, 13, 10),
        AccessControlMode::Confirm => (52, 9, 7),
        AccessControlMode::None => (48, 8, 6),
    };
    centered_overlay_area(area, desired_width, desired_height, min_height)
}

fn transfer_modal_area(area: Rect, mode: TransferModalMode) -> Option<Rect> {
    let (desired_width, desired_height, min_height) = match mode {
        TransferModalMode::Edit => (60, 16, 13),
        TransferModalMode::Confirm => (62, 12, 10),
    };
    centered_overlay_area(area, desired_width, desired_height, min_height)
}

fn centered_overlay_area(
    area: Rect,
    desired_width: u16,
    desired_height: u16,
    min_height: u16,
) -> Option<Rect> {
    let (width, height) = fit_overlay_rect(area, desired_width, desired_height, min_height)?;
    Some(Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    })
}

fn render_modal_overlay(
    ui: &TuiKitUi<'_>,
    overlay_area: Rect,
    buf: &mut Buffer,
    title: &str,
    lines: Vec<Line<'static>>,
) {
    Clear.render(overlay_area, buf);
    Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(ui.theme.style_border_focused())
                .title(format!(" {title} "))
                .style(Style::default().bg(ui.theme.bg_panel)),
        )
        .wrap(Wrap { trim: false })
        .render(overlay_area, buf);
}

fn blank_line() -> Line<'static> {
    Line::from("")
}

fn push_error_line(ui: &TuiKitUi<'_>, lines: &mut Vec<Line<'static>>, error: Option<&str>) {
    let Some(error) = error else {
        return;
    };
    lines.push(blank_line());
    lines.push(Line::from(Span::styled(
        format!("  {error}"),
        ui.theme.style_error(),
    )));
}

fn push_hint_line(ui: &TuiKitUi<'_>, lines: &mut Vec<Line<'static>>, hint: &str) {
    lines.push(blank_line());
    lines.push(Line::from(Span::styled(
        hint.to_string(),
        ui.theme.style_muted(),
    )));
}

fn render_confirm_choice_modal(
    ui: &TuiKitUi<'_>,
    overlay_area: Rect,
    buf: &mut Buffer,
    title: &str,
    message: &str,
    summary_lines: Vec<Line<'static>>,
    confirm_yes: bool,
    submit_state: CreateSubmitState,
    error: Option<&str>,
    submitting_hint: &'static str,
) {
    let mut lines = vec![Line::from(Span::styled(
        message.to_string(),
        ui.theme.style_accent_bold(),
    ))];
    if !summary_lines.is_empty() {
        lines.push(blank_line());
        lines.extend(summary_lines);
    }
    lines.push(blank_line());
    lines.push(Line::from(Span::styled(
        if confirm_yes {
            "[yes] / no".to_string()
        } else {
            " yes / [no]".to_string()
        },
        ui.theme.style_accent_bold(),
    )));
    lines.push(blank_line());
    lines.push(Line::from(Span::styled(
        match submit_state {
            CreateSubmitState::Submitting => submitting_hint,
            CreateSubmitState::Idle | CreateSubmitState::Error => "Enter: continue  Esc: close",
        },
        ui.theme.style_muted(),
    )));
    push_error_line(ui, &mut lines, error);
    render_modal_overlay(ui, overlay_area, buf, title, lines);
}

fn access_principal_value(ui: &TuiKitUi<'_>) -> String {
    if ui.access_control.principal_id.is_empty() {
        "<principal or anonymous>".to_string()
    } else if ui.access_control.mode == AccessControlMode::Add {
        ui.access_control.principal_id.to_string()
    } else {
        shorten_principal(ui.access_control.principal_id.as_str())
    }
}

fn access_principal_span(ui: &TuiKitUi<'_>) -> Span<'static> {
    if ui.access_control.principal_id.is_empty() {
        Span::styled(access_principal_value(ui), ui.theme.style_muted())
    } else {
        Span::styled(
            access_principal_value(ui),
            access_field_style(ui, AccessControlFocus::Principal),
        )
    }
}

fn access_role_value(ui: &TuiKitUi<'_>) -> String {
    match ui.access_control.role {
        AccessControlRole::Admin => "[admin] / writer / reader".to_string(),
        AccessControlRole::Writer => " admin / [writer] / reader".to_string(),
        AccessControlRole::Reader => " admin / writer / [reader]".to_string(),
    }
}

fn access_field_style(ui: &TuiKitUi<'_>, focus: AccessControlFocus) -> ratatui::style::Style {
    if ui.access_control.focus == focus {
        ui.theme.style_accent_bold()
    } else {
        ui.theme.style_normal()
    }
}

fn access_overlay_copy(ui: &TuiKitUi<'_>) -> (&'static str, Vec<Line<'static>>) {
    match ui.access_control.mode {
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
                    format!(
                        "Current role: {}",
                        role_label(ui.access_control.current_role)
                    ),
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
        AccessControlMode::None => ("Access", vec![]),
        AccessControlMode::Confirm => ("Confirm Access", vec![]),
    }
}

fn transfer_overlay_copy(ui: &TuiKitUi<'_>) -> (&'static str, Vec<Line<'static>>) {
    match ui.transfer_modal.mode {
        TransferModalMode::Edit => {
            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled("Recipient principal", ui.theme.style_dim())),
                transfer_value_line(
                    ui,
                    ui.transfer_modal.principal_id.as_str(),
                    "<enter recipient principal>",
                    TransferModalFocus::Principal,
                ),
                Line::from(""),
                Line::from(Span::styled("Amount (KINIC)", ui.theme.style_dim())),
                transfer_amount_line(ui),
                Line::from(""),
                transfer_submit_line(ui),
                Line::from(""),
            ];
            if ui.transfer_modal.prerequisites_loading {
                lines.push(Line::from(Span::styled(
                    "Loading available balance and ledger fee...",
                    ui.theme.style_muted(),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    transfer_balance_hint(ui),
                    ui.theme.style_muted(),
                )));
            }
            lines.extend([Line::from(Span::styled(
                if ui.transfer_modal.prerequisites_loading {
                    "Tab: next field  Shift+Tab: previous field  Esc: close"
                } else {
                    "Tab: next field  Shift+Tab: previous field  Enter: action  Esc: close"
                },
                ui.theme.style_muted(),
            ))]);
            ("Send KINIC", lines)
        }
        TransferModalMode::Confirm => ("Confirm Transfer", vec![]),
    }
}

fn transfer_confirm_summary(ui: &TuiKitUi<'_>) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            format!("  To: {}", ui.transfer_modal.principal_id),
            ui.theme.style_normal(),
        )),
        Line::from(Span::styled(
            format!("  Amount: {}", transfer_amount_display(ui)),
            ui.theme.style_normal(),
        )),
        Line::from(Span::styled(
            format!("  Fee: {}", transfer_fee_display(ui)),
            ui.theme.style_normal(),
        )),
        Line::from(Span::styled(
            format!("  Total debit: {}", transfer_total_display(ui)),
            ui.theme.style_normal(),
        )),
    ]
}

fn transfer_value_line(
    ui: &TuiKitUi<'_>,
    value: &str,
    placeholder: &str,
    focus: TransferModalFocus,
) -> Line<'static> {
    let text = if value.is_empty() { placeholder } else { value };
    let style = if value.is_empty() {
        ui.theme.style_muted()
    } else if ui.transfer_modal.focus == focus {
        ui.theme.style_accent_bold()
    } else {
        ui.theme.style_normal()
    };
    Line::from(vec![Span::raw("  "), Span::styled(text.to_string(), style)])
}

fn transfer_amount_line(ui: &TuiKitUi<'_>) -> Line<'static> {
    let amount_text = if ui.transfer_modal.amount.is_empty() {
        "<enter amount>".to_string()
    } else {
        ui.transfer_modal.amount.to_string()
    };
    let amount_style = if ui.transfer_modal.amount.is_empty() {
        if ui.transfer_modal.focus == TransferModalFocus::Amount {
            ui.theme.style_accent_bold()
        } else {
            ui.theme.style_muted()
        }
    } else if ui.transfer_modal.focus == TransferModalFocus::Amount {
        ui.theme.style_accent_bold()
    } else {
        ui.theme.style_normal()
    };

    let max_text = if ui.transfer_modal.prerequisites_loading {
        "Max".to_string()
    } else if ui.transfer_modal.focus == TransferModalFocus::Max {
        "[Max]".to_string()
    } else {
        "Max".to_string()
    };
    let max_style = if ui.transfer_modal.prerequisites_loading {
        ui.theme.style_muted()
    } else {
        transfer_action_style(ui, TransferModalFocus::Max)
    };

    Line::from(vec![
        Span::raw("  "),
        Span::styled(amount_text, amount_style),
        Span::raw("   "),
        Span::styled(max_text, max_style),
    ])
}

fn transfer_submit_line(ui: &TuiKitUi<'_>) -> Line<'static> {
    if ui.transfer_modal.prerequisites_loading {
        return Line::from(vec![
            Span::raw("  "),
            Span::styled("Submit", ui.theme.style_muted()),
        ]);
    }
    let submit_text = if ui.transfer_modal.focus == TransferModalFocus::Submit {
        if ui.transfer_modal.submit_state == CreateSubmitState::Submitting {
            "[Submitting...]"
        } else {
            "[Submit]"
        }
    } else if ui.transfer_modal.submit_state == CreateSubmitState::Submitting {
        "Submitting..."
    } else {
        "Submit"
    };
    Line::from(vec![
        Span::raw("  "),
        Span::styled(
            submit_text.to_string(),
            transfer_action_style(ui, TransferModalFocus::Submit),
        ),
    ])
}

fn transfer_action_style(ui: &TuiKitUi<'_>, focus: TransferModalFocus) -> ratatui::style::Style {
    if ui.transfer_modal.focus == focus {
        ui.theme.style_accent_bold()
    } else {
        ui.theme.style_normal()
    }
}

fn transfer_balance_hint(ui: &TuiKitUi<'_>) -> String {
    let balance = ui
        .transfer_modal
        .available_balance_base_units
        .map(format_e8s_to_kinic_string_u128)
        .map(truncate_kinic_fraction_to_three_digits)
        .unwrap_or_else(|| "unavailable".to_string());
    let fee = transfer_fee_display(ui);
    format!("Balance: {balance} KINIC  |  Fee: {fee}")
}

fn transfer_fee_display(ui: &TuiKitUi<'_>) -> String {
    ui.transfer_modal
        .fee_base_units
        .map(format_e8s_to_kinic_string_u128)
        .map(|value| format!("{} KINIC", truncate_kinic_fraction_to_three_digits(value)))
        .unwrap_or_else(|| "unavailable".to_string())
}

/// See `rust/tui/settings.rs`: compact display, first three fractional digits only.
fn truncate_kinic_fraction_to_three_digits(value: String) -> String {
    match value.split_once('.') {
        Some((whole, fraction)) => {
            let limited = &fraction[..fraction.len().min(3)];
            format!("{whole}.{limited}")
        }
        None => value,
    }
}

fn transfer_amount_display(ui: &TuiKitUi<'_>) -> String {
    if ui.transfer_modal.amount.is_empty() {
        "0.00000000 KINIC".to_string()
    } else {
        format!("{} KINIC", ui.transfer_modal.amount)
    }
}

fn transfer_total_display(ui: &TuiKitUi<'_>) -> String {
    let amount_base_units = parse_kinic_display_to_e8s(ui.transfer_modal.amount.as_str());
    match (amount_base_units, ui.transfer_modal.fee_base_units) {
        (Some(amount), Some(fee)) => {
            format!(
                "{} KINIC",
                format_e8s_to_kinic_string_u128(amount.saturating_add(fee))
            )
        }
        _ => "unavailable".to_string(),
    }
}

fn parse_kinic_display_to_e8s(value: &str) -> Option<u128> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Some(0);
    }
    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() > 2 {
        return None;
    }
    let whole = parts[0];
    let fraction = if parts.len() == 2 { parts[1] } else { "" };
    if !whole.chars().all(|char| char.is_ascii_digit())
        || !fraction.chars().all(|char| char.is_ascii_digit())
        || fraction.len() > 8
    {
        return None;
    }
    let whole_value = if whole.is_empty() {
        0u128
    } else {
        whole.parse::<u128>().ok()?
    };
    let fractional_value = format!("{fraction:0<8}").parse::<u128>().ok()?;
    whole_value
        .checked_mul(100_000_000u128)?
        .checked_add(fractional_value)
}

fn add_user_principal_line(ui: &TuiKitUi<'_>) -> Line<'static> {
    let mut spans = vec![Span::raw("  "), access_principal_span(ui)];
    if ui.access_control.principal_id.is_empty() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(r#"type "anonymous""#, ui.theme.style_muted()));
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
        let style = if role == ui.access_control.current_role {
            ui.theme.style_muted()
        } else if ui.access_control.action == AccessControlAction::Change
            && ui.access_control.role == role
        {
            ui.theme.style_accent_bold()
        } else {
            ui.theme.style_normal()
        };
        let text = if role == ui.access_control.current_role {
            label.to_string()
        } else if ui.access_control.action == AccessControlAction::Change
            && ui.access_control.role == role
        {
            format!("[{label}]")
        } else {
            label.to_string()
        };
        spans.push(Span::styled(text, style));
    }
    spans.push(Span::raw(" / "));
    let remove_text = if ui.access_control.action == AccessControlAction::Remove {
        "[remove]".to_string()
    } else {
        "remove".to_string()
    };
    let remove_style = if ui.access_control.action == AccessControlAction::Remove {
        ui.theme.style_accent_bold()
    } else {
        ui.theme.style_normal()
    };
    spans.push(Span::styled(remove_text, remove_style));
    Line::from(spans)
}

fn confirm_message(ui: &TuiKitUi<'_>) -> String {
    match ui.access_control.action {
        AccessControlAction::Remove => {
            format!("Remove access for {}?", access_principal_value(ui))
        }
        AccessControlAction::Change => {
            format!(
                "Change role for {} to {}?",
                access_principal_value(ui),
                role_label(ui.access_control.role)
            )
        }
        AccessControlAction::Add => {
            format!(
                "Add {} as {}?",
                access_principal_value(ui),
                role_label(ui.access_control.role)
            )
        }
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

fn render_picker_line<'a>(
    theme: &'a crate::ui::theme::Theme,
) -> impl Fn((String, PickerLineKind)) -> Line<'static> + 'a {
    move |(line, kind)| match kind {
        PickerLineKind::Selected => Line::from(Span::styled(line, theme.style_accent_bold())),
        PickerLineKind::CurrentDefault => Line::from(Span::styled(line, theme.style_accent())),
        PickerLineKind::Hint => Line::from(Span::styled(line, theme.style_muted())),
        PickerLineKind::Normal => Line::from(Span::styled(line, theme.style_normal())),
    }
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

fn visible_picker_lines(
    lines: Vec<(String, PickerLineKind)>,
    overlay_height: u16,
) -> Vec<(String, PickerLineKind)> {
    let fixed_suffix_len = 2usize;
    if lines.len() <= fixed_suffix_len {
        return lines;
    }

    let candidate_rows = overlay_height.saturating_sub(2) as usize;
    let visible_body_len = candidate_rows.saturating_sub(fixed_suffix_len);
    if visible_body_len == 0 {
        return lines;
    }

    let body_end = lines.len().saturating_sub(fixed_suffix_len);
    let body = &lines[..body_end];
    if body.len() <= visible_body_len {
        return center_picker_body(body, &lines[body_end..], visible_body_len);
    }

    let selected_in_body = body
        .iter()
        .position(|(_, kind)| *kind == PickerLineKind::Selected)
        .unwrap_or(0);
    let max_start = body.len().saturating_sub(visible_body_len);
    let start = selected_in_body
        .saturating_sub(visible_body_len / 2)
        .min(max_start);
    let end = start + visible_body_len;

    let mut visible = Vec::with_capacity(visible_body_len + fixed_suffix_len);
    visible.extend_from_slice(&body[start..end]);
    visible.extend_from_slice(&lines[body_end..]);
    visible
}

fn center_picker_body(
    body: &[(String, PickerLineKind)],
    suffix: &[(String, PickerLineKind)],
    visible_body_len: usize,
) -> Vec<(String, PickerLineKind)> {
    let extra_space = visible_body_len.saturating_sub(body.len());
    let top_padding = extra_space / 2;
    let bottom_padding = extra_space.saturating_sub(top_padding);

    let mut visible = Vec::with_capacity(visible_body_len + suffix.len());
    visible.extend((0..top_padding).map(|_| (String::new(), PickerLineKind::Normal)));
    visible.extend_from_slice(body);
    visible.extend((0..bottom_padding).map(|_| (String::new(), PickerLineKind::Normal)));
    visible.extend_from_slice(suffix);
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
    use crate::ui::app::screens::settings::{PickerPresentation, picker_input_placeholder};
    use crate::ui::theme::Theme;
    use crate::ui::{TabId, TuiKitUi};
    use ratatui::{buffer::Buffer, widgets::Widget};
    use tui_kit_runtime::{
        AccessControlAction, AccessControlModalState, AccessControlMode, CreateSubmitState,
        PickerContext, PickerItem, SettingsEntry, SettingsSection, SettingsSnapshot,
        TextInputModalState, TransferModalMode, TransferModalState,
    };

    fn render_ui(ui: TuiKitUi<'_>, area: Rect) -> String {
        let mut buf = Buffer::empty(area);
        Widget::render(ui, area, &mut buf);
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
        assert!(joined.contains("Item 7"));
        assert!(!joined.contains("Item 8"));
    }

    #[test]
    fn visible_picker_lines_keep_selected_row() {
        let items = (0..12)
            .map(|index| {
                PickerItem::option(format!("id-{index}"), format!("Memory {index}"), false)
            })
            .collect::<Vec<_>>();
        let lines = picker_lines(&PickerPresentation::List {
            context: PickerContext::DefaultMemory,
            items: &items,
            selected_index: 10,
        });
        let visible = visible_picker_lines(lines, 8);
        assert!(
            visible
                .iter()
                .any(|(_, kind)| *kind == PickerLineKind::Selected)
        );
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
    fn help_overlay_insert_line_mentions_target_picker() {
        let cfg = UiConfig::default().help;

        assert!(cfg.lines.iter().any(|line| {
            line
                == "Insert form: ←/→ switch mode, Enter moves to next field / opens pickers / browses file / submits"
        }));
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
        let items = (0..12)
            .map(|index| {
                PickerItem::option(format!("id-{index}"), format!("Memory {index}"), index == 2)
            })
            .collect::<Vec<_>>();
        let lines = picker_lines(&PickerPresentation::List {
            context: PickerContext::DefaultMemory,
            items: &items,
            selected_index: 10,
        });
        let visible = visible_picker_lines(lines, 8);
        let visible_lines = visible
            .iter()
            .map(|(line, _)| line.as_str())
            .collect::<Vec<_>>();
        assert!(visible_lines.iter().any(|line| line.contains("Memory 10")));
    }

    #[test]
    fn picker_input_placeholder_is_available_for_add_tag() {
        assert_eq!(
            picker_input_placeholder(PickerContext::AddTag),
            "<enter a new tag>"
        );
    }

    #[test]
    fn add_memory_overlay_renders_error_line() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .current_tab_id(TabId::new("tab-1"))
            .add_memory_modal(TextInputModalState {
                open: true,
                submit_state: CreateSubmitState::Error,
                error: Some("boom".to_string()),
                ..TextInputModalState::default()
            });

        let rendered = render_ui(ui, Rect::new(0, 0, 100, 30));

        assert!(rendered.contains("Add Existing Memory Canister"));
        assert!(rendered.contains("boom"));
        assert!(rendered.contains("Enter: submit"));
    }

    #[test]
    fn access_overlay_renders_hint_line() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .current_tab_id(TabId::new("tab-1"))
            .access_control_modal(AccessControlModalState {
                open: true,
                mode: AccessControlMode::Add,
                ..AccessControlModalState::default()
            });

        let rendered = render_ui(ui, Rect::new(0, 0, 100, 30));

        assert!(rendered.contains("Arrow: cycle selector"));
        assert!(rendered.contains("Esc: close"));
    }

    #[test]
    fn transfer_confirm_overlay_renders_summary_lines() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .current_tab_id(TabId::new("tab-1"))
            .transfer_modal(TransferModalState {
                open: true,
                mode: TransferModalMode::Confirm,
                principal_id: "aaaaa-aa".to_string(),
                amount: "1.25000000".to_string(),
                fee_base_units: Some(100_000),
                confirm_yes: true,
                ..TransferModalState::default()
            });

        let rendered = render_ui(ui, Rect::new(0, 0, 100, 30));

        assert!(rendered.contains("Confirm Transfer"));
        assert!(rendered.contains("Total debit"));
        assert!(rendered.contains("[yes] / no"));
    }

    #[test]
    fn transfer_edit_overlay_renders_inline_max_action() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .current_tab_id(TabId::new("tab-1"))
            .transfer_modal(TransferModalState {
                open: true,
                amount: String::new(),
                ..TransferModalState::default()
            });

        let rendered = render_ui(ui, Rect::new(0, 0, 100, 30));

        assert!(rendered.contains("Amount (KINIC)"));
        assert!(rendered.contains("<enter amount>"));
        assert!(rendered.contains("Max"));
    }

    #[test]
    fn access_confirm_overlay_renders_shared_confirm_copy() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .current_tab_id(TabId::new("tab-1"))
            .access_control_modal(AccessControlModalState {
                open: true,
                mode: AccessControlMode::Confirm,
                action: AccessControlAction::Remove,
                principal_id: "aaaaa-aa".to_string(),
                confirm_yes: false,
                ..AccessControlModalState::default()
            });

        let rendered = render_ui(ui, Rect::new(0, 0, 100, 30));

        assert!(rendered.contains("Confirm Access"));
        assert!(rendered.contains("Remove access"));
        assert!(rendered.contains("yes / [no]"));
    }

    #[test]
    fn remove_memory_overlay_renders_shared_confirm_copy() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .current_tab_id(TabId::new("tab-1"))
            .remove_memory_open(true)
            .remove_memory_confirm_yes(false);

        let rendered = render_ui(ui, Rect::new(0, 0, 100, 30));

        assert!(rendered.contains("Remove Memory"));
        assert!(rendered.contains("Remove this manually added memory from the list?"));
        assert!(rendered.contains("yes / [no]"));
    }
}
