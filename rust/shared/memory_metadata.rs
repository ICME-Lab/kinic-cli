//! Shared memory metadata parsing and encoding.
//! Where: reused by CLI commands and TUI flows that read or write `metadata.name`.
//! What: strictly parses the JSON envelope and rebuilds it for rename operations.
//! Why: keep one contract for `metadata.name` and avoid ad-hoc string scanning.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedMemoryMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct MemoryMetadataEnvelope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

pub(crate) fn parse_memory_metadata(raw: &str) -> Option<ParsedMemoryMetadata> {
    let envelope = serde_json::from_str::<MemoryMetadataEnvelope>(raw.trim()).ok()?;
    let parsed = ParsedMemoryMetadata {
        name: normalize_metadata_field(envelope.name),
        description: normalize_metadata_field(envelope.description),
    };
    (parsed.name.is_some() || parsed.description.is_some()).then_some(parsed)
}

pub(crate) fn parse_memory_name_fields(raw: &str) -> (String, Option<String>) {
    let trimmed = raw.trim();
    match parse_memory_metadata(trimmed) {
        Some(parsed) => (
            parsed.name.unwrap_or_else(|| trimmed.to_string()),
            parsed.description,
        ),
        None => (trimmed.to_string(), None),
    }
}

pub(crate) fn encode_memory_metadata(
    name: &str,
    description: Option<&str>,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(&MemoryMetadataEnvelope {
        name: Some(name.trim().to_string()),
        description: normalize_metadata_field(description.map(str::to_string)),
    })
}

pub(crate) fn encode_renamed_memory_metadata(
    existing_raw: &str,
    next_name: &str,
) -> Result<String, serde_json::Error> {
    let description = parse_memory_metadata(existing_raw).and_then(|parsed| parsed.description);
    encode_memory_metadata(next_name, description.as_deref())
}

fn normalize_metadata_field(value: Option<String>) -> Option<String> {
    value
        .map(|current| current.trim().to_string())
        .filter(|current| !current.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_memory_metadata_reads_strict_json_object() {
        let parsed = parse_memory_metadata(
            "{\"description\":\"Backend development resources\",\"name\":\"tetetete\"}",
        )
        .expect("metadata should parse");

        assert_eq!(parsed.name.as_deref(), Some("tetetete"));
        assert_eq!(
            parsed.description.as_deref(),
            Some("Backend development resources")
        );
    }

    #[test]
    fn parse_memory_metadata_keeps_escaped_quotes() {
        let parsed = parse_memory_metadata("{\"name\":\"a\\\"b\",\"description\":\"x\"}")
            .expect("metadata should parse");

        assert_eq!(parsed.name.as_deref(), Some("a\"b"));
        assert_eq!(parsed.description.as_deref(), Some("x"));
    }

    #[test]
    fn parse_memory_metadata_rejects_jsonish_strings() {
        let parsed = parse_memory_metadata("prefix \"name\":\"fake\"");

        assert_eq!(parsed, None);
    }

    #[test]
    fn parse_memory_name_fields_falls_back_to_raw_string_on_invalid_json() {
        let parsed = parse_memory_name_fields("{\"name\":\"Alpha\"");

        assert_eq!(parsed, ("{\"name\":\"Alpha\"".to_string(), None));
    }

    #[test]
    fn encode_renamed_memory_metadata_preserves_existing_description() {
        let encoded = encode_renamed_memory_metadata(
            "{\"name\":\"Alpha\",\"description\":\"Quarterly goals\"}",
            "Beta",
        )
        .expect("metadata should encode");

        assert_eq!(
            encoded,
            "{\"name\":\"Beta\",\"description\":\"Quarterly goals\"}"
        );
    }

    #[test]
    fn encode_renamed_memory_metadata_omits_unknown_description() {
        let encoded =
            encode_renamed_memory_metadata("Alpha", "Beta").expect("metadata should encode");

        assert_eq!(encoded, "{\"name\":\"Beta\"}");
    }
}
