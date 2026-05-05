//! Where: local chunk preparation for insert and PDF ingestion.
//! What: splits markdown/text into bounded chunks before passage embedding.
//! Why: local backends need an in-process late-chunking path without changing insert callers.

use crate::embedding_config::ChunkingConfig;

pub(crate) fn chunk_markdown(markdown: &str, config: &ChunkingConfig) -> Vec<String> {
    let normalized = markdown.replace("\r\n", "\n");
    let blocks = normalized
        .split("\n\n")
        .map(str::trim)
        .filter(|block| !block.is_empty())
        .flat_map(|block| chunk_block(block, config))
        .collect::<Vec<_>>();
    if blocks.is_empty() {
        return Vec::new();
    }
    blocks
}

fn chunk_block(block: &str, config: &ChunkingConfig) -> Vec<String> {
    if block.chars().count() <= config.hard_limit {
        return vec![block.to_string()];
    }

    let sentences = split_sentences(block);
    if sentences.len() <= 1 {
        return split_long_text(block, config);
    }

    let mut chunks = Vec::new();
    let mut current = String::new();
    for sentence in sentences {
        if sentence.chars().count() > config.hard_limit {
            if !current.is_empty() {
                chunks.push(current.trim().to_string());
                current.clear();
            }
            chunks.extend(split_long_text(&sentence, config));
            continue;
        }

        let separator = if current.is_empty() { "" } else { " " };
        if current.chars().count() + separator.len() + sentence.chars().count() > config.soft_limit
            && !current.is_empty()
        {
            chunks.push(current.trim().to_string());
            current.clear();
        }

        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(sentence.trim());
    }

    if !current.is_empty() {
        chunks.push(current.trim().to_string());
    }
    chunks
}

fn split_sentences(block: &str) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut current = String::new();
    for ch in block.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?' | '\n') {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                pieces.push(trimmed.to_string());
            }
            current.clear();
        }
    }
    if !current.trim().is_empty() {
        pieces.push(current.trim().to_string());
    }
    pieces
}

fn split_long_text(text: &str, config: &ChunkingConfig) -> Vec<String> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut chunks = Vec::new();
    let mut start = 0usize;
    while start < chars.len() {
        let end = usize::min(start + config.hard_limit, chars.len());
        let slice = chars[start..end]
            .iter()
            .collect::<String>()
            .trim()
            .to_string();
        if !slice.is_empty() {
            chunks.push(slice);
        }
        if end == chars.len() {
            break;
        }
        start = end.saturating_sub(config.overlap);
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ChunkingConfig {
        ChunkingConfig {
            soft_limit: 16,
            hard_limit: 24,
            overlap: 4,
        }
    }

    #[test]
    fn chunk_markdown_splits_paragraphs_and_long_blocks() {
        let chunks = chunk_markdown(
            "First sentence. Second sentence.\n\nThis block is definitely long enough to split.",
            &config(),
        );

        assert!(chunks.len() >= 3);
        assert!(chunks.iter().all(|chunk| !chunk.trim().is_empty()));
    }

    #[test]
    fn chunk_markdown_ignores_empty_input() {
        assert!(chunk_markdown(" \n\n ", &config()).is_empty());
    }
}
