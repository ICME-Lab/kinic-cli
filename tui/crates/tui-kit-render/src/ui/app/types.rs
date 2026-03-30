//! Shared UI types: tabs, focus.

/// Generic tab identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TabId(pub String);

impl TabId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Data-driven tab definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabSpec {
    pub id: TabId,
    pub title: String,
    pub search_placeholder: String,
}

/// Runtime UI configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiConfig {
    pub tabs: Vec<TabSpec>,
    pub branding: BrandingText,
    pub header: HeaderText,
    pub chat: ChatPanelText,
    pub insert: InsertScreenText,
    pub create: CreateOverlayText,
    pub settings: SettingsOverlayText,
    pub help: HelpOverlayText,
    pub status: StatusText,
}

/// Header branding text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrandingText {
    pub logo_lines: Vec<String>,
    pub attribution: String,
}

/// Header metrics text configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderText {
    pub visible_icon: String,
    pub visible_suffix: String,
    pub contexts_icon: String,
    pub contexts_suffix: String,
    pub data_label: String,
}

/// Chat panel text configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatPanelText {
    pub title: String,
    pub loading_hint: String,
    pub input_placeholder: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateOverlayText {
    pub title: String,
    pub intro_description: String,
    pub intro_enter_hint: String,
    pub intro_cycle_hint: String,
    pub intro_escape_hint: String,
    pub name_label: String,
    pub description_label: String,
    pub submit_label: String,
    pub submit_pending_label: String,
    pub open_hint: String,
    pub tabs_focus_hint: String,
    pub close_hint: String,
    pub account_title: String,
    pub loading_message: String,
    pub principal_label: String,
    pub balance_label: String,
    pub price_label: String,
    pub status_label: String,
    pub status_ready_label: String,
    pub status_insufficient_label: String,
    pub unavailable_message: String,
    pub error_prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsertScreenText {
    pub title: String,
    pub intro_description: String,
    pub mode_label: String,
    pub memory_id_label: String,
    pub tag_label: String,
    pub text_label: String,
    pub file_path_label: String,
    pub embedding_label: String,
    pub submit_label: String,
    pub submit_pending_label: String,
    pub close_hint: String,
}

/// Settings overlay text configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsOverlayText {
    pub title: String,
    pub close_hint: String,
}

/// Help overlay text configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpOverlayText {
    pub title: String,
    pub lines: Vec<String>,
    pub close_hint: String,
}

/// Status bar text configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusText {
    pub commands_label: String,
    pub tabs_label: String,
    pub quit_label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InsertFormCopy {
    pub close_hint: &'static str,
    pub help_line: &'static str,
    pub status_enter_hint: &'static str,
}

pub(crate) fn insert_form_copy() -> InsertFormCopy {
    InsertFormCopy {
        close_hint: "Tab: cycle fields, Enter: cycle mode / open target picker / submit, Esc: back to tab focus",
        help_line: "Insert form: ←/→ switch mode, Enter cycles mode / opens target picker / submits",
        status_enter_hint: " cycle/picker/submit ",
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        let insert_form_copy = insert_form_copy();
        Self {
            tabs: default_tab_specs(),
            branding: BrandingText {
                logo_lines: default_branding_lines(),
                attribution: "👤 created by yashksaini-coder".to_string(),
            },
            header: HeaderText {
                visible_icon: "📦".to_string(),
                visible_suffix: "visible items".to_string(),
                contexts_icon: "📚".to_string(),
                contexts_suffix: "contexts".to_string(),
                data_label: "data".to_string(),
            },
            chat: ChatPanelText {
                title: " ◇ Chat ".to_string(),
                loading_hint: "  … Assistant is thinking…".to_string(),
                input_placeholder: "Ask about this item… (Enter to send, Esc to close)".to_string(),
            },
            insert: InsertScreenText {
                title: "Insert Memory Content".to_string(),
                intro_description:
                    "Insert text, raw embeddings, or PDFs without leaving the tab view."
                        .to_string(),
                mode_label: "Mode".to_string(),
                memory_id_label: "Memory ID".to_string(),
                tag_label: "Tag".to_string(),
                text_label: "Text".to_string(),
                file_path_label: "File Path".to_string(),
                embedding_label: "Embedding JSON".to_string(),
                submit_label: "Insert".to_string(),
                submit_pending_label: "Inserting...".to_string(),
                close_hint: insert_form_copy.close_hint.to_string(),
            },
            create: CreateOverlayText {
                title: "Create Memory".to_string(),
                intro_description: "Provision a new memory canister without leaving the tab view."
                    .to_string(),
                intro_enter_hint: "enter the form when tabs are focused".to_string(),
                intro_cycle_hint: "cycle fields".to_string(),
                intro_escape_hint: "step back one level".to_string(),
                name_label: "Name".to_string(),
                description_label: "Description".to_string(),
                submit_label: "Create".to_string(),
                submit_pending_label: "Creating...".to_string(),
                open_hint: "Press Ctrl-N to create a new memory".to_string(),
                tabs_focus_hint:
                    "Tabs focused. Press Enter or Tab to edit from Name, or Esc for Memories."
                        .to_string(),
                close_hint:
                    "Tab: cycle fields, Enter: submit, F5: refresh account info, Esc: back to tab focus"
                        .to_string(),
                account_title: "Account & Cost".to_string(),
                loading_message: "Loading account info...".to_string(),
                principal_label: "Principal".to_string(),
                balance_label: "KINIC balance".to_string(),
                price_label: "Create cost".to_string(),
                status_label: "Status".to_string(),
                status_ready_label: "Ready to create".to_string(),
                status_insufficient_label: "Insufficient balance".to_string(),
                unavailable_message: "Live account info unavailable in mock mode.".to_string(),
                error_prefix: "Account info error".to_string(),
            },
            settings: SettingsOverlayText {
                title: "Settings".to_string(),
                close_hint: "Esc: close".to_string(),
            },
            help: HelpOverlayText {
                title: "Help".to_string(),
                lines: vec![
                    "Tab: enter selected tab or move focus, Shift+Tab: previous focus"
                        .to_string(),
                    insert_form_copy.help_line.to_string(),
                    "/: focus search".to_string(),
                    "Esc: back / clear / close".to_string(),
                    "F5: refresh current view".to_string(),
                    "↑/↓: move selection".to_string(),
                    "Enter or →: open/focus content".to_string(),
                    "C: toggle chat panel".to_string(),
                    "?: toggle help".to_string(),
                    "q: quit".to_string(),
                ],
                close_hint: "Press any key to close".to_string(),
            },
            status: StatusText {
                commands_label: "Commands".to_string(),
                tabs_label: "tabs".to_string(),
                quit_label: "quit".to_string(),
            },
        }
    }
}

/// Generic default tabs used when hosts do not provide custom tab specs.
pub fn default_tab_specs() -> Vec<TabSpec> {
    vec![
        TabSpec {
            id: TabId::new("tab-1"),
            title: "Tab 1".to_string(),
            search_placeholder: "Search tab 1...".to_string(),
        },
        TabSpec {
            id: TabId::new("tab-2"),
            title: "Tab 2".to_string(),
            search_placeholder: "Search tab 2...".to_string(),
        },
        TabSpec {
            id: TabId::new("tab-3"),
            title: "Tab 3".to_string(),
            search_placeholder: "Search tab 3...".to_string(),
        },
        TabSpec {
            id: TabId::new("tab-4"),
            title: "Tab 4".to_string(),
            search_placeholder: "Search tab 4...".to_string(),
        },
    ]
}

fn default_branding_lines() -> Vec<String> {
    vec![
        "████████╗██╗   ██╗██╗".to_string(),
        "╚══██╔══╝██║   ██║██║".to_string(),
        "   ██║   ██║   ██║██║".to_string(),
        "   ██║   ██║   ██║██║".to_string(),
        "   ██║   ╚██████╔╝██║".to_string(),
        "   ╚═╝    ╚═════╝ ╚═╝".to_string(),
    ]
}

/// Focus state for keyboard navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    #[default]
    Search,
    Items,
    Tabs,
    Content,
    Form,
    /// In-TUI chat panel (only when chat is open)
    Chat,
}

impl Focus {
    /// Next focus; when `chat_open` is true, Content -> Chat -> Search.
    pub fn next(&self, chat_open: bool) -> Self {
        match self {
            Focus::Search => Focus::Items,
            Focus::Items => Focus::Content,
            Focus::Content => {
                if chat_open {
                    Focus::Chat
                } else {
                    Focus::Tabs
                }
            }
            Focus::Form => Focus::Tabs,
            Focus::Chat => Focus::Tabs,
            Focus::Tabs => Focus::Search,
        }
    }

    /// Previous focus.
    pub fn prev(&self, chat_open: bool) -> Self {
        match self {
            Focus::Search => Focus::Tabs,
            Focus::Items => Focus::Search,
            Focus::Content => Focus::Items,
            Focus::Form => Focus::Tabs,
            Focus::Chat => Focus::Content,
            Focus::Tabs => {
                if chat_open {
                    Focus::Chat
                } else {
                    Focus::Content
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_config_uses_shared_insert_form_copy() {
        let config = UiConfig::default();
        let copy = insert_form_copy();

        assert_eq!(config.insert.close_hint, copy.close_hint);
        assert!(config.help.lines.iter().any(|line| line == copy.help_line));
    }
}
