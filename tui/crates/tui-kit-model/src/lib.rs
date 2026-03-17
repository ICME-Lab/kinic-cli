//! UI-facing generic data model.
//!
//! This crate intentionally avoids parser/analyzer-specific types so the UI
//! can be reused with non-Rust data sources.

/// Source location shown by the UI.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UiSourceLocation {
    pub file: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

/// UI visibility indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UiVisibility {
    Public,
    Internal,
    #[default]
    Private,
}

/// Generic item kind used by the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiItemKind {
    Function,
    Type,
    Module,
    Trait,
    Constant,
    Implementation,
    Custom(String),
}

impl UiItemKind {
    /// Stable short label for list/inspector rendering.
    pub fn label(&self) -> &str {
        match self {
            UiItemKind::Function => "fn",
            UiItemKind::Type => "type",
            UiItemKind::Module => "mod",
            UiItemKind::Trait => "trait",
            UiItemKind::Constant => "const",
            UiItemKind::Implementation => "impl",
            UiItemKind::Custom(v) => v.as_str(),
        }
    }
}

/// List row summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiItemSummary {
    pub id: String,
    pub name: String,
    pub kind: UiItemKind,
    pub visibility: UiVisibility,
    pub qualified_name: Option<String>,
    pub subtitle: Option<String>,
    pub tags: Vec<String>,
}

/// Arbitrary label/value row for detail sections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiRow {
    pub label: String,
    pub value: String,
}

/// Optional section block displayed in detail pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiSection {
    pub heading: String,
    pub rows: Vec<UiRow>,
    pub body_lines: Vec<String>,
}

/// Inspector/detail payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiItemDetail {
    pub id: String,
    pub title: String,
    pub kind: UiItemKind,
    pub definition: String,
    pub location: Option<UiSourceLocation>,
    pub docs: Option<String>,
    pub badges: Vec<String>,
    pub sections: Vec<UiSection>,
}

/// Dependency or package list node.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UiContextNode {
    pub name: String,
    pub version: Option<String>,
    pub relation: Option<String>,
    pub description: Option<String>,
    pub docs_url: Option<String>,
    pub homepage_url: Option<String>,
    pub repository_url: Option<String>,
    pub metadata: Vec<UiRow>,
}
