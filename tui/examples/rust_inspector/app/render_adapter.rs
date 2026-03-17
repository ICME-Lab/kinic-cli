use crate::app::{App, Tab};
use crate::ui::UiContextNode;

/// Render-facing context props derived from app state.
pub struct RenderContext<'a> {
    pub show_context_panel: bool,
    pub in_context_items_view: bool,
    pub context_tree: &'a [(String, usize)],
    pub filtered_context_indices: &'a [usize],
    pub context_details_loading: bool,
    pub context_details_failed: bool,
    pub ui_context_node: Option<&'a UiContextNode>,
}

/// Build a render-layer context projection from root app state.
pub fn build_render_context(app: &App) -> RenderContext<'_> {
    let selected_context_name = app.selected_dependency_name();
    let context_details_loading =
        app.crate_docs_loading.as_deref() == selected_context_name.as_deref();
    let context_details_failed = selected_context_name
        .as_ref()
        .is_some_and(|n| app.crate_docs_failed.contains(n));

    RenderContext {
        show_context_panel: app.current_tab == Tab::Crates,
        in_context_items_view: app.ui_in_crate_items_view,
        context_tree: &app.dependency_tree,
        filtered_context_indices: &app.filtered_dependency_indices,
        context_details_loading,
        context_details_failed,
        ui_context_node: app.ui_dependency_node.as_ref(),
    }
}
