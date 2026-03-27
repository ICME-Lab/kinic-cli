use tui_kit_model::{UiItemContent, UiItemKind, UiItemSummary, UiSection, UiVisibility};
use tui_kit_runtime::{
    CoreAction, CoreResult, CoreState, DataProvider, ProviderOutput, ProviderSnapshot,
    apply_snapshot,
};

#[derive(Clone)]
struct Record {
    id: String,
    title: String,
    owner: String,
    summary: String,
}

struct DemoProvider {
    all: Vec<Record>,
    query: String,
}

impl DemoProvider {
    fn new() -> Self {
        Self {
            all: vec![
                Record {
                    id: "r1".to_string(),
                    title: "Welcome".to_string(),
                    owner: "team@example.com".to_string(),
                    summary: "This is a domain-agnostic UI runtime demo.".to_string(),
                },
                Record {
                    id: "r2".to_string(),
                    title: "Invoice ready".to_string(),
                    owner: "billing@example.com".to_string(),
                    summary: "Your monthly invoice is attached.".to_string(),
                },
            ],
            query: String::new(),
        }
    }

    fn filtered(&self) -> Vec<&Record> {
        if self.query.is_empty() {
            return self.all.iter().collect();
        }
        let q = self.query.to_lowercase();
        self.all
            .iter()
            .filter(|r| {
                r.title.to_lowercase().contains(&q)
                    || r.owner.to_lowercase().contains(&q)
                    || r.summary.to_lowercase().contains(&q)
            })
            .collect()
    }

    fn snapshot(&self, state: &CoreState) -> ProviderSnapshot {
        let filtered = self.filtered();
        let items = filtered
            .iter()
            .map(|r| UiItemSummary {
                id: r.id.clone(),
                name: r.title.clone(),
                leading_marker: None,
                kind: UiItemKind::Custom("record".to_string()),
                visibility: UiVisibility::Private,
                qualified_name: Some(r.owner.clone()),
                subtitle: Some(r.summary.clone()),
                tags: vec!["demo".to_string()],
            })
            .collect::<Vec<_>>();

        let sel = state.selected_index.unwrap_or(0);
        let selected_content = filtered.get(sel).map(|r| UiItemContent {
            id: r.id.clone(),
            title: r.title.clone(),
            kind: UiItemKind::Custom("record".to_string()),
            definition: format!("Owner: {}", r.owner),
            location: None,
            docs: Some(r.summary.clone()),
            badges: vec!["demo".to_string()],
            sections: vec![UiSection {
                heading: "Content".to_string(),
                rows: vec![],
                body_lines: vec![r.summary.clone()],
            }],
        });

        ProviderSnapshot {
            items,
            selected_content,
            selected_context: None,
            total_count: self.all.len(),
            status_message: Some(format!("{} records", filtered.len())),
            ..ProviderSnapshot::default()
        }
    }
}

impl DataProvider for DemoProvider {
    fn initialize(&mut self) -> CoreResult<ProviderSnapshot> {
        Ok(self.snapshot(&CoreState::default()))
    }

    fn handle_action(
        &mut self,
        action: &CoreAction,
        state: &CoreState,
    ) -> CoreResult<ProviderOutput> {
        match action {
            CoreAction::SetQuery(q) => self.query = q.clone(),
            CoreAction::SearchInput(c) => self.query.push(*c),
            CoreAction::SearchBackspace => {
                self.query.pop();
            }
            _ => {}
        }

        Ok(ProviderOutput {
            snapshot: Some(self.snapshot(state)),
            effects: vec![],
        })
    }
}

fn main() -> CoreResult<()> {
    let mut provider = DemoProvider::new();
    let mut state = CoreState::default();

    let init = provider.initialize()?;
    apply_snapshot(&mut state, init);

    let out = provider.handle_action(&CoreAction::SetQuery("invoice".to_string()), &state)?;
    if let Some(snapshot) = out.snapshot {
        apply_snapshot(&mut state, snapshot);
    }

    println!(
        "{}",
        state.status_message.unwrap_or_else(|| "ok".to_string())
    );
    Ok(())
}
