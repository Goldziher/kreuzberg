//! Word-validity lookup across every language a Tesseract run was configured with.

use std::cell::OnceCell;
use xberg_tesseract::TesseractAPI;

/// Languages beyond the primary one that a lookup may open a dictionary for.
///
/// A configuration with more languages than this (the `all` wildcard resolves to every
/// installed pack) cannot be judged cheaply, so a word the primary dictionary rejects gets no
/// verdict instead of one opened handle per language. ~keep
const MAX_SECONDARY_DICTIONARIES: usize = 4;

struct SecondaryDictionary {
    language: String,
    api: OnceCell<Option<TesseractAPI>>,
}

/// Dictionary membership for a `lang1+lang2` run.
///
/// `TessBaseAPIIsValidWord` consults only the first language's dictionary, so in `rus+eng` every
/// English word reads as not-a-word and the dictionary noise filters drop whole lines of the
/// second language (#2158). A word is valid here when any configured language's dictionary
/// accepts it. The other languages' dictionaries are opened lazily, once, and only when the
/// primary one rejects a word, so a single-language run never pays for them.
pub(super) struct WordDictionary<'a> {
    primary: &'a TesseractAPI,
    tessdata_path: &'a str,
    oem: i32,
    secondary: Vec<SecondaryDictionary>,
    too_many_languages: bool,
}

impl<'a> WordDictionary<'a> {
    pub(super) fn new(primary: &'a TesseractAPI, tessdata_path: &'a str, language: &str, oem: u8) -> Self {
        let mut languages = language.split('+').map(str::trim).filter(|lang| !lang.is_empty());
        let first = languages.next();
        let mut secondary: Vec<SecondaryDictionary> = Vec::new();
        for lang in languages {
            if Some(lang) != first && !secondary.iter().any(|existing| existing.language == lang) {
                secondary.push(SecondaryDictionary {
                    language: lang.to_string(),
                    api: OnceCell::new(),
                });
            }
        }
        let too_many_languages = secondary.len() > MAX_SECONDARY_DICTIONARIES;
        Self {
            primary,
            tessdata_path,
            oem: i32::from(oem),
            secondary,
            too_many_languages,
        }
    }

    /// `Some(true)` valid in some configured language, `Some(false)` invalid in all of them,
    /// `None` no verdict (the lookup failed, or a dictionary needed to rule the word out is
    /// unavailable).
    pub(super) fn is_valid_word(&self, text: &str) -> Option<bool> {
        let primary = match self.primary.is_valid_word(text) {
            Ok(0) => Some(false),
            Ok(_) => Some(true),
            Err(_) => None,
        };
        if self.secondary.is_empty() {
            return primary;
        }
        if self.too_many_languages {
            return if primary == Some(true) { primary } else { None };
        }
        combine_verdicts(
            primary,
            self.secondary.iter().map(|dictionary| self.lookup(dictionary, text)),
        )
    }

    fn lookup(&self, dictionary: &SecondaryDictionary, text: &str) -> Option<bool> {
        let api = dictionary
            .api
            .get_or_init(|| {
                let api = TesseractAPI::new().ok()?;
                api.init_2(self.tessdata_path, &dictionary.language, self.oem).ok()?;
                Some(api)
            })
            .as_ref()?;
        match api.is_valid_word(text) {
            Ok(0) => Some(false),
            Ok(_) => Some(true),
            Err(_) => None,
        }
    }
}

/// A word is valid when any language accepts it and invalid only when every language
/// rejects it; an unanswered lookup keeps "invalid" from being proven. The secondary lookups
/// are lazy, so none runs once the primary dictionary has accepted the word.
fn combine_verdicts(primary: Option<bool>, secondary: impl Iterator<Item = Option<bool>>) -> Option<bool> {
    if primary == Some(true) {
        return Some(true);
    }
    let mut unproven = primary.is_none();
    for verdict in secondary {
        match verdict {
            Some(true) => return Some(true),
            Some(false) => {}
            None => unproven = true,
        }
    }
    if unproven { None } else { Some(false) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn a_word_any_language_accepts_is_valid() {
        assert_eq!(
            combine_verdicts(Some(false), [Some(false), Some(true)].into_iter()),
            Some(true)
        );
        assert_eq!(combine_verdicts(None, [Some(true)].into_iter()), Some(true));
    }

    #[test]
    fn a_word_is_invalid_only_when_every_language_rejects_it() {
        assert_eq!(
            combine_verdicts(Some(false), [Some(false), Some(false)].into_iter()),
            Some(false)
        );
        assert_eq!(combine_verdicts(Some(false), std::iter::empty()), Some(false));
    }

    #[test]
    fn an_unanswered_lookup_cannot_prove_a_word_invalid() {
        assert_eq!(combine_verdicts(Some(false), [None, Some(false)].into_iter()), None);
        assert_eq!(combine_verdicts(None, [Some(false)].into_iter()), None);
    }

    #[test]
    fn secondary_lookups_are_skipped_once_the_primary_accepts() {
        let calls = Cell::new(0);
        let lookups = std::iter::from_fn(|| {
            calls.set(calls.get() + 1);
            Some(Some(false))
        });
        assert_eq!(combine_verdicts(Some(true), lookups), Some(true));
        assert_eq!(calls.get(), 0);
    }
}
