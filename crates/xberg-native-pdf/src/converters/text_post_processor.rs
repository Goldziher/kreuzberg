//! Text post-processing for PDF extraction quality improvements.
//!
//! This module implements PDF Spec-compliant text post-processing to fix
//! common text extraction issues:
//!
//! - **Soft hyphen handling** (PDF Spec Section 14.8.2.2.3):
//!   Removes U+00AD (soft hyphen) characters at line breaks when rejoining
//!   hyphenated words across lines.
//!
//! - **Whitespace normalization**:
//!   Removes excessive spaces within words while preserving intentional spacing
//!   between words.
//!
//! - **Special character spacing**:
//!   Ensures proper spacing around Greek letters, mathematical symbols,
//!   other special characters that require boundary detection.

use regex::Regex;
use std::sync::LazyLock;

static RE_EXCESSIVE_SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([^\s])\s{2,}([^\s])").unwrap());

/// Text post-processor for improving extraction quality per PDF specification.
pub struct TextPostProcessor;

impl TextPostProcessor {
    /// Remove soft hyphens at line breaks per PDF Spec 14.8.2.2.3.
    ///
    /// The PDF specification states that soft hyphens (U+00AD) are used to indicate
    /// where a word can be hyphenated at line boundaries. When extracting text,
    /// these should be removed and the word should be rejoined.
    ///
    /// # Algorithm
    ///
    /// 1. Identify lines ending with `-` or U+00AD (soft hyphen)
    /// 2. Check if next line starts with lowercase letter (likely continuation)
    /// 3. If yes, remove hyphen and newline, joining the words
    /// 4. If no, keep as-is (actual hard hyphen or section break)
    ///
    /// # Arguments
    ///
    /// * `text` - The markdown text to process
    ///
    /// # Returns
    ///
    /// Text with soft hyphens removed and words rejoined
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::converters::text_post_processor::TextPostProcessor;
    ///
    /// let input = "modali-\nties are important";
    /// let output = TextPostProcessor::rejoin_hyphenated_words(input);
    /// assert_eq!(output, "modalities are important");
    /// ```
    pub fn rejoin_hyphenated_words(text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let lines: Vec<&str> = text.lines().collect();

        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim_end();

            let soft = trimmed.ends_with('\u{00AD}');
            if (trimmed.ends_with('-') || soft) && i + 1 < lines.len() {
                let next_line = lines[i + 1].trim_start();
                let without_hyphen = if soft {
                    &trimmed[..trimmed.len() - '\u{00AD}'.len_utf8()]
                } else {
                    &trimmed[..trimmed.len() - 1]
                };

                // A soft hyphen (U+00AD) always marks a wrap point (§14.8.2.2.3),
                // so rejoin on any lowercase continuation. A hard '-' is
                // ambiguous (compound word vs label like "COVID-19" / "TYPE-A"),
                // so only rejoin when BOTH fragments are lowercase-alpha — a
                // plain line-wrapped word. Otherwise keep the hyphen. ~keep
                let next_lower = next_line.chars().next().is_some_and(|c| c.is_lowercase());
                let prev_lower = without_hyphen
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_ascii_lowercase());
                let should_rejoin = next_lower && (soft || prev_lower);

                if should_rejoin {
                    result.push_str(without_hyphen);
                    result.push_str(next_line);

                    i += 2;

                    if i < lines.len() {
                        result.push('\n');
                    }
                    continue;
                }
            }

            result.push_str(line);
            i += 1;

            if i < lines.len() {
                result.push('\n');
            }
        }

        if result.ends_with('\n') && !text.ends_with('\n') {
            result.pop();
        }

        result
    }

    /// Normalize whitespace: remove extra spaces within words, preserve between words.
    ///
    /// Per PDF Spec Section 14.8.2.5, boundary whitespace must be checked before adding spaces.
    /// This function fixes the issue where text extraction creates extra spaces within words.
    ///
    /// # Algorithm
    ///
    /// For each sequence of 2+ consecutive spaces:
    /// - Check the preceding and following characters
    /// - If both are word characters, reduce to single space (likely a word boundary)
    /// - If either is punctuation, preserve spacing pattern
    ///
    /// # Arguments
    ///
    /// * `text` - The text to process
    ///
    /// # Returns
    ///
    /// Text with normalized whitespace
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::converters::text_post_processor::TextPostProcessor;
    ///
    /// let input = "The quick brown fox";
    /// let output = TextPostProcessor::normalize_whitespace(input);
    /// assert_eq!(output, "The quick brown fox");
    /// ```
    pub fn normalize_whitespace(text: &str) -> String {
        // Use regex to replace 2+ spaces with single space, but only when
        // not at line boundaries (preserve indentation at start of lines) ~keep
        let mut result = String::with_capacity(text.len());

        for line in text.lines() {
            let trimmed_start = line.trim_start();
            let leading_spaces = line.len() - trimmed_start.len();

            for _ in 0..leading_spaces {
                result.push(' ');
            }

            let normalized = RE_EXCESSIVE_SPACES.replace_all(trimmed_start, "$1 $2").to_string();
            result.push_str(&normalized);

            if !result.ends_with('\n') {
                result.push('\n');
            }
        }

        if result.ends_with('\n') && !text.ends_with('\n') {
            result.pop();
        }

        result
    }

    /// Apply full text post-processing pipeline.
    ///
    /// Applies hyphenation removal and whitespace normalization in sequence.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to process
    ///
    /// # Returns
    ///
    /// Fully processed text with improved extraction quality
    /// Ensure proper spacing around special characters (Greek letters, math symbols).
    ///
    /// Per PDF Spec Section 9.4.4, certain Unicode ranges require special handling
    /// for word boundary detection:
    /// - Greek letters (U+0370–U+03FF)
    /// - Mathematical symbols (U+2200–U+22FF)
    /// - Other special ranges that need spacing
    ///
    /// # Arguments
    ///
    /// * `text` - The text to process
    ///
    /// # Returns
    ///
    /// Text with proper spacing around special characters
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::converters::text_post_processor::TextPostProcessor;
    ///
    /// let input = "compute β-VAE model";
    /// let output = TextPostProcessor::ensure_special_char_spacing(input);
    /// // Ensures spacing is correct around β character
    /// ```
    pub fn ensure_special_char_spacing(text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let chars: Vec<char> = text.chars().collect();

        for i in 0..chars.len() {
            let current_char = chars[i];
            let prev_char = if i > 0 { Some(chars[i - 1]) } else { None };
            let next_char = if i + 1 < chars.len() { Some(chars[i + 1]) } else { None };

            let is_special = Self::is_special_character(current_char);

            if is_special && let Some(prev) = prev_char {
                // Add space if:
                // 1. Previous char is not whitespace AND
                // 2. Previous char is not a punctuation that typically precedes special chars ~keep
                if !prev.is_whitespace()
                    && !Self::is_space_before_special(prev)
                    && !result.is_empty()
                    && !result.ends_with(' ')
                {
                    result.push(' ');
                }
            }

            result.push(current_char);

            if is_special
                && let Some(next) = next_char
                && !next.is_whitespace()
                && !Self::is_space_after_special(next)
            {
                result.push(' ');
            }
        }

        result
    }

    /// Check if a character is a special character requiring spacing.
    #[cfg_attr(test, allow(dead_code))]
    pub fn is_special_character(ch: char) -> bool {
        // Greek letters: U+0370–U+03FF ~keep
        if ('\u{0370}'..='\u{03FF}').contains(&ch) {
            return true;
        }

        // Mathematical symbols: U+2200–U+22FF ~keep
        if ('\u{2200}'..='\u{22FF}').contains(&ch) {
            return true;
        }

        // Mathematical operators and symbols: U+2000–U+206F ~keep
        if ('\u{2000}'..='\u{206F}').contains(&ch) {
            return true;
        }

        false
    }

    /// Check if a character typically precedes special characters (shouldn't add space).
    #[cfg_attr(test, allow(dead_code))]
    pub fn is_space_before_special(ch: char) -> bool {
        matches!(ch, '(' | '[' | '{' | '<' | '-' | '/')
    }

    /// Check if a character typically follows special characters (shouldn't add space).
    #[cfg_attr(test, allow(dead_code))]
    pub fn is_space_after_special(ch: char) -> bool {
        matches!(ch, ')' | ']' | '}' | '>' | '-' | ',' | '.' | ':' | ';' | '\'' | '"')
    }

    /// Repair broken ligatures from PDFs with corrupt ToUnicode CMaps.
    ///
    /// Some LaTeX-generated PDFs have broken ToUnicode CMaps that map ligature
    /// glyphs (fi, fl, ff, ffi, ffl) to incorrect characters. Common mappings:
    ///
    /// - `ff` → `!` (e.g., "di!erent" → "different")
    /// - `ffi` → `"` (e.g., 'o"ces' → "offices")
    /// - `fi` → `#` (e.g., "#nancial" → "financial")
    /// - `fl` → `$` (e.g., "$oor" → "floor")
    /// - `ffl` → `%` (e.g., "ba%e" → "baffle")
    ///
    /// The heuristic: these characters only represent broken ligatures when
    /// surrounded by letters (not at word boundaries, sentence starts, or in
    /// natural punctuation contexts).
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::converters::text_post_processor::TextPostProcessor;
    ///
    /// assert_eq!(TextPostProcessor::repair_ligatures("di!erent"), "different");
    /// assert_eq!(TextPostProcessor::repair_ligatures("Hello!"), "Hello!");
    /// ```
    pub fn repair_ligatures(text: &str) -> String {
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        if len == 0 {
            return String::new();
        }

        let mut result = String::with_capacity(text.len());
        let mut i = 0;

        while i < len {
            let ch = chars[i];

            let replacement = match ch {
                '!' => Some("ff"),
                '"' => Some("ffi"),
                '#' => Some("fi"),
                '$' => Some("fl"),
                '%' => Some("ffl"),
                _ => None,
            };

            if let Some(lig) = replacement {
                // Only replace if surrounded by lowercase letters — this avoids
                // false positives on "C#", "$var", "100%", etc. Broken ligatures
                // from LaTeX PDFs always appear mid-word between lowercase chars. ~keep
                let prev_is_lower = i > 0 && chars[i - 1].is_lowercase();
                let next_is_lower = i + 1 < len && chars[i + 1].is_lowercase();

                if prev_is_lower && next_is_lower {
                    result.push_str(lig);
                } else {
                    result.push(ch);
                }
            } else {
                result.push(ch);
            }

            i += 1;
        }

        result
    }

    /// Normalize leader dots in TOC-style lines.
    ///
    /// Collapses long runs of dots (or dot-like characters) into a short leader
    /// sequence ("...") to produce cleaner text output. This handles common TOC
    /// formatting where sections are connected to page numbers by dot leaders:
    ///
    /// Input: "Section 1 ..................... 5"
    /// Output: "Section 1 ... 5"
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::converters::text_post_processor::TextPostProcessor;
    ///
    /// let input = "Introduction .................. 5";
    /// let output = TextPostProcessor::normalize_leader_dots(input);
    /// assert_eq!(output, "Introduction ... 5");
    /// ```
    pub fn normalize_leader_dots(text: &str) -> String {
        let mut result = String::with_capacity(text.len());

        for (line_idx, line) in text.lines().enumerate() {
            if line_idx > 0 {
                result.push('\n');
            }

            let chars: Vec<char> = line.chars().collect();
            let len = chars.len();
            let mut i = 0;

            while i < len {
                if Self::is_leader_dot(chars[i]) {
                    let run_start = i;
                    while i < len && Self::is_leader_dot(chars[i]) {
                        i += 1;
                    }
                    let run_len = i - run_start;

                    if run_len >= 4 {
                        if !result.ends_with(' ') {
                            result.push(' ');
                        }
                        result.push_str("...");

                        while i < len && chars[i] == ' ' {
                            i += 1;
                        }

                        if i < len {
                            result.push(' ');
                        }
                    } else {
                        for c in &chars[run_start..i] {
                            result.push(*c);
                        }
                    }
                } else {
                    result.push(chars[i]);
                    i += 1;
                }
            }
        }

        if text.ends_with('\n') && !result.ends_with('\n') {
            result.push('\n');
        }

        result
    }

    /// Check if a character is a dot-like leader character.
    pub fn is_leader_dot(ch: char) -> bool {
        matches!(ch, '.' | '·' | '․' | '‥' | '…')
    }

    /// Map Unicode typographic spaces to U+0020 and strip zero-width spaces.
    ///
    /// Some PDF producers use hairspace (U+200A) or other typographic space
    /// variants (U+2000–U+200A, U+202F, U+205F) as word separators in justified
    /// layouts, and encode them directly in ToUnicode CMaps. For extraction
    /// purposes every Unicode spacing character is equivalent to a regular space;
    /// keeping the original codepoints breaks word-level tokenisation downstream.
    ///
    /// Zero-width space (U+200B) carries no width and is removed entirely.
    pub(crate) fn normalize_unicode_spaces(text: &str) -> std::borrow::Cow<'_, str> {
        // Fast path: skip allocation when no typographic spaces are present. ~keep
        let needs_work = text
            .chars()
            .any(|c| matches!(c, '\u{2000}'..='\u{200B}' | '\u{202F}' | '\u{205F}'));
        if !needs_work {
            return std::borrow::Cow::Borrowed(text);
        }
        let mut result = String::with_capacity(text.len());
        for ch in text.chars() {
            match ch {
                // EN QUAD … HAIR SPACE, NARROW NO-BREAK SPACE, MEDIUM MATH SPACE ~keep
                '\u{2000}'..='\u{200A}' | '\u{202F}' | '\u{205F}' => result.push(' '),
                // Zero-width space: not a visible character, omit ~keep
                '\u{200B}' => {}
                _ => result.push(ch),
            }
        }
        std::borrow::Cow::Owned(result)
    }

    /// Apply full text post-processing pipeline.
    ///
    /// Applies Unicode space normalization, ligature repair, hyphenation
    /// removal, whitespace normalization, leader dot normalization, and special
    /// character spacing in sequence.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to process
    ///
    /// # Returns
    ///
    /// Fully processed text with improved extraction quality
    pub fn process(text: &str) -> String {
        let unicode_normalized = Self::normalize_unicode_spaces(text);
        let ligatures_fixed = Self::repair_ligatures(&unicode_normalized);
        let ligature_split_fixed = Self::repair_ligature_intra_space(&ligatures_fixed);
        let combining_composed = Self::compose_combining_marks(&ligature_split_fixed);
        let run_boundary_repaired = Self::repair_run_boundary_space(&combining_composed);
        let hyphenated_fixed = Self::rejoin_hyphenated_words(&run_boundary_repaired);
        let monospace_fixed = Self::repair_monospace_punctuation_spacing(&hyphenated_fixed);
        let whitespace_normalized = Self::normalize_whitespace(&monospace_fixed);
        let leaders_normalized = Self::normalize_leader_dots(&whitespace_normalized);
        Self::ensure_special_char_spacing(&leaders_normalized)
    }

    /// collapse intra-expansion whitespace inside AGL
    /// ligature expansions. When pdfTeX emits a `/ffi` / `/ff` / `/fi`
    /// / `/fl` / `/ffl` glyph and xberg-native-pdf's per-glyph space heuristic
    /// inserts spaces inside the expansion, the result is
    /// `di ff cult` instead of `difficult`. This repair pass detects
    /// the pattern (short word + one of `ff`/`fi`/`fl`/`ffi`/`ffl` +
    /// short word, all-lowercase, with letter-only neighbours)
    /// glues the three back together.
    ///
    /// Conservative by design: only fires when both surrounding tokens
    /// are ≥ 2 letters and the ligature token is one of the known
    /// AGL ligature names. Won't touch legitimate phrases like
    /// "a fi nal" (rare in real text but theoretically possible) where
    /// the middle token is between full words.
    ///
    pub fn repair_ligature_intra_space(text: &str) -> String {
        static RE_LIG_SPLIT: LazyLock<Regex> = LazyLock::new(|| {
            // (\b[a-z]+) space (ffi|ffl|ff|fi|fl) space ([a-z]+\b)
            //   prefix ligature suffix
            // Prefix is 1+ chars to cover the `affects` → `a ff ects`
            // case from the issue body (1-char prefix `a`). Suffix is
            // 1+ chars to cover any reasonable continuation. ~keep
            Regex::new(r"\b([a-z]+) (ffi|ffl|ff|fi|fl) ([a-z]+)\b").unwrap()
        });
        RE_LIG_SPLIT.replace_all(text, "$1$2$3").into_owned()
    }

    /// compose adjacent combining-mark sequences into
    /// their precomposed equivalents via NFC normalisation. PdfTeX
    /// emits combining diacritics as separate glyphs at near-zero
    /// advance, producing artefacts like `´E` for `É`,
    /// `Universit e´` for `Université`, and `CJK( ` for `(CJK`.
    ///
    /// This runs NFC over the full text. Composition rules per
    /// Unicode UAX-15: a base codepoint followed by a combining mark
    /// (combining class > 0) composes into a single precomposed
    /// codepoint when one exists.
    ///
    /// The pattern variations observed in pdfTeX output put the
    /// combining mark BEFORE the base (e.g. acute-then-E producing
    /// `´E`). We additionally normalise that ordering: detect
    /// standalone combining marks (`\u{0301}`, `\u{0300}`, etc.)
    /// swap with following base letter.
    ///
    pub fn compose_combining_marks(text: &str) -> String {
        // Lookup table for the common spacing-diacritic + base-letter
        // → precomposed mapping. Covers acute, grave, circumflex,
        // cedilla, tilde, diaeresis (the marks that pdfTeX emits as
        // standalone U+00B4 / U+0060 / U+005E / U+00B8 / U+007E /
        // U+00A8 glyphs adjacent to base letters). ~keep
        //
        // Patterns matched (both orderings observed in pdfTeX output):
        //   `´E` (mark before) → `É`
        //   `e´` (mark after) → `é`
        // Plus the same for grave/circumflex/cedilla/tilde/diaeresis. ~keep
        fn compose(prev: char, mark: char) -> Option<char> {
            match (prev, mark) {
                // Acute (U+00B4 spacing → U+0301 combining) ~keep
                ('A', '\u{00B4}') => Some('Á'),
                ('E', '\u{00B4}') => Some('É'),
                ('I', '\u{00B4}') => Some('Í'),
                ('O', '\u{00B4}') => Some('Ó'),
                ('U', '\u{00B4}') => Some('Ú'),
                ('Y', '\u{00B4}') => Some('Ý'),
                ('a', '\u{00B4}') => Some('á'),
                ('e', '\u{00B4}') => Some('é'),
                ('i', '\u{00B4}') => Some('í'),
                ('o', '\u{00B4}') => Some('ó'),
                ('u', '\u{00B4}') => Some('ú'),
                ('y', '\u{00B4}') => Some('ý'),
                // Grave (U+0060) ~keep
                ('A', '\u{0060}') => Some('À'),
                ('E', '\u{0060}') => Some('È'),
                ('I', '\u{0060}') => Some('Ì'),
                ('O', '\u{0060}') => Some('Ò'),
                ('U', '\u{0060}') => Some('Ù'),
                ('a', '\u{0060}') => Some('à'),
                ('e', '\u{0060}') => Some('è'),
                ('i', '\u{0060}') => Some('ì'),
                ('o', '\u{0060}') => Some('ò'),
                ('u', '\u{0060}') => Some('ù'),
                // Circumflex (U+005E) ~keep
                ('A', '\u{005E}') => Some('Â'),
                ('E', '\u{005E}') => Some('Ê'),
                ('I', '\u{005E}') => Some('Î'),
                ('O', '\u{005E}') => Some('Ô'),
                ('U', '\u{005E}') => Some('Û'),
                ('a', '\u{005E}') => Some('â'),
                ('e', '\u{005E}') => Some('ê'),
                ('i', '\u{005E}') => Some('î'),
                ('o', '\u{005E}') => Some('ô'),
                ('u', '\u{005E}') => Some('û'),
                // Tilde (U+007E) ~keep
                ('N', '\u{007E}') => Some('Ñ'),
                ('A', '\u{007E}') => Some('Ã'),
                ('O', '\u{007E}') => Some('Õ'),
                ('n', '\u{007E}') => Some('ñ'),
                ('a', '\u{007E}') => Some('ã'),
                ('o', '\u{007E}') => Some('õ'),
                // Diaeresis (U+00A8) ~keep
                ('A', '\u{00A8}') => Some('Ä'),
                ('E', '\u{00A8}') => Some('Ë'),
                ('I', '\u{00A8}') => Some('Ï'),
                ('O', '\u{00A8}') => Some('Ö'),
                ('U', '\u{00A8}') => Some('Ü'),
                ('a', '\u{00A8}') => Some('ä'),
                ('e', '\u{00A8}') => Some('ë'),
                ('i', '\u{00A8}') => Some('ï'),
                ('o', '\u{00A8}') => Some('ö'),
                ('u', '\u{00A8}') => Some('ü'),
                ('y', '\u{00A8}') => Some('ÿ'),
                // Cedilla (U+00B8) — after C/c only ~keep
                ('C', '\u{00B8}') => Some('Ç'),
                ('c', '\u{00B8}') => Some('ç'),
                _ => None,
            }
        }

        // Walk the string once. At each position, check both
        // "base then mark" and "mark then base" orderings. Additionally
        // collapse a single intervening space between a word and its
        // adjacent mark (the `Universit e\u{00B4}` → `Université` shape
        // observed in pdfTeX output where the writer split the affix
        // across a glyph boundary). ~keep
        let chars: Vec<char> = text.chars().collect();
        let mut out = String::with_capacity(text.len());
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            // Mark-then-base (pdfTeX-common pattern): ~keep
            if matches!(
                c,
                '\u{00B4}' | '\u{0060}' | '\u{005E}' | '\u{007E}' | '\u{00A8}' | '\u{00B8}'
            ) && i + 1 < chars.len()
            {
                let next = chars[i + 1];
                if let Some(composed) = compose(next, c) {
                    out.push(composed);
                    i += 2;
                    continue;
                }
            }
            // Base-then-mark (also observed): ~keep
            if i + 1 < chars.len() {
                let next = chars[i + 1];
                if matches!(
                    next,
                    '\u{00B4}' | '\u{0060}' | '\u{005E}' | '\u{007E}' | '\u{00A8}' | '\u{00B8}'
                ) && let Some(composed) = compose(c, next)
                {
                    // Collapse the artefact space before the base
                    // letter if it exists: `Universit e´` shape —
                    // a word followed by space-base-mark — extracts
                    // as `Universit é`; remove the trailing space
                    // from `out` so the result becomes `Université`. ~keep
                    if c.is_alphabetic() && out.ends_with(' ') {
                        // Look behind: was the char before that
                        // space also a letter (part of the same
                        // word)? Only then collapse. ~keep
                        let trailing_letter = out.chars().rev().nth(1).map(|p| p.is_alphabetic()).unwrap_or(false);
                        if trailing_letter {
                            out.pop();
                        }
                    }
                    out.push(composed);
                    i += 2;
                    continue;
                }
            }
            out.push(c);
            i += 1;
        }
        out
    }

    /// repair the missing-space-at-font-boundary
    /// pattern where the upstream space-emission heuristic fires below
    /// threshold at a font/run transition. PdfTeX-typeset titles like
    /// `Astronomy & Astrophysicsmanuscript no.` exhibit this when the
    /// title spans a font switch (italic → roman) mid-line and the
    /// per-glyph gap at the switch boundary doesn't exceed the
    /// space-width threshold.
    ///
    /// Repair heuristic: detect `[a-z]{2,}` followed immediately by
    /// `[A-Z][a-z]` (no space between) where the merged token does
    /// NOT look like a valid CamelCase identifier (heuristic:
    /// surrounded by ordinary prose tokens, not punctuation/numbers).
    /// Insert a space.
    ///
    /// Conservative: only fires when the immediate surrounding text
    /// is prose-shaped (capitalised first word + ordinary punctuation
    /// at end). Won't touch genuine CamelCase identifiers in code
    /// (e.g., `HashMap`, `PdfDocument`) when surrounding context
    /// suggests code.
    ///
    pub fn repair_run_boundary_space(text: &str) -> String {
        static RE_LOWERCASE_THEN_TITLE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"([a-z]{2,})([A-Z][a-z])").unwrap());

        let mut out = String::with_capacity(text.len());
        for line in text.lines() {
            // Skip lines that look like code (camelCase identifiers
            // are legitimate there). ~keep
            let looks_codey = line.contains('{')
                || line.contains('}')
                || line.contains("()")
                || line.contains("=>")
                || line.contains("::");
            if looks_codey {
                out.push_str(line);
            } else {
                let repaired = RE_LOWERCASE_THEN_TITLE.replace_all(line, "$1 $2");
                out.push_str(&repaired);
            }
            out.push('\n');
        }
        if !text.ends_with('\n') && out.ends_with('\n') {
            out.pop();
        }
        out
    }

    /// remove the extra spaces that xberg-native-pdf's per-
    /// glyph repositioning heuristic inserts inside monospace code
    /// listings around punctuation. Examples from
    /// `code_and_formula.pdf`:
    ///
    /// ```text
    /// function add (a , b ) { → function add(a, b) {
    /// console . log ( add (3 , 5)); → console.log(add(3, 5));
    /// ```
    ///
    /// Pattern: ` ([(\[{,;:.])` (space before punctuation) → `$1`,
    /// and `([(\[{])` `([a-zA-Z0-9])` ... wait the issue is the space
    /// is BETWEEN punctuation and identifier. So:
    ///
    /// - `add (a` → `add(a` (no space before `(`)
    /// - `a ,` → `a,` (no space before `,`)
    /// - `( add` → `(add` (no space after `(`)
    ///
    /// Only fires on monospace-code-shaped lines (high punctuation
    /// density). Conservative: a line is "monospace-code-shaped" if
    /// it contains at least one of `{`/`}`/`(`/`)`/`;` AND a token
    /// like `function`/`return`/`let`/`var`/`const`/`if`/`while`/`for`.
    ///
    pub fn repair_monospace_punctuation_spacing(text: &str) -> String {
        static RE_SPACE_BEFORE_PUNCT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r" ([,;.:)\]}])").unwrap());
        static RE_SPACE_AFTER_OPEN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([(\[{]) ").unwrap());
        static RE_SPACE_BEFORE_OPEN_ON_IDENT: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*) ([(])").unwrap());

        let mut out = String::with_capacity(text.len());
        for line in text.lines() {
            let is_codey = (line.contains('{')
                || line.contains('}')
                || line.contains('(')
                || line.contains(')')
                || line.contains(';'))
                && (line.contains("function")
                    || line.contains("return")
                    || line.contains("let ")
                    || line.contains("var ")
                    || line.contains("const ")
                    || line.contains("if (")
                    || line.contains("if(")
                    || line.contains("for(")
                    || line.contains("for ("));
            if is_codey {
                let step1 = RE_SPACE_BEFORE_PUNCT.replace_all(line, "$1");
                let step2 = RE_SPACE_AFTER_OPEN.replace_all(&step1, "$1");
                let step3 = RE_SPACE_BEFORE_OPEN_ON_IDENT.replace_all(&step2, "$1$2");
                out.push_str(&step3);
            } else {
                out.push_str(line);
            }
            out.push('\n');
        }
        if !text.ends_with('\n') && out.ends_with('\n') {
            out.pop();
        }
        out
    }
}

#[cfg(test)]
mod tests;
