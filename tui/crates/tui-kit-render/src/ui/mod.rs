//! User interface module

pub mod animation;
pub mod app;
pub mod components;
pub mod context_view;
pub mod inspector;
pub mod model;
pub mod search;
pub mod splash;
pub mod theme;

pub use animation::{AnimationState, Easing, SmoothScroll};
pub use app::{
    BrandingText, ChatPanelText, Focus, HeaderText, TabId, TabSpec, TuiKitUi, UiConfig,
    tabs_rect_for_area,
};
pub use context_view::ContextView;
pub use inspector::InspectorPanel;
pub use model::{
    UiContextNode, UiItemDetail, UiItemKind, UiItemSummary, UiRow, UiSection, UiSourceLocation,
    UiVisibility,
};
pub use search::{
    CandidateKind, CompletionCandidate, SearchBar, SearchCompletion, filter_candidates,
};
