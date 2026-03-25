#[path = "adapter.rs"]
mod adapter;

use tui_kit_runtime::{
    CoreAction, CoreEffect, CoreResult, CoreState, DataProvider, ProviderOutput, ProviderSnapshot,
};

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

pub struct KinicProvider {
    all: Vec<KinicRecord>,
    query: String,
    tab_id: String,
}

impl KinicProvider {
    pub fn new(records: Vec<KinicRecord>) -> Self {
        Self {
            all: records,
            query: String::new(),
            tab_id: "kinic-memories".to_string(),
        }
    }

    pub fn sample() -> Self {
        Self::new(vec![
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
            KinicRecord::new(
                "kinic-2",
                "Theme Presets",
                "create",
                "Built-in themes including the new pink preset.",
                "Feature memo for create tab.",
            ),
            KinicRecord::new(
                "kinic-3",
                "Navigation Upgrade",
                "market",
                "Tabs focus flow and keyboard-driven tab switching.",
                "Market note: keyboard-first interactions.",
            ),
            KinicRecord::new(
                "kinic-4",
                "Design Notes",
                "settings",
                "Keep runtime domain-agnostic and move app specifics to examples.",
                "Settings note: keep defaults simple and explicit.",
            ),
        ])
    }

    fn filtered(&self) -> Vec<&KinicRecord> {
        let by_tab: Vec<&KinicRecord> = self
            .all
            .iter()
            .filter(|r| match self.tab_id.as_str() {
                "kinic-memories" => r.group == "memories",
                "kinic-create" => r.group == "create",
                "kinic-market" => r.group == "market",
                "kinic-settings" => r.group == "settings",
                _ => true,
            })
            .collect();

        if self.query.is_empty() {
            return by_tab;
        }

        let q = self.query.to_lowercase();
        by_tab
            .into_iter()
            .filter(|r| {
                r.title.to_lowercase().contains(&q)
                    || r.summary.to_lowercase().contains(&q)
                    || r.group.to_lowercase().contains(&q)
            })
            .collect()
    }

    fn build_snapshot(&self, state: &CoreState) -> ProviderSnapshot {
        let filtered = self.filtered();
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
            total_count: self.all.len(),
            status_message: Some(format!(
                "kinic(runtime): {} filtered / {} total",
                filtered.len(),
                self.all.len()
            )),
            settings: Default::default(),
        }
    }
}

impl DataProvider for KinicProvider {
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
                effects.push(CoreEffect::Notify(format!("Switched kinic tab: {}", id.0)));
            }
            _ => {}
        }

        Ok(ProviderOutput {
            snapshot: Some(self.build_snapshot(state)),
            effects,
        })
    }
}
