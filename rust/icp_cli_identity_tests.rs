//! rust/icp_cli_identity_tests.rs
//! Where: unit tests for icp-cli Internet Identity metadata handling.
//! What: verifies resolver branching and user-facing recovery messages.
//! Why: keep linked-II support safe without touching the real keychain in tests.

use std::{fs, path::PathBuf};

use ic_agent::identity::{Delegation, SignedDelegation};

use crate::icp_cli_identity::{
    IcpCliIdentity, IcpCliStorage, ensure_chain_not_expired, ensure_keyring_storage,
    ensure_kinic_portal_host, read_internet_identity_metadata,
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

    let metadata = read_internet_identity_metadata("alice", &dir).unwrap().unwrap();

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

    assert!(error.to_string().contains("--host https://memory.kinic.xyz"));
}

#[test]
fn kinic_portal_host_rejects_missing_host() {
    let identity = internet_identity_with_host(None);

    let error = ensure_kinic_portal_host("alice", &identity).unwrap_err();

    assert!(error.to_string().contains("--host https://memory.kinic.xyz"));
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
