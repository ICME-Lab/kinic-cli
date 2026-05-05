// Where: shared Rust core used by both CLI handlers and the Kinic TUI bridge.
// What: centralizes insert/insert-raw/insert-pdf preparation and execution.
// Why: keep the actual insert path in one place so UI additions do not duplicate logic.

use std::{
    fs,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use ic_agent::export::Principal;
use kinic_core::derive_file_tag;
use serde_json::json;

use crate::{
    clients::memory::MemoryClient,
    commands::convert_pdf::pdf_to_markdown,
    embedding::{ensure_vector_dim_matches, late_chunking},
    embedding_config::selected_embedding_dimension,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertMode {
    Normal,
    Raw,
    Pdf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertRequest {
    Normal {
        memory_id: String,
        tag: String,
        text: Option<String>,
        file_path: Option<PathBuf>,
    },
    Raw {
        memory_id: String,
        tag: String,
        text: String,
        embedding_json: String,
    },
    Pdf {
        memory_id: String,
        tag: String,
        file_path: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsertExecutionResult {
    pub mode: InsertMode,
    pub memory_id: String,
    pub tag: String,
    pub inserted_count: usize,
    pub source_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct PreparedInsertItem {
    embedding: Vec<f32>,
    payload: String,
}

#[derive(Debug, Clone, PartialEq)]
enum ValidatedInsertRequest {
    Normal {
        memory_id: String,
        tag: String,
        text: Option<String>,
        file_path: Option<PathBuf>,
    },
    Raw {
        memory_id: String,
        tag: String,
        text: String,
        embedding: Vec<f32>,
    },
    Pdf {
        memory_id: String,
        tag: String,
        file_path: PathBuf,
    },
}

pub async fn execute_insert_request(
    client: &MemoryClient,
    request: &InsertRequest,
) -> Result<InsertExecutionResult> {
    validate_insert_request_fields(request)?;
    let validated = validate_and_transform_insert_request(request)?;
    let expected_dim = client
        .get_dim()
        .await
        .context("Failed to load memory embedding dimension")?;
    ensure_request_matches_memory_dim(&validated, request.memory_id(), expected_dim)?;
    let prepared = prepare_insert_request(&validated).await?;
    ensure_prepared_items_match_memory(request.memory_id(), &prepared, expected_dim)?;
    let inserted_count = prepared.len();
    let source_name = validated.source_name();

    for item in prepared {
        client.insert(item.embedding, &item.payload).await?;
    }

    Ok(InsertExecutionResult {
        mode: validated.mode(),
        memory_id: validated.memory_id().to_string(),
        tag: validated.tag().to_string(),
        inserted_count,
        source_name,
    })
}

pub fn parse_embedding_json(raw: &str) -> Result<Vec<f32>> {
    let parsed: Vec<f32> = serde_json::from_str(raw)
        .with_context(|| "Embedding must be a JSON array of floats, e.g. [0.1, 0.2]")?;
    if parsed.is_empty() {
        bail!("Embedding array cannot be empty");
    }
    Ok(parsed)
}

pub fn validate_insert_request_fields(request: &InsertRequest) -> Result<()> {
    validate_memory_id_required(request.memory_id())?;

    match request {
        InsertRequest::Normal {
            text, file_path, ..
        } => {
            let has_inline_text = text.as_ref().is_some_and(|value| !value.trim().is_empty());
            let has_file_path = file_path
                .as_ref()
                .is_some_and(|path| !path.as_os_str().is_empty());
            if has_inline_text && has_file_path {
                bail!("Provide either text or file path, not both.");
            }
            if !has_inline_text && !has_file_path {
                bail!("Provide text or file path for normal insert.");
            }
            if !has_file_path && request.tag().trim().is_empty() {
                bail!("Tag is required for inline text insert.");
            }
        }
        InsertRequest::Raw {
            text,
            embedding_json,
            ..
        } => {
            validate_required_tag(request.tag(), "Tag is required for raw insert.")?;
            if text.trim().is_empty() {
                bail!("Text is required for raw insert.");
            }
            if embedding_json.trim().is_empty() {
                bail!("Embedding JSON is required for raw insert.");
            }
        }
        InsertRequest::Pdf { file_path, .. } => {
            if file_path.as_os_str().is_empty() {
                bail!("File path is required for PDF insert.");
            }
        }
    }

    Ok(())
}

fn ensure_request_matches_memory_dim(
    request: &ValidatedInsertRequest,
    memory_id: &str,
    expected_dim: u64,
) -> Result<()> {
    match request {
        ValidatedInsertRequest::Raw { embedding, .. } => {
            ensure_vector_dim_matches(memory_id, embedding.len(), expected_dim)
        }
        ValidatedInsertRequest::Normal { .. } | ValidatedInsertRequest::Pdf { .. } => {
            let selected_dim = selected_embedding_dimension()?;
            ensure_vector_dim_matches(memory_id, selected_dim, expected_dim)
        }
    }
}

fn ensure_prepared_items_match_memory(
    memory_id: &str,
    items: &[PreparedInsertItem],
    expected_dim: u64,
) -> Result<()> {
    let Some(first) = items.first() else {
        bail!("Insert content did not produce any chunks.");
    };
    ensure_vector_dim_matches(memory_id, first.embedding.len(), expected_dim)
}

pub fn validate_insert_request_for_submit(request: &InsertRequest) -> Result<()> {
    let _ = validate_and_transform_insert_request(request)?;
    Ok(())
}

fn validate_and_transform_insert_request(
    request: &InsertRequest,
) -> Result<ValidatedInsertRequest> {
    validate_insert_request_fields(request)?;
    validate_memory_id(request.memory_id())?;

    match request {
        InsertRequest::Normal {
            memory_id,
            tag,
            text,
            file_path,
        } => {
            let text = normalized_optional_text(text.clone());
            let file_path = normalized_optional_path(file_path.clone());
            if text.is_none() && file_path.is_none() {
                bail!("Provide text or file path for normal insert.");
            }
            if text.is_some() && file_path.is_some() {
                bail!("Provide either text or file path, not both.");
            }
            if text.is_none() {
                let path = file_path
                    .as_ref()
                    .expect("normal insert should have file path when text is absent");
                validate_text_file_path(path)?;
            }

            Ok(ValidatedInsertRequest::Normal {
                memory_id: memory_id.clone(),
                tag: resolved_tag(tag, text.as_deref(), file_path.as_deref())?,
                text,
                file_path,
            })
        }
        InsertRequest::Raw {
            memory_id,
            tag,
            text,
            embedding_json,
        } => {
            if text.trim().is_empty() {
                bail!("Text is required for raw insert.");
            }
            if embedding_json.trim().is_empty() {
                bail!("Embedding JSON is required for raw insert.");
            }
            Ok(ValidatedInsertRequest::Raw {
                memory_id: memory_id.clone(),
                tag: validate_required_tag(tag, "Tag is required for raw insert.")?,
                text: text.clone(),
                embedding: parse_embedding_json(embedding_json)?,
            })
        }
        InsertRequest::Pdf {
            memory_id,
            tag,
            file_path,
        } => {
            if file_path.as_os_str().is_empty() {
                bail!("File path is required for PDF insert.");
            }
            validate_file_path(file_path)?;

            Ok(ValidatedInsertRequest::Pdf {
                memory_id: memory_id.clone(),
                tag: resolved_tag(tag, None, Some(file_path.as_path()))?,
                file_path: file_path.clone(),
            })
        }
    }
}

async fn prepare_insert_request(
    request: &ValidatedInsertRequest,
) -> Result<Vec<PreparedInsertItem>> {
    match request {
        ValidatedInsertRequest::Normal {
            tag,
            text,
            file_path,
            ..
        } => {
            let content = load_normal_content(text.as_ref(), file_path.as_ref())?;
            prepare_chunked_insert(tag, &content).await
        }
        ValidatedInsertRequest::Raw {
            tag,
            text,
            embedding,
            ..
        } => Ok(vec![PreparedInsertItem {
            embedding: embedding.clone(),
            payload: payload_for(tag, text),
        }]),
        ValidatedInsertRequest::Pdf { tag, file_path, .. } => {
            let markdown = pdf_to_markdown(file_path).map_err(|error| {
                anyhow::anyhow!(
                    "Failed to convert PDF {} to markdown: {error}",
                    file_path.display()
                )
            })?;
            prepare_chunked_insert(tag, &markdown).await
        }
    }
}

async fn prepare_chunked_insert(tag: &str, markdown: &str) -> Result<Vec<PreparedInsertItem>> {
    let chunks = late_chunking(markdown).await?;
    Ok(chunks
        .into_iter()
        .map(|chunk| PreparedInsertItem {
            embedding: chunk.embedding,
            payload: payload_for(tag, &chunk.sentence),
        })
        .collect())
}

fn load_normal_content(text: Option<&String>, file_path: Option<&PathBuf>) -> Result<String> {
    let inline_text = text.filter(|value| !value.trim().is_empty());
    if inline_text.is_some() && file_path.is_some() {
        bail!("Provide either text or file path, not both.");
    }

    if let Some(content) = inline_text {
        return Ok(content.to_string());
    }

    if let Some(path) = file_path {
        return fs::read_to_string(path)
            .with_context(|| format!("Failed to read --file-path {}", path.display()));
    }

    bail!("Provide text or file path for normal insert.")
}

pub fn preview_file_insert_tag(tag: &str, file_path: Option<&Path>) -> Option<String> {
    resolved_tag(tag, None, file_path).ok()
}

fn validate_memory_id_required(memory_id: &str) -> Result<()> {
    if memory_id.trim().is_empty() {
        bail!("Memory ID is required.");
    }
    Ok(())
}

fn validate_required_tag(tag: &str, message: &str) -> Result<String> {
    let normalized = tag.trim();
    if normalized.is_empty() {
        bail!("{message}");
    }
    Ok(normalized.to_string())
}

fn resolved_tag(tag: &str, text: Option<&str>, file_path: Option<&Path>) -> Result<String> {
    let normalized = tag.trim();
    if !normalized.is_empty() {
        return Ok(normalized.to_string());
    }
    if text.is_some() {
        bail!("Tag is required for inline text insert.");
    }
    let derived = file_path.and_then(derive_file_tag);
    derived.ok_or_else(|| anyhow::anyhow!("Tag is required for file insert."))
}

fn validate_memory_id(memory_id: &str) -> Result<()> {
    Principal::from_text(memory_id)
        .with_context(|| format!("Memory ID must be a valid principal: {memory_id}"))?;
    Ok(())
}

fn validate_file_path(path: &PathBuf) -> Result<()> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            bail!("File path does not exist: {}", path.display())
        }
        Err(_) => bail!("Could not access file path: {}", path.display()),
    };
    if !metadata.is_file() {
        bail!("File path is not a file: {}", path.display());
    }
    if File::open(path).is_err() {
        bail!("File path is not readable: {}", path.display());
    }
    Ok(())
}

fn validate_text_file_path(path: &PathBuf) -> Result<()> {
    validate_file_path(path)?;
    fs::read_to_string(path)
        .map(|_| ())
        .with_context(|| format!("File path is not valid UTF-8 text: {}", path.display()))
}

fn normalized_optional_text(text: Option<String>) -> Option<String> {
    text.filter(|value| !value.trim().is_empty())
}

fn normalized_optional_path(path: Option<PathBuf>) -> Option<PathBuf> {
    path.filter(|candidate| !candidate.as_os_str().is_empty())
}

fn payload_for(tag: &str, sentence: &str) -> String {
    json!({ "tag": tag, "sentence": sentence }).to_string()
}

impl InsertRequest {
    pub fn memory_id(&self) -> &str {
        match self {
            Self::Normal { memory_id, .. }
            | Self::Raw { memory_id, .. }
            | Self::Pdf { memory_id, .. } => memory_id.as_str(),
        }
    }

    pub fn tag(&self) -> &str {
        match self {
            Self::Normal { tag, .. } | Self::Raw { tag, .. } | Self::Pdf { tag, .. } => {
                tag.as_str()
            }
        }
    }
}

impl ValidatedInsertRequest {
    fn mode(&self) -> InsertMode {
        match self {
            Self::Normal { .. } => InsertMode::Normal,
            Self::Raw { .. } => InsertMode::Raw,
            Self::Pdf { .. } => InsertMode::Pdf,
        }
    }

    fn memory_id(&self) -> &str {
        match self {
            Self::Normal { memory_id, .. }
            | Self::Raw { memory_id, .. }
            | Self::Pdf { memory_id, .. } => memory_id.as_str(),
        }
    }

    fn tag(&self) -> &str {
        match self {
            Self::Normal { tag, .. } | Self::Raw { tag, .. } | Self::Pdf { tag, .. } => {
                tag.as_str()
            }
        }
    }

    fn source_name(&self) -> Option<String> {
        match self {
            Self::Normal {
                file_path: Some(file_path),
                ..
            }
            | Self::Pdf { file_path, .. } => Some(display_source_name(file_path)),
            Self::Normal {
                file_path: None, ..
            }
            | Self::Raw { .. } => None,
        }
    }
}

fn display_source_name(file_path: &Path) -> String {
    file_path
        .file_name()
        .unwrap_or(file_path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env, fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    fn write_temp_markdown_file(contents: &str) -> PathBuf {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let path = env::temp_dir().join(format!("kinic-insert-test-{unique_suffix}.md"));
        fs::write(&path, contents).expect("temporary markdown file should be writable");
        path
    }

    fn write_temp_bytes_file(extension: &str, contents: &[u8]) -> PathBuf {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let path = env::temp_dir().join(format!("kinic-insert-test-{unique_suffix}.{extension}"));
        fs::write(&path, contents).expect("temporary bytes file should be writable");
        path
    }

    fn write_workspace_markdown_file(relative_path: &str, contents: &str) -> PathBuf {
        let path = env::current_dir()
            .expect("workspace current dir should resolve")
            .join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("workspace test directory should be creatable");
        }
        fs::write(&path, contents).expect("workspace markdown file should be writable");
        path
    }

    #[test]
    fn parse_embedding_json_rejects_empty_arrays() {
        let err = parse_embedding_json("[]").unwrap_err();

        assert!(err.to_string().contains("cannot be empty"));
    }

    #[test]
    fn parse_embedding_json_parses_float_arrays() {
        let embedding = parse_embedding_json("[0.1, 0.2]").unwrap();

        assert_eq!(embedding, vec![0.1, 0.2]);
    }

    #[test]
    fn load_normal_content_rejects_inline_text_and_file_path_together() {
        let err = load_normal_content(
            Some(&"  inline text  ".to_string()),
            Some(&PathBuf::from("/tmp/unused.md")),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Provide either text or file path, not both."
        );
    }

    #[test]
    fn payload_for_wraps_tag_and_sentence_as_json() {
        let payload = payload_for("docs", "hello");

        assert_eq!(payload, "{\"sentence\":\"hello\",\"tag\":\"docs\"}");
    }

    #[test]
    fn prepared_items_match_expected_dimension() {
        ensure_prepared_items_match_memory(
            "aaaaa-aa",
            &[PreparedInsertItem {
                embedding: vec![0.1, 0.2],
                payload: "{}".to_string(),
            }],
            2,
        )
        .unwrap();
    }

    #[test]
    fn prepared_items_reject_dimension_mismatch() {
        let error = ensure_prepared_items_match_memory(
            "aaaaa-aa",
            &[PreparedInsertItem {
                embedding: vec![0.1, 0.2],
                payload: "{}".to_string(),
            }],
            3,
        )
        .unwrap_err();

        assert!(error.to_string().contains("Embedding dimension mismatch"));
    }

    #[test]
    fn raw_insert_request_fails_fast_on_dimension_mismatch() {
        let request = ValidatedInsertRequest::Raw {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            text: "payload".to_string(),
            embedding: vec![0.1, 0.2],
        };

        let error = ensure_request_matches_memory_dim(&request, "aaaaa-aa", 3).unwrap_err();

        assert!(error.to_string().contains("Embedding dimension mismatch"));
    }

    #[test]
    fn normal_insert_request_uses_selected_backend_dimension_for_fail_fast_checks() {
        let request = ValidatedInsertRequest::Normal {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            text: Some("payload".to_string()),
            file_path: None,
        };

        let error = ensure_request_matches_memory_dim(&request, "aaaaa-aa", 3).unwrap_err();

        assert!(error.to_string().contains("Embedding dimension mismatch"));
    }

    #[test]
    fn validated_insert_request_source_name_uses_file_name() {
        let request = ValidatedInsertRequest::Normal {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            text: None,
            file_path: Some(PathBuf::from("/tmp/nested/doc.md")),
        };

        assert_eq!(request.source_name(), Some("doc.md".to_string()));
    }

    #[test]
    fn validated_insert_request_source_name_is_none_for_inline_text() {
        let request = ValidatedInsertRequest::Normal {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            text: Some("payload".to_string()),
            file_path: None,
        };

        assert_eq!(request.source_name(), None);
    }

    #[test]
    fn validate_insert_request_fields_rejects_empty_normal_payload() {
        let err = validate_insert_request_fields(&InsertRequest::Normal {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            text: None,
            file_path: None,
        })
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Provide text or file path for normal insert."
        );
    }

    #[test]
    fn validate_insert_request_fields_rejects_blank_raw_embedding() {
        let err = validate_insert_request_fields(&InsertRequest::Raw {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            text: "payload".to_string(),
            embedding_json: "   ".to_string(),
        })
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Embedding JSON is required for raw insert."
        );
    }

    #[test]
    fn validate_insert_request_fields_rejects_blank_raw_text() {
        let err = validate_insert_request_fields(&InsertRequest::Raw {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            text: "   ".to_string(),
            embedding_json: "[0.1]".to_string(),
        })
        .unwrap_err();

        assert_eq!(err.to_string(), "Text is required for raw insert.");
    }

    #[test]
    fn validate_insert_request_fields_rejects_missing_pdf_path() {
        let err = validate_insert_request_fields(&InsertRequest::Pdf {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            file_path: PathBuf::new(),
        })
        .unwrap_err();

        assert_eq!(err.to_string(), "File path is required for PDF insert.");
    }

    #[test]
    fn validate_insert_request_fields_rejects_whitespace_only_inline_text_without_file() {
        let err = validate_insert_request_fields(&InsertRequest::Normal {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            text: Some("   ".to_string()),
            file_path: None,
        })
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Provide text or file path for normal insert."
        );
    }

    #[test]
    fn validate_insert_request_fields_accepts_non_empty_file_path_without_reading_file() {
        validate_insert_request_fields(&InsertRequest::Normal {
            memory_id: "aaaaa-aa".to_string(),
            tag: String::new(),
            text: Some("   ".to_string()),
            file_path: Some(PathBuf::from("/path/that/does/not/need/to/exist.md")),
        })
        .unwrap();
    }

    #[test]
    fn validate_insert_request_fields_rejects_inline_text_and_file_path_together() {
        let err = validate_insert_request_fields(&InsertRequest::Normal {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            text: Some("payload".to_string()),
            file_path: Some(PathBuf::from("/tmp/doc.md")),
        })
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Provide either text or file path, not both."
        );
    }

    #[test]
    fn validate_insert_request_fields_rejects_blank_inline_text_tag() {
        let err = validate_insert_request_fields(&InsertRequest::Normal {
            memory_id: "aaaaa-aa".to_string(),
            tag: "   ".to_string(),
            text: Some("payload".to_string()),
            file_path: None,
        })
        .unwrap_err();

        assert_eq!(err.to_string(), "Tag is required for inline text insert.");
    }

    #[test]
    fn validate_insert_request_for_submit_rejects_invalid_memory_id() {
        let err = validate_insert_request_for_submit(&InsertRequest::Normal {
            memory_id: "not-a-principal".to_string(),
            tag: "docs".to_string(),
            text: Some("payload".to_string()),
            file_path: None,
        })
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("Memory ID must be a valid principal")
        );
    }

    #[test]
    fn validate_insert_request_for_submit_rejects_invalid_raw_embedding_json() {
        let err = validate_insert_request_for_submit(&InsertRequest::Raw {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            text: "payload".to_string(),
            embedding_json: "not-json".to_string(),
        })
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("Embedding must be a JSON array of floats")
        );
    }

    #[test]
    fn validate_insert_request_for_submit_rejects_missing_normal_file_path() {
        let err = validate_insert_request_for_submit(&InsertRequest::Normal {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            text: Some("   ".to_string()),
            file_path: Some(PathBuf::from("/path/that/does/not/need/to/exist.md")),
        })
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("File path does not exist: /path/that/does/not/need/to/exist.md")
        );
    }

    #[test]
    fn validate_insert_request_for_submit_rejects_non_utf8_normal_file_path() {
        let path = write_temp_bytes_file("md", &[0xff, 0xfe, 0xfd]);

        let err = validate_insert_request_for_submit(&InsertRequest::Normal {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            text: Some("   ".to_string()),
            file_path: Some(path.clone()),
        })
        .unwrap_err();

        assert!(err.to_string().contains(&format!(
            "File path is not valid UTF-8 text: {}",
            path.display()
        )));

        fs::remove_file(path).expect("temporary bytes file should be removable");
    }

    #[test]
    fn validate_insert_request_for_submit_rejects_inline_text_with_file_path() {
        let err = validate_insert_request_for_submit(&InsertRequest::Normal {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            text: Some("payload".to_string()),
            file_path: Some(PathBuf::from("/path/that/does/not/need/to/exist.md")),
        })
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Provide either text or file path, not both."
        );
    }

    #[test]
    fn validate_insert_request_for_submit_rejects_missing_pdf_file_path() {
        let err = validate_insert_request_for_submit(&InsertRequest::Pdf {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            file_path: PathBuf::from("/path/that/does/not/need/to/exist.pdf"),
        })
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("File path does not exist: /path/that/does/not/need/to/exist.pdf")
        );
    }

    #[test]
    fn validate_insert_request_for_submit_rejects_directory_paths() {
        let dir = env::temp_dir();
        let err = validate_insert_request_for_submit(&InsertRequest::Pdf {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            file_path: dir.clone(),
        })
        .unwrap_err();

        assert!(
            err.to_string()
                .contains(&format!("File path is not a file: {}", dir.display()))
        );
    }

    #[test]
    fn validate_insert_request_for_submit_reuses_parsed_raw_embedding() {
        let validated = validate_and_transform_insert_request(&InsertRequest::Raw {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            text: "payload".to_string(),
            embedding_json: "[0.1, 0.2]".to_string(),
        })
        .unwrap();

        assert!(matches!(
            validated,
            ValidatedInsertRequest::Raw { embedding, .. } if embedding == vec![0.1, 0.2]
        ));
    }

    #[test]
    fn validate_insert_request_for_submit_derives_tag_from_file_path() {
        let path =
            write_workspace_markdown_file("target/kinic-insert-tests/docs/spec/api.md", "# title");
        let validated = validate_and_transform_insert_request(&InsertRequest::Normal {
            memory_id: "aaaaa-aa".to_string(),
            tag: String::new(),
            text: Some("   ".to_string()),
            file_path: Some(path.clone()),
        })
        .unwrap();

        assert!(matches!(
            validated,
            ValidatedInsertRequest::Normal { tag, .. }
                if tag.strip_prefix("api-").is_some_and(|suffix| suffix.len() == 16)
        ));
        fs::remove_file(path).expect("workspace markdown file should be removable");
    }

    #[test]
    fn preview_file_insert_tag_preserves_existing_tag() {
        let tag = preview_file_insert_tag("docs", Some(Path::new("nested/spec.md")));

        assert_eq!(tag.as_deref(), Some("docs"));
    }

    #[test]
    fn preview_file_insert_tag_returns_none_when_tag_and_file_path_are_missing() {
        let tag = preview_file_insert_tag("", None);

        assert_eq!(tag, None);
    }

    #[test]
    fn load_normal_content_uses_file_when_inline_text_is_whitespace_only() {
        let path = write_temp_markdown_file("# title");

        let content = load_normal_content(Some(&"   ".to_string()), Some(&path)).unwrap();

        assert_eq!(content, "# title");

        fs::remove_file(path).expect("temporary markdown file should be removable");
    }

    #[test]
    fn validate_insert_request_fields_accepts_non_empty_pdf_path_without_conversion() {
        validate_insert_request_fields(&InsertRequest::Pdf {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            file_path: PathBuf::from("/path/that/does/not/need/to/exist.pdf"),
        })
        .unwrap();
    }
}
