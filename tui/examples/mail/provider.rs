//! Mail-domain provider implementing tui-kit runtime contracts.

#[path = "adapter.rs"]
mod adapter;

use tui_kit_runtime::{
    CoreAction, CoreEffect, CoreResult, CoreState, DataProvider, ProviderOutput, ProviderSnapshot,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailRecord {
    pub id: String,
    pub from: String,
    pub subject: String,
    pub preview: String,
}

impl MailRecord {
    pub fn new(
        id: impl Into<String>,
        from: impl Into<String>,
        subject: impl Into<String>,
        preview: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            from: from.into(),
            subject: subject.into(),
            preview: preview.into(),
        }
    }
}

pub struct MailProvider {
    all: Vec<MailRecord>,
    query: String,
    tab_id: String,
}

impl MailProvider {
    pub fn new(records: Vec<MailRecord>) -> Self {
        Self {
            all: records,
            query: String::new(),
            tab_id: "mail-inbox".to_string(),
        }
    }

    pub fn sample() -> Self {
        Self::new(vec![
            MailRecord::new(
                "mail-1",
                "team@example.com",
                "Welcome to Oracle UI",
                "Thanks for trying the reusable Oracle UI shell.",
            ),
            MailRecord::new(
                "mail-2",
                "billing@example.com",
                "Invoice ready",
                "Your monthly invoice is attached.",
            ),
            MailRecord::new(
                "mail-3",
                "alerts@example.com",
                "New login detected",
                "A new sign-in was detected from Tokyo.",
            ),
            MailRecord::new(
                "mail-4",
                "ops@example.com",
                "Maintenance window",
                "Service maintenance is scheduled for tonight.",
            ),
            MailRecord::new(
                "mail-5",
                "news@example.com",
                "Weekly digest",
                "Top updates from this week.",
            ),
        ])
    }

    fn filtered(&self) -> Vec<&MailRecord> {
        let by_tab: Vec<&MailRecord> = self
            .all
            .iter()
            .filter(|m| match self.tab_id.as_str() {
                "mail-inbox" => true,
                "mail-invoices" => m.subject.to_lowercase().contains("invoice"),
                "mail-alerts" => m.subject.to_lowercase().contains("login"),
                "mail-news" => {
                    m.subject.to_lowercase().contains("welcome")
                        || m.subject.to_lowercase().contains("digest")
                }
                _ => true,
            })
            .collect();

        if self.query.is_empty() {
            return by_tab;
        }

        let q = self.query.to_lowercase();
        by_tab
            .into_iter()
            .filter(|m| {
                m.subject.to_lowercase().contains(&q)
                    || m.from.to_lowercase().contains(&q)
                    || m.preview.to_lowercase().contains(&q)
            })
            .collect()
    }

    fn build_snapshot(&self, state: &CoreState) -> ProviderSnapshot {
        let filtered = self.filtered();
        let items = filtered
            .iter()
            .map(|m| adapter::to_summary(m))
            .collect::<Vec<_>>();

        let sel = state.selected_index.unwrap_or(0);
        let selected_detail = filtered.get(sel).map(|m| adapter::to_detail(m));

        ProviderSnapshot {
            items,
            selected_detail,
            selected_context: None,
            total_count: self.all.len(),
            status_message: Some(format!(
                "mail(runtime): {} filtered / {} total",
                filtered.len(),
                self.all.len()
            )),
            ..ProviderSnapshot::default()
        }
    }
}

impl DataProvider for MailProvider {
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
            CoreAction::SetQuery(q) => self.query = q.clone(),
            CoreAction::SearchInput(c) => self.query.push(*c),
            CoreAction::SearchBackspace => {
                self.query.pop();
            }
            CoreAction::SetTab(id) => {
                self.tab_id = id.0.clone();
                effects.push(CoreEffect::Notify(format!("Switched mail tab: {}", id.0)));
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
    use tui_kit_runtime::{apply_snapshot, CoreAction, CoreState, DataProvider};

    #[test]
    fn query_filters_results() {
        let mut provider = MailProvider::sample();
        let mut state = CoreState::default();

        let init = provider.initialize().unwrap();
        apply_snapshot(&mut state, init);
        assert!(state.total_count >= 5);

        let out = provider
            .handle_action(&CoreAction::SetQuery("invoice".to_string()), &state)
            .unwrap();
        apply_snapshot(&mut state, out.snapshot.unwrap());

        assert_eq!(state.list_items.len(), 1);
        assert!(state.list_items[0].name.to_lowercase().contains("invoice"));
    }

    #[test]
    fn set_tab_emits_notify_effect() {
        let mut provider = MailProvider::sample();
        let state = CoreState::default();
        let out = provider
            .handle_action(
                &CoreAction::SetTab(tui_kit_runtime::CoreTabId::new("mail-invoices")),
                &state,
            )
            .unwrap();
        assert!(out
            .effects
            .iter()
            .any(|e| matches!(e, CoreEffect::Notify(msg) if msg.contains("mail-invoices"))));
    }
}
