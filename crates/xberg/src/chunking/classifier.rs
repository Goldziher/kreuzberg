//! Heuristic semantic classifier for text chunks.
//!
//! Assigns a [`ChunkType`] to each text chunk based on structural signals
//! (heading context, Markdown syntax) and content-level keyword patterns.
//! Rules are evaluated in priority order; the first match wins.
//!
//! # Design
//!
//! - **No ML**: fully deterministic, zero-latency overhead, no external deps.
//! - **Ordered rules**: higher-precision structural signals run before
//!   lower-precision keyword heuristics.
//! - **Extensible**: add new variants to [`ChunkType`] and insert new rules
//!   without breaking existing classifications.

use crate::types::{ChunkType, HeadingContext};

/// Classify a single chunk based on its content and optional heading context.
///
/// Rules are evaluated in priority order. The first matching rule determines
/// the returned [`ChunkType`]. When no rule matches, [`ChunkType::Unknown`]
/// is returned.
///
/// # Arguments
///
/// * `content` - The text content of the chunk (may be trimmed or raw).
/// * `heading_context` - Optional heading hierarchy this chunk falls under
///   (only available when using `ChunkerType::Markdown`).
///
/// # Examples
///
/// ```ignore
/// use xberg::chunking::classifier::classify_chunk;
/// use xberg::types::ChunkType;
///
/// assert_eq!(classify_chunk("# Introduction", None), ChunkType::Heading);
/// assert_eq!(
///     classify_chunk("The Investor shall subscribe for the Shares and agrees to pay the subscription price. The Company shall deliver the Share certificates upon receipt.", None),
///     ChunkType::OperativeClause,
/// );
/// assert_eq!(classify_chunk("Some unrecognized text.", None), ChunkType::Unknown);
/// ```
pub(crate) fn classify_chunk(content: &str, heading_context: Option<&HeadingContext>) -> ChunkType {
    let trimmed = content.trim();

    if is_heading(trimmed, heading_context) {
        return ChunkType::Heading;
    }

    if is_math_fence(trimmed) {
        return ChunkType::Formula;
    }

    if is_code_block(content) {
        return ChunkType::CodeBlock;
    }

    if is_table_like(trimmed) {
        return ChunkType::TableLike;
    }

    if is_formula(trimmed) {
        return ChunkType::Formula;
    }

    if is_schedule(trimmed, heading_context) {
        return ChunkType::Schedule;
    }

    if is_definitions(trimmed) {
        return ChunkType::Definitions;
    }

    if is_signature_block(trimmed) {
        return ChunkType::SignatureBlock;
    }

    if is_operative_clause(trimmed) {
        return ChunkType::OperativeClause;
    }

    if is_party_list(trimmed) {
        return ChunkType::PartyList;
    }

    ChunkType::Unknown
}

fn is_heading(content: &str, _ctx: Option<&HeadingContext>) -> bool {
    if content.starts_with('#') {
        return true;
    }
    let mut lines = content.lines();
    if let (Some(_title), Some(underline)) = (lines.next(), lines.next()) {
        let u = underline.trim();
        if !u.is_empty() && (u.chars().all(|c| c == '=') || u.chars().all(|c| c == '-')) {
            return true;
        }
    }
    false
}

fn is_code_block(content: &str) -> bool {
    if content.starts_with("```") || content.starts_with("~~~") {
        return true;
    }
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() >= 2 {
        let all_indented = lines
            .iter()
            .filter(|l| !l.trim().is_empty())
            .all(|l| l.starts_with("    ") || l.starts_with('\t'));
        if all_indented {
            return true;
        }
    }
    false
}

/// A ```` ```math ```` fence opened outside any other fence, the form the Markdown writer gives
/// display math. Quote markers before a fence are skipped.
fn is_math_fence(content: &str) -> bool {
    let mut open: Option<(char, usize)> = None;
    for line in content.lines() {
        let line = line.trim_start_matches(['>', ' ', '\t']).trim_end();
        let Some(fence) = line.chars().next().filter(|c| *c == '`' || *c == '~') else {
            continue;
        };
        let info = line.trim_start_matches(fence);
        let run = line.len() - info.len();
        if run < 3 {
            continue;
        }
        match open {
            Some((ch, len)) if ch == fence && run >= len && info.is_empty() => open = None,
            Some(_) => {}
            None if info.split_ascii_whitespace().next() == Some("math") => return true,
            None => open = Some((fence, run)),
        }
    }
    false
}

fn is_table_like(content: &str) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() < 2 {
        return false;
    }
    let pipe_lines = lines.iter().filter(|l| l.contains('|')).count();
    if pipe_lines >= 2 {
        return true;
    }
    let sep_lines = lines
        .iter()
        .filter(|l| {
            let t = l.trim();
            t.len() >= 4 && t.chars().all(|c| c == '-' || c == '+' || c == '|' || c == ' ')
        })
        .count();
    sep_lines >= 3
}

fn is_formula(content: &str) -> bool {
    const MATH_SYMBOLS: &[char] = &['∑', '∫', '√', '∂', '∏', '≤', '≥', '≠', '→', '←', '⊂', '⊃'];
    if content.chars().any(|c| MATH_SYMBOLS.contains(&c)) {
        return true;
    }
    let lower = content.to_lowercase();
    let latex_patterns = [
        r"\frac", r"\sum", r"\int", r"\sqrt", r"\alpha", r"\beta", r"\delta", r"$$", r"\[",
    ];
    if latex_patterns.iter().any(|p| lower.contains(p)) {
        return true;
    }
    false
}

fn is_schedule(content: &str, ctx: Option<&HeadingContext>) -> bool {
    const KEYWORDS: &[&str] = &["schedule", "annex", "appendix", "exhibit"];
    let lower = content.to_lowercase();

    if let Some(ctx) = ctx {
        for h in &ctx.headings {
            let hl = h.text.to_lowercase();
            if KEYWORDS.iter().any(|k| hl.contains(k)) {
                return true;
            }
        }
    }
    let first_line = content.lines().next().unwrap_or("").to_lowercase();
    if KEYWORDS.iter().any(|k| first_line.starts_with(k)) {
        return true;
    }

    KEYWORDS.iter().any(|k| {
        if let Some(idx) = lower.find(k) {
            let rest = &lower[idx + k.len()..];
            rest.starts_with(' ')
                && rest
                    .trim_start()
                    .chars()
                    .next()
                    .map(|c| c.is_alphanumeric())
                    .unwrap_or(false)
        } else {
            false
        }
    })
}

fn is_definitions(content: &str) -> bool {
    let lower = content.to_lowercase();
    let patterns = [
        "\" means ",
        "\" shall mean ",
        "\" has the meaning",
        "' means ",
        "' shall mean ",
        "means, for purposes",
        "is defined as",
        "shall be construed as",
    ];
    patterns.iter().any(|p| lower.contains(p))
}

fn is_signature_block(content: &str) -> bool {
    let lower = content.to_lowercase();
    let keywords = [
        "signature",
        "signed by",
        "witnessed by",
        "date:",
        "in witness whereof",
        "authorized signatory",
        "duly authorized",
        "____",
    ];
    let hits = keywords.iter().filter(|k| lower.contains(*k)).count();
    hits >= 2
}

fn is_operative_clause(content: &str) -> bool {
    let lower = content.to_lowercase();
    let verbs = [
        "shall ",
        "agree ",
        "agrees ",
        "transfer",
        "grant ",
        "grants ",
        "undertake",
        "obligat",
        "covenant",
        "warrant",
        "represent",
        "indemnif",
        "assign ",
        "assigns ",
        "license ",
        "licenses ",
        "purchase",
        "sell ",
        "sells ",
        "pay ",
        "pays ",
        "deliver",
    ];
    let hits = verbs.iter().filter(|v| lower.contains(*v)).count();
    hits >= 3
}

fn is_party_list(content: &str) -> bool {
    let lines: Vec<&str> = content.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

    if lines.len() < 3 {
        return false;
    }

    let party_like = lines.iter().filter(|l| is_party_line(l)).count();
    party_like >= (lines.len() * 2 / 3).max(2)
}

/// Heuristic for a single "party-like" line.
///
/// A line looks like a party entry when it:
/// - Is short (≤ 120 chars), AND
/// - Starts with an uppercase letter (Title Case name or role), AND
/// - Contains at least one of: a comma, a digit (address), or a role keyword.
fn is_party_line(line: &str) -> bool {
    if line.len() > 120 {
        return false;
    }
    let starts_upper = line.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
    if !starts_upper {
        return false;
    }
    let has_digit = line.chars().any(|c| c.is_ascii_digit());
    let has_comma = line.contains(',');
    let lower = line.to_lowercase();
    let has_role = [
        "investor",
        "company",
        "borrower",
        "lender",
        "seller",
        "buyer",
        "party",
        "subscriber",
        "guarantor",
    ]
    .iter()
    .any(|r| lower.contains(r));
    has_digit || has_comma || has_role
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classify(content: &str) -> ChunkType {
        classify_chunk(content, None)
    }

    #[test]
    fn test_heading_atx() {
        assert_eq!(classify("# Introduction"), ChunkType::Heading);
        assert_eq!(classify("## Section 2"), ChunkType::Heading);
        assert_eq!(classify("### Sub-section"), ChunkType::Heading);
    }

    #[test]
    fn test_heading_setext() {
        assert_eq!(classify("Introduction\n============"), ChunkType::Heading);
        assert_eq!(classify("Section 2\n---------"), ChunkType::Heading);
    }

    #[test]
    fn test_not_heading_plain_text() {
        assert_ne!(classify("This is plain paragraph text."), ChunkType::Heading);
    }

    #[test]
    fn test_code_block_fenced() {
        assert_eq!(classify("```rust\nfn main() {}\n```"), ChunkType::CodeBlock);
        assert_eq!(classify("~~~python\nprint('hi')\n~~~"), ChunkType::CodeBlock);
    }

    #[test]
    fn test_code_block_indented() {
        let indented = "    fn main() {\n        println!(\"hello\");\n    }";
        assert_eq!(classify(indented), ChunkType::CodeBlock);
    }

    #[test]
    fn test_table_markdown() {
        let table = "| Name | Age |\n|------|-----|\n| Alice | 30 |";
        assert_eq!(classify(table), ChunkType::TableLike);
    }

    #[test]
    fn test_table_single_pipe_line_not_table() {
        assert_ne!(classify("Just one | separator here"), ChunkType::TableLike);
    }

    #[test]
    fn test_formula_unicode_symbols() {
        assert_eq!(classify("The total ∑ of all values equals 1."), ChunkType::Formula);
        assert_eq!(classify("∫ f(x) dx from 0 to ∞"), ChunkType::Formula);
    }

    #[test]
    fn test_formula_latex() {
        assert_eq!(classify(r"The result is $\frac{a}{b}$"), ChunkType::Formula);
        assert_eq!(classify(r"$$\sum_{i=0}^{n} x_i$$"), ChunkType::Formula);
        assert_eq!(classify("```math\nx_i + y\n```"), ChunkType::Formula);
        assert_eq!(classify("```math display\nx_i + y\n```"), ChunkType::Formula);
        assert_eq!(classify("Intro.\n\n```math\nx_i + y\n```"), ChunkType::Formula);
        assert_eq!(classify("````math\na ``` b\n````"), ChunkType::Formula);
        assert_eq!(classify("```mathematica\nPlot[x]\n```"), ChunkType::CodeBlock);
        assert_eq!(classify("````markdown\n```math\nx^2\n```\n````"), ChunkType::CodeBlock);
        assert_eq!(classify("> ```math\n> x^2\n> ```"), ChunkType::Formula);
    }

    #[test]
    fn test_schedule_first_line() {
        assert_eq!(
            classify("Schedule 1 – Definitions\n\nThis schedule sets out..."),
            ChunkType::Schedule
        );
        assert_eq!(classify("annex A: Technical Specifications"), ChunkType::Schedule);
    }

    #[test]
    fn test_definitions_means() {
        assert_eq!(
            classify("\"Agreement\" means this Investment and Subscription Agreement."),
            ChunkType::Definitions
        );
        assert_eq!(
            classify("\"Closing Date\" shall mean the date on which..."),
            ChunkType::Definitions
        );
    }

    #[test]
    fn test_definitions_is_defined_as() {
        assert_eq!(
            classify("The term 'Net Revenue' is defined as all revenue..."),
            ChunkType::Definitions
        );
    }

    #[test]
    fn test_signature_block() {
        let sig = "Signed by: John Smith\nDate: 2026-03-30\nWitnessed by: Jane Doe";
        assert_eq!(classify(sig), ChunkType::SignatureBlock);
    }

    #[test]
    fn test_signature_block_in_witness() {
        let sig = "In witness whereof the parties have duly authorized this agreement.\n____________________\nDate: ___________";
        assert_eq!(classify(sig), ChunkType::SignatureBlock);
    }

    #[test]
    fn test_operative_clause_basic() {
        let clause = "The Investor shall subscribe for the Shares and agrees to pay the subscription price. The Company shall deliver the Share certificates upon receipt.";
        assert_eq!(classify(clause), ChunkType::OperativeClause);
    }

    #[test]
    fn test_operative_clause_grant() {
        let clause = "The Licensor hereby grants, assigns, and transfers all right, title, and interest. The Licensee shall pay and deliver consideration.";
        assert_eq!(classify(clause), ChunkType::OperativeClause);
    }

    #[test]
    fn test_party_list_basic() {
        let parties = "Gregor Guggisberg, Winkelstrasse 12, Zurich\nInvestor\nAlpha Capital AG, Bahnhofstrasse 1, Zurich\nSubscriber\nBeta Holdings Ltd, 10 City Road, London\nBorrower";
        assert_eq!(classify(parties), ChunkType::PartyList);
    }

    #[test]
    fn test_unknown_plain_text() {
        assert_eq!(
            classify("This document contains general information."),
            ChunkType::Unknown
        );
    }

    #[test]
    fn test_unknown_empty() {
        assert_eq!(classify(""), ChunkType::Unknown);
    }

    #[test]
    fn test_heading_context_schedule() {
        use crate::types::{HeadingContext, HeadingLevel};
        let ctx = HeadingContext {
            headings: vec![HeadingLevel {
                level: 1,
                text: "Schedule 1 – Definitions".to_string(),
            }],
        };
        let result = classify_chunk("This schedule sets out the defined terms.", Some(&ctx));
        assert_eq!(result, ChunkType::Schedule);
    }
}
