//! Shared prompt helpers.
//! Where: reused by CLI ask-ai and the embedded TUI chat prompt builders.
//! What: escapes XML-like prompt payloads and performs lightweight language detection.
//! Why: keep prompt construction safe and consistent without adding dependencies.

pub(crate) fn escape_xml(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

pub(crate) fn detect_language<I>(latest_query: &str, history: I) -> &'static str
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let history_messages = history
        .into_iter()
        .map(|message| message.as_ref().to_string())
        .collect::<Vec<_>>();
    detect_language_from_text(latest_query)
        .or_else(|| {
            history_messages
                .iter()
                .rev()
                .find_map(|message| detect_language_from_text(message.as_str()))
        })
        .unwrap_or("en")
}

fn detect_language_from_text(text: &str) -> Option<&'static str> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Track script families first, then choose once we have the whole input.
    // CJK unified ideographs are shared by Japanese and Chinese, so we must
    // not force ideograph-only text to Japanese. Kana still wins for Japanese.
    let mut saw_japanese_kana = false;
    let mut saw_cjk_ideograph = false;
    let mut saw_korean = false;
    let mut saw_latin = false;
    for ch in trimmed.chars() {
        if ('\u{3040}'..='\u{30ff}').contains(&ch) || ('\u{31f0}'..='\u{31ff}').contains(&ch) {
            saw_japanese_kana = true;
            continue;
        }
        if ('\u{4e00}'..='\u{9fff}').contains(&ch) {
            saw_cjk_ideograph = true;
            continue;
        }
        if ('\u{ac00}'..='\u{d7af}').contains(&ch) || ('\u{1100}'..='\u{11ff}').contains(&ch) {
            saw_korean = true;
            continue;
        }
        if ch.is_ascii_alphabetic() || ('\u{00c0}'..='\u{024f}').contains(&ch) {
            saw_latin = true;
        }
    }

    if saw_japanese_kana {
        return Some("ja");
    }
    if saw_korean {
        return Some("ko");
    }
    if saw_cjk_ideograph {
        return Some("zh");
    }

    saw_latin.then_some("en")
}

#[cfg(test)]
mod tests {
    use super::{detect_language, escape_xml};

    #[test]
    fn escape_xml_escapes_reserved_characters() {
        assert_eq!(
            escape_xml("<doc attr=\"x\">Tom & 'Jerry'</doc>"),
            "&lt;doc attr=&quot;x&quot;&gt;Tom &amp; &apos;Jerry&apos;&lt;/doc&gt;"
        );
    }

    #[test]
    fn detect_language_prefers_latest_query_then_history() {
        assert_eq!(detect_language("これは何?", ["hello"]), "ja");
        assert_eq!(detect_language("", ["ignored", "안녕하세요"]), "ko");
        assert_eq!(detect_language("", ["hello"]), "en");
    }

    #[test]
    fn detect_language_treats_ideograph_only_queries_as_chinese_fallback() {
        assert_eq!(detect_language("概要", ["hello"]), "zh");
        assert_eq!(detect_language("搜索结果", ["hello"]), "zh");
    }

    #[test]
    fn detect_language_uses_kana_as_a_stronger_signal_than_kanji() {
        assert_eq!(detect_language("検索結果を見せて", ["hello"]), "ja");
    }
}
