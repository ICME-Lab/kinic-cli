//! Risk evaluation and interactive confirmation helpers for CLI access changes.
//! Where: used by `commands/config.rs` before `config users change/remove` mutates canister state.
//! What: computes high-risk access-control outcomes and requires explicit terminal confirmation.
//! Why: the current role-change implementation is non-atomic, so risky changes need stronger safeguards.

use std::io::{self, BufRead, IsTerminal, Write};

use anyhow::{Result, bail};
use ic_agent::export::Principal;

use crate::shared::access::MemoryRole;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMutation {
    Change { next_role: MemoryRole },
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskReason {
    ZeroAdmins,
    SelfAccessLoss,
}

impl RiskReason {
    fn message(self) -> &'static str {
        match self {
            Self::ZeroAdmins => "This change may leave the memory with zero admins.",
            Self::SelfAccessLoss => "This change may remove your own admin access.",
        }
    }
}

pub fn evaluate_access_change_risks(
    users: &[(String, u8)],
    current_principal: &Principal,
    target_principal: &Principal,
    mutation: AccessMutation,
    launcher_id: &str,
) -> Vec<RiskReason> {
    let current_role = users.iter().find_map(|(principal_id, role_code)| {
        if principal_id == launcher_id {
            return None;
        }
        if Principal::from_text(principal_id).ok().as_ref() == Some(target_principal) {
            MemoryRole::from_code(*role_code)
        } else {
            None
        }
    });
    let next_role = match mutation {
        AccessMutation::Change { next_role } => Some(next_role),
        AccessMutation::Remove => None,
    };

    let mut admin_count_after = 0usize;
    for (principal_id, role_code) in users {
        if principal_id == launcher_id {
            continue;
        }
        let principal = match Principal::from_text(principal_id) {
            Ok(principal) => principal,
            Err(_) => continue,
        };
        let effective_role = if principal == *target_principal {
            next_role
        } else {
            MemoryRole::from_code(*role_code)
        };
        if effective_role == Some(MemoryRole::Admin) {
            admin_count_after += 1;
        }
    }

    let mut reasons = Vec::new();
    if admin_count_after == 0 {
        reasons.push(RiskReason::ZeroAdmins);
    }
    if *target_principal == *current_principal
        && current_role == Some(MemoryRole::Admin)
        && next_role != Some(MemoryRole::Admin)
    {
        reasons.push(RiskReason::SelfAccessLoss);
    }
    reasons
}

pub fn confirm_risky_access_change(reasons: &[RiskReason]) -> Result<()> {
    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let mut stdout = io::stdout();
    confirm_risky_access_change_with_io(reasons, stdin.is_terminal(), &mut stdin_lock, &mut stdout)
}

fn confirm_risky_access_change_with_io<R: BufRead, W: Write>(
    reasons: &[RiskReason],
    stdin_is_terminal: bool,
    stdin: &mut R,
    stdout: &mut W,
) -> Result<()> {
    if reasons.is_empty() {
        return Ok(());
    }
    if !stdin_is_terminal {
        bail!(
            "Risky access change requires manual confirmation. Run this command in an interactive terminal and type \"yes\" to continue."
        );
    }

    writeln!(stdout, "Warning:")?;
    for reason in reasons {
        writeln!(stdout, "- {}", reason.message())?;
    }
    writeln!(
        stdout,
        "- User changes are currently non-atomic: remove_user runs before add_new_user. If the second step fails, access can be lost."
    )?;
    write!(stdout, "Type \"yes\" to continue: ")?;
    stdout.flush()?;

    let mut answer = String::new();
    stdin.read_line(&mut answer)?;
    if answer.trim() != "yes" {
        bail!("Access change cancelled because manual confirmation was not approved");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal(id: &str) -> Principal {
        Principal::from_text(id).expect("principal")
    }

    #[test]
    fn detects_zero_admins_when_last_admin_is_removed() {
        let users = vec![
            ("aaaaa-aa".to_string(), 1u8),
            ("ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(), 3u8),
        ];

        let risks = evaluate_access_change_risks(
            &users,
            &principal("r7inp-6aaaa-aaaaa-aaabq-cai"),
            &principal("aaaaa-aa"),
            AccessMutation::Remove,
            "launcher-aa",
        );

        assert_eq!(risks, vec![RiskReason::ZeroAdmins]);
    }

    #[test]
    fn detects_zero_admins_when_last_admin_is_downgraded() {
        let users = vec![
            ("aaaaa-aa".to_string(), 1u8),
            ("ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(), 3u8),
        ];

        let risks = evaluate_access_change_risks(
            &users,
            &principal("r7inp-6aaaa-aaaaa-aaabq-cai"),
            &principal("aaaaa-aa"),
            AccessMutation::Change {
                next_role: MemoryRole::Writer,
            },
            "launcher-aa",
        );

        assert_eq!(risks, vec![RiskReason::ZeroAdmins]);
    }

    #[test]
    fn does_not_flag_zero_admins_when_another_admin_remains() {
        let users = vec![
            ("aaaaa-aa".to_string(), 1u8),
            ("ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(), 1u8),
        ];

        let risks = evaluate_access_change_risks(
            &users,
            &principal("r7inp-6aaaa-aaaaa-aaabq-cai"),
            &principal("aaaaa-aa"),
            AccessMutation::Remove,
            "launcher-aa",
        );

        assert!(risks.is_empty());
    }

    #[test]
    fn detects_self_remove_and_zero_admins_independently() {
        let users = vec![("aaaaa-aa".to_string(), 1u8)];

        let risks = evaluate_access_change_risks(
            &users,
            &principal("aaaaa-aa"),
            &principal("aaaaa-aa"),
            AccessMutation::Remove,
            "launcher-aa",
        );

        assert_eq!(
            risks,
            vec![RiskReason::ZeroAdmins, RiskReason::SelfAccessLoss]
        );
    }

    #[test]
    fn detects_self_admin_downgrade() {
        let users = vec![
            ("aaaaa-aa".to_string(), 1u8),
            ("ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(), 1u8),
        ];

        let risks = evaluate_access_change_risks(
            &users,
            &principal("aaaaa-aa"),
            &principal("aaaaa-aa"),
            AccessMutation::Change {
                next_role: MemoryRole::Reader,
            },
            "launcher-aa",
        );

        assert_eq!(risks, vec![RiskReason::SelfAccessLoss]);
    }

    #[test]
    fn does_not_flag_non_admin_self_change() {
        let users = vec![
            ("aaaaa-aa".to_string(), 2u8),
            ("ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(), 1u8),
        ];

        let risks = evaluate_access_change_risks(
            &users,
            &principal("aaaaa-aa"),
            &principal("aaaaa-aa"),
            AccessMutation::Change {
                next_role: MemoryRole::Reader,
            },
            "launcher-aa",
        );

        assert!(risks.is_empty());
    }

    #[test]
    fn confirmation_accepts_exact_yes() {
        let mut input = io::Cursor::new(b"yes\n".to_vec());
        let mut output = Vec::new();

        let result = confirm_risky_access_change_with_io(
            &[RiskReason::ZeroAdmins],
            true,
            &mut input,
            &mut output,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn confirmation_rejects_other_answers() {
        let mut input = io::Cursor::new(b"no\n".to_vec());
        let mut output = Vec::new();

        let result = confirm_risky_access_change_with_io(
            &[RiskReason::SelfAccessLoss],
            true,
            &mut input,
            &mut output,
        );

        assert_eq!(
            result.expect_err("confirmation should fail").to_string(),
            "Access change cancelled because manual confirmation was not approved"
        );
    }

    #[test]
    fn confirmation_rejects_non_interactive_stdin() {
        let mut input = io::Cursor::new(Vec::new());
        let mut output = Vec::new();

        let result = confirm_risky_access_change_with_io(
            &[RiskReason::ZeroAdmins],
            false,
            &mut input,
            &mut output,
        );

        assert_eq!(
            result
                .expect_err("non-interactive confirmation should fail")
                .to_string(),
            "Risky access change requires manual confirmation. Run this command in an interactive terminal and type \"yes\" to continue."
        );
    }
}
