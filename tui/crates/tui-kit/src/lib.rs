//! Unified `tui-kit` facade crate.
//!
//! This crate re-exports the split crates as public modules so consumers can
//! depend on a single package while keeping module boundaries explicit.

pub mod host {
    pub use tui_kit_host::*;
}

pub mod runtime {
    pub use tui_kit_runtime::*;
}

pub mod model {
    pub use tui_kit_model::*;
}

pub mod render {
    pub use tui_kit_render::*;
}

pub mod prelude {
    pub use crate::host;
    pub use crate::model;
    pub use crate::render;
    pub use crate::runtime;
}
