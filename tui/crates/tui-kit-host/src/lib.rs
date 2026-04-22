// Where: tui/crates/tui-kit-host/src/lib.rs
// What: re-exports the canonical tui-kit-host implementation used by the root package.
// Why: keep a single source tree while preserving the legacy crate entrypoint.

#[path = "../../../../rust/internal/tui_kit_host/lib.rs"]
mod canonical;

pub use canonical::*;
