//! Application module

mod intent_domain;
mod interaction;
mod render_adapter;
mod state;
mod tab;

pub use interaction::{
    RuntimeApplyResult, UiEffect, UiIntent, apply_intent, apply_runtime_set_tab, intents_for_key,
    try_apply_runtime_action,
};
pub use render_adapter::{RenderContext, build_render_context};
pub use state::App;
pub use tab::Tab;
