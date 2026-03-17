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

/// Settings overlay text configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsOverlayText {
    pub title: String,
    pub theme_label: String,
    pub theme_action_key: String,
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
    pub github_label: String,
    pub sponsor_label: String,
}

impl Default for UiConfig {
    fn default() -> Self {
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
            settings: SettingsOverlayText {
                title: "Settings".to_string(),
                theme_label: "Theme".to_string(),
                theme_action_key: "t".to_string(),
                close_hint: "Press Esc or S to close".to_string(),
            },
            help: HelpOverlayText {
                title: "Help".to_string(),
                lines: vec![
                    "Tab / Shift+Tab: switch panel focus".to_string(),
                    "/: focus search".to_string(),
                    "Esc: back / clear / close".to_string(),
                    "↑/↓ or j/k: move selection".to_string(),
                    "Enter or →: open/focus detail".to_string(),
                    "o/c: open primary / secondary context links".to_string(),
                    "C: toggle chat panel".to_string(),
                    "t: cycle theme".to_string(),
                    "?: toggle help".to_string(),
                    "q: quit".to_string(),
                    "g/s: open GitHub / Sponsor".to_string(),
                ],
                close_hint: "Press any key to close".to_string(),
            },
            status: StatusText {
                commands_label: "Commands".to_string(),
                tabs_label: "tabs".to_string(),
                quit_label: "quit".to_string(),
                github_label: "GitHub".to_string(),
                sponsor_label: "Sponsor".to_string(),
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
    List,
    Tabs,
    Inspector,
    /// In-TUI chat panel (only when chat is open)
    Chat,
}

impl Focus {
    /// Next focus; when `chat_open` is true, Inspector -> Chat -> Search.
    pub fn next(&self, chat_open: bool) -> Self {
        match self {
            Focus::Search => Focus::List,
            Focus::List => Focus::Inspector,
            Focus::Inspector => {
                if chat_open {
                    Focus::Chat
                } else {
                    Focus::Tabs
                }
            }
            Focus::Chat => Focus::Tabs,
            Focus::Tabs => Focus::Search,
        }
    }

    /// Previous focus.
    pub fn prev(&self, chat_open: bool) -> Self {
        match self {
            Focus::Search => Focus::Tabs,
            Focus::List => Focus::Search,
            Focus::Inspector => Focus::List,
            Focus::Chat => Focus::Inspector,
            Focus::Tabs => {
                if chat_open {
                    Focus::Chat
                } else {
                    Focus::Inspector
                }
            }
        }
    }
}
