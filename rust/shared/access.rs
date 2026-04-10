//! Shared access-control and memory-user helpers.
//! Where: consumed by CLI command handlers and TUI bridge code.
//! What: provides role conversion, principal parsing, validation, and user list normalization.
//! Why: keep access-control rules and user rendering inputs identical across interfaces.

use anyhow::{Result, bail};
use ic_agent::export::Principal;
use kinic_core::principal::{PrincipalTextError, parse_required_principal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRole {
    Admin,
    Writer,
    Reader,
}

impl MemoryRole {
    pub fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "admin" => Ok(Self::Admin),
            "writer" => Ok(Self::Writer),
            "reader" => Ok(Self::Reader),
            _ => bail!("role must be one of: admin, writer, reader"),
        }
    }

    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Admin),
            2 => Some(Self::Writer),
            3 => Some(Self::Reader),
            _ => None,
        }
    }

    pub fn code(self) -> u8 {
        match self {
            Self::Admin => 1,
            Self::Writer => 2,
            Self::Reader => 3,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Writer => "writer",
            Self::Reader => "reader",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleMemoryUser {
    pub principal_id: String,
    pub role_code: u8,
}

pub fn format_role(code: u8) -> String {
    MemoryRole::from_code(code)
        .map(|role| role.as_str().to_string())
        .unwrap_or_else(|| format!("unknown({code})"))
}

pub fn parse_user_principal(value: &str) -> Result<Principal> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("principal must not be empty");
    }
    if trimmed == "anonymous" {
        return Ok(Principal::anonymous());
    }

    match parse_required_principal(trimmed) {
        Ok(principal) => Ok(principal),
        Err(PrincipalTextError::Empty) => bail!("principal must not be empty"),
        Err(PrincipalTextError::Invalid) => bail!("invalid principal text: {trimmed}"),
    }
}

pub fn validate_role_assignment(principal_text: &str, role: MemoryRole) -> Result<()> {
    if principal_text.trim() == "anonymous" && role == MemoryRole::Admin {
        bail!("cannot grant admin role to anonymous");
    }
    Ok(())
}

pub fn validate_access_control_target(
    principal_text: &str,
    launcher_id: &str,
    role: Option<MemoryRole>,
) -> Result<Principal> {
    let principal = parse_user_principal(principal_text)?;
    ensure_not_launcher_principal(&principal, launcher_id)?;
    if let Some(role) = role {
        validate_role_assignment(principal_text, role)?;
    }
    Ok(principal)
}

pub fn ensure_not_launcher_principal(principal: &Principal, launcher_id: &str) -> Result<()> {
    if principal.to_text() == launcher_id {
        bail!("launcher canister access cannot be modified");
    }
    Ok(())
}

pub fn visible_memory_users(users: Vec<(String, u8)>, launcher_id: &str) -> Vec<VisibleMemoryUser> {
    let mut visible_users = users
        .into_iter()
        .filter(|(principal_id, _)| principal_id != launcher_id)
        .map(|(principal_id, role_code)| VisibleMemoryUser {
            principal_id,
            role_code,
        })
        .collect::<Vec<_>>();
    visible_users.sort_by(|left, right| left.principal_id.cmp(&right.principal_id));
    visible_users
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_user_principal_supports_anonymous() {
        assert_eq!(
            parse_user_principal("anonymous").expect("anonymous principal"),
            Principal::anonymous()
        );
    }

    #[test]
    fn validate_role_assignment_rejects_anonymous_admin() {
        let error = validate_role_assignment("anonymous", MemoryRole::Admin).unwrap_err();
        assert_eq!(error.to_string(), "cannot grant admin role to anonymous");
    }

    #[test]
    fn parse_user_principal_rejects_empty_values() {
        let error = parse_user_principal("   ").unwrap_err();
        assert_eq!(error.to_string(), "principal must not be empty");
    }

    #[test]
    fn parse_user_principal_rejects_invalid_text() {
        let error = parse_user_principal("not-a-principal").unwrap_err();
        assert_eq!(error.to_string(), "invalid principal text: not-a-principal");
    }

    #[test]
    fn validate_access_control_target_rejects_launcher_principal() {
        let error =
            validate_access_control_target("aaaaa-aa", "aaaaa-aa", Some(MemoryRole::Reader))
                .unwrap_err();
        assert_eq!(
            error.to_string(),
            "launcher canister access cannot be modified"
        );
    }

    #[test]
    fn format_role_keeps_unknown_codes_visible() {
        assert_eq!(format_role(9), "unknown(9)");
    }

    #[test]
    fn visible_memory_users_excludes_launcher_and_sorts_rows() {
        let users = vec![
            ("zzzzz-zz".to_string(), 3),
            ("launcher-aa".to_string(), 0),
            ("aaaaa-aa".to_string(), 1),
        ];

        let filtered = visible_memory_users(users, "launcher-aa");

        assert_eq!(
            filtered,
            vec![
                VisibleMemoryUser {
                    principal_id: "aaaaa-aa".to_string(),
                    role_code: 1,
                },
                VisibleMemoryUser {
                    principal_id: "zzzzz-zz".to_string(),
                    role_code: 3,
                },
            ]
        );
    }
}
