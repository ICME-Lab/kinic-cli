//! Rust-domain package: analyzer, crate registry, dependency metadata and crates.io docs.

pub mod analyzer;
pub mod crates_io;
pub mod error;

pub use analyzer::*;
pub use error::{OracleError, Result};
