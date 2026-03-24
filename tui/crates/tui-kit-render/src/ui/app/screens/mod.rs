//! Screen-oriented modules for the app body.

pub mod create;
pub mod memories;
pub mod placeholder;

use ratatui::{buffer::Buffer, layout::Rect};
use tui_kit_runtime::kinic_tabs::{TabKind, tab_kind};

use crate::ui::app::TuiKitUi;

struct PlaceholderScreenSpec<'a> {
    title: &'a str,
    lead: &'a str,
    detail: &'a str,
}

fn placeholder_screen_spec(kind: TabKind) -> Option<PlaceholderScreenSpec<'static>> {
    match kind {
        TabKind::PlaceholderMarket => Some(PlaceholderScreenSpec {
            title: "Market",
            lead: "Market tab is reserved for future discovery and purchase flows.",
            detail: "Use Memories to browse and Create to provision a new memory today.",
        }),
        TabKind::PlaceholderSettings => Some(PlaceholderScreenSpec {
            title: "Settings",
            lead: "Settings will move into this tab once the current overlay flows are retired.",
            detail: "For now use Shift+S to open settings.",
        }),
        _ => None,
    }
}

impl<'a> TuiKitUi<'a> {
    pub(crate) fn render_tab_screen(&self, area: Rect, buf: &mut Buffer) -> bool {
        match tab_kind(self.current_tab_id.0.as_str()) {
            TabKind::Form => {
                self.render_create_screen(area, buf);
                true
            }
            kind => {
                let Some(spec) = placeholder_screen_spec(kind) else {
                    return false;
                };
                self.render_placeholder_screen(area, buf, spec.title, spec.lead, spec.detail);
                true
            }
        }
    }
}
