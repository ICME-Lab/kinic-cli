//! Task-domain provider implementing tui-kit runtime contracts.

#[path = "adapter.rs"]
mod adapter;

use tui_kit_runtime::{
    CoreAction, CoreResult, CoreState, DataProvider, ProviderOutput, ProviderSnapshot,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRecord {
    pub id: String,
    pub title: String,
    pub project: String,
    pub status: String,
    pub note: String,
}

impl TaskRecord {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        project: impl Into<String>,
        status: impl Into<String>,
        note: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            project: project.into(),
            status: status.into(),
            note: note.into(),
        }
    }
}

pub struct TaskProvider {
    all: Vec<TaskRecord>,
    query: String,
}

impl TaskProvider {
    pub fn new(tasks: Vec<TaskRecord>) -> Self {
        Self {
            all: tasks,
            query: String::new(),
        }
    }

    pub fn sample() -> Self {
        Self::new(vec![
            TaskRecord::new(
                "task-1",
                "Ship UI runtime reducer",
                "tui-kit-runtime",
                "doing",
                "Move interaction reducer from app into runtime crate.",
            ),
            TaskRecord::new(
                "task-2",
                "Create task adapter",
                "tui-kit-adapter-task",
                "todo",
                "Add generic provider for non-Rust domain sample.",
            ),
            TaskRecord::new(
                "task-3",
                "Record migration status",
                "oracle",
                "done",
                "Document split progress in project notes.",
            ),
        ])
    }

    fn filtered(&self) -> Vec<&TaskRecord> {
        if self.query.is_empty() {
            return self.all.iter().collect();
        }
        let q = self.query.to_lowercase();
        self.all
            .iter()
            .filter(|t| {
                t.title.to_lowercase().contains(&q)
                    || t.project.to_lowercase().contains(&q)
                    || t.status.to_lowercase().contains(&q)
                    || t.note.to_lowercase().contains(&q)
            })
            .collect()
    }

    fn build_snapshot(&self, state: &CoreState) -> ProviderSnapshot {
        let filtered = self.filtered();
        let items = filtered
            .iter()
            .map(|t| adapter::to_summary(t))
            .collect::<Vec<_>>();

        let sel = state.selected_index.unwrap_or(0);
        let selected_detail = filtered.get(sel).map(|t| adapter::to_detail(t));

        ProviderSnapshot {
            items,
            selected_detail,
            selected_context: None,
            total_count: self.all.len(),
            status_message: Some(format!(
                "task(runtime): {} filtered / {} total",
                filtered.len(),
                self.all.len()
            )),
            settings: Default::default(),
        }
    }
}

impl DataProvider for TaskProvider {
    fn initialize(&mut self) -> CoreResult<ProviderSnapshot> {
        Ok(self.build_snapshot(&CoreState::default()))
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
            snapshot: Some(self.build_snapshot(state)),
            effects: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tui_kit_runtime::{apply_snapshot, CoreAction, CoreState, DataProvider};

    #[test]
    fn query_filters_tasks() {
        let mut provider = TaskProvider::sample();
        let mut state = CoreState::default();

        let init = provider.initialize().unwrap();
        apply_snapshot(&mut state, init);
        assert_eq!(state.total_count, 3);

        let out = provider
            .handle_action(&CoreAction::SetQuery("adapter".to_string()), &state)
            .unwrap();
        apply_snapshot(&mut state, out.snapshot.unwrap());

        assert_eq!(state.list_items.len(), 1);
        assert!(state.list_items[0].name.to_lowercase().contains("adapter"));
    }
}
