//! Theme system for tui-kit TUI

use ratatui::style::{Color, Modifier, Style};

/// Color palette for the UI
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub accent: Color,
    pub accent_dim: Color,
    pub bg: Color,
    pub bg_highlight: Color,
    pub bg_panel: Color,
    pub fg: Color,
    pub fg_dim: Color,
    pub fg_muted: Color,
    pub border: Color,
    pub border_focused: Color,
    /// Active tab background (darker tone so white text is always visible)
    pub tab_active_bg: Color,
    /// Active tab text (white for consistent contrast across themes)
    pub tab_active_fg: Color,
    pub error: Color,
    pub warning: Color,
    pub success: Color,
    pub info: Color,
    // Syntax colors
    pub keyword: Color,
    pub function: Color,
    pub type_: Color,
    pub string: Color,
    pub number: Color,
    pub comment: Color,
}

impl Theme {
    pub fn kinic() -> Self {
        Self {
            name: "Kinic".into(),
            // Keep the current accent-forward palette used by the live UI.
            accent: Color::Rgb(255, 105, 180),
            accent_dim: Color::Rgb(214, 85, 152),
            bg: Color::Rgb(24, 24, 24),
            bg_highlight: Color::Rgb(45, 45, 45),
            bg_panel: Color::Rgb(32, 32, 32),
            fg: Color::Rgb(230, 230, 230),
            fg_dim: Color::Rgb(195, 195, 200),
            fg_muted: Color::Rgb(140, 140, 145),
            border: Color::Rgb(60, 60, 60),
            border_focused: Color::Rgb(255, 105, 180),
            tab_active_bg: Color::Rgb(166, 66, 126),
            tab_active_fg: Color::Rgb(255, 255, 255),
            error: Color::Rgb(244, 67, 54),
            warning: Color::Rgb(255, 152, 0),
            success: Color::Rgb(76, 175, 80),
            info: Color::Rgb(33, 150, 243),
            keyword: Color::Rgb(255, 121, 198),
            function: Color::Rgb(255, 183, 221),
            type_: Color::Rgb(235, 186, 255),
            string: Color::Rgb(152, 195, 121),
            number: Color::Rgb(255, 160, 198),
            comment: Color::Rgb(92, 99, 112),
        }
    }

    // Style builders
    pub fn style_accent(&self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn style_accent_bold(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn style_normal(&self) -> Style {
        Style::default().fg(self.fg)
    }

    pub fn style_dim(&self) -> Style {
        Style::default().fg(self.fg_dim)
    }

    pub fn style_muted(&self) -> Style {
        Style::default().fg(self.fg_muted)
    }

    pub fn style_highlight(&self) -> Style {
        Style::default().bg(self.bg_highlight)
    }

    /// Style for selected list rows. Uses explicit fg so text stays readable on the highlight background.
    pub fn style_selected(&self) -> Style {
        Style::default()
            .fg(self.fg)
            .bg(self.bg_highlight)
            .add_modifier(Modifier::BOLD)
    }

    pub fn style_border(&self) -> Style {
        Style::default().fg(self.border)
    }

    /// Active tab: button-style highlight (e.g. lavender bg, light text).
    pub fn style_tab_active(&self) -> Style {
        Style::default()
            .fg(self.tab_active_fg)
            .bg(self.tab_active_bg)
            .add_modifier(Modifier::BOLD)
    }

    /// Subtle accent-tinted border for the outer frame (soft glow effect).
    pub fn style_border_glow(&self) -> Style {
        Style::default().fg(self.accent).add_modifier(Modifier::DIM)
    }

    pub fn style_border_focused(&self) -> Style {
        Style::default().fg(self.border_focused)
    }

    pub fn style_error(&self) -> Style {
        Style::default().fg(self.error)
    }

    pub fn style_warning(&self) -> Style {
        Style::default().fg(self.warning)
    }

    pub fn style_success(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn style_info(&self) -> Style {
        Style::default().fg(self.info)
    }

    pub fn style_keyword(&self) -> Style {
        Style::default().fg(self.keyword)
    }

    pub fn style_function(&self) -> Style {
        Style::default().fg(self.function)
    }

    pub fn style_type(&self) -> Style {
        Style::default().fg(self.type_)
    }

    pub fn style_string(&self) -> Style {
        Style::default().fg(self.string)
    }

    pub fn style_number(&self) -> Style {
        Style::default().fg(self.number)
    }

    pub fn style_comment(&self) -> Style {
        Style::default().fg(self.comment)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::kinic()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_matches_kinic_theme() {
        let theme = Theme::default();

        assert_eq!(theme.name, "Kinic");
        assert_eq!(theme.accent, Color::Rgb(255, 105, 180));
    }

    #[test]
    fn selected_style_uses_highlight_background_and_foreground() {
        let theme = Theme::default();
        let style = theme.style_selected();

        assert_eq!(style.fg, Some(theme.fg));
        assert_eq!(style.bg, Some(theme.bg_highlight));
    }
}
