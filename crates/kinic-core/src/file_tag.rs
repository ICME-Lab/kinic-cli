//! File-derived tag normalization rules.
//! Where: reused by CLI insert flows and TUI file selection helpers.
//! What: derives stable tag text from a file path using its stem and absolute-path hash.
//! Why: keep delete-sensitive tag generation stable across working directories.

use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub fn normalize_insert_file_path_input(path: &str) -> &str {
    let trimmed = path.trim();
    if let Some(inner) = trimmed
        .strip_prefix('\'')
        .and_then(|value| value.strip_suffix('\''))
    {
        return inner;
    }
    if let Some(inner) = trimmed
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    {
        return inner;
    }
    trimmed
}

pub fn derive_file_tag(file_path: &Path) -> Option<String> {
    let resolved = resolved_absolute_path(file_path)?;
    let stem = file_stem_tag(&resolved)?;
    Some(format!("{stem}-{}", short_path_hash(&resolved)))
}

fn resolved_absolute_path(file_path: &Path) -> Option<PathBuf> {
    let absolute = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        env::current_dir().ok()?.join(file_path)
    };
    Some(fs::canonicalize(&absolute).unwrap_or(absolute))
}

fn file_stem_tag(file_path: &Path) -> Option<String> {
    file_path
        .file_stem()
        .or_else(|| file_path.file_name())
        .map(|value| value.to_string_lossy().trim().to_string())
        .filter(|value| !value.is_empty())
}

fn short_path_hash(file_path: &Path) -> String {
    let digest = Sha256::digest(file_path.to_string_lossy().as_bytes());
    let mut short = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        use std::fmt::Write;
        let _ = write!(&mut short, "{byte:02x}");
    }
    short
}

#[cfg(test)]
mod tests {
    use super::{derive_file_tag, normalize_insert_file_path_input};
    use std::{
        env, fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    fn unique_temp_dir() -> PathBuf {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let path = env::temp_dir().join(format!("kinic-core-file-tag-{unique_suffix}"));
        fs::create_dir_all(&path).expect("temporary directory should be creatable");
        path
    }

    fn write_file(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("temporary file parent should be creatable");
        }
        fs::write(path, "payload").expect("temporary file should be writable");
    }

    #[test]
    fn derive_file_tag_uses_stem_and_short_hash() {
        let dir = unique_temp_dir();
        let path = dir.join("docs/spec/api.md");
        write_file(&path);

        let tag = derive_file_tag(&path);

        assert!(
            matches!(tag, Some(value) if value.strip_prefix("api-").is_some_and(|suffix| suffix.len() == 16))
        );
        fs::remove_dir_all(dir).expect("temporary directory should be removable");
    }

    #[test]
    fn derive_file_tag_is_stable_for_same_absolute_path() {
        let dir = unique_temp_dir();
        let path = dir.join("same/report.pdf");
        write_file(&path);

        assert_eq!(derive_file_tag(&path), derive_file_tag(&path));
        fs::remove_dir_all(dir).expect("temporary directory should be removable");
    }

    #[test]
    fn derive_file_tag_changes_for_different_absolute_paths_with_same_name() {
        let dir = unique_temp_dir();
        let left_path = dir.join("a/report.pdf");
        let right_path = dir.join("b/report.pdf");
        write_file(&left_path);
        write_file(&right_path);

        assert!(matches!(
            (derive_file_tag(&left_path), derive_file_tag(&right_path)),
            (Some(left), Some(right)) if left.starts_with("report-") && right.starts_with("report-") && left != right
        ));
        fs::remove_dir_all(dir).expect("temporary directory should be removable");
    }

    #[test]
    fn derive_file_tag_uses_absolute_resolution_when_canonicalize_fails() {
        let dir = unique_temp_dir();
        let path = dir.join("missing/note.txt");

        let tag = derive_file_tag(&path);

        assert!(matches!(tag, Some(value) if value.starts_with("note-")));
        fs::remove_dir_all(dir).expect("temporary directory should be removable");
    }

    #[test]
    fn normalize_insert_file_path_input_strips_wrapping_single_quotes() {
        assert_eq!(
            normalize_insert_file_path_input(" '/tmp/doc.pdf' "),
            "/tmp/doc.pdf"
        );
    }

    #[test]
    fn normalize_insert_file_path_input_strips_wrapping_double_quotes() {
        assert_eq!(
            normalize_insert_file_path_input(" \"/tmp/doc.pdf\" "),
            "/tmp/doc.pdf"
        );
    }

    #[test]
    fn normalize_insert_file_path_input_keeps_unquoted_path() {
        assert_eq!(
            normalize_insert_file_path_input(" /tmp/doc.pdf "),
            "/tmp/doc.pdf"
        );
    }
}
