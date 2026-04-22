use super::*;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
struct TestSettings {
    default_memory_id: Option<String>,
}

fn unique_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    std::env::temp_dir().join(format!("tui-kit-host-settings-{name}-{nanos}.yaml"))
}

#[test]
fn save_yaml_at_path_creates_parent_dirs_and_roundtrips() {
    let path = unique_path("roundtrip");
    let settings = TestSettings {
        default_memory_id: Some("aaaaa-aa".to_string()),
    };
    save_yaml_at_path(path.as_path(), &settings).expect("save should succeed");

    let loaded: TestSettings =
        load_yaml_or_default_at_path(path.as_path()).expect("load should succeed");

    assert_eq!(loaded, settings);
    assert!(path.exists());
    let _ = std::fs::remove_file(path);
}

#[test]
fn save_yaml_at_path_replaces_previous_contents() {
    let path = unique_path("replace");
    save_yaml_at_path(
        path.as_path(),
        &TestSettings {
            default_memory_id: Some("aaaaa-aa".to_string()),
        },
    )
    .expect("initial save should succeed");
    save_yaml_at_path(
        path.as_path(),
        &TestSettings {
            default_memory_id: Some("bbbbb-bb".to_string()),
        },
    )
    .expect("replacement save should succeed");

    let loaded: TestSettings =
        load_yaml_or_default_at_path(path.as_path()).expect("load should succeed");
    let content = std::fs::read_to_string(path.as_path()).expect("saved file should be readable");

    assert_eq!(loaded.default_memory_id.as_deref(), Some("bbbbb-bb"));
    assert!(content.contains("bbbbb-bb"));
    assert!(!content.contains("aaaaa-aa"));
    let _ = std::fs::remove_file(path);
}

#[cfg(windows)]
#[test]
fn replace_file_replaces_existing_destination() {
    let from = unique_path("replace-src");
    let to = unique_path("replace-dst");
    std::fs::write(from.as_path(), "new contents").expect("source file should be writable");
    std::fs::write(to.as_path(), "old contents").expect("dest file should be writable");

    replace_file(from.as_path(), to.as_path()).expect("replace should succeed");

    assert!(!from.exists());
    let content = std::fs::read_to_string(to.as_path()).expect("dest file should be readable");
    assert_eq!(content, "new contents");
    let _ = std::fs::remove_file(to);
}
