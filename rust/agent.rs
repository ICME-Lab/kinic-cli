use std::{io::Cursor, sync::Arc};

use anyhow::Result;
use ic_agent::{
    Agent, Identity,
    export::reqwest::Url,
    identity::{BasicIdentity, Secp256k1Identity},
};

pub const KEYRING_SERVICE_NAME: &str = "internet_computer_identities";
pub const KEYRING_IDENTITY_PREFIX: &str = "internet_computer_identity_";

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

    pub async fn build(&self) -> Result<Agent> {
        let builder = if let Some(identity) = &self.identity_override {
            Agent::builder().with_arc_identity(identity.clone())
        } else {
            let pem_bytes = load_pem_from_keyring(&self.identity_suffix)?;
            let pem_text = String::from_utf8(pem_bytes.clone())?;
            let pem = pem::parse(pem_text.as_bytes())?;
            match pem.tag() {
                "PRIVATE KEY" => {
                    let identity = BasicIdentity::from_pem(Cursor::new(pem_text.clone()))?;
                    Agent::builder().with_identity(identity)
                }
                "EC PRIVATE KEY" => {
                    let identity = Secp256k1Identity::from_pem(Cursor::new(pem_text.clone()))?;
                    Agent::builder().with_identity(identity)
                }
                _ => anyhow::bail!("Unsupported PEM tag: {}", pem.tag()),
            }
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

fn load_pem_from_keyring(suffix: &str) -> anyhow::Result<Vec<u8>> {
    let account = format!("{KEYRING_IDENTITY_PREFIX}{suffix}");
    let entry = keyring::Entry::new(KEYRING_SERVICE_NAME, &account)?;
    let encoded_pem = entry.get_password().map_err(|e| {
        let msg = format!("{e:?}");
        if msg.contains("-67671") || msg.contains("errSecInteractionNotAllowed") {
            anyhow::anyhow!(
                "macOS keychain returned -67671 (errSecInteractionNotAllowed). This is a known bug when using the x86 build of dfx; please install and use the arm64 build instead. See more detail: https://github.com/dfinity/sdk/blob/0.28.0/docs/migration/dfx-0.28.0-migration-guide.md"
            )
        } else {
            anyhow::anyhow!("Keychain Error: {msg}")
        }
    })?;
    Ok(hex::decode(encoded_pem)?)
}
