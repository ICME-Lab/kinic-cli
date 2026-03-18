use std::future::Future;

use super::adapter;
use super::app::{self, MemorySummary, SearchResultItem};
use tokio::runtime::{Handle, Runtime};
use tui_kit_runtime::{
    CoreAction, CoreEffect, CoreResult, CoreState, DataProvider, ProviderOutput, ProviderSnapshot,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiConfig {
    pub identity: Option<String>,
    pub use_mainnet: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KinicRecord {
    pub id: String,
    pub title: String,
    pub group: String,
    pub summary: String,
    pub content_md: String,
}

impl KinicRecord {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        group: impl Into<String>,
        summary: impl Into<String>,
        content_md: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            group: group.into(),
            summary: summary.into(),
            content_md: content_md.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemoriesMode {
    Browser,
    Results,
}

enum BlockingRuntime {
    Owned(Runtime),
    Handle(Handle),
}

impl BlockingRuntime {
    fn new() -> Self {
        if let Ok(handle) = Handle::try_current() {
            Self::Handle(handle)
        } else {
            Self::Owned(Runtime::new().expect("failed to create tokio runtime for tui"))
        }
    }

    fn block_on<F>(&self, future: F) -> F::Output
    where
        F: Future,
    {
        match self {
            Self::Owned(runtime) => runtime.block_on(future),
            Self::Handle(handle) => tokio::task::block_in_place(|| handle.block_on(future)),
        }
    }
}

pub struct KinicProvider {
    all: Vec<KinicRecord>,
    query: String,
    tab_id: String,
    runtime: BlockingRuntime,
    config: TuiConfig,
    active_memory_id: Option<String>,
    memory_records: Vec<KinicRecord>,
    result_records: Vec<KinicRecord>,
    memories_mode: MemoriesMode,
}

impl KinicProvider {
    pub fn new(config: TuiConfig) -> Self {
        Self {
            all: sample_records(),
            query: String::new(),
            tab_id: "kinic-memories".to_string(),
            runtime: BlockingRuntime::new(),
            config,
            active_memory_id: None,
            memory_records: Vec::new(),
            result_records: Vec::new(),
            memories_mode: MemoriesMode::Browser,
        }
    }

    fn initialize_live_memories(&mut self) {
        let Some(identity) = self.config.identity.clone() else {
            return;
        };

        match self.runtime.block_on(app::list_memories(self.config.use_mainnet, identity)) {
            Ok(memories) => {
                self.memory_records = memories.into_iter().map(record_from_memory_summary).collect();
                self.all = self.memory_records.clone();
            }
            Err(error) => {
                self.all = vec![KinicRecord::new(
                    "kinic-live-error",
                    "Unable to load memories",
                    "memories",
                    "Check your identity or network configuration.",
                    format!("## Live Load Error\n\n{error}"),
                )];
            }
        }
    }

    fn reload_live_memories(&mut self, prioritize_id: Option<&str>) -> Result<(), String> {
        let Some(identity) = self.config.identity.clone() else {
            return Ok(());
        };
        let memories = self
            .runtime
            .block_on(app::list_memories(self.config.use_mainnet, identity))
            .map_err(|error| error.to_string())?;
        self.memory_records = memories.into_iter().map(record_from_memory_summary).collect();
        if let Some(id) = prioritize_id {
            if let Some(index) = self.memory_records.iter().position(|r| r.id == id) {
                let record = self.memory_records.remove(index);
                self.memory_records.insert(0, record);
            }
        }
        self.all = self.memory_records.clone();
        self.memories_mode = MemoriesMode::Browser;
        self.result_records.clear();
        Ok(())
    }

    fn is_live(&self) -> bool {
        self.config.identity.is_some()
    }

    fn current_records(&self) -> Vec<&KinicRecord> {
        let base = if self.is_live() {
            match self.memories_mode {
                MemoriesMode::Browser => &self.memory_records,
                MemoriesMode::Results => &self.result_records,
            }
        } else {
            &self.all
        };

        if self.memories_mode == MemoriesMode::Results || self.query.is_empty() {
            return base.iter().collect();
        }

        let q = self.query.to_lowercase();
        base.iter()
            .filter(|r| {
                r.title.to_lowercase().contains(&q)
                    || r.summary.to_lowercase().contains(&q)
                    || r.group.to_lowercase().contains(&q)
                    || r.id.to_lowercase().contains(&q)
            })
            .collect()
    }

    fn build_snapshot(&self, state: &CoreState) -> ProviderSnapshot {
        let filtered = self.current_records();
        let items = filtered
            .iter()
            .map(|r| adapter::to_summary(r))
            .collect::<Vec<_>>();
        let sel = state.selected_index.unwrap_or(0);
        let selected_detail = filtered.get(sel).map(|r| adapter::to_detail(r));

        ProviderSnapshot {
            items,
            selected_detail,
            selected_context: None,
            total_count: filtered.len(),
            status_message: Some(self.status_message(filtered.len())),
        }
    }

    fn status_message(&self, visible_count: usize) -> String {
        if !self.is_live() {
            return format!("kinic(mock): {visible_count} filtered / {} total", self.all.len());
        }

        match self.memories_mode {
            MemoriesMode::Browser => match self.active_memory_id.as_deref() {
                Some(memory_id) => format!(
                    "kinic(live): {visible_count} memories | target {memory_id} | Enter in search runs remote search"
                ),
                None => format!(
                    "kinic(live): {visible_count} memories | select a memory, then press Enter in search"
                ),
            },
            MemoriesMode::Results => match self.active_memory_id.as_deref() {
                Some(memory_id) => format!(
                    "kinic(live): {visible_count} search results in {memory_id} | Esc clears search and returns"
                ),
                None => format!("kinic(live): {visible_count} search results"),
            },
        }
    }

    fn select_active_memory(&mut self, state: &CoreState) {
        if self.tab_id != "kinic-memories" || !self.is_live() {
            return;
        }
        if self.memories_mode != MemoriesMode::Browser {
            return;
        }
        let Some(index) = state.selected_index else {
            return;
        };
        let Some(record) = self.memory_records.get(index) else {
            return;
        };
        self.active_memory_id = Some(record.id.clone());
    }

    fn run_live_search(&mut self) -> Option<CoreEffect> {
        let Some(identity) = self.config.identity.clone() else {
            return None;
        };
        let Some(memory_id) = self.active_memory_id.clone() else {
            return Some(CoreEffect::Notify(
                "Select a memory in the list before running search.".to_string(),
            ));
        };
        let query = self.query.trim().to_string();
        if query.is_empty() {
            self.memories_mode = MemoriesMode::Browser;
            self.result_records.clear();
            return Some(CoreEffect::Notify(
                "Cleared search query and returned to memories.".to_string(),
            ));
        }

        match self.runtime.block_on(app::search_memory(
            self.config.use_mainnet,
            identity,
            memory_id.clone(),
            query.clone(),
        )) {
            Ok(results) => {
                self.result_records = results
                    .into_iter()
                    .enumerate()
                    .map(|(index, item)| record_from_search_result(index, &memory_id, item))
                    .collect();
                self.memories_mode = MemoriesMode::Results;
                Some(CoreEffect::Notify(format!(
                    "Loaded {} search results for {}",
                    self.result_records.len(),
                    memory_id
                )))
            }
            Err(error) => Some(CoreEffect::Notify(format!("Search failed: {error}"))),
        }
    }

    fn reset_memories_browser(&mut self) {
        if self.is_live() {
            self.memories_mode = MemoriesMode::Browser;
            self.result_records.clear();
        }
    }
}

impl DataProvider for KinicProvider {
    fn initialize(&mut self) -> CoreResult<ProviderSnapshot> {
        self.initialize_live_memories();
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
                if self.tab_id == "kinic-memories" && q.is_empty() {
                    self.reset_memories_browser();
                }
            }
            CoreAction::SearchInput(c) => self.query.push(*c),
            CoreAction::SearchBackspace => {
                self.query.pop();
                if self.query.is_empty() {
                    self.reset_memories_browser();
                }
            }
            CoreAction::SearchSubmit => {
                if self.is_live() {
                    if let Some(effect) = self.run_live_search() {
                        effects.push(effect);
                    }
                }
            }
            CoreAction::OpenSelected => {
                self.select_active_memory(state);
            }
            CoreAction::SetTab(id) => {
                self.tab_id = id.0.clone();
                effects.push(CoreEffect::Notify(format!("Switched kinic tab: {}", id.0)));
            }
            CoreAction::ChatSubmit => {
                effects.push(CoreEffect::Notify(
                    "Chat is still mock-only; search is live first.".to_string(),
                ));
            }
            CoreAction::CreateSubmit => {
                let name = state.create_name.trim().to_string();
                let description = state.create_description.trim().to_string();
                if name.is_empty() || description.is_empty() {
                    effects.push(CoreEffect::Custom {
                        id: "create_modal_error".to_string(),
                        payload: Some("Name and description are required.".to_string()),
                    });
                } else if let Some(identity) = self.config.identity.clone() {
                    match self.runtime.block_on(app::create_memory(
                        self.config.use_mainnet,
                        identity,
                        name.clone(),
                        description,
                    )) {
                        Ok(created_id) => match self.reload_live_memories(Some(&created_id)) {
                            Ok(()) => {
                                self.active_memory_id = Some(created_id.clone());
                                effects.push(CoreEffect::Custom {
                                    id: "select_first".to_string(),
                                    payload: None,
                                });
                                effects.push(CoreEffect::Custom {
                                    id: "create_modal_close".to_string(),
                                    payload: None,
                                });
                                effects.push(CoreEffect::Notify(format!(
                                    "Created memory {created_id}"
                                )));
                            }
                            Err(error) => {
                                effects.push(CoreEffect::Custom {
                                    id: "create_modal_error".to_string(),
                                    payload: Some(error),
                                });
                            }
                        },
                        Err(error) => {
                            effects.push(CoreEffect::Custom {
                                id: "create_modal_error".to_string(),
                                payload: Some(error.to_string()),
                            });
                        }
                    }
                } else {
                    let new_id = format!("mock-memory-{}", self.all.len() + 1);
                    let record = KinicRecord::new(
                        new_id.clone(),
                        name.clone(),
                        "memories",
                        "Status: mock".to_string(),
                        format!(
                            "## Memory\n\n- Id: `{new_id}`\n- Status: `mock`\n\n### Detail\n{}\n",
                            state.create_description.trim()
                        ),
                    );
                    self.all.insert(0, record);
                    self.active_memory_id = Some(new_id.clone());
                    effects.push(CoreEffect::Custom {
                        id: "select_first".to_string(),
                        payload: None,
                    });
                    effects.push(CoreEffect::Custom {
                        id: "create_modal_close".to_string(),
                        payload: None,
                    });
                    effects.push(CoreEffect::Notify(format!("Created mock memory {name}")));
                }
            }
            _ => {}
        }

        Ok(ProviderOutput {
            snapshot: Some(self.build_snapshot(state)),
            effects,
        })
    }
}

fn record_from_memory_summary(memory: MemorySummary) -> KinicRecord {
    KinicRecord::new(
        memory.id.clone(),
        memory.id.clone(),
        "memories",
        format!("Status: {}", memory.status),
        format!(
            "## Memory\n\n- Id: `{}`\n- Status: `{}`\n\n### Detail\n{}\n\n### Search\nSelect this item, then type a query and press Enter in the search box.",
            memory.id, memory.status, memory.detail
        ),
    )
}

fn record_from_search_result(index: usize, memory_id: &str, item: SearchResultItem) -> KinicRecord {
    let title = format!("Result #{:02}", index + 1);
    let score = format!("{:.4}", item.score);
    KinicRecord::new(
        format!("{memory_id}-result-{}", index + 1),
        title,
        "memories",
        format!("Score: {score}"),
        format!(
            "## Search Hit\n\n- Memory: `{memory_id}`\n- Score: `{score}`\n\n### Payload\n{}\n",
            item.payload
        ),
    )
}

fn sample_records() -> Vec<KinicRecord> {
    vec![
        KinicRecord::new(
            "kinic-1",
            "Unified TUI Kit",
            "memories",
            "Single facade crate with modular host/runtime/render layers.",
            r#"## Daily Note
- Split crate structure into `host/runtime/render/model`
- Added unified facade crate: `tui-kit`

### Why it mattered
Keeping runtime domain-agnostic made every demo easier to compose.

```rust
let ui = TuiKitUi::new(&theme);
```
"#,
        ),
        KinicRecord::new(
            "kinic-5",
            "Keyboard Workflow Snapshot",
            "memories",
            "Focused on tab-first navigation and predictable pane order.",
            r#"## Navigation Log
1. Search for an entry
2. Move to list with `Tab`
3. Open content with `Enter`

### Notes
- Keep tabs at the end of the focus cycle
- Prioritize consistency over shortcuts
"#,
        ),
        KinicRecord::new(
            "kinic-6",
            "UI Polish Memo",
            "memories",
            "Captured tweaks around scrollbars, placeholders, and cursor behavior.",
            r#"## UI Polish
- Placeholder uses muted/italic style
- Cursor only appears in active input fields
- Scrollbar uses a custom track + thumb

### TODO
- [ ] Mouse wheel support per pane
- [ ] Unified toast notifications
"#,
        ),
        KinicRecord::new(
            "kinic-7",
            "Release Draft 0.1",
            "memories",
            "First release draft notes for the reusable tui-kit package.",
            r#"## Release Draft 0.1
- Stabilize facade crate exports
- Freeze keyboard navigation defaults
- Add one polished domain sample

### Changelog Snippet
- `feat`: tabs focus cycle
- `fix`: list scrollbar behavior
"#,
        ),
        KinicRecord::new(
            "kinic-8",
            "Design Review: Left Pane",
            "memories",
            "Discussed list density and readability under compact terminals.",
            r#"## Left Pane Review
- Keep icon + kind prefix
- Avoid truncating item title too early
- Prefer subtle divider over heavy borders

```text
Goal: scanability first, decoration second.
```
"#,
        ),
        KinicRecord::new(
            "kinic-9",
            "Design Review: Right Pane",
            "memories",
            "Evaluated section hierarchy and markdown-ish readability.",
            r#"## Right Pane Review
1. Strong title
2. Definition block
3. Sections with clear heading

### Decision
Use `◇ Content` naming consistently.
"#,
        ),
        KinicRecord::new(
            "kinic-10",
            "Keyboard Mapping Matrix",
            "memories",
            "Captured focus navigation matrix for Search/List/Content/Tabs/Chat.",
            r#"## Keyboard Matrix
- `Tab`: next focus
- `Shift+Tab`: previous focus
- Tabs focus: `j/k` or `↑/↓` to switch tab

### Regression Check
- Ensure `Enter` from Tabs reaches Content.
"#,
        ),
        KinicRecord::new(
            "kinic-11",
            "Interaction Bug Notes",
            "memories",
            "Log of edge cases found during runtime-first migration.",
            r#"## Bug Notes
- Chat focus consumed key without sync
- List scroll anchor drifted on reverse motion
- Status row index mismatch after layout update

### Action
Patch quickly, then add snapshot tests.
"#,
        ),
        KinicRecord::new(
            "kinic-12",
            "Theme Study",
            "memories",
            "Compared contrast ratios across dark presets and pink variant.",
            r#"## Theme Study
- Nord: calm, high legibility
- Dracula: vivid syntax emphasis
- Pink: branded accent direction

### Follow-up
Add high-contrast accessibility preset.
"#,
        ),
        KinicRecord::new(
            "kinic-13",
            "Provider Contract Memo",
            "memories",
            "Summarized DataProvider boundaries for domain teams.",
            r#"## Provider Contract
- Provider owns data shaping
- Runtime owns local interaction state
- Render stays domain-agnostic

```rust
fn handle_action(&mut self, action: &CoreAction, state: &CoreState)
```
"#,
        ),
        KinicRecord::new(
            "kinic-14",
            "Host Boundary Memo",
            "memories",
            "Clarified responsibilities of host loop and side-effect execution.",
            r#"## Host Boundary
- Poll input
- Resolve global commands
- Execute effects (`OpenExternal`, notifications)

### Keep out of runtime
Anything terminal/platform-specific.
"#,
        ),
        KinicRecord::new(
            "kinic-15",
            "Render Boundary Memo",
            "memories",
            "Defined what belongs in render and what does not.",
            r#"## Render Boundary
- Layout and visuals
- Cursor coordinates
- No business/domain side effects

### Practical Rule
If it needs OS I/O, it is not render.
"#,
        ),
        KinicRecord::new(
            "kinic-16",
            "Migration Checklist",
            "memories",
            "Checklist for moving legacy app flows into shared contracts.",
            r#"## Migration Checklist
- [x] Split model/runtime/host/render
- [x] Add facade crate
- [x] Move common runtime loop
- [ ] Replace remaining domain loops with hooks
"#,
        ),
        KinicRecord::new(
            "kinic-17",
            "UX Copy Candidates",
            "memories",
            "Alternatives for generic labels in reusable UI kit.",
            r#"## Copy Candidates
- Tabs -> Views / Sections
- Inspector -> Content
- Context -> Group

### Principle
Prefer domain-neutral defaults with app-level overrides.
"#,
        ),
        KinicRecord::new(
            "kinic-18",
            "Sample Data Expansion Plan",
            "memories",
            "Prepared larger datasets for manual scrolling and search QA.",
            r#"## Expansion Plan
1. Add 20+ memory records
2. Ensure varied title lengths
3. Include markdown-like body text

### Why
Better confidence in viewport + scrollbar behavior.
"#,
        ),
        KinicRecord::new(
            "kinic-19",
            "Command Palette Idea",
            "memories",
            "Rough concept for command palette integration.",
            r#"## Command Palette Idea
- Trigger with `:`
- Fuzzy command search
- Action dispatch into runtime

### Future
Could become a separate optional module.
"#,
        ),
        KinicRecord::new(
            "kinic-20",
            "Copilot-to-Chat Rename",
            "memories",
            "Documented terminology cleanup for domain neutrality.",
            r#"## Terminology Cleanup
- Remove product-specific terms from shared crates
- Keep neutral naming in runtime/render/host

### Result
UI kit can be reused across mail/task/other domains.
"#,
        ),
        KinicRecord::new(
            "kinic-21",
            "Mouse Support TODO",
            "memories",
            "Pending mouse wheel and click interactions for each pane.",
            r#"## Mouse Support TODO
- Wheel scroll in List/Content/Chat
- Click to focus pane
- Click tabs to switch

### Constraint
Maintain keyboard-first behavior as baseline.
"#,
        ),
        KinicRecord::new(
            "kinic-22",
            "Content Mock Library",
            "memories",
            "Centralized mock snippets for demos and screenshots.",
            r#"## Content Mock Library
- Keep short and long variants
- Include bullets, headings, and pseudo code
- Avoid domain-sensitive terms by default

```md
## Heading
- item
```
"#,
        ),
    ]
}
