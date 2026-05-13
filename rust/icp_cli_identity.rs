//! rust/icp_cli_identity.rs
//! Where: identity loading bridge between icp-cli linked identities and Kinic CLI.
//! What: reads icp-cli Internet Identity delegations and turns them into ic-agent identities.
//! Why: let `kinic-cli --identity <name>` reuse II-linked identities created by `icp identity link ii`.

use std::{
    collections::HashMap,
    ffi::OsString,
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow};
use ic_agent::{
    Identity,
    export::Principal,
    identity::{BasicIdentity, DelegatedIdentity, Delegation, DelegationError, SignedDelegation},
};
use serde::Deserialize;
use tracing::warn;

use crate::identity_store::normalize_spki_key;

const ICP_CLI_KEYRING_SERVICE_NAME: &str = "icp-cli";
const ICP_CLI_DELEGATION_ACCOUNT_PREFIX: &str = "delegation_";
const ICP_CLI_IDENTITY_DIR_ENV: &str = "KINIC_ICP_CLI_IDENTITY_DIR";
const KINIC_PORTAL_ORIGIN: &str = "https://memory.kinic.xyz";

#[derive(Debug, Deserialize)]
struct IdentityList {
    identities: HashMap<String, IcpCliIdentity>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct IcpCliIdentity {
    pub(crate) kind: String,
    pub(crate) storage: Option<IcpCliStorage>,
    pub(crate) host: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct IcpCliStorage {
    pub(crate) kind: String,
}

#[derive(Debug, Deserialize)]
struct HexDelegationChain {
    #[serde(rename = "publicKey")]
    public_key: String,
    delegations: Vec<HexSignedDelegation>,
}

#[derive(Debug, Deserialize)]
struct HexSignedDelegation {
    delegation: HexDelegation,
    signature: String,
}

#[derive(Debug, Deserialize)]
struct HexDelegation {
    pubkey: String,
    expiration: String,
    targets: Option<Vec<String>>,
}

pub(crate) fn load_icp_cli_internet_identity(
    identity_name: &str,
) -> Result<Option<Arc<dyn Identity>>> {
    let identity_dir = match default_identity_dir()? {
        Some(path) => path,
        None => return Ok(None),
    };
    load_icp_cli_internet_identity_from_dir(identity_name, &identity_dir)
}

fn load_icp_cli_internet_identity_from_dir(
    identity_name: &str,
    identity_dir: &Path,
) -> Result<Option<Arc<dyn Identity>>> {
    let Some(identity) = read_internet_identity_metadata(identity_name, identity_dir)? else {
        return Ok(None);
    };
    ensure_kinic_portal_host(identity_name, &identity)?;
    ensure_keyring_storage(identity_name, &identity)?;

    let delegation_path = identity_dir
        .join("delegations")
        .join(format!("{identity_name}.json"));
    let chain = read_delegation_chain(&delegation_path)?;
    ensure_chain_not_expired(identity_name, &chain.delegations)?;

    let session_pem = load_session_key_from_keyring(identity_name)?;
    let delegated = new_delegated_identity(chain.public_key, &session_pem, chain.delegations)?;
    Ok(Some(Arc::new(delegated)))
}

pub(crate) fn read_internet_identity_metadata(
    identity_name: &str,
    identity_dir: &Path,
) -> Result<Option<IcpCliIdentity>> {
    let list_path = identity_dir.join("identity_list.json");
    let payload = match fs::read_to_string(&list_path) {
        Ok(payload) => payload,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "Failed to read icp-cli identity list at {}",
                    list_path.display()
                )
            });
        }
    };
    let list: IdentityList = serde_json::from_str(&payload).with_context(|| {
        format!(
            "Failed to parse icp-cli identity list at {}",
            list_path.display()
        )
    })?;
    let Some(identity) = list.identities.into_iter().find_map(|(name, identity)| {
        if name == identity_name {
            Some(identity)
        } else {
            None
        }
    }) else {
        return Ok(None);
    };
    if identity.kind == "internet-identity" {
        Ok(Some(identity))
    } else {
        Ok(None)
    }
}

pub(crate) fn ensure_keyring_storage(identity_name: &str, identity: &IcpCliIdentity) -> Result<()> {
    let storage_kind = identity
        .storage
        .as_ref()
        .map(|storage| storage.kind.as_str())
        .unwrap_or("keyring");
    if storage_kind != "keyring" {
        anyhow::bail!(
            "icp-cli Internet Identity `{identity_name}` uses unsupported storage `{storage_kind}`. Recreate it with `icp identity link ii {identity_name} --host https://memory.kinic.xyz --storage keyring`."
        );
    }
    Ok(())
}

pub(crate) fn ensure_kinic_portal_host(
    identity_name: &str,
    identity: &IcpCliIdentity,
) -> Result<()> {
    let host = identity
        .host
        .as_deref()
        .ok_or_else(|| kinic_portal_host_error(identity_name, "missing host"))?;
    let origin = normalize_host_origin(host)
        .ok_or_else(|| kinic_portal_host_error(identity_name, "invalid host"))?;
    if origin != KINIC_PORTAL_ORIGIN {
        return Err(kinic_portal_host_error(
            identity_name,
            &format!("host `{origin}`"),
        ));
    }
    Ok(())
}

fn kinic_portal_host_error(identity_name: &str, reason: &str) -> anyhow::Error {
    anyhow!(
        "icp-cli Internet Identity `{identity_name}` is linked to {reason}. Recreate it with `icp identity link ii {identity_name} --host {KINIC_PORTAL_ORIGIN}`."
    )
}

fn normalize_host_origin(host: &str) -> Option<String> {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return None;
    }
    let without_trailing_slash = trimmed.trim_end_matches('/');
    let scheme_end = without_trailing_slash.find("://")?;
    let scheme = without_trailing_slash[..scheme_end].to_ascii_lowercase();
    let authority_and_path = &without_trailing_slash[scheme_end + 3..];
    let authority = authority_and_path.split('/').next()?.to_ascii_lowercase();
    if authority.is_empty() {
        return None;
    }
    Some(format!("{scheme}://{authority}"))
}

fn read_delegation_chain(path: &Path) -> Result<ParsedDelegationChain> {
    let payload = fs::read_to_string(path).with_context(|| {
        format!(
            "Failed to read icp-cli delegation file at {}",
            path.display()
        )
    })?;
    parse_delegation_chain_json(&payload)
}

pub(crate) fn parse_delegation_chain_json(payload: &str) -> Result<ParsedDelegationChain> {
    let chain: HexDelegationChain =
        serde_json::from_str(payload).context("Failed to parse icp-cli delegation chain")?;
    let public_key = decode_hex_key("delegation publicKey", &chain.public_key)?;
    let public_key =
        normalize_spki_key(&public_key).context("Unsupported II user public key format")?;
    let delegations = chain
        .delegations
        .into_iter()
        .map(parse_signed_delegation)
        .collect::<Result<Vec<_>>>()?;
    Ok(ParsedDelegationChain {
        public_key,
        delegations,
    })
}

fn parse_signed_delegation(entry: HexSignedDelegation) -> Result<SignedDelegation> {
    let pubkey = decode_hex_key("delegation pubkey", &entry.delegation.pubkey)?;
    let pubkey = normalize_spki_key(&pubkey).context("Unsupported delegation public key format")?;
    let expiration = u64::from_str_radix(entry.delegation.expiration.trim(), 16)
        .context("Failed to parse delegation expiration")?;
    let targets = entry
        .delegation
        .targets
        .map(|targets| {
            targets
                .into_iter()
                .map(Principal::from_text)
                .collect::<Result<Vec<_>, _>>()
                .context("Invalid delegation target principal")
        })
        .transpose()?;
    Ok(SignedDelegation {
        delegation: Delegation {
            pubkey,
            expiration,
            targets,
        },
        signature: decode_hex_key("delegation signature", &entry.signature)?,
    })
}

fn load_session_key_from_keyring(identity_name: &str) -> Result<String> {
    let account = format!("{ICP_CLI_DELEGATION_ACCOUNT_PREFIX}{identity_name}");
    let entry = keyring::Entry::new(ICP_CLI_KEYRING_SERVICE_NAME, &account)?;
    entry.get_password().with_context(|| {
        format!(
            "Failed to read icp-cli session key for `{identity_name}` from keyring entry `{ICP_CLI_KEYRING_SERVICE_NAME}/{account}`"
        )
    })
}

fn new_delegated_identity(
    public_key: Vec<u8>,
    session_pem: &str,
    delegations: Vec<SignedDelegation>,
) -> Result<DelegatedIdentity> {
    let session_identity = parse_session_identity(session_pem)?;
    let delegated = DelegatedIdentity::new(
        public_key.clone(),
        Box::new(session_identity),
        delegations.clone(),
    );
    match delegated {
        Ok(identity) => Ok(identity),
        Err(DelegationError::UnknownAlgorithm) => {
            warn!(
                "icp-cli delegation chain uses an unknown algorithm; skipping local verification."
            );
            let session_identity = parse_session_identity(session_pem)?;
            Ok(DelegatedIdentity::new_unchecked(
                public_key,
                Box::new(session_identity),
                delegations,
            ))
        }
        Err(error) => Err(error.into()),
    }
}

fn parse_session_identity(session_pem: &str) -> Result<BasicIdentity> {
    BasicIdentity::from_pem(Cursor::new(session_pem.to_string()))
        .context("Failed to parse icp-cli session key")
}

pub(crate) fn ensure_chain_not_expired(
    identity_name: &str,
    delegations: &[SignedDelegation],
) -> Result<()> {
    let expiration = delegations
        .iter()
        .map(|entry| entry.delegation.expiration)
        .min()
        .ok_or_else(|| anyhow!("icp-cli delegation chain is empty"))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System time before UNIX_EPOCH")?;
    let now_ns = u64::try_from(now.as_nanos()).context("System time overflow")?;
    if now_ns >= expiration {
        anyhow::bail!(
            "icp-cli Internet Identity `{identity_name}` delegation has expired. Run `icp identity login {identity_name}`."
        );
    }
    Ok(())
}

fn decode_hex_key(label: &str, value: &str) -> Result<Vec<u8>> {
    hex::decode(value.trim()).with_context(|| format!("Failed to decode {label}"))
}

fn default_identity_dir() -> Result<Option<PathBuf>> {
    if let Ok(path) = std::env::var(ICP_CLI_IDENTITY_DIR_ENV) {
        return Ok(Some(PathBuf::from(path)));
    }
    let Some(home) = std::env::var_os("HOME") else {
        return Ok(None);
    };
    Ok(Some(resolve_default_identity_dir(
        home,
        std::env::var_os("XDG_DATA_HOME"),
    )))
}

pub(crate) fn resolve_default_identity_dir(
    home: OsString,
    xdg_data_home: Option<OsString>,
) -> PathBuf {
    platform_identity_dir(home, xdg_data_home)
}

#[cfg(target_os = "macos")]
fn platform_identity_dir(home: OsString, xdg_data_home: Option<OsString>) -> PathBuf {
    let _ = xdg_data_home;
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("org.dfinity.icp-cli")
        .join("identity")
}

#[cfg(target_os = "linux")]
fn platform_identity_dir(home: OsString, xdg_data_home: Option<OsString>) -> PathBuf {
    let data_home = xdg_data_home
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(home).join(".local").join("share"));
    data_home.join("org.dfinity.icp-cli").join("identity")
}

#[cfg(all(not(target_os = "macos"), not(target_os = "linux")))]
fn platform_identity_dir(home: OsString, xdg_data_home: Option<OsString>) -> PathBuf {
    let _ = xdg_data_home;
    PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("org.dfinity.icp-cli")
        .join("identity")
}

pub(crate) struct ParsedDelegationChain {
    pub(crate) public_key: Vec<u8>,
    pub(crate) delegations: Vec<SignedDelegation>,
}
