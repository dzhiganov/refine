//! Small text helpers: word diff, word count, locating fragments.

use similar::{Algorithm, ChangeTag, TextDiff};

use crate::model::{DiffOp, DiffPart, TextSpan};

/// Word-level diff from `before` to `after`, with consecutive runs of the same kind merged.
pub fn word_diff(before: &str, after: &str) -> Vec<DiffPart> {
    let diff = TextDiff::configure().algorithm(Algorithm::Patience).diff_unicode_words(before, after);
    let mut parts: Vec<DiffPart> = Vec::new();
    for change in diff.iter_all_changes() {
        let op = match change.tag() {
            ChangeTag::Equal => DiffOp::Equal,
            ChangeTag::Insert => DiffOp::Insert,
            ChangeTag::Delete => DiffOp::Delete,
        };
        match parts.last_mut() {
            Some(last) if last.op == op => last.text.push_str(change.value()),
            _ => parts.push(DiffPart { op, text: change.value().to_string() }),
        }
    }
    parts
}

/// Words that contain at least one letter or digit.
pub fn word_count(text: &str) -> u32 {
    text.split_whitespace().filter(|w| w.chars().any(char::is_alphanumeric)).count() as u32
}

fn utf16_len(s: &str) -> u32 {
    s.encode_utf16().count() as u32
}

/// Finds `needle` in `haystack`, preferring the first match at or after byte `from` (so repeated
/// fragments map to successive occurrences). Returns the byte range.
pub fn find_from(haystack: &str, needle: &str, from: usize) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return None;
    }
    let from = (0..=from.min(haystack.len())).rev().find(|i| haystack.is_char_boundary(*i)).unwrap_or(0);
    haystack[from..]
        .find(needle)
        .map(|i| i + from)
        .or_else(|| haystack.find(needle))
        .map(|start| (start, start + needle.len()))
}

/// Like [`find_from`], but prefers a match that starts and ends on word boundaries, so "an"
/// doesn't land inside "and".
pub fn find_word_from(haystack: &str, needle: &str, from: usize) -> Option<(usize, usize)> {
    let bounded = |start: usize| {
        let end = start + needle.len();
        let before_ok = !needle.starts_with(char::is_alphanumeric)
            || !haystack[..start].chars().next_back().is_some_and(char::is_alphanumeric);
        let after_ok = !needle.ends_with(char::is_alphanumeric)
            || !haystack[end..].chars().next().is_some_and(char::is_alphanumeric);
        before_ok && after_ok
    };
    if needle.is_empty() {
        return None;
    }
    let from = (0..=from.min(haystack.len())).rev().find(|i| haystack.is_char_boundary(*i)).unwrap_or(0);
    let after = haystack[from..].match_indices(needle).map(|(i, _)| i + from).find(|i| bounded(*i));
    let anywhere = || haystack.match_indices(needle).map(|(i, _)| i).find(|i| bounded(*i));
    after.or_else(anywhere).map(|s| (s, s + needle.len())).or_else(|| find_from(haystack, needle, from))
}

/// Converts a byte range of `text` to UTF-16 offsets for the UI.
pub fn to_span(text: &str, (start, end): (usize, usize)) -> TextSpan {
    TextSpan { start: utf16_len(&text[..start]), end: utf16_len(&text[..end]) }
}

/// Lowercased, whitespace-collapsed form for matching the same fragment across texts.
pub fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_marks_changed_words() {
        let d = word_diff("He go to school.", "He goes to school.");
        let changed: Vec<_> = d.iter().filter(|p| p.op != DiffOp::Equal).map(|p| (p.op, p.text.as_str())).collect();
        assert_eq!(changed, vec![(DiffOp::Delete, "go"), (DiffOp::Insert, "goes")]);
        let rebuilt: String = d.iter().filter(|p| p.op != DiffOp::Delete).map(|p| p.text.as_str()).collect();
        assert_eq!(rebuilt, "He goes to school.");
    }

    #[test]
    fn spans_are_utf16() {
        let text = "Grüße 👋 an alle";
        let range = find_from(text, "an", 0).unwrap();
        // "Grüße " = 6 units, emoji = 2 units, space = 1.
        assert_eq!(to_span(text, range), TextSpan { start: 9, end: 11 });
    }

    #[test]
    fn repeated_fragments_map_in_order() {
        let text = "a cat and a dog";
        let first = find_from(text, "a", 0).unwrap();
        let second = find_from(text, "a ", first.1).unwrap();
        assert_eq!(first.0, 0);
        assert_eq!(second.0, 10);
    }

    #[test]
    fn prefers_whole_words() {
        let text = "and then an apple";
        assert_eq!(find_word_from(text, "an", 0), Some((9, 11)));
        // Falls back to a partial match when there is no whole-word one.
        assert_eq!(find_word_from("banana", "an", 0), Some((1, 3)));
    }

    #[test]
    fn counts_words_not_symbols() {
        assert_eq!(word_count("Hi — this is  a test :) 42"), 6);
    }
}
