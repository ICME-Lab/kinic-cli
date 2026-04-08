//! Slash commands in the Memories chat input (`/new`, `/all`).

use crate::CoreAction;

/// Commands recognized after `/` for autocomplete and status hints.
pub const CHAT_SLASH_COMMANDS: [&str; 2] = ["/new", "/all"];

/// Status bar hint: space-separated list of commands.
pub const CHAT_SLASH_COMMAND_HINT: &str = "/new /all";

/// User-facing message when input starts with `/` but does not match a known command.
pub const UNKNOWN_SLASH_COMMAND_MESSAGE: &str = "Unknown chat command. Try /new or /all.";

/// Commands whose prefix matches `input` (trimmed), for autocomplete UI.
pub fn matching_slash_commands(input: &str) -> Vec<&'static str> {
    let trimmed_end = input.trim_end();
    let command_input = trimmed_end.trim_start();
    if !command_input.starts_with('/') {
        return Vec::new();
    }
    if command_input == "/" {
        return CHAT_SLASH_COMMANDS.to_vec();
    }
    CHAT_SLASH_COMMANDS
        .iter()
        .copied()
        .filter(|command| command.starts_with(command_input))
        .collect()
}

pub fn chat_slash_command_action(input: &str) -> Option<CoreAction> {
    match input.trim() {
        "/new" => Some(CoreAction::ChatNewThread),
        "/all" => Some(CoreAction::ChatScopeAll),
        _ => None,
    }
}

pub fn selected_slash_command_action(input: &str, selected: usize) -> Option<CoreAction> {
    let matches = matching_slash_commands(input);
    matches
        .get(selected.min(matches.len().saturating_sub(1)))
        .copied()
        .and_then(chat_slash_command_action)
}

/// Collapse multiline chat input to one line for in-progress display.
/// This preserves user-typed spacing and only replaces line breaks with spaces.
pub fn flatten_chat_input_for_display(value: &str) -> String {
    value.split('\n').collect::<Vec<_>>().join(" ")
}

/// Collapse multiline chat input to one line (trimmed lines joined with spaces).
pub fn normalize_chat_input_lines(value: &str) -> String {
    value.lines().map(str::trim).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_slash_commands_filters_by_prefix() {
        assert_eq!(matching_slash_commands("/"), vec!["/new", "/all"]);
        assert_eq!(matching_slash_commands("/all "), vec!["/all"]);
        assert!(matching_slash_commands("/s").is_empty());
        assert!(matching_slash_commands("hello").is_empty());
    }

    #[test]
    fn flatten_chat_input_for_display_preserves_spacing() {
        assert_eq!(flatten_chat_input_for_display("a \n  b"), "a    b");
    }

    #[test]
    fn normalize_chat_input_lines_joins_trimmed_lines() {
        assert_eq!(normalize_chat_input_lines("a\n  b  \n c"), "a b c");
    }
}
