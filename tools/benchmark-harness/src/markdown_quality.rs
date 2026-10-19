//! Markdown block parsing and shared reading-order helpers.
//!
//! Canonical SF1 scoring lives in [`crate::quality::structural_sidecar`]. This
//! module only provides the CommonMark parser used by ground-truth validation
//! and the order helpers shared by the structural sidecar.

/// Block types in a markdown document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MdBlockType {
    Heading1,
    Heading2,
    Heading3,
    Heading4,
    Heading5,
    Heading6,
    Paragraph,
    CodeBlock,
    Formula,
    Table,
    ListItem,
    Image,
}

impl MdBlockType {
    pub(crate) fn is_heading(self) -> bool {
        matches!(
            self,
            Self::Heading1 | Self::Heading2 | Self::Heading3 | Self::Heading4 | Self::Heading5 | Self::Heading6
        )
    }

    fn name(&self) -> &'static str {
        match self {
            MdBlockType::Heading1 => "H1",
            MdBlockType::Heading2 => "H2",
            MdBlockType::Heading3 => "H3",
            MdBlockType::Heading4 => "H4",
            MdBlockType::Heading5 => "H5",
            MdBlockType::Heading6 => "H6",
            MdBlockType::Paragraph => "Paragraph",
            MdBlockType::CodeBlock => "Code",
            MdBlockType::Formula => "Formula",
            MdBlockType::Table => "Table",
            MdBlockType::ListItem => "ListItem",
            MdBlockType::Image => "Image",
        }
    }
}

impl std::fmt::Display for MdBlockType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// A parsed markdown block with its type and content.
#[derive(Debug, Clone)]
pub struct MdBlock {
    pub block_type: MdBlockType,
    pub content: String,
    pub index: usize,
}

/// Shared pulldown-cmark parse options for all markdown structural analysis.
///
/// Enables GFM tables, `$…$`/`$$…$$` math, and strikethrough. The structural
/// sidecar ([`crate::quality::structural_sidecar`]) derives its typed node list
/// with these *exact* options so its parse tree stays identical to
/// [`parse_markdown_blocks`].
pub(crate) fn md_parser_options() -> pulldown_cmark::Options {
    use pulldown_cmark::Options;
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_MATH);
    opts
}

/// The TeX of an `Event::InlineMath`: pulldown-cmark reads code math, `` $`x`$ ``, as inline
/// math that keeps its backticks.
pub(crate) fn inline_math_tex(text: &str) -> &str {
    text.strip_prefix('`')
        .and_then(|tex| tex.strip_suffix('`'))
        .unwrap_or(text)
}

/// Parse a markdown string into a sequence of typed blocks using pulldown-cmark.
///
/// This uses a proper CommonMark parser, so it correctly handles all markdown
/// variants: fenced and indented code blocks, ATX and setext headings, different
/// list markers (-, *, +, 1.), tables with any separator style, etc.
pub fn parse_markdown_blocks(md: &str) -> Vec<MdBlock> {
    use pulldown_cmark::Parser;

    let mut state = BlockAccumulator::default();
    for event in Parser::new_ext(md, md_parser_options()) {
        if state.handle_block_tag(&event) || state.handle_table_tag(&event) || state.handle_inline_tag(&event) {
            continue;
        }
        state.handle_content(event);
    }
    state.finish()
}

/// Running state of the [`parse_markdown_blocks`] event walk.
///
/// The `in_*` flags are the parser context the handlers branch on: pulldown-cmark reports text
/// inside a table cell, a code block or a list item with the same `Event::Text`, so the block a
/// run of text belongs to is only knowable from the enclosing tags seen so far.
#[derive(Default)]
struct BlockAccumulator {
    blocks: Vec<MdBlock>,
    index: usize,
    current_text: String,
    table_content: String,
    in_heading: Option<u8>,
    in_code_block: bool,
    in_math_fence: bool,
    in_table: bool,
    in_list_item: bool,
}

impl BlockAccumulator {
    fn flush_paragraph(&mut self) {
        flush_text(
            &mut self.current_text,
            &mut self.blocks,
            &mut self.index,
            MdBlockType::Paragraph,
        );
    }

    fn push_block(&mut self, block_type: MdBlockType, content: String) {
        self.blocks.push(MdBlock {
            block_type,
            content,
            index: self.index,
        });
        self.index += 1;
    }

    fn finish(mut self) -> Vec<MdBlock> {
        self.flush_paragraph();
        self.blocks
    }

    /// Handle heading, code-block, list and image tags. Returns whether the event was consumed.
    fn handle_block_tag(&mut self, event: &pulldown_cmark::Event<'_>) -> bool {
        use pulldown_cmark::{CodeBlockKind, Event, Tag, TagEnd};

        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                self.flush_paragraph();
                self.in_heading = Some(*level as u8);
            }
            Event::End(TagEnd::Heading(_)) => self.end_heading(),
            Event::Start(Tag::CodeBlock(kind)) => {
                self.flush_paragraph();
                self.in_code_block = true;
                self.in_math_fence =
                    matches!(kind, CodeBlockKind::Fenced(info) if info.split_ascii_whitespace().next() == Some("math"));
            }
            Event::End(TagEnd::CodeBlock) => self.end_code_block(),
            Event::Start(Tag::List(_)) => self.flush_paragraph(),
            Event::End(TagEnd::List(_)) => {}
            Event::Start(Tag::Item) => {
                self.flush_paragraph();
                self.in_list_item = true;
            }
            Event::End(TagEnd::Item) => {
                self.in_list_item = false;
                let content = std::mem::take(&mut self.current_text);
                if !content.trim().is_empty() {
                    self.push_block(MdBlockType::ListItem, content.trim().to_string());
                }
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                self.flush_paragraph();
                self.current_text.push_str("![");
                let _ = dest_url;
            }
            Event::End(TagEnd::Image) if self.current_text.starts_with("![") => {
                self.current_text.push(']');
                let content = std::mem::take(&mut self.current_text);
                self.push_block(MdBlockType::Image, content);
            }
            _ => return false,
        }
        true
    }

    fn end_heading(&mut self) {
        let Some(level) = self.in_heading.take() else {
            return;
        };
        let block_type = match level {
            1 => MdBlockType::Heading1,
            2 => MdBlockType::Heading2,
            3 => MdBlockType::Heading3,
            4 => MdBlockType::Heading4,
            5 => MdBlockType::Heading5,
            _ => MdBlockType::Heading6,
        };
        let content = std::mem::take(&mut self.current_text);
        if !content.trim().is_empty() {
            self.push_block(block_type, content.trim().to_string());
        }
    }

    fn end_code_block(&mut self) {
        self.in_code_block = false;
        let content = std::mem::take(&mut self.current_text);
        if content.trim().is_empty() {
            return;
        }
        let block_type = if self.in_math_fence
            || content.trim().starts_with("\\")
            || content.contains("\\frac")
            || content.contains("\\sum")
            || content.contains("\\int")
        {
            MdBlockType::Formula
        } else {
            MdBlockType::CodeBlock
        };
        self.push_block(block_type, content.trim_end().to_string());
    }

    /// Handle table tags. Returns whether the event was consumed.
    fn handle_table_tag(&mut self, event: &pulldown_cmark::Event<'_>) -> bool {
        use pulldown_cmark::{Event, Tag, TagEnd};

        match event {
            Event::Start(Tag::Table(_)) => {
                self.flush_paragraph();
                self.in_table = true;
                self.table_content.clear();
            }
            Event::End(TagEnd::Table) => {
                self.in_table = false;
                let content = std::mem::take(&mut self.table_content);
                if !content.trim().is_empty() {
                    self.push_block(MdBlockType::Table, content.trim().to_string());
                }
            }
            Event::Start(Tag::TableHead) | Event::End(TagEnd::TableHead) => {}
            Event::Start(Tag::TableRow) => {
                if !self.table_content.is_empty() {
                    self.table_content.push('\n');
                }
                self.table_content.push('|');
            }
            Event::End(TagEnd::TableRow) | Event::Start(Tag::TableCell) => {}
            Event::End(TagEnd::TableCell) => {
                let cell_text = std::mem::take(&mut self.current_text);
                self.table_content.push(' ');
                self.table_content.push_str(cell_text.trim());
                self.table_content.push_str(" |");
            }
            _ => return false,
        }
        true
    }

    /// Handle paragraph and strong-emphasis tags, which are context-sensitive. Returns whether
    /// the event was consumed; a guard that does not hold leaves the event unhandled, as the
    /// original single-match walk did.
    fn handle_inline_tag(&mut self, event: &pulldown_cmark::Event<'_>) -> bool {
        use pulldown_cmark::{Event, Tag, TagEnd};

        match event {
            Event::Start(Tag::Paragraph) | Event::End(TagEnd::Paragraph) if !self.in_list_item && !self.in_table => {
                self.flush_paragraph();
            }
            Event::Start(Tag::Strong) | Event::End(TagEnd::Strong) if !self.in_table && !self.in_code_block => {
                self.current_text.push_str("**");
            }
            _ => return false,
        }
        true
    }

    /// Handle text, breaks, math and raw HTML.
    fn handle_content(&mut self, event: pulldown_cmark::Event<'_>) {
        use pulldown_cmark::Event;

        match event {
            Event::Text(text) | Event::Code(text) => self.push_text(&text),
            Event::SoftBreak => {
                if self.in_code_block {
                    self.current_text.push('\n');
                } else {
                    self.current_text.push(' ');
                }
            }
            Event::HardBreak => self.current_text.push('\n'),
            Event::InlineMath(text) => self.current_text.push_str(inline_math_tex(&text)),
            Event::DisplayMath(text) => {
                self.flush_paragraph();
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    self.push_block(MdBlockType::Formula, trimmed.to_string());
                }
            }
            Event::Html(html) => {
                let text = strip_html_tags(&html);
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    self.flush_paragraph();
                    self.push_block(MdBlockType::Paragraph, trimmed.to_string());
                }
            }
            Event::InlineHtml(html) => {
                let text = strip_html_tags(&html);
                if !text.is_empty() {
                    self.current_text.push_str(&text);
                }
            }
            _ => {}
        }
    }

    fn push_text(&mut self, text: &str) {
        if self.in_table {
            self.current_text.push_str(text);
            return;
        }
        if !self.current_text.is_empty()
            && !self.current_text.ends_with(' ')
            && !self.current_text.ends_with('\n')
            && !self.current_text.ends_with("**")
        {
            self.current_text.push(' ');
        }
        self.current_text.push_str(text);
    }
}

/// Flush accumulated text into a block if non-empty.
fn flush_text(text: &mut String, blocks: &mut Vec<MdBlock>, index: &mut usize, block_type: MdBlockType) {
    let content = std::mem::take(text);
    let trimmed = content.trim();
    if !trimmed.is_empty() {
        let actual_type = if block_type == MdBlockType::Paragraph && looks_like_formula(trimmed) {
            MdBlockType::Formula
        } else {
            block_type
        };
        blocks.push(MdBlock {
            block_type: actual_type,
            content: trimmed.to_string(),
            index: *index,
        });
        *index += 1;
    }
}

/// Check if content looks like a math/LaTeX formula.
///
/// Deliberately conservative: a long prose paragraph that merely *contains* an inline
/// superscript (`x^{2}`) or begins with a formatting command (`\textbf{...}`, `\section*{...}`)
/// is NOT a formula. Misclassifying such prose as `Formula` used to score it 0 against its
/// plain-text ground-truth twin (Formula↔Paragraph had no compatibility), asymmetrically
/// penalizing whichever extractor's markup diverged from the GT.
fn looks_like_formula(content: &str) -> bool {
    if content.contains("\\frac")
        || content.contains("\\sum")
        || content.contains("\\int")
        || content.contains("\\begin{")
        || content.contains("\\end{")
        || content.contains("\\left")
        || content.contains("\\right")
        || content.contains("\\sqrt")
        || content.contains("\\mathbb")
        || content.contains("\\mathcal")
    {
        return true;
    }
    // Weak signal: a `^{…}` superscript only counts when the block is short and math-dominant,
    // not an inline superscript embedded in a full sentence of prose. ~keep
    if content.contains("^{") && content.contains('}') {
        return content.split_whitespace().count() <= 6;
    }
    false
}

/// Strip HTML tags from a string, preserving text content.
///
/// Handles common HTML formatting tags that appear in pandoc output or
/// ground truth. Converts `<br>` and `<br/>` to spaces.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut tag_name = String::new();

    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
            tag_name.clear();
        } else if ch == '>' && in_tag {
            in_tag = false;
            let lower = tag_name.to_lowercase();
            if lower == "br" || lower == "br/" || lower == "/br" {
                result.push(' ');
            }
        } else if in_tag {
            tag_name.push(ch);
        } else {
            result.push(ch);
        }
    }
    result
}

/// Compute reading order score using longest increasing subsequence.
/// Minimum SF1 multiplier when reading order is fully scrambled. The structural metric is defined
/// over block structure AND reading order (LIS); a perfectly-ordered document keeps its full score,
/// a fully-scrambled one is penalized by at most `1 - ORDER_SCORE_FLOOR`. Kept modest so ordering
/// refines, rather than dominates, the content-structure score.
pub(crate) const ORDER_SCORE_FLOOR: f64 = 0.8;

/// Fold the LIS reading-order score into the block-structure SF1. Skipped when fewer than three
/// blocks matched, since order is not meaningful for one or two blocks.
pub(crate) fn fold_order_into_sf1(base_sf1: f64, order_score: f64, matched_blocks: usize) -> f64 {
    if matched_blocks < 3 {
        return base_sf1;
    }
    base_sf1 * (ORDER_SCORE_FLOOR + (1.0 - ORDER_SCORE_FLOOR) * order_score)
}

pub(crate) fn compute_order_score(matches: &[(usize, usize)]) -> f64 {
    if matches.is_empty() {
        return 0.0;
    }

    let mut sorted: Vec<(usize, usize)> = matches.to_vec();
    sorted.sort_by_key(|m| m.0);

    let ext_indices: Vec<usize> = sorted.iter().map(|m| m.1).collect();
    let lis_len = longest_increasing_subsequence_length(&ext_indices);
    lis_len as f64 / matches.len() as f64
}

/// Compute the length of the longest increasing subsequence.
fn longest_increasing_subsequence_length(seq: &[usize]) -> usize {
    if seq.is_empty() {
        return 0;
    }

    let mut tails: Vec<usize> = Vec::new();
    for &val in seq {
        match tails.binary_search(&val) {
            Ok(_) => {}
            Err(pos) => {
                if pos == tails.len() {
                    tails.push(val);
                } else {
                    tails[pos] = val;
                }
            }
        }
    }
    tails.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_heading_levels() {
        let blocks = parse_markdown_blocks("# Title\n\n## Section\n\n### Subsection\n\nBody text.\n");
        assert_eq!(blocks.len(), 4);
        assert_eq!(blocks[0].block_type, MdBlockType::Heading1);
        assert_eq!(blocks[0].content, "Title");
        assert_eq!(blocks[1].block_type, MdBlockType::Heading2);
        assert_eq!(blocks[2].block_type, MdBlockType::Heading3);
        assert_eq!(blocks[3].block_type, MdBlockType::Paragraph);
    }

    #[test]
    fn parses_code_math_as_inline_math() {
        let blocks = parse_markdown_blocks("$`x^2`$\n");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].content, "x^2");
    }

    #[test]
    fn parses_code_formula_table_lists_and_images() {
        let cases = [
            ("```rust\nfn main() {}\n```", MdBlockType::CodeBlock),
            ("$$\nE = mc^2\n$$", MdBlockType::Formula),
            ("```math\nE = mc^2\n```", MdBlockType::Formula),
            ("```math display\nE = mc^2\n```", MdBlockType::Formula),
            ("| Name | Age |\n|---|---|\n| Alice | 30 |", MdBlockType::Table),
            ("- Item one", MdBlockType::ListItem),
            ("![Alt text](image.png)", MdBlockType::Image),
        ];
        for (markdown, expected) in cases {
            let blocks = parse_markdown_blocks(markdown);
            assert!(
                blocks.iter().any(|block| block.block_type == expected),
                "expected {expected} in {blocks:?}"
            );
        }
    }

    #[test]
    fn groups_paragraph_lines() {
        let blocks =
            parse_markdown_blocks("Line one of a paragraph.\nLine two of the same paragraph.\n\nNew paragraph.\n");
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].content.contains("Line one"));
        assert!(blocks[0].content.contains("Line two"));
    }

    #[test]
    fn preserves_bold_markers_in_paragraph_content() {
        let blocks = parse_markdown_blocks("**Pricing**\n\nDetails here.\n");
        let bold_block = blocks.iter().find(|block| block.content.contains("Pricing")).unwrap();
        assert_eq!(bold_block.content, "**Pricing**");
    }

    #[test]
    fn computes_longest_increasing_subsequence() {
        assert_eq!(longest_increasing_subsequence_length(&[1, 3, 2, 4, 5]), 4);
        assert_eq!(longest_increasing_subsequence_length(&[5, 4, 3, 2, 1]), 1);
        assert_eq!(longest_increasing_subsequence_length(&[1, 2, 3, 4, 5]), 5);
        assert_eq!(longest_increasing_subsequence_length(&[]), 0);
    }

    #[test]
    fn scores_reading_order() {
        assert!((compute_order_score(&[(0, 0), (1, 1), (2, 2)]) - 1.0).abs() < 0.01);
        assert!((compute_order_score(&[(0, 2), (1, 1), (2, 0)]) - 1.0 / 3.0).abs() < 0.01);
        assert_eq!(compute_order_score(&[]), 0.0);
    }

    #[test]
    fn skips_order_penalty_for_fewer_than_three_matches() {
        assert_eq!(fold_order_into_sf1(0.9, 0.0, 2), 0.9);
        assert_eq!(fold_order_into_sf1(0.9, 0.0, 1), 0.9);
        assert!((fold_order_into_sf1(1.0, 0.0, 5) - ORDER_SCORE_FLOOR).abs() < 1e-9);
        assert!((fold_order_into_sf1(1.0, 1.0, 5) - 1.0).abs() < 1e-9);
    }
}
