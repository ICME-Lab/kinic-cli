//! Shared KINIC domain rules.
//! Where: reused by CLI and TUI crates.
//! What: centralizes pure amount, principal, tag, and preference-policy helpers.
//! Why: keep domain behavior aligned across interfaces without coupling to I/O or UI state.

pub mod amount;
pub mod prefs_policy;
pub mod principal;
pub mod tag;

pub use amount::{KinicAmountParseError, parse_required_kinic_amount_to_e8s};
pub use amount::{editing_kinic_amount_accepts_char, parse_editing_kinic_display_to_e8s};
pub use amount::{
    format_e8s_to_kinic_string_nat, format_e8s_to_kinic_string_u128, normalize_kinic_display,
};
