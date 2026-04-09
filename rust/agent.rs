use std::{fmt, io::Cursor, sync::Arc};

use anyhow::Result;
use ic_agent::{
    Agent, Identity,
    export::reqwest::Url,
    identity::{BasicIdentity, Secp256k1Identity},
};
use keyring::Error as KeyringError;

pub const KEYRING_SERVICE_NAME: &str = "internet_computer_identities";
pub const KEYRING_IDENTITY_PREFIX: &str = "internet_computer_identity_";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeychainErrorCode {
    LookupFailed,
    AccessDenied,
    InteractionNotAllowed,
    KeychainError,
}

impl KeychainErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LookupFailed => "KEYCHAIN_LOOKUP_FAILED",
            Self::AccessDenied => "KEYCHAIN_ACCESS_DENIED",
            Self::InteractionNotAllowed => "KEYCHAIN_INTERACTION_NOT_ALLOWED",
            Self::KeychainError => "KEYCHAIN_ERROR",
        }
    }
}

impl fmt::Display for KeychainErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeychainErrorInfo {
    pub code: KeychainErrorCode,
    pub identity_name: String,
    pub entry_name: String,
    pub retryable: bool,
    pub user_action: &'static str,
    pub details: Option<String>,
}

impl KeychainErrorInfo {
    pub fn from_keyring_error(identity_name: &str, error: &KeyringError) -> Self {
        let entry_name = format!("{KEYRING_IDENTITY_PREFIX}{identity_name}");
        match error {
            KeyringError::NoEntry => Self {
                code: KeychainErrorCode::LookupFailed,
                identity_name: identity_name.to_string(),
                entry_name,
                retryable: true,
                user_action: "recheck_keychain_lookup_state",
                details: None,
            },
            KeyringError::NoStorageAccess(platform_error) => {
                let details = platform_error.to_string();
                if details.contains("-67671") || details.contains("errSecInteractionNotAllowed") {
                    Self {
                        code: KeychainErrorCode::InteractionNotAllowed,
                        identity_name: identity_name.to_string(),
                        entry_name,
                        retryable: false,
                        user_action: "switch_to_arm64_dfx",
                        details: Some(details),
                    }
                } else {
                    Self {
                        code: KeychainErrorCode::AccessDenied,
                        identity_name: identity_name.to_string(),
                        entry_name,
                        retryable: true,
                        user_action: "approve_keychain_or_unlock",
                        details: Some(details),
                    }
                }
            }
            other => {
                let details = format!("{other:?}");
                if details.contains("-67671") || details.contains("errSecInteractionNotAllowed") {
                    Self {
                        code: KeychainErrorCode::InteractionNotAllowed,
                        identity_name: identity_name.to_string(),
                        entry_name,
                        retryable: false,
                        user_action: "switch_to_arm64_dfx",
                        details: Some(details),
                    }
                } else {
                    Self {
                        code: KeychainErrorCode::KeychainError,
                        identity_name: identity_name.to_string(),
                        entry_name,
                        retryable: true,
                        user_action: "inspect_keychain_error",
                        details: Some(details),
                    }
                }
            }
        }
    }

    pub fn to_agent_message(&self) -> String {
        let body = match self.code {
            KeychainErrorCode::LookupFailed => format!(
                "Keychain lookup for identity \"{}\" could not be confirmed. The entry may be missing, access may have been delayed, or macOS may not have completed the lookup. Expected entry: \"{}\".",
                self.identity_name, self.entry_name
            ),
            KeychainErrorCode::AccessDenied => format!(
                "Keychain access was not granted for identity \"{}\". Approve the macOS Keychain prompt, unlock the keychain if needed, and try again. Cause: {}",
                self.identity_name,
                self.details.as_deref().unwrap_or("storage access unavailable")
            ),
            KeychainErrorCode::InteractionNotAllowed => "macOS keychain returned -67671 (errSecInteractionNotAllowed). This is a known bug when using the x86 build of dfx; please install and use the arm64 build instead. See more detail: https://github.com/dfinity/sdk/blob/0.28.0/docs/migration/dfx-0.28.0-migration-guide.md".to_string(),
            KeychainErrorCode::KeychainError => format!(
                "Keychain Error: {}",
                self.details.as_deref().unwrap_or("unknown keychain error")
            ),
        };

        format!("[{}] {}", self.code, body)
    }

    pub fn to_user_message(&self) -> String {
        self.to_agent_message()
    }

    pub fn to_context_note(&self) -> &'static str {
        match self.code {
            KeychainErrorCode::LookupFailed => {
                "Check the macOS Keychain entry and whether approval was delayed or interrupted."
            }
            KeychainErrorCode::AccessDenied | KeychainErrorCode::InteractionNotAllowed => {
                "Check the macOS Keychain prompt and the selected identity entry."
            }
            KeychainErrorCode::KeychainError => {
                "Check the macOS Keychain entry and local security settings."
            }
        }
    }
}

pub fn extract_keychain_error_code(message: &str) -> Option<KeychainErrorCode> {
    let code = message.strip_prefix('[')?.split_once(']')?.0;
    match code {
        "KEYCHAIN_LOOKUP_FAILED" => Some(KeychainErrorCode::LookupFailed),
        "KEYCHAIN_ACCESS_DENIED" => Some(KeychainErrorCode::AccessDenied),
        "KEYCHAIN_INTERACTION_NOT_ALLOWED" => Some(KeychainErrorCode::InteractionNotAllowed),
        "KEYCHAIN_ERROR" => Some(KeychainErrorCode::KeychainError),
        _ => None,
    }
}

#[derive(Clone)]
pub struct AgentFactory {
    use_mainnet: bool,
    identity_suffix: String,
    identity_override: Option<Arc<dyn Identity>>,
}

impl AgentFactory {
    pub fn new(use_mainnet: bool, identity_suffix: impl Into<String>) -> Self {
        Self {
            use_mainnet,
            identity_suffix: identity_suffix.into(),
            identity_override: None,
        }
    }

    pub fn new_with_identity<I>(use_mainnet: bool, identity: I) -> Self
    where
        I: Identity + 'static,
    {
        Self {
            use_mainnet,
            identity_suffix: String::new(),
            identity_override: Some(Arc::new(identity)),
        }
    }

    pub fn new_with_arc_identity(use_mainnet: bool, identity: Arc<dyn Identity>) -> Self {
        Self {
            use_mainnet,
            identity_suffix: String::new(),
            identity_override: Some(identity),
        }
    }

    pub async fn build(&self) -> Result<Agent> {
        let builder = if let Some(identity) = &self.identity_override {
            Agent::builder().with_arc_identity(identity.clone())
        } else {
            Agent::builder().with_arc_identity(load_identity_from_keyring(&self.identity_suffix)?)
        };

        let url = if self.use_mainnet {
            "https://ic0.app"
        } else {
            "http://127.0.0.1:4943"
        };
        let url = Url::parse(url)?;
        let agent = builder.with_url(url).build()?;

        if !self.use_mainnet {
            agent.fetch_root_key().await?;
        }
        Ok(agent)
    }
}

pub fn load_identity_from_keyring(suffix: &str) -> Result<Arc<dyn Identity>> {
    let pem_bytes = load_pem_from_keyring(suffix)?;
    parse_identity_from_pem_bytes(&pem_bytes)
}

fn parse_identity_from_pem_bytes(pem_bytes: &[u8]) -> Result<Arc<dyn Identity>> {
    let pem_text = String::from_utf8(pem_bytes.to_vec())?;
    let pem = pem::parse(pem_text.as_bytes())?;
    match pem.tag() {
        "PRIVATE KEY" => {
            let identity = BasicIdentity::from_pem(Cursor::new(pem_text))?;
            Ok(Arc::new(identity))
        }
        "EC PRIVATE KEY" => {
            let identity = Secp256k1Identity::from_pem(Cursor::new(pem_text))?;
            Ok(Arc::new(identity))
        }
        _ => anyhow::bail!("Unsupported PEM tag: {}", pem.tag()),
    }
}

fn load_pem_from_keyring(suffix: &str) -> anyhow::Result<Vec<u8>> {
    let account = format!("{KEYRING_IDENTITY_PREFIX}{suffix}");
    let entry = keyring::Entry::new(KEYRING_SERVICE_NAME, &account)?;
    let encoded_pem = entry.get_password().map_err(|error| {
        anyhow::anyhow!(KeychainErrorInfo::from_keyring_error(suffix, &error).to_user_message())
    })?;
    Ok(hex::decode(encoded_pem)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{error::Error, fmt as std_fmt};

    #[derive(Debug)]
    struct StubError(&'static str);

    impl std_fmt::Display for StubError {
        fn fmt(&self, f: &mut std_fmt::Formatter<'_>) -> std_fmt::Result {
            f.write_str(self.0)
        }
    }

    impl Error for StubError {}

    #[test]
    fn keychain_error_info_reports_lookup_failure_for_no_entry() {
        let info = KeychainErrorInfo::from_keyring_error("alice", &KeyringError::NoEntry);

        assert_eq!(info.code, KeychainErrorCode::LookupFailed);
        assert_eq!(info.entry_name, "internet_computer_identity_alice");
        assert!(info.retryable);
        assert_eq!(info.user_action, "recheck_keychain_lookup_state");
        assert!(info.to_agent_message().starts_with("[KEYCHAIN_LOOKUP_FAILED]"));
        assert!(info.to_agent_message().contains("could not be confirmed"));
    }

    #[test]
    fn keychain_error_info_reports_storage_access_issue() {
        let info = KeychainErrorInfo::from_keyring_error(
            "alice",
            &KeyringError::NoStorageAccess(Box::new(StubError("User interaction is not allowed"))),
        );

        assert_eq!(info.code, KeychainErrorCode::AccessDenied);
        assert!(info.retryable);
        assert_eq!(info.user_action, "approve_keychain_or_unlock");
        assert_eq!(
            info.details.as_deref(),
            Some("User interaction is not allowed")
        );
        assert!(info.to_agent_message().starts_with("[KEYCHAIN_ACCESS_DENIED]"));
    }

    #[test]
    fn keychain_error_info_preserves_err_sec_interaction_not_allowed_guidance() {
        let info = KeychainErrorInfo::from_keyring_error(
            "alice",
            &KeyringError::NoStorageAccess(Box::new(StubError(
                "OSStatus -67671 errSecInteractionNotAllowed",
            ))),
        );

        assert_eq!(info.code, KeychainErrorCode::InteractionNotAllowed);
        assert!(!info.retryable);
        assert_eq!(info.user_action, "switch_to_arm64_dfx");
        let message = info.to_agent_message();
        assert!(message.starts_with("[KEYCHAIN_INTERACTION_NOT_ALLOWED]"));
        assert!(message.contains("errSecInteractionNotAllowed"));
        assert!(message.contains("arm64 build instead"));
    }

    #[test]
    fn keychain_error_info_falls_back_to_generic_message() {
        let info = KeychainErrorInfo::from_keyring_error(
            "alice",
            &KeyringError::PlatformFailure(Box::new(StubError("security framework failed"))),
        );

        assert_eq!(
            info.to_agent_message(),
            "[KEYCHAIN_ERROR] Keychain Error: PlatformFailure(StubError(\"security framework failed\"))"
        );
        assert_eq!(info.code, KeychainErrorCode::KeychainError);
        assert!(info.retryable);
        assert_eq!(info.user_action, "inspect_keychain_error");
    }

    #[test]
    fn extract_keychain_error_code_reads_prefixed_messages() {
        assert_eq!(
            extract_keychain_error_code("[KEYCHAIN_ACCESS_DENIED] denied"),
            Some(KeychainErrorCode::AccessDenied)
        );
        assert_eq!(
            extract_keychain_error_code("[KEYCHAIN_LOOKUP_FAILED] lookup"),
            Some(KeychainErrorCode::LookupFailed)
        );
        assert_eq!(extract_keychain_error_code("plain error"), None);
    }
}
