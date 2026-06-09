//! rust/icp_cli_identity_tests.rs
//! Where: unit tests for icp-cli Internet Identity metadata handling.
//! What: verifies resolver branching and user-facing recovery messages.
//! Why: keep linked-II support safe without touching the real keychain in tests.

use std::{ffi::OsString, fs, path::PathBuf};

use ic_agent::identity::{Delegation, SignedDelegation};

use crate::{
    icp_cli_identity::{
        IcpCliIdentity, IcpCliStorage, ensure_chain_not_expired, ensure_keyring_storage,
        ensure_kinic_portal_host, parse_delegation_chain_json, read_internet_identity_metadata,
        resolve_default_identity_dir,
    },
    identity_store::normalize_spki_key,
};

#[test]
fn read_metadata_returns_none_for_keyring_identity() {
    let dir = tempfile_dir("icp-cli-metadata-keyring");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("identity_list.json"),
        r#"{"v":1,"identities":{"alice":{"kind":"keyring","algorithm":"secp256k1"}}}"#,
    )
    .unwrap();

    let metadata = read_internet_identity_metadata("alice", &dir).unwrap();

    assert!(metadata.is_none());
}

#[test]
fn read_metadata_detects_internet_identity() {
    let dir = tempfile_dir("icp-cli-metadata-ii");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("identity_list.json"),
        r#"{"v":1,"identities":{"alice":{"kind":"internet-identity","storage":{"kind":"keyring"}}}}"#,
    )
    .unwrap();

    let metadata = read_internet_identity_metadata("alice", &dir)
        .unwrap()
        .unwrap();

    assert_eq!(metadata.kind, "internet-identity");
    assert_eq!(metadata.storage.unwrap().kind, "keyring");
}

#[test]
fn kinic_portal_host_accepts_exact_origin() {
    let identity = internet_identity_with_host(Some("https://memory.kinic.xyz"));

    ensure_kinic_portal_host("alice", &identity).unwrap();
}

#[test]
fn kinic_portal_host_accepts_trailing_slash() {
    let identity = internet_identity_with_host(Some("https://memory.kinic.xyz/"));

    ensure_kinic_portal_host("alice", &identity).unwrap();
}

#[test]
fn kinic_portal_host_rejects_other_origin() {
    let identity = internet_identity_with_host(Some("https://cli.id.ai/"));

    let error = ensure_kinic_portal_host("alice", &identity).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("--host https://memory.kinic.xyz")
    );
}

#[test]
fn kinic_portal_host_rejects_missing_host() {
    let identity = internet_identity_with_host(None);

    let error = ensure_kinic_portal_host("alice", &identity).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("--host https://memory.kinic.xyz")
    );
}

#[test]
fn unsupported_storage_mentions_recreate_command() {
    let identity = IcpCliIdentity {
        kind: "internet-identity".to_string(),
        storage: Some(IcpCliStorage {
            kind: "plaintext".to_string(),
        }),
        host: Some("https://memory.kinic.xyz".to_string()),
    };

    let error = ensure_keyring_storage("alice", &identity).unwrap_err();

    assert!(error.to_string().contains("--storage keyring"));
}

#[test]
fn expired_chain_mentions_icp_login() {
    let entry = SignedDelegation {
        delegation: Delegation {
            pubkey: vec![1],
            expiration: 1,
            targets: None,
        },
        signature: vec![2],
    };

    let error = ensure_chain_not_expired("alice", &[entry]).unwrap_err();

    assert!(error.to_string().contains("icp identity login alice"));
}

#[test]
fn parses_icp_cli_delegation_chain_fixture() {
    let chain = parse_delegation_chain_json(include_str!(
        "../tests/fixtures/icp_cli_delegation_chain.json"
    ))
    .unwrap();
    let expected_user_key = normalize_spki_key(&hex::decode(
        "302a300506032b657003210079b5562e8fe654f94078b112e8a98ba7901f853ae695bed7e0e3910bad049664",
    )
    .unwrap())
    .unwrap();
    let expected_session_key = normalize_spki_key(&hex::decode(
        "302a300506032b6570032100da29e95b02e00ffa15645775fb1d2ba222a1943395eea06b94e2c057b7be69d0",
    )
    .unwrap())
    .unwrap();

    assert_eq!(chain.public_key, expected_user_key);
    assert_eq!(chain.delegations.len(), 1);
    assert_eq!(chain.delegations[0].delegation.pubkey, expected_session_key);
    assert_eq!(
        chain.delegations[0].delegation.expiration,
        u64::from_str_radix("1a46e83335d50000", 16).unwrap()
    );
    assert!(chain.delegations[0].delegation.targets.is_none());
    assert_eq!(
        hex::encode(&chain.delegations[0].signature),
        "f28335854ee8d4d398c598a09287dc6e5361bb1235c558b06b68c9752e0d5872c725f20b6787e9b9396aeafb4f9356ad3347b2b636bb47ed71477e7b603c5b0d"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn default_identity_dir_uses_macos_application_support() {
    let path = resolve_default_identity_dir(OsString::from("/Users/alice"), None);

    assert_eq!(
        path,
        PathBuf::from("/Users/alice")
            .join("Library")
            .join("Application Support")
            .join("org.dfinity.icp-cli")
            .join("identity")
    );
}

#[cfg(target_os = "linux")]
#[test]
fn default_identity_dir_uses_xdg_data_home_on_linux() {
    let path = resolve_default_identity_dir(
        OsString::from("/home/alice"),
        Some(OsString::from("/tmp/data-home")),
    );

    assert_eq!(
        path,
        PathBuf::from("/tmp/data-home")
            .join("org.dfinity.icp-cli")
            .join("identity")
    );
}

#[cfg(target_os = "linux")]
#[test]
fn default_identity_dir_falls_back_to_local_share_on_linux() {
    let path = resolve_default_identity_dir(OsString::from("/home/alice"), None);

    assert_eq!(
        path,
        PathBuf::from("/home/alice")
            .join(".local")
            .join("share")
            .join("org.dfinity.icp-cli")
            .join("identity")
    );
}

fn tempfile_dir(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    path
}

fn internet_identity_with_host(host: Option<&str>) -> IcpCliIdentity {
    IcpCliIdentity {
        kind: "internet-identity".to_string(),
        storage: Some(IcpCliStorage {
            kind: "keyring".to_string(),
        }),
        host: host.map(ToString::to_string),
    }
}
