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
    tabs_rect_for_area, BrandingText, ChatPanelText, Focus, HeaderText, TuiKitUi, TabId, TabSpec,
    UiConfig,
};
pub use context_view::ContextView;
pub use inspector::InspectorPanel;
pub use model::{
    UiContextNode, UiItemDetail, UiItemKind, UiItemSummary, UiRow, UiSection, UiSourceLocation,
    UiVisibility,
};
pub use search::{
    filter_candidates, CandidateKind, CompletionCandidate, SearchBar, SearchCompletion,
};
