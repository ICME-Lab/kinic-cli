//! TUI chat similarity helpers.
//! Where: shared by chat retrieval reranking and diversity scoring.
//! What: normalizes text and computes token/ngram-based similarity.
//! Why: keep retrieval selection code focused on ranking flow and caps.

const DIVERSITY_TEXT_LIMIT_CHARS: usize = 480;

pub(crate) fn content_similarity(left: &str, right: &str) -> f32 {
    let left_window = similarity_window(left);
    let right_window = similarity_window(right);
    let token_similarity = token_jaccard(left_window.as_str(), right_window.as_str());
    let ngram_similarity = ngram_jaccard(left_window.as_str(), right_window.as_str());
    (0.35 * token_similarity) + (0.65 * ngram_similarity)
}

pub(crate) fn has_query_fragment_overlap(
    payload: &str,
    latest_fragments: &[String],
    rewritten_fragments: &[String],
) -> bool {
    let payload_lower = payload.to_lowercase();
    latest_fragments
        .iter()
        .chain(rewritten_fragments.iter())
        .any(|fragment| payload_lower.contains(fragment.as_str()))
}

pub(crate) fn has_token_overlap(left: &[String], right: &[String]) -> bool {
    left.iter()
        .any(|token| right.iter().any(|candidate| candidate == token))
}

pub(crate) fn query_fragments(text: &str) -> Vec<String> {
    let normalized = normalized_text(text);
    let mut fragments = Vec::new();
    for token in normalized.split_whitespace() {
        let char_count = token.chars().count();
        let is_ascii_token = token
            .chars()
            .all(|character| character.is_ascii_alphanumeric());
        if char_count >= 3 || (!is_ascii_token && char_count >= 2) {
            fragments.push(token.to_string());
        }
    }
    fragments.sort();
    fragments.dedup();
    fragments
}

pub(crate) fn character_ngrams(text: &str) -> Vec<String> {
    let normalized = normalized_text(text);
    let compact = normalized
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let char_count = compact.chars().count();
    if char_count == 0 {
        return Vec::new();
    }
    let gram_size = if char_count >= 3 { 3 } else { 2 };
    let chars = compact.chars().collect::<Vec<_>>();
    if chars.len() < gram_size {
        return vec![compact];
    }
    let mut grams = chars
        .windows(gram_size)
        .map(|window| window.iter().collect::<String>())
        .collect::<Vec<_>>();
    grams.sort();
    grams.dedup();
    grams
}

pub(crate) fn tokenize(text: &str) -> Vec<String> {
    normalized_text(text)
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn normalized_text(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || character.is_whitespace() {
                character
            } else {
                ' '
            }
        })
        .collect()
}

fn token_jaccard(left: &str, right: &str) -> f32 {
    let left_tokens = tokenize(left);
    let right_tokens = tokenize(right);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 0.0;
    }
    let intersection = left_tokens
        .iter()
        .filter(|token| right_tokens.iter().any(|candidate| candidate == *token))
        .count();
    let union = left_tokens.len() + right_tokens.len() - intersection;
    if union == 0 {
        0.0
    } else {
        let intersection_u16 = u16::try_from(intersection).unwrap_or(u16::MAX);
        let union_u16 = u16::try_from(union).unwrap_or(u16::MAX);
        f32::from(intersection_u16) / f32::from(union_u16)
    }
}

fn ngram_jaccard(left: &str, right: &str) -> f32 {
    let left_grams = character_ngrams(left);
    let right_grams = character_ngrams(right);
    if left_grams.is_empty() || right_grams.is_empty() {
        return 0.0;
    }
    let intersection = left_grams
        .iter()
        .filter(|gram| right_grams.iter().any(|candidate| candidate == *gram))
        .count();
    let union = left_grams.len() + right_grams.len() - intersection;
    if union == 0 {
        return 0.0;
    }
    let intersection_u16 = u16::try_from(intersection).unwrap_or(u16::MAX);
    let union_u16 = u16::try_from(union).unwrap_or(u16::MAX);
    f32::from(intersection_u16) / f32::from(union_u16)
}

fn similarity_window(text: &str) -> String {
    normalized_text(text)
        .chars()
        .take(DIVERSITY_TEXT_LIMIT_CHARS)
        .collect()
}
