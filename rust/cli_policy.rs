//! Single source of truth for agent-facing CLI semantics that are not fully expressed in clap.
//! Used by `capabilities` JSON and must stay aligned with runtime validation in `lib.rs`.

use crate::cli::Command;

/// Human-readable summary embedded in `kinic-cli capabilities` output.
pub const AUTH_SUMMARY: &str = "Network commands use global --identity or --ii unless noted otherwise. The TUI requires --identity. tools serve is environment-auth only. Some commands expose conditional auth requirements based on flags such as --validate.";

/// Conditional auth: when a flag is present, additional auth sources apply.
#[derive(Clone, Copy, Debug)]
pub struct ConditionalAuthRule {
    pub when_argument_present: &'static str,
    pub required: bool,
    pub sources: &'static [&'static str],
}

/// Per-command-path policy for capabilities JSON (auth, output, supported global flags).
#[derive(Clone, Copy, Debug)]
pub struct CommandPolicy {
    pub auth_sources: &'static [&'static str],
    pub conditional_auth: &'static [ConditionalAuthRule],
    pub output_default: &'static str,
    pub output_supported: &'static [&'static str],
    pub interactive: bool,
    pub global_flags_supported: &'static [&'static str],
}

const GLOBAL_FLAGS_ALL: &[&str] = &["verbose", "ic", "identity", "ii", "identity_path"];
const GLOBAL_FLAGS_VERBOSE_ONLY: &[&str] = &["verbose"];
const GLOBAL_FLAGS_PREFS_ADD_MEMORY: &[&str] =
    &["verbose", "ic", "identity", "ii", "identity_path"];
const GLOBAL_FLAGS_LOGIN: &[&str] = &["verbose", "identity_path"];
const GLOBAL_FLAGS_TUI: &[&str] = &["verbose", "ic", "identity"];

const CONDITIONAL_AUTH_PREFS_ADD_MEMORY_VALIDATE: &[ConditionalAuthRule] = &[ConditionalAuthRule {
    when_argument_present: "validate",
    required: true,
    sources: &["global_identity", "global_ii"],
}];

/// Policy for each subcommand path (`search`, `prefs.add-memory`, `tools.serve`, ...).
pub fn command_policy_for_path(path: &str) -> CommandPolicy {
    if path == "tools" || path.starts_with("tools.") {
        return CommandPolicy {
            auth_sources: &["environment_identity"],
            conditional_auth: &[],
            output_default: "text",
            output_supported: &["text"],
            interactive: false,
            global_flags_supported: GLOBAL_FLAGS_VERBOSE_ONLY,
        };
    }

    if path == "tui" {
        return CommandPolicy {
            auth_sources: &["global_identity"],
            conditional_auth: &[],
            output_default: "interactive",
            output_supported: &["interactive"],
            interactive: true,
            global_flags_supported: GLOBAL_FLAGS_TUI,
        };
    }

    if path == "prefs.add-memory" {
        return CommandPolicy {
            auth_sources: &[],
            conditional_auth: CONDITIONAL_AUTH_PREFS_ADD_MEMORY_VALIDATE,
            output_default: "json",
            output_supported: &["json"],
            interactive: false,
            global_flags_supported: GLOBAL_FLAGS_PREFS_ADD_MEMORY,
        };
    }

    if path == "capabilities"
        || path == "convert-pdf"
        || path == "prefs"
        || path.starts_with("prefs.")
    {
        return CommandPolicy {
            auth_sources: &[],
            conditional_auth: &[],
            output_default: if path == "capabilities"
                || path == "prefs"
                || path.starts_with("prefs.")
            {
                "json"
            } else {
                "text"
            },
            output_supported: if path == "capabilities"
                || path == "prefs"
                || path.starts_with("prefs.")
            {
                &["json"]
            } else {
                &["text"]
            },
            interactive: false,
            global_flags_supported: GLOBAL_FLAGS_VERBOSE_ONLY,
        };
    }

    if path == "login" {
        return CommandPolicy {
            auth_sources: &[],
            conditional_auth: &[],
            output_default: "text",
            output_supported: &["text"],
            interactive: false,
            global_flags_supported: GLOBAL_FLAGS_LOGIN,
        };
    }

    CommandPolicy {
        auth_sources: &["global_identity", "global_ii"],
        conditional_auth: &[],
        output_default: "text",
        output_supported: if matches!(path, "list" | "show" | "search") {
            &["text", "json"]
        } else {
            &["text"]
        },
        interactive: false,
        global_flags_supported: GLOBAL_FLAGS_ALL,
    }
}

/// Commands where `validate_keyring_identity` does not require `--identity` / `--ii`.
/// Must match the intent of `validate_keyring_identity` in `lib.rs`.
pub fn skips_keyring_identity_requirement(command: &Command) -> bool {
    matches!(
        command,
        Command::Login(_)
            | Command::Capabilities(_)
            | Command::Prefs(_)
            | Command::Tools(_)
            | Command::Tui(_)
    )
}
