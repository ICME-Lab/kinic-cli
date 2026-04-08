//! Thin re-export for shared access helpers used by CLI command handlers.
//! Where: imported by CLI commands under `commands/`.
//! What: forwards to crate-level shared logic without keeping a second implementation.
//! Why: allow incremental migration of command imports while centralizing behavior.

pub use crate::shared::access::{MemoryRole, parse_user_principal, validate_role_assignment};
