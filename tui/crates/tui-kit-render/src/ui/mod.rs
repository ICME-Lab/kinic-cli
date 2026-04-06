//! User interface module

pub mod animation;
pub mod app;
pub mod components;
pub mod content;
pub mod context_view;
pub mod model;
pub mod search;
pub mod splash;
pub mod theme;

pub use animation::{AnimationState, Easing, SmoothScroll};
pub use app::{
    BrandingText, ChatPanelText, Focus, HeaderText, TabId, TabSpec, TuiKitUi, UiConfig,
    tabs_rect_for_area,
};
pub use content::ContentPanel;
pub use context_view::ContextView;
pub use model::{
    UiContextNode, UiItemContent, UiItemKind, UiItemSummary, UiRow, UiSection, UiSourceLocation,
    UiVisibility,
};
pub use search::{
    CandidateKind, CompletionCandidate, SearchBar, SearchCompletion, filter_candidates,
};
