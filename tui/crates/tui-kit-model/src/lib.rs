// Where: tui/crates/tui-kit-model/src/lib.rs
// What: re-exports the canonical tui-kit-model implementation used by the root package.
// Why: keep a single source tree while preserving the legacy crate entrypoint.

#[path = "../../../../rust/internal/tui_kit_model/lib.rs"]
mod canonical;

pub use canonical::*;
