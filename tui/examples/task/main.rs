mod provider;

use provider::TaskProvider;
use tui_kit_host::runtime_loop::{run_provider_app, RuntimeLoopConfig};
use tui_kit_render::ui::{BrandingText, HeaderText, TabId, TabSpec, UiConfig};
use tui_kit_runtime::PaneFocus;

// Example architecture:
// - `examples/task/provider.rs` provides provider behavior
// - `examples/task/adapter.rs` provides domain-to-UI mappings
// - `tui-kit-runtime` provides interaction contracts and shared state
// - `tui-kit-render` renders state into a ratatui widget tree
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut provider = TaskProvider::sample();
    run_provider_app(
        &mut provider,
        RuntimeLoopConfig {
            initial_tab_id: "task-backlog",
            tab_ids: &[
                "task-backlog",
                "task-in-progress",
                "task-blocked",
                "task-done",
            ],
            initial_focus: PaneFocus::List,
            ui_config: task_ui_config,
        },
    )
}

fn task_ui_config() -> UiConfig {
    UiConfig {
        branding: BrandingText {
            logo_lines: vec![
                "████████╗ █████╗ ███████╗██╗  ██╗".to_string(),
                "╚══██╔══╝██╔══██╗██╔════╝██║ ██╔╝".to_string(),
                "   ██║   ███████║███████╗█████╔╝ ".to_string(),
                "   ██║   ██╔══██║╚════██║██╔═██╗ ".to_string(),
                "   ██║   ██║  ██║███████║██║  ██╗".to_string(),
                "   ╚═╝   ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝".to_string(),
            ],
            attribution: "task demo".to_string(),
        },
        header: HeaderText {
            visible_icon: "✓".to_string(),
            visible_suffix: "tasks".to_string(),
            contexts_icon: "🗂".to_string(),
            contexts_suffix: "boards".to_string(),
            data_label: "cache".to_string(),
        },
        tabs: vec![
            TabSpec {
                id: TabId::new("task-backlog"),
                title: "Backlog".to_string(),
                search_placeholder: "Search backlog tasks...".to_string(),
            },
            TabSpec {
                id: TabId::new("task-in-progress"),
                title: "In Progress".to_string(),
                search_placeholder: "Search active tasks...".to_string(),
            },
            TabSpec {
                id: TabId::new("task-blocked"),
                title: "Blocked".to_string(),
                search_placeholder: "Search blocked tasks...".to_string(),
            },
            TabSpec {
                id: TabId::new("task-done"),
                title: "Done".to_string(),
                search_placeholder: "Search completed tasks...".to_string(),
            },
        ],
        ..UiConfig::default()
    }
}
