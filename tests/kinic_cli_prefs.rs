use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::json;

fn temp_config_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kinic-cli-prefs-{name}-{nanos}"));
    fs::create_dir_all(&path).expect("temp config dir should be created");
    path
}

fn app_config_root(home_dir: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        home_dir.join("Library").join("Application Support")
    } else {
        home_dir.join(".config")
    }
}

fn prefs_command(config_dir: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kinic-cli"));
    command.env("HOME", config_dir);
    command.env("XDG_CONFIG_HOME", app_config_root(config_dir));
    command
}

fn read_prefs_yaml(config_dir: &Path) -> String {
    fs::read_to_string(app_config_root(config_dir).join("kinic").join("tui.yaml"))
        .expect("prefs yaml should exist after mutation")
}

#[test]
fn prefs_show_runs_without_identity() {
    let config_dir = temp_config_dir("show");

    let output = prefs_command(&config_dir)
        .args(["prefs", "show"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        parsed,
        json!({
            "default_memory_id": null,
            "saved_tags": [],
            "manual_memory_ids": [],
            "chat_overall_top_k": 8,
            "chat_per_memory_cap": 3,
            "chat_mmr_lambda": 70,
            "embedding_model_id": "api",
        })
    );
}

#[test]
fn prefs_mutations_update_shared_yaml_and_preserve_chat_fields() {
    let config_dir = temp_config_dir("mutations");
    let kinic_dir = app_config_root(&config_dir).join("kinic");
    fs::create_dir_all(&kinic_dir).unwrap();
    fs::write(
        kinic_dir.join("tui.yaml"),
        concat!(
            "default_memory_id: yta6k-5x777-77774-aaaaa-cai\n",
            "saved_tags:\n",
            "  - keep\n",
            "manual_memory_ids:\n",
            "  - yta6k-5x777-77774-aaaaa-cai\n",
            "chat_overall_top_k: 10\n",
            "chat_per_memory_cap: 4\n",
            "chat_mmr_lambda: 80\n"
        ),
    )
    .unwrap();

    let output = prefs_command(&config_dir)
        .args(["prefs", "set-default-memory", "--memory-id", "aaaaa-aa"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        parsed,
        json!({
            "resource": "default_memory_id",
            "action": "set",
            "status": "updated",
            "value": "aaaaa-aa"
        })
    );

    let output = prefs_command(&config_dir)
        .args(["prefs", "add-tag", "--tag", "  docs  "])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        parsed,
        json!({
            "resource": "saved_tags",
            "action": "add",
            "status": "updated",
            "value": "docs"
        })
    );

    let output = prefs_command(&config_dir)
        .args([
            "prefs",
            "add-memory",
            "--memory-id",
            "ryjl3-tyaaa-aaaaa-aaaba-cai",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        parsed,
        json!({
            "resource": "manual_memory_ids",
            "action": "add",
            "status": "updated",
            "value": "ryjl3-tyaaa-aaaaa-aaaba-cai"
        })
    );

    let yaml = read_prefs_yaml(&config_dir);
    assert!(yaml.contains("default_memory_id: aaaaa-aa"));
    assert!(yaml.contains("- docs"));
    assert!(yaml.contains("- keep"));
    assert!(yaml.contains("- ryjl3-tyaaa-aaaaa-aaaba-cai"));
    assert!(yaml.contains("chat_overall_top_k: 10"));
    assert!(yaml.contains("chat_per_memory_cap: 4"));
    assert!(yaml.contains("chat_mmr_lambda: 80"));

    let output = prefs_command(&config_dir)
        .args(["prefs", "show"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        parsed,
        json!({
            "default_memory_id": "aaaaa-aa",
            "saved_tags": ["docs", "keep"],
            "manual_memory_ids": [
                "yta6k-5x777-77774-aaaaa-cai",
                "ryjl3-tyaaa-aaaaa-aaaba-cai"
            ],
            "chat_overall_top_k": 10,
            "chat_per_memory_cap": 4,
            "chat_mmr_lambda": 80,
            "embedding_model_id": "api",
        })
    );
}

#[test]
fn prefs_remove_commands_report_unchanged_when_entry_is_missing() {
    let config_dir = temp_config_dir("remove-missing");

    let output = prefs_command(&config_dir)
        .args(["prefs", "remove-tag", "--tag", "docs"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        parsed,
        json!({
            "resource": "saved_tags",
            "action": "remove",
            "status": "unchanged",
            "value": "docs"
        })
    );

    let output = prefs_command(&config_dir)
        .args(["prefs", "remove-memory", "--memory-id", "aaaaa-aa"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        parsed,
        json!({
            "resource": "manual_memory_ids",
            "action": "remove",
            "status": "unchanged",
            "value": "aaaaa-aa"
        })
    );
}

#[test]
fn prefs_clear_default_memory_updates_yaml() {
    let config_dir = temp_config_dir("clear-default");
    let kinic_dir = app_config_root(&config_dir).join("kinic");
    fs::create_dir_all(&kinic_dir).unwrap();
    fs::write(
        kinic_dir.join("tui.yaml"),
        concat!(
            "default_memory_id: aaaaa-aa\n",
            "saved_tags: []\n",
            "manual_memory_ids: []\n"
        ),
    )
    .unwrap();

    let output = prefs_command(&config_dir)
        .args(["prefs", "clear-default-memory"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        parsed,
        json!({
            "resource": "default_memory_id",
            "action": "clear",
            "status": "updated",
            "value": null
        })
    );
    let yaml = read_prefs_yaml(&config_dir);
    assert!(!yaml.contains("default_memory_id: aaaaa-aa"));
    assert!(yaml.contains("saved_tags: []"));
    assert!(yaml.contains("manual_memory_ids: []"));
}

#[test]
fn prefs_chat_retrieval_mutations_update_yaml_and_show_output() {
    let config_dir = temp_config_dir("chat-retrieval");

    let output = prefs_command(&config_dir)
        .args(["prefs", "set-chat-overall-top-k", "--value", "10"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("json response should parse");
    assert_eq!(
        parsed,
        json!({
            "resource": "chat_overall_top_k",
            "action": "set",
            "status": "updated",
            "value": 10
        })
    );

    let output = prefs_command(&config_dir)
        .args(["prefs", "set-chat-per-memory-cap", "--value", "4"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = prefs_command(&config_dir)
        .args(["prefs", "set-chat-mmr-lambda", "--value", "80"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let yaml = read_prefs_yaml(&config_dir);
    assert!(yaml.contains("chat_overall_top_k: 10"));
    assert!(yaml.contains("chat_per_memory_cap: 4"));
    assert!(yaml.contains("chat_mmr_lambda: 80"));

    let output = prefs_command(&config_dir)
        .args(["prefs", "show"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("show output should parse");
    assert_eq!(parsed["chat_overall_top_k"], json!(10));
    assert_eq!(parsed["chat_per_memory_cap"], json!(4));
    assert_eq!(parsed["chat_mmr_lambda"], json!(80));
}

#[test]
fn prefs_add_memory_validate_requires_identity_or_ii() {
    let config_dir = temp_config_dir("add-memory-validate");

    let output = prefs_command(&config_dir)
        .args([
            "prefs",
            "add-memory",
            "--memory-id",
            "ryjl3-tyaaa-aaaaa-aaaba-cai",
            "--validate",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--identity is required unless --ii is set"));
}
