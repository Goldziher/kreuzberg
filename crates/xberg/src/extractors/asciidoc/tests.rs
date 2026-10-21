use super::*;
use crate::types::internal::ElementKind;

fn parse(source: &str) -> InternalDocument {
    let normalized = normalize_line_endings(source);
    AsciiDocParser::new(&normalized).parse().document
}

/// The LaTeX of every formula element, in document order.
fn formulas(source: &str) -> Vec<String> {
    parse(source)
        .elements
        .iter()
        .filter(|e| matches!(e.kind, ElementKind::Formula))
        .map(|e| e.text.clone())
        .collect()
}

/// The text of every paragraph, in document order.
fn paragraphs(source: &str) -> Vec<String> {
    parse(source)
        .elements
        .iter()
        .filter(|e| matches!(e.kind, ElementKind::Paragraph))
        .map(|e| e.text.clone())
        .collect()
}

#[test]
fn should_emit_a_latexmath_block_as_a_formula() {
    let source = "[latexmath]\n++++\n\\int_0^1 x\\,dx = \\frac{1}{2}\n++++\n";
    assert_eq!(formulas(source), vec!["\\int_0^1 x\\,dx = \\frac{1}{2}"]);
}

/// A `stem` block is AsciiMath unless the document says otherwise, so it
/// converts rather than passing through.
#[cfg(feature = "office")]
#[test]
fn should_convert_a_stem_block_from_asciimath() {
    let source = "[stem]\n++++\nsqrt(4) = 2\n++++\n";
    assert_eq!(formulas(source), vec!["\\sqrt{4}=2"]);
}

/// An `[asciimath]` block names its notation, whatever `:stem:` says.
#[cfg(feature = "office")]
#[test]
fn should_convert_an_asciimath_block_whatever_the_stem_attribute_says() {
    let source = "= Doc\n:stem: latexmath\n\n[asciimath]\n++++\nsqrt(4) = 2\n++++\n";
    assert_eq!(formulas(source), vec!["\\sqrt{4}=2"]);
}

/// Without the feature that provides the converter, an AsciiMath block keeps
/// its own notation rather than losing the mathematics.
#[cfg(not(feature = "office"))]
#[test]
fn should_keep_asciimath_source_when_the_converter_is_absent() {
    let source = "[asciimath]\n++++\nsqrt(4) = 2\n++++\n";
    assert_eq!(formulas(source), vec!["sqrt(4) = 2"]);
}

/// `:stem: latexmath` makes `stem` mean LaTeX, which passes through.
#[test]
fn should_treat_a_stem_block_as_latex_when_the_document_says_so() {
    let source = "= Doc\n:stem: latexmath\n\n[stem]\n++++\n\\alpha + \\beta\n++++\n";
    assert_eq!(formulas(source), vec!["\\alpha + \\beta"]);
}

/// A passthrough block with no math attribute is not math.
#[test]
fn should_leave_a_plain_passthrough_block_alone() {
    let source = "++++\n<hr/>\n++++\n";
    assert!(formulas(source).is_empty());
}

/// Inline math stays in the sentence, as it does for markdown, but reaches
/// the text as delimited LaTeX rather than as the raw macro.
#[test]
fn should_render_inline_math_macros_as_delimited_latex() {
    let source = "The value latexmath:[E = mc^2] holds.\n";
    assert_eq!(paragraphs(source), vec!["The value $E = mc^2$ holds."]);
    assert!(formulas(source).is_empty());
}

#[cfg(feature = "office")]
#[test]
fn should_convert_an_inline_stem_macro_from_asciimath() {
    let source = "Take stem:[sqrt(4)] as given.\n";
    assert_eq!(paragraphs(source), vec!["Take $\\sqrt{4}$ as given."]);
}

/// Macro content may hold brackets of its own.
#[test]
fn should_read_a_math_macro_containing_brackets() {
    let parsed = parse_math_macro("latexmath:[a[i] + b]rest", MathNotation::AsciiMath);
    let (consumed, source, notation) = parsed.expect("macro parses");
    assert_eq!(source, "a[i] + b");
    assert_eq!(notation, MathNotation::Latex);
    assert_eq!(&"latexmath:[a[i] + b]rest"[consumed..], "rest");
}

#[test]
fn should_map_equals_runs_to_section_levels() {
    assert_eq!(section_level("= Title"), Some(1));
    assert_eq!(section_level("=== Deep"), Some(3));
    assert_eq!(section_level("======= Too deep"), None);
    assert_eq!(section_level("=="), None);
    assert_eq!(section_level("no heading"), None);
}

#[test]
fn should_detect_list_markers_with_depth() {
    assert_eq!(list_marker("* one"), Some((1, false)));
    assert_eq!(list_marker("*** three"), Some((3, false)));
    assert_eq!(list_marker("- dash"), Some((1, false)));
    assert_eq!(list_marker(". first"), Some((1, true)));
    assert_eq!(list_marker(".. second"), Some((2, true)));
    assert_eq!(list_marker(".Block title"), None);
}

#[test]
fn should_count_columns_from_cols_spec() {
    assert_eq!(count_columns(">,<,^,"), 4);
    assert_eq!(count_columns("3*"), 3);
    assert_eq!(count_columns("^20%,<17%,>20%,<43%"), 4);
}

#[test]
fn should_push_document_title_and_headings() {
    let doc = parse("= Doc Title\n\n== Section One\n\nBody text.\n");
    assert_eq!(doc.elements[0].kind, ElementKind::Title);
    assert_eq!(doc.elements[0].text, "Doc Title");
    assert_eq!(doc.elements[1].kind, ElementKind::Heading { level: 2 });
    assert_eq!(doc.elements[1].text, "Section One");
    assert_eq!(doc.elements[2].kind, ElementKind::Paragraph);
    assert_eq!(doc.elements[2].text, "Body text.");
}

#[test]
fn should_extract_source_block_language() {
    let doc = parse("[source,rust]\n----\nfn main() {}\n----\n");
    let code = doc.elements.iter().find(|e| e.kind == ElementKind::Code).expect("code");
    assert_eq!(code.text, "fn main() {}");
    assert_eq!(
        code.attributes
            .as_ref()
            .and_then(|a| a.get("language"))
            .map(String::as_str),
        Some("rust")
    );
}

#[test]
fn should_not_panic_on_unterminated_block() {
    let doc = parse("[source,rust]\n----\nfn main() {}\n");
    assert_eq!(doc.processing_warnings.len(), 1);
    assert!(doc.elements.iter().any(|e| e.kind == ElementKind::Code));
}

/// Warnings emitted for a lossy decode specifically, identified by message content
/// since `"asciidoc"` is also the source of unrelated structural warnings (e.g. an
/// unterminated block).
fn decode_warnings(doc: &InternalDocument) -> Vec<String> {
    doc.processing_warnings
        .iter()
        .filter(|w| w.message.contains("not valid UTF-8"))
        .map(|w| w.message.to_string())
        .collect()
}

/// #395: invalid UTF-8 bytes must be reported via `processing_warnings`, not just
/// silently replaced -- `decode_with_provenance` captures the loss at decode time
/// so it survives regardless of the `quality` feature's downstream mojibake cleanup.
///
/// Deliberately not run under `quality`: there chardetng resolves these bytes to a
/// single-byte encoding that maps all of 0x00-0xFF, so nothing is *replaced* -- see
/// the identical note on `extractors::text::should_warn_when_text_source_is_not_valid_utf8`.
#[cfg(not(feature = "quality"))]
#[tokio::test]
async fn should_warn_when_asciidoc_source_is_not_valid_utf8() {
    let extractor = AsciiDocExtractor::new();
    let config = ExtractionConfig::default();
    let content: &[u8] = &[b'A', 0xFF, 0xFE, b'B'];

    let result = extractor
        .extract_content(content, "text/asciidoc", &config)
        .await
        .expect("extraction of invalid UTF-8 must still succeed");

    let warnings = decode_warnings(&result);
    assert_eq!(
        warnings.len(),
        1,
        "expected exactly one decode warning, got {warnings:?}"
    );
    assert!(
        warnings[0].contains("replacement character"),
        "warning must describe the lossy decode, got {warnings:?}"
    );
}

/// A valid UTF-8 document must not produce a lossy-decode warning.
#[tokio::test]
async fn valid_utf8_asciidoc_source_produces_zero_decode_warnings() {
    let extractor = AsciiDocExtractor::new();
    let config = ExtractionConfig::default();
    let content = "= Title\n\nBody text.\n".as_bytes();

    let result = extractor
        .extract_content(content, "text/asciidoc", &config)
        .await
        .expect("extraction should succeed");

    assert!(
        decode_warnings(&result).is_empty(),
        "valid UTF-8 must not warn about a lossy decode, got {:?}",
        decode_warnings(&result)
    );
}

#[test]
fn should_read_cross_references_as_their_text() {
    assert_eq!(
        paragraphs(
            "See <<install,the install guide>>, <<setup>> and xref:faq[the FAQ].\n\nShift: a << b >> c, x <<y and bypass:[z]."
        ),
        [
            "See the install guide, setup and the FAQ.",
            "Shift: a << b >> c, x <<y and bypass:[z]."
        ]
    );
}

#[test]
fn should_keep_passthrough_text_and_escaped_markup_literal() {
    assert_eq!(
        paragraphs(":name: value\n\nRaw pass:[<b>x</b>] and +++<i>y</i>+++, \\*not bold* and \\{name} but {name}."),
        ["Raw <b>x</b> and <i>y</i>, *not bold* and {name} but value."]
    );
}

#[test]
fn should_end_a_line_with_a_hard_break_at_a_trailing_plus() {
    assert_eq!(
        paragraphs("First line +\nsecond line\nstill second +\nthird\n\nC++ stays."),
        ["First line\nsecond line still second\nthird", "C++ stays."]
    );
}

#[test]
fn should_keep_attribute_references_in_a_passthrough_unless_it_asks_for_them() {
    assert_eq!(
        paragraphs(":name: value\n\npass:[{name}] and +++{name}+++ but {name} and pass:a[{name}]."),
        ["{name} and {name} but value and value."]
    );
}

#[test]
fn should_drop_the_backslash_of_an_escaped_reference_without_any_attributes() {
    assert_eq!(paragraphs("Literal \\{name}."), ["Literal {name}."]);
}

#[test]
fn should_leave_unterminated_references_as_written() {
    assert_eq!(
        paragraphs("<<a <<b,the text>> <<c and xref:x[y xref:z[w] and pass:[p pass:[q]."),
        ["<<a the text <<c and xref:x[y w and pass:[p q."]
    );
}

#[test]
fn should_read_many_unterminated_openers_in_linear_time() {
    // Each opener scans only up to the next one; a scan to the end per opener would take
    // billions of comparisons here
    let source = "<<a xref:b[ pass:[ ".repeat(20_000);
    let text = paragraphs(&source).concat();
    assert_eq!(text, source.trim_end());
}
