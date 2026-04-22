//! Shared tag normalization rules.
//! Where: reused by CLI preference management and TUI tag lists.
//! What: validates required tag input and canonicalizes saved tag collections.
//! Why: keep tag handling consistent without coupling callers to persistence details.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagError {
    Empty,
}

pub fn normalize_tag_text(value: &str) -> Result<String, TagError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(TagError::Empty)
    } else {
        Ok(trimmed.to_string())
    }
}

pub fn normalize_saved_tags(tags: Vec<String>) -> Vec<String> {
    let mut normalized = tags
        .into_iter()
        .filter_map(|tag| normalize_tag_text(&tag).ok())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

#[cfg(test)]
mod tests {
    use super::{TagError, normalize_saved_tags, normalize_tag_text};

    #[test]
    fn normalize_tag_text_trims_valid_values() {
        assert_eq!(normalize_tag_text("  docs  ").expect("tag"), "docs");
    }

    #[test]
    fn normalize_tag_text_rejects_empty_values() {
        assert_eq!(normalize_tag_text(" "), Err(TagError::Empty));
    }

    #[test]
    fn normalize_saved_tags_sorts_and_deduplicates_values() {
        assert_eq!(
            normalize_saved_tags(vec![
                " docs ".to_string(),
                "zeta".to_string(),
                "docs".to_string(),
                "".to_string(),
            ]),
            vec!["docs".to_string(), "zeta".to_string()]
        );
    }
}
