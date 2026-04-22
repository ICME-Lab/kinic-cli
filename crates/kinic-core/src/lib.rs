// Where: crates/kinic-core/src/lib.rs
// What: re-exports the canonical kinic-core implementation used by the root package.
// Why: keep a single source tree while preserving the legacy crate entrypoint.

#[path = "../../../rust/internal/kinic_core/lib.rs"]
mod canonical;

pub use canonical::*;
