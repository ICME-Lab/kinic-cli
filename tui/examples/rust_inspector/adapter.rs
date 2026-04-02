//! Adapter from Rust analyzer output to generic UI DTOs.

use crate::domain_rust::analyzer::{
    AnalyzedItem, CrateInfo, DependencyKind, InstalledCrate, SourceLocation, Visibility,
};
use crate::domain_rust::crates_io::CrateDocInfo;
use tui_kit_model::{
    UiContextNode, UiItemContent, UiItemKind, UiItemSummary, UiRow, UiSection, UiSourceLocation,
    UiVisibility,
};

/// Convert an analyzer item into list-friendly summary data.
pub fn item_to_summary(item: &AnalyzedItem) -> UiItemSummary {
    UiItemSummary {
        id: item.qualified_name(),
        name: item.name().to_string(),
        leading_marker: None,
        kind: map_item_kind(item),
        visibility: map_visibility(item.visibility()),
        qualified_name: Some(item.qualified_name()),
        subtitle: item.documentation().and_then(first_line),
        tags: item_tags(item),
    }
}

/// Convert an analyzer item into a content payload for content-pane views.
pub fn item_to_content(item: &AnalyzedItem) -> UiItemContent {
    let docs = item.documentation().map(ToString::to_string);
    let mut sections = vec![UiSection {
        heading: "Overview".to_string(),
        rows: vec![
            UiRow {
                label: "Name".to_string(),
                value: item.name().to_string(),
            },
            UiRow {
                label: "Kind".to_string(),
                value: map_item_kind(item).label().to_string(),
            },
            UiRow {
                label: "Qualified".to_string(),
                value: item.qualified_name(),
            },
            UiRow {
                label: "Visibility".to_string(),
                value: visibility_str(map_visibility(item.visibility())).to_string(),
            },
        ],
        body_lines: Vec::new(),
    }];

    if let Some(d) = docs.as_ref() {
        sections.push(UiSection {
            heading: "Documentation".to_string(),
            rows: Vec::new(),
            body_lines: d.lines().map(ToString::to_string).collect(),
        });
    }

    UiItemContent {
        id: item.qualified_name(),
        title: item.name().to_string(),
        subtitle: None,
        kind: map_item_kind(item),
        definition: item.definition(),
        location: item.source_location().map(map_source_location),
        docs,
        badges: item_tags(item),
        sections,
    }
}

/// Convert root crate metadata to UI dependency node.
pub fn crate_info_to_node(info: &CrateInfo) -> UiContextNode {
    UiContextNode {
        name: info.name.clone(),
        version: Some(info.version.clone()),
        relation: Some("root".to_string()),
        description: info.description.clone(),
        docs_url: info.documentation.clone(),
        homepage_url: info.homepage.clone(),
        repository_url: info.repository.clone(),
        metadata: vec![
            UiRow {
                label: "Edition".to_string(),
                value: info.edition.clone(),
            },
            UiRow {
                label: "Features".to_string(),
                value: info.features.len().to_string(),
            },
            UiRow {
                label: "Dependencies".to_string(),
                value: info.dependencies.len().to_string(),
            },
        ],
    }
}

/// Convert a crates.io response to UI dependency node.
pub fn crate_doc_to_node(doc: &CrateDocInfo) -> UiContextNode {
    UiContextNode {
        name: doc.name.clone(),
        version: Some(doc.version.clone()),
        relation: Some("dependency".to_string()),
        description: doc.description.clone(),
        docs_url: doc.documentation.clone(),
        homepage_url: doc.homepage.clone(),
        repository_url: doc.repository.clone(),
        metadata: Vec::new(),
    }
}

/// Convert an installed crate metadata row to UI dependency node.
pub fn installed_crate_to_node(crate_info: &InstalledCrate) -> UiContextNode {
    UiContextNode {
        name: crate_info.name.clone(),
        version: Some(crate_info.version.clone()),
        relation: Some("installed".to_string()),
        description: crate_info.description.clone(),
        docs_url: crate_info.documentation.clone(),
        homepage_url: None,
        repository_url: crate_info.repository.clone(),
        metadata: vec![
            UiRow {
                label: "Path".to_string(),
                value: crate_info.path.display().to_string(),
            },
            UiRow {
                label: "Keywords".to_string(),
                value: crate_info.keywords.join(", "),
            },
            UiRow {
                label: "Categories".to_string(),
                value: crate_info.categories.join(", "),
            },
        ],
    }
}

/// Convert dependency kind to a generic relation string.
pub fn dependency_kind_label(kind: DependencyKind) -> &'static str {
    match kind {
        DependencyKind::Normal => "normal",
        DependencyKind::Dev => "dev",
        DependencyKind::Build => "build",
    }
}

fn map_source_location(loc: &SourceLocation) -> UiSourceLocation {
    UiSourceLocation {
        file: loc.file.as_ref().map(|p| p.display().to_string()),
        line: loc.line,
        column: loc.column,
    }
}

fn map_item_kind(item: &AnalyzedItem) -> UiItemKind {
    match item {
        AnalyzedItem::Function(_) => UiItemKind::Function,
        AnalyzedItem::Struct(_) | AnalyzedItem::Enum(_) | AnalyzedItem::TypeAlias(_) => {
            UiItemKind::Type
        }
        AnalyzedItem::Module(_) => UiItemKind::Module,
        AnalyzedItem::Trait(_) => UiItemKind::Trait,
        AnalyzedItem::Const(_) | AnalyzedItem::Static(_) => UiItemKind::Constant,
        AnalyzedItem::Impl(_) => UiItemKind::Implementation,
    }
}

fn map_visibility(vis: Option<Visibility>) -> UiVisibility {
    match vis {
        Some(Visibility::Public) => UiVisibility::Public,
        Some(Visibility::Crate) | Some(Visibility::Super) | Some(Visibility::SelfOnly) => {
            UiVisibility::Internal
        }
        _ => UiVisibility::Private,
    }
}

fn first_line(s: &str) -> Option<String> {
    let line = s.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        None
    } else {
        Some(line.to_string())
    }
}

fn item_tags(item: &AnalyzedItem) -> Vec<String> {
    match item {
        AnalyzedItem::Function(f) => {
            let mut tags = Vec::new();
            if f.is_async {
                tags.push("async".to_string());
            }
            if f.is_const {
                tags.push("const".to_string());
            }
            if f.is_unsafe {
                tags.push("unsafe".to_string());
            }
            tags
        }
        AnalyzedItem::Trait(t) => {
            let mut tags = Vec::new();
            if t.is_unsafe {
                tags.push("unsafe".to_string());
            }
            if t.is_auto {
                tags.push("auto".to_string());
            }
            tags
        }
        _ => Vec::new(),
    }
}

fn visibility_str(v: UiVisibility) -> &'static str {
    match v {
        UiVisibility::Public => "public",
        UiVisibility::Internal => "internal",
        UiVisibility::Private => "private",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain_rust::analyzer::RustAnalyzer;

    #[test]
    fn test_item_to_summary_and_detail() {
        let source = "pub async fn ping() -> Result<(), ()> { Ok(()) }";
        let items = RustAnalyzer::new().analyze_source(source).unwrap();
        let item = items.first().unwrap();

        let summary = item_to_summary(item);
        assert_eq!(summary.name, "ping");
        assert_eq!(summary.kind, UiItemKind::Function);
        assert!(summary.tags.iter().any(|t| t == "async"));

        let detail = item_to_detail(item);
        assert_eq!(detail.title, "ping");
        assert!(detail.definition.contains("ping"));
    }
}
