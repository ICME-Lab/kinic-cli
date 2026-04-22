//! Shared KINIC amount rules.
//! Where: reused by CLI and TUI crates.
//! What: centralizes KINIC <-> e8s parsing, formatting, normalization, and input filtering.
//! Why: keep amount behavior aligned across interfaces while leaving user-facing error text local.

mod display;
mod editing;
mod parsing;

pub use display::{
    format_e8s_to_kinic_string_nat, format_e8s_to_kinic_string_u128, normalize_kinic_display,
};
pub use editing::{editing_kinic_amount_accepts_char, parse_editing_kinic_display_to_e8s};
pub use parsing::{KinicAmountParseError, parse_required_kinic_amount_to_e8s};

pub const KINIC_FRACTION_DIGITS: usize = 8;
pub const E8S_PER_KINIC: u128 = 100_000_000;
