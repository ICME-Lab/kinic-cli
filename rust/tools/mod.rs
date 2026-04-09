// Where: shared external-tool integration surface for Kinic.
// What: exposes Rust-first service and MCP server for Kinic external-tool integrations.
// Why: keep local MCP integrations on one implementation path without extra transports.

pub mod mcp;
pub mod service;
mod service_helpers;
mod types;
