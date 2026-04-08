//! Shared pure logic reused across CLI and TUI.
//! Where: crate-level internal helpers used by command handlers and TUI bridge/provider code.
//! What: centralizes memory access/user normalization and cross-memory search batching.
//! Why: keep behavior aligned across interfaces without coupling the implementations together.

pub mod access;
pub mod cross_memory_search;
