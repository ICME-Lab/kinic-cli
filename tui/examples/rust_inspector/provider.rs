//! Rust-domain provider for domain-agnostic runtime contracts.

use crate::adapter::{item_to_detail, item_to_summary};
use crate::domain_rust::analyzer::AnalyzedItem;
use tui_kit_model::UiItemSummary;
use tui_kit_runtime::{
    CoreAction, CoreEffect, CoreResult, CoreState, DataProvider, ProviderOutput, ProviderSnapshot,
};

/// Provider that keeps Rust-analyzer items and exposes filtered snapshots.
pub struct RustProvider {
    base_items: Vec<AnalyzedItem>,
    query: String,
}

impl RustProvider {
    pub fn new(items: Vec<AnalyzedItem>) -> Self {
        Self {
            base_items: items,
            query: String::new(),
        }
    }

    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self
    }

    fn filtered(&self) -> Vec<&AnalyzedItem> {
        if self.query.is_empty() {
            return self.base_items.iter().collect();
        }
        let q = self.query.to_lowercase();
        self.base_items
            .iter()
            .filter(|it| {
                it.name().to_lowercase().contains(&q)
                    || it.qualified_name().to_lowercase().contains(&q)
            })
            .collect()
    }

    fn build_snapshot(&self, state: &CoreState) -> ProviderSnapshot {
        let filtered = self.filtered();
        let items: Vec<UiItemSummary> = filtered.iter().map(|it| item_to_summary(it)).collect();
        let sel = state.selected_index.unwrap_or(0);
        let selected_detail = filtered.get(sel).map(|it| item_to_detail(it));

        ProviderSnapshot {
            items,
            selected_detail,
            selected_context: None,
            total_count: self.base_items.len(),
            status_message: if self.query.is_empty() {
                Some(format!("Loaded {} Rust items", self.base_items.len()))
            } else {
                Some(format!("{} items match '{}'", filtered.len(), self.query))
            },
        }
    }
}

impl DataProvider for RustProvider {
    fn initialize(&mut self) -> CoreResult<ProviderSnapshot> {
        Ok(self.build_snapshot(&CoreState::default()))
    }

    fn handle_action(
        &mut self,
        action: &CoreAction,
        state: &CoreState,
    ) -> CoreResult<ProviderOutput> {
        let mut effects = Vec::new();
        match action {
            CoreAction::SetQuery(q) => {
                self.query = q.clone();
            }
            CoreAction::SearchInput(c) => {
                self.query.push(*c);
            }
            CoreAction::SearchBackspace => {
                self.query.pop();
            }
            CoreAction::OpenExternal(url) => {
                effects.push(CoreEffect::OpenExternal(url.clone()));
            }
            CoreAction::Custom(custom) => {
                effects.push(CoreEffect::Custom {
                    id: custom.id.clone(),
                    payload: custom.payload.clone(),
                });
            }
            _ => {}
        }

        Ok(ProviderOutput {
            snapshot: Some(self.build_snapshot(state)),
            effects,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain_rust::analyzer::RustAnalyzer;
    use tui_kit_runtime::{CoreAction, CoreState, DataProvider};

    #[test]
    fn test_rust_provider_filters_with_query() {
        let source = r#"
            pub fn alpha() {}
            pub fn beta() {}
        "#;
        let items = RustAnalyzer::new().analyze_source(source).unwrap();
        let mut provider = RustProvider::new(items);
        let mut state = CoreState::default();

        let init = provider.initialize().unwrap();
        assert_eq!(init.items.len(), 2);

        let out = provider
            .handle_action(&CoreAction::SetQuery("alp".to_string()), &state)
            .unwrap();
        let snap = out.snapshot.unwrap();
        assert_eq!(snap.items.len(), 1);
        assert_eq!(snap.items[0].name, "alpha");

        state.selected_index = Some(0);
        let out = provider
            .handle_action(&CoreAction::OpenSelected, &state)
            .unwrap();
        let snap = out.snapshot.unwrap();
        assert!(snap.selected_detail.is_some());
    }
}
