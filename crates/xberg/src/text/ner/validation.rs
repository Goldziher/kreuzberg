//! Contextual validation of model-predicted entities.
//!
//! The default GLiNER model labels identifiers such as `SYN-004` or `INV-2026-0042` as `email`
//! at confidence 0.5-0.7 (xberg-io/xberg#2133). A fixed label rename or a plain `@` filter was
//! measured to lose genuine addresses, so the guard here only drops an `email` span that has no
//! address cue anywhere near it and keeps every span it cannot judge.

use crate::types::entity::{Entity, EntityCategory};

/// Words either side of the span's own token that may carry the cue of an obfuscated address.
const CUE_CONTEXT_WORDS: usize = 3;

const BRACKETED_AT: &[&str] = &["[at]", "(at)", "{at}", "<at>"];
const DOT_WORDS: &[&str] = &["dot", "[dot]", "(dot)", "{dot}", "<dot>"];

/// Drop `email` entities that have no address cue in or around their span.
///
/// A cue is an `@` in the whitespace-delimited token that holds the span or in one of the
/// [`CUE_CONTEXT_WORDS`] words on either side, a bracketed at-sign (`[at]`), or a bare `at`
/// together with a `dot` word (`name at host dot com`). A span whose offsets do not index `text`
/// is kept, so the guard can only remove what it has examined. ~keep
#[cfg_attr(alef, alef(skip))]
pub fn drop_unanchored_emails(text: &str, entities: Vec<Entity>) -> Vec<Entity> {
    if !entities.iter().any(|entity| entity.category == EntityCategory::Email) {
        return entities;
    }
    let words = word_spans(text);
    entities
        .into_iter()
        .filter(|entity| {
            entity.category != EntityCategory::Email
                || has_address_cue(text, &words, entity.start as usize, entity.end as usize).unwrap_or(true)
        })
        .collect()
}

fn has_address_cue(text: &str, words: &[(usize, usize)], start: usize, end: usize) -> Option<bool> {
    text.get(start..end)?;
    let first = words.iter().position(|&(_, word_end)| word_end > start)?;
    let last = words.iter().rposition(|&(word_start, _)| word_start < end)?;
    let from = first.saturating_sub(CUE_CONTEXT_WORDS);
    let to = (last + CUE_CONTEXT_WORDS + 1).min(words.len());
    let window: Vec<String> = words[from..to]
        .iter()
        .map(|&(word_start, word_end)| {
            text[word_start..word_end]
                .trim_matches(|c: char| matches!(c, '.' | ',' | ';' | ':' | '!' | '?'))
                .to_lowercase()
        })
        .collect();

    let at_sign = window.iter().any(|word| word.contains('@'));
    let bracketed_at = window
        .iter()
        .any(|word| BRACKETED_AT.iter().any(|at| word.contains(at)));
    let spoken_at =
        window.iter().any(|word| word == "at") && window.iter().any(|word| DOT_WORDS.contains(&word.as_str()));
    Some(at_sign || bracketed_at || spoken_at)
}

/// Byte ranges of the whitespace-delimited words of `text`.
fn word_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut word_start: Option<usize> = None;
    for (index, c) in text.char_indices() {
        match (c.is_whitespace(), word_start) {
            (true, Some(begin)) => {
                spans.push((begin, index));
                word_start = None;
            }
            (false, None) => word_start = Some(index),
            _ => {}
        }
    }
    if let Some(begin) = word_start {
        spans.push((begin, text.len()));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    fn email(text: &str, span: &str) -> Entity {
        let start = text.find(span).expect("span in text");
        Entity {
            category: EntityCategory::Email,
            text: span.to_string(),
            start: start as u32,
            end: (start + span.len()) as u32,
            confidence: Some(0.5),
        }
    }

    fn kept(text: &str, span: &str) -> bool {
        !drop_unanchored_emails(text, vec![email(text, span)]).is_empty()
    }

    #[test]
    fn invoice_identifiers_labelled_email_are_dropped() {
        for (text, span) in [
            ("Invoice number SYN-004. Total amount due is USD 125.50.", "SYN-004"),
            ("Invoice number INV-2026-0042 is due.", "INV-2026-0042"),
            ("PO-778812 was approved.", "PO-778812"),
            ("Account ID usr_8812 is active.", "usr_8812"),
        ] {
            assert!(!kept(text, span), "{span} has no address cue and must be dropped");
        }
    }

    #[test]
    fn spans_with_an_address_cue_are_kept() {
        for (text, span) in [
            ("Contact morgan.reed@example.com.", "example.com"),
            ("Contact morgan.reed@example.com.", "morgan.reed@example.com"),
            ("Orders go to SALES@EXAMPLE.COM only.", "EXAMPLE.COM"),
            (
                "Write to john at example dot com for details.",
                "john at example dot com",
            ),
            ("Write to john [at] example [dot] com now.", "john [at] example"),
            ("Contact: first.last(at)company.com for help.", "company.com"),
            ("Schreiben Sie an jürgen@beispiel.de bitte.", "beispiel"),
        ] {
            assert!(kept(text, span), "{span} is part of an address and must be kept");
        }
    }

    #[test]
    fn a_bare_at_without_a_dot_word_is_not_a_cue() {
        let text = "Meet at INV-2026-0042 tomorrow.";
        assert!(!kept(text, "INV-2026-0042"));
    }

    #[test]
    fn other_categories_and_unjudgeable_spans_pass_through() {
        let text = "Invoice number SYN-004.";
        let mut person = email(text, "SYN-004");
        person.category = EntityCategory::Person;
        assert_eq!(drop_unanchored_emails(text, vec![person]).len(), 1);

        let mut outside = email(text, "SYN-004");
        outside.end = 9_999;
        assert_eq!(drop_unanchored_emails(text, vec![outside]).len(), 1);
    }
}
