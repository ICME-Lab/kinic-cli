//! Application module

mod intent_domain;
mod interaction;
mod render_adapter;
mod state;
mod tab;

pub use interaction::{
    apply_intent, apply_runtime_set_tab, intents_for_key, try_apply_runtime_action,
    RuntimeApplyResult, UiEffect, UiIntent,
};
pub use render_adapter::{build_render_context, RenderContext};
pub use state::App;
pub use tab::Tab;
