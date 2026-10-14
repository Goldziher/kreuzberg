use super::*;

#[tokio::test]
async fn test_rtf_extractor_plugin_interface() {
    let extractor = RtfExtractor::new();
    assert_eq!(extractor.name(), "rtf-extractor");
    assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
    assert!(extractor.supported_mime_types().contains(&"application/rtf"));
    assert_eq!(extractor.priority(), 50);
}

#[test]
fn test_simple_rtf_extraction() {
    let _extractor = RtfExtractor;
    let rtf_content = r#"{\rtf1 Hello World}"#;
    let (extracted, _, _, _, _) = extract_text_from_rtf(rtf_content, false);
    assert!(extracted.contains("Hello") || extracted.contains("World"));
}

#[test]
fn test_plain_text_no_image_markdown() {
    let rtf_content = r#"{\rtf1 Before image {\pict\jpegblip\picw100\pich100 ffd8ffe0} After image}"#;
    let (plain, _, _, _, _) = extract_text_from_rtf(rtf_content, true);
    assert!(!plain.contains("!["), "Plain text should not contain image markdown");
    assert!(
        !plain.contains("]("),
        "Plain text should not contain image markdown links"
    );

    let (md, _, _, _, _) = extract_text_from_rtf(rtf_content, false);
    assert!(md.contains("![image]"), "Markdown output should contain image markers");
}

#[test]
fn should_extract_shppict_primary_without_nonshppict_fallback_duplicate() {
    let rtf_content = concat!(
        r"{\rtf1",
        r"{\*\shppict{\pict\pngblip 89504e47}}",
        r"{\nonshppict{\pict\pngblip 89504e47}}",
        r"}"
    );

    let (_, _, images, _, _) = extract_text_from_rtf(rtf_content, true);

    assert_eq!(images.len(), 1);
    assert_eq!(images[0].data, b"\x89PNG");
}

#[test]
fn should_pair_shppict_fallback_across_escaped_line_break() {
    let rtf_content = concat!(
        r"{\rtf1",
        r"{\*\shppict{\pict\pngblip 89504e47}}",
        "\\\r\n  ",
        r"{\nonshppict{\pict\pngblip 89504e47}}",
        r"}"
    );

    let (_, _, images, _, _) = extract_text_from_rtf(rtf_content, true);

    assert_eq!(images.len(), 1);
    assert_eq!(images[0].data, b"\x89PNG");
}

#[test]
fn should_extract_property_bearing_shppict_without_nonshppict_fallback_duplicate() {
    let rtf_content = concat!(
        r"{\rtf1",
        r"{\*\shppict{\pict{\*\picprop{\sp{\sn wzDescription}{\sv label}}}\pngblip 89504e47}}",
        r"{\nonshppict{\pict\pngblip 89504e47}}",
        r"}"
    );

    let (_, _, images, _, _) = extract_text_from_rtf(rtf_content, true);

    assert_eq!(images.len(), 1);
    assert_eq!(images[0].data, b"\x89PNG");
}

#[test]
fn should_extract_standalone_nonshppict_image() {
    let rtf_content = r"{\rtf1{\nonshppict{\pict\pngblip 89504e47}}}";

    let (_, _, images, _, _) = extract_text_from_rtf(rtf_content, true);

    assert_eq!(images.len(), 1);
    assert_eq!(images[0].data, b"\x89PNG");
}

#[test]
fn should_use_nonshppict_fallback_when_shppict_is_not_decodable() {
    let rtf_content = concat!(
        r"{\rtf1",
        r"{\*\shppict{\pict\pngblip not-hex}}",
        r"{\nonshppict{\pict\pngblip 89504e47}}",
        r"}"
    );

    let (_, _, images, _, _) = extract_text_from_rtf(rtf_content, true);

    assert_eq!(images.len(), 1);
    assert_eq!(images[0].data, b"\x89PNG");
}

#[tokio::test]
async fn should_extract_binary_picture_payload() {
    let mut rtf = br"{\rtf1{\pict\pngblip\bin8 ".to_vec();
    rtf.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    rtf.extend_from_slice(b"}}AFTER");

    let document = RtfExtractor::new()
        .extract_content(&rtf, "application/rtf", &ExtractionConfig::default())
        .await
        .expect("RTF extraction must succeed");

    assert_eq!(document.images.len(), 1);
    assert_eq!(document.images[0].data.as_ref(), b"\x89PNG\r\n\x1a\n");
}

#[tokio::test]
async fn should_reject_binary_picture_expansion_before_exceeding_content_limit() {
    let mut rtf = br"{\rtf1{\pict\pngblip\bin8 ".to_vec();
    rtf.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    rtf.extend_from_slice(b"}}AFTER");
    let config = ExtractionConfig {
        security_limits: Some(crate::extractors::security::SecurityLimits {
            max_content_size: rtf.len(),
            ..Default::default()
        }),
        ..Default::default()
    };

    let error = RtfExtractor::new()
        .extract_content(&rtf, "application/rtf", &config)
        .await
        .expect_err("binary expansion must respect max_content_size");

    assert!(
        error.to_string().contains("Content too large"),
        "unexpected error: {error}"
    );
}

#[test]
fn should_not_expand_escaped_bin_text_as_a_binary_control_word() {
    let input = br"{\rtf1 literal \\bin4 ABCD}";
    let mut budget = SecurityBudget::from_limits(&crate::extractors::security::SecurityLimits::default());

    assert_eq!(
        expand_binary_payloads(input, &mut budget)
            .expect("escaped control word must not consume budget")
            .as_ref(),
        input
    );
}

#[test]
fn test_plain_text_table_uses_pipes() {
    let rtf_content = r#"{\rtf1 {\trowd Cell1\cell Cell2\cell\row}}"#;
    let (plain, tables, _, _, _) = extract_text_from_rtf(rtf_content, true);
    assert!(
        plain.contains("[TABLE_ROW]"),
        "Plain text should contain table row placeholder, got: {}",
        plain
    );
    assert!(!tables.is_empty(), "Tables should still be extracted");
}

#[test]
fn test_rtf_bold_formatting_extraction() {
    let rtf_content = r#"{\rtf1 Normal {\b Bold text} more normal}"#;
    let formatting = extract_rtf_formatting(rtf_content);
    let has_bold = formatting.spans.iter().any(|s| s.bold);
    assert!(has_bold, "Should detect bold formatting in RTF");
}

#[test]
fn test_rtf_italic_formatting_extraction() {
    let rtf_content = r#"{\rtf1 Normal {\i Italic text} more normal}"#;
    let formatting = extract_rtf_formatting(rtf_content);
    let has_italic = formatting.spans.iter().any(|s| s.italic);
    assert!(has_italic, "Should detect italic formatting in RTF");
}

#[test]
fn test_rtf_underline_formatting_extraction() {
    let rtf_content = r#"{\rtf1 Normal {\ul Underlined text} more normal}"#;
    let formatting = extract_rtf_formatting(rtf_content);
    let has_underline = formatting.spans.iter().any(|s| s.underline);
    assert!(has_underline, "Should detect underline formatting in RTF");
}

#[test]
fn test_rtf_color_table_extraction() {
    let rtf_content = r#"{\rtf1{\colortbl;\red255\green0\blue0;\red0\green255\blue0;}Normal {\cf1 Red text} more}"#;
    let formatting = extract_rtf_formatting(rtf_content);
    assert!(
        formatting.color_table.len() >= 2,
        "Should parse at least 2 color entries"
    );
    assert_eq!(formatting.color_table[1], "#ff0000", "First color should be red");
}

#[test]
fn test_rtf_header_footer_extraction() {
    let rtf_content = r#"{\rtf1{\header My Header Text}{\footer My Footer Text}Body text}"#;
    let formatting = extract_rtf_formatting(rtf_content);
    assert!(formatting.header_text.is_some(), "Should extract header text");
    assert!(formatting.footer_text.is_some(), "Should extract footer text");
    assert!(
        formatting.header_text.as_deref().unwrap_or("").contains("Header"),
        "Header text should contain 'Header'"
    );
    assert!(
        formatting.footer_text.as_deref().unwrap_or("").contains("Footer"),
        "Footer text should contain 'Footer'"
    );
}

#[tokio::test]
async fn test_rtf_document_structure_with_annotations() {
    let rtf_content = r#"{\rtf1 Normal text\par {\b Bold paragraph}\par More normal text}"#;
    let extractor = RtfExtractor::new();
    let config = ExtractionConfig {
        include_document_structure: true,
        ..Default::default()
    };
    let result = extractor
        .extract_content(rtf_content.as_bytes(), "application/rtf", &config)
        .await
        .expect("RTF extraction failed");
    let result =
        crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);
    assert!(result.document.is_some(), "Should produce document structure");
    let doc = result.document.unwrap();
    assert!(!doc.nodes.is_empty(), "Document structure should have nodes");
}

#[test]
fn test_bookmark_hyperlink_boundaries() {
    let rtf_content = r#"{\rtf1\ansi
\pard
{\*\bkmkstart bookmark_1}Bookmark_1{\*\bkmkend bookmark_1}
\par
\pard
{\field{\*\fldinst { HYPERLINK  \\l "bookmark_1" }}{\fldrslt{click me}}}
\par
}"#;
    let (text, _, _, _, formatting) = extract_text_from_rtf(rtf_content, false);
    assert!(!formatting.hyperlinks.is_empty(), "Should have hyperlinks");
    let (start, end, ref url) = formatting.hyperlinks[0];
    let link_text = &text[start..end];
    assert_eq!(
        link_text, "click me",
        "Hyperlink should cover 'click me', got: {:?}",
        link_text
    );
    assert_eq!(url, "#bookmark_1");
}

#[test]
fn test_rtf_ansicpg1251_hex_escape_extraction() {
    let rtf_content =
        r#"{\rtf1\ansi\ansicpg1251\deff0{\fonttbl{\f0\fnil\fcharset204 Arial;}}\f0 \'cf\'f0\'e8\'e2\'e5\'f2}"#;
    let (text, _, _, _, _) = extract_text_from_rtf(rtf_content, true);

    assert!(text.contains("Привет"), "expected readable Cyrillic, got: {text:?}");
    assert!(
        !text.contains("Ïðèâåò"),
        "should not decode CP1251 bytes as Windows-1252"
    );
}

#[test]
fn test_rtf_fcharset_overrides_ansicpg() {
    // \ansicpg1252 (Latin) but the active font declares \fcharset204 (Cyrillic).
    // The font's fcharset must win over the document-level \ansicpg.
    let rtf_content =
        r#"{\rtf1\ansi\ansicpg1252\deff0{\fonttbl{\f0\fnil\fcharset204 Arial;}}\f0 \'cf\'f0\'e8\'e2\'e5\'f2}"#;
    let (text, _, _, _, _) = extract_text_from_rtf(rtf_content, true);

    assert!(
        text.contains("Привет"),
        "fcharset204 on the active font should decode as cp1251, got: {text:?}"
    );
    assert!(
        !text.contains("Ïðèâåò"),
        "should not decode CP1251 bytes as Windows-1252 from \\ansicpg, got: {text:?}"
    );
}

#[test]
fn test_rtf_fcharset_font_switch_mid_document() {
    // f0 is Cyrillic (fcharset204), f1 is Greek (fcharset161). Each run must
    // decode per the font active at that point in the document.
    let rtf_content = r#"{\rtf1\ansi\ansicpg1252\deff0{\fonttbl{\f0\fnil\fcharset204 Arial;}{\f1\fnil\fcharset161 Arial;}}\f0 \'cf\'f0\'e8\'e2\'e5\'f2 \f1 \'e3\'e5\'e9\'e1}"#;
    let (text, _, _, _, _) = extract_text_from_rtf(rtf_content, true);

    assert!(
        text.contains("Привет"),
        "f0 run should decode via fcharset204 (cp1251), got: {text:?}"
    );
    assert!(
        text.contains("γεια"),
        "f1 run should decode via fcharset161 (cp1253), got: {text:?}"
    );
}

#[test]
fn test_rtf_fcharset_ignores_cpg_on_same_font() {
    // Per spec, \cpgN on a font entry is ignored when \fcharsetN is present.
    let rtf_content =
        r#"{\rtf1\ansi\ansicpg1252\deff0{\fonttbl{\f0\fnil\fcharset204\cpg1252 Arial;}}\f0 \'cf\'f0\'e8\'e2\'e5\'f2}"#;
    let (text, _, _, _, _) = extract_text_from_rtf(rtf_content, true);

    assert!(
        text.contains("Привет"),
        "\\fcharset204 must win over the redundant \\cpg1252 on the same font, got: {text:?}"
    );
}

#[test]
fn test_rtf_fcharset_falls_back_to_ansicpg_when_font_has_none() {
    // f0 declares no \fcharset at all, so decoding must fall back to \ansicpg1251.
    let rtf_content = r#"{\rtf1\ansi\ansicpg1251\deff0{\fonttbl{\f0\fnil Arial;}}\f0 \'cf\'f0\'e8\'e2\'e5\'f2}"#;
    let (text, _, _, _, _) = extract_text_from_rtf(rtf_content, true);

    assert!(
        text.contains("Привет"),
        "font with no \\fcharset should fall back to \\ansicpg1251, got: {text:?}"
    );
}

#[test]
fn test_rtf_fcharset_deff_alone_is_sufficient() {
    // No explicit \f0 before the text run: \deff0 alone must select the default font.
    let rtf_content =
        r#"{\rtf1\ansi\ansicpg1252\deff0{\fonttbl{\f0\fnil\fcharset204 Arial;}}\'cf\'f0\'e8\'e2\'e5\'f2}"#;
    let (text, _, _, _, _) = extract_text_from_rtf(rtf_content, true);

    assert!(
        text.contains("Привет"),
        "\\deff0 alone should select font 0's fcharset204 (cp1251), got: {text:?}"
    );
}

#[test]
fn test_rtf_ansicpg932_multibyte_run() {
    let rtf_content = r#"{\rtf1\ansi\ansicpg932\deff0{\fonttbl{\f0\fnil\fcharset128 MS Mincho;}}\f0 \'93\'fa\'96\'7b}"#;
    let (text, _, _, _, _) = extract_text_from_rtf(rtf_content, true);

    assert!(
        text.contains("日本"),
        "adjacent escapes should decode as one Shift-JIS run, got: {text:?}"
    );
    assert!(!text.contains('\u{FFFD}'), "run must not decode byte-by-byte");
}

#[test]
fn test_rtf_hex_escape_run_split_by_newline() {
    let rtf_content = "{\\rtf1\\ansi\\ansicpg932 \\'93\r\n\\'fa}";
    let (text, _, _, _, _) = extract_text_from_rtf(rtf_content, true);

    assert!(
        text.contains('日'),
        "a line wrap between the bytes of one character must not split the run, got: {text:?}"
    );
    assert!(!text.contains('\u{FFFD}'), "wrapped run must not decode as U+FFFD");
}

#[test]
fn test_rtf_ansicpg_codepage_table() {
    let cases = [
        (1251, r"\'cf\'f0\'e8\'e2\'e5\'f2", "Привет"),
        (932, r"\'93\'fa\'96\'7b", "日本"),
        (936, r"\'c4\'e3\'ba\'c3", "你好"),
        (874, r"\'a1", "ก"),
        (866, r"\'80\'81", "АБ"),
        (20866, r"\'e1\'e2", "АБ"),
        // Unknown codepage falls back to Windows-1252.
        (1717, r"\'e9", "é"),
    ];
    for (codepage, escapes, expected) in cases {
        let rtf_content = format!(r"{{\rtf1\ansi\ansicpg{codepage} {escapes}}}");
        let (text, _, _, _, _) = extract_text_from_rtf(&rtf_content, true);
        assert!(
            text.contains(expected),
            "codepage {codepage}: expected {expected:?}, got: {text:?}"
        );
    }
}

#[test]
fn test_rtf_multibyte_formatting_span_alignment() {
    let rtf_content = r#"{\rtf1\ansi\ansicpg932 before {\b \'93\'fa\'96\'7b} after}"#;
    let (text, _, _, _, formatting) = extract_text_from_rtf(rtf_content, false);
    let bold_spans: Vec<_> = formatting.spans.iter().filter(|s| s.bold).collect();
    assert!(!bold_spans.is_empty(), "Should have bold spans");
    let span = &bold_spans[0];
    let bold_text = &text[span.start..span.end];
    assert_eq!(
        bold_text, "日本",
        "Bold span should exactly cover the decoded multi-byte run, got: {bold_text:?}"
    );
}

#[test]
fn test_bold_italic_span_alignment() {
    let rtf_content = r#"{\rtf1 Normal {\b bold text} more normal}"#;
    let (text, _, _, _, formatting) = extract_text_from_rtf(rtf_content, false);
    let bold_spans: Vec<_> = formatting.spans.iter().filter(|s| s.bold).collect();
    assert!(!bold_spans.is_empty(), "Should have bold spans");
    let span = &bold_spans[0];
    let bold_text = &text[span.start..span.end];
    assert_eq!(
        bold_text, "bold text",
        "Bold span should exactly cover 'bold text', got: {:?}",
        bold_text
    );
}

#[test]
fn test_italic_span_alignment() {
    let rtf_content = r#"{\rtf1 Normal {\i italic text} more normal}"#;
    let (text, _, _, _, formatting) = extract_text_from_rtf(rtf_content, false);
    let italic_spans: Vec<_> = formatting.spans.iter().filter(|s| s.italic).collect();
    assert!(!italic_spans.is_empty(), "Should have italic spans");
    let span = &italic_spans[0];
    let italic_text = &text[span.start..span.end];
    assert_eq!(
        italic_text, "italic text",
        "Italic span should exactly cover 'italic text', got: {:?}",
        italic_text
    );
}

#[test]
fn test_bold_and_italic_span_alignment() {
    let rtf_content = r#"{\rtf1 {\b bold }{\b\i and italics} end}"#;
    let (text, _, _, _, formatting) = extract_text_from_rtf(rtf_content, false);
    let bold_spans: Vec<_> = formatting.spans.iter().filter(|s| s.bold).collect();
    assert!(!bold_spans.is_empty(), "Should have bold spans");
    let full_bold: String = bold_spans
        .iter()
        .map(|s| &text[s.start..s.end])
        .collect::<Vec<_>>()
        .join("");
    assert!(
        full_bold.contains("bold"),
        "Bold text should include 'bold', got: {:?}",
        full_bold
    );
    let bi_spans: Vec<_> = formatting.spans.iter().filter(|s| s.bold && s.italic).collect();
    assert!(!bi_spans.is_empty(), "Should have bold+italic spans");
    let bi_text = &text[bi_spans[0].start..bi_spans[0].end];
    assert_eq!(
        bi_text, "and italics",
        "Bold+italic span should cover 'and italics', got: {:?}",
        bi_text
    );
}

#[test]
fn test_formatting_with_paragraph_breaks() {
    let rtf_content = r#"{\rtf1 Normal text\par {\b bold}\par {\i italics}\par }"#;
    let (text, _, _, _, formatting) = extract_text_from_rtf(rtf_content, true);
    let bold_spans: Vec<_> = formatting.spans.iter().filter(|s| s.bold).collect();
    if !bold_spans.is_empty() {
        let span = &bold_spans[0];
        assert!(
            span.end <= text.len(),
            "Bold span end {} exceeds text length {}",
            span.end,
            text.len()
        );
        let bold_text = &text[span.start..span.end];
        assert_eq!(bold_text, "bold", "Bold span should cover 'bold', got: {:?}", bold_text);
    }
    let italic_spans: Vec<_> = formatting.spans.iter().filter(|s| s.italic).collect();
    if !italic_spans.is_empty() {
        let span = &italic_spans[0];
        assert!(
            span.end <= text.len(),
            "Italic span end {} exceeds text length {}",
            span.end,
            text.len()
        );
        let italic_text = &text[span.start..span.end];
        assert_eq!(
            italic_text, "italics",
            "Italic span should cover 'italics', got: {:?}",
            italic_text
        );
    }
}

#[test]
fn test_bookmark_hyperlink_with_formatting() {
    let rtf_content = r#"{\rtf1\ansi
\pard
{\*\bkmkstart bookmark_1}Bookmark_1{\*\bkmkend bookmark_1}
\par
\pard
{\field{\*\fldinst { HYPERLINK  \\l "bookmark_1" }}{\fldrslt {\cf1\ul click me}}}
\par
}"#;
    let (text, _, _, _, formatting) = extract_text_from_rtf(rtf_content, false);
    assert!(!formatting.hyperlinks.is_empty(), "Should have hyperlinks");
    let (start, end, ref url) = formatting.hyperlinks[0];
    let link_text = &text[start..end];
    assert_eq!(
        link_text, "click me",
        "Hyperlink should cover 'click me', got: {:?}",
        link_text
    );
    assert_eq!(url, "#bookmark_1");
}

#[test]
fn test_bookmark_hyperlink_with_extra_spaces() {
    let rtf_content = r#"{\rtf1\ansi
\pard
{\*\bkmkstart bookmark_1}Bookmark_1{\*\bkmkend bookmark_1}
\par
\pard
Some  text  before {\field{\*\fldinst { HYPERLINK  \\l "bookmark_1" }}{\fldrslt {\cf1\ul click me}}} after
\par
}"#;
    let (text, _, _, _, formatting) = extract_text_from_rtf(rtf_content, false);
    assert!(!formatting.hyperlinks.is_empty(), "Should have hyperlinks");
    let (start, end, ref url) = formatting.hyperlinks[0];
    let link_text = &text[start..end];
    assert_eq!(
        link_text, "click me",
        "Hyperlink should cover 'click me', got: {:?}",
        link_text
    );
    assert_eq!(url, "#bookmark_1");
}

#[test]
fn test_listtext_ordered_detection() {
    let rtf_content = r#"{\rtf1\ansi
\pard\ilvl0\ls1{\listtext 1.\tab}First item\par
\pard\ilvl0\ls1{\listtext 2.\tab}Second item\par
}"#;
    let (_text, _, _, metas, _) = extract_text_from_rtf(rtf_content, true);
    let ordered_items: Vec<_> = metas.iter().filter(|m| m.ordered && m.list_level.is_some()).collect();
    assert!(
        !ordered_items.is_empty(),
        "Should have ordered list items, metas: {:?}",
        metas
    );
}

#[test]
fn test_bookmark_hyperlink_end_to_end() {
    let rtf_content = r#"{\rtf1\ansi
\pard
{\*\bkmkstart bookmark_1}Bookmark_1{\*\bkmkend bookmark_1}
\par
\pard
{\field{\*\fldinst { HYPERLINK  \\l "bookmark_1" }}{\fldrslt{click me}}}
\par
}"#;
    let doc = RtfExtractor::build_internal_document(rtf_content, true);
    for elem in &doc.elements {
        for ann in &elem.annotations {
            if let crate::types::document_structure::AnnotationKind::Link { ref url, .. } = ann.kind {
                let link_text = &elem.text[ann.start as usize..ann.end as usize];
                assert_eq!(
                    link_text, "click me",
                    "E2E: Hyperlink should cover 'click me', got: {:?}",
                    link_text
                );
                assert_eq!(url, "#bookmark_1");
            }
        }
    }
}

#[test]
fn test_bookmark_hyperlink_rendered_markdown() {
    let rtf_content = r#"{\rtf1\ansi
\pard
{\*\bkmkstart bookmark_1}Bookmark_1{\*\bkmkend bookmark_1}
\par
\pard
{\field{\*\fldinst { HYPERLINK  \\l "bookmark_1" }}{\fldrslt{click me}}}
\par
}"#;
    let doc = RtfExtractor::build_internal_document(rtf_content, false);
    let rendered = crate::rendering::render_markdown(&doc);
    assert!(
        rendered.contains("[click me](#bookmark_1)"),
        "Should render as [click me](#bookmark_1), got: {:?}",
        rendered
    );
    assert!(
        !rendered.contains("[click m](#bookmark_1)e"),
        "Link boundary should not be off by one"
    );
}

#[test]
fn test_fake_doc_table_extraction() {
    let rtf_content = r#"{\rtf1\ansi\deff0
{\pard \ql \f0 \sa180 \li0 \fi0 \outlinelevel0 \b \fs36 My First Heading\par}
{\pard \ql \f0 \sa180 \li0 \fi0 My first paragraph.\par}
{\pard \sa180 \li0 \fi0 \b Table Example:\par}
{\trowd\cellx3000\cellx6000
\pard\intbl\qc\fs20 Column 1\cell Column 2\cell\row
\pard\intbl\qc\fs20 Row 1, Cell 1\cell Row 1, Cell 2\cell\row
\pard\intbl\qc\fs20 Row 2, Cell 1\cell Row 2, Cell 2\cell\row
}
}"#;
    let (_text, tables, _, metas, _) = extract_text_from_rtf(rtf_content, true);
    assert!(!tables.is_empty(), "Should extract at least one table");
    assert_eq!(
        tables[0].cells.len(),
        3,
        "Table should have 3 rows (header + 2 data), got: {:?}",
        tables[0].cells
    );
    assert!(_text.contains("Table Example:"), "Should have 'Table Example:' in text");
    let table_metas: Vec<_> = metas.iter().filter(|m| m.is_table).collect();
    assert_eq!(
        table_metas.len(),
        3,
        "Should have 3 table row metas, got: {}",
        table_metas.len()
    );
}

#[test]
fn test_multiple_tables_with_captions() {
    let rtf_content = r#"{\rtf1\ansi
{\pard First Caption\par}
{\trowd\cellx3000\cellx6000
\pard\intbl A1\cell A2\cell\row
\pard\intbl B1\cell B2\cell\row
}
{\pard Second Caption\par}
{\trowd\cellx3000\cellx6000
\pard\intbl C1\cell C2\cell\row
\pard\intbl D1\cell D2\cell\row
}
}"#;
    let (text, tables, _, _metas, _) = extract_text_from_rtf(rtf_content, true);
    assert_eq!(tables.len(), 2, "Should have 2 tables, got: {}", tables.len());
    assert_eq!(tables[0].cells.len(), 2, "First table: 2 rows");
    assert_eq!(tables[1].cells.len(), 2, "Second table: 2 rows");
    assert!(text.contains("First Caption"), "Should have 'First Caption'");
    assert!(text.contains("Second Caption"), "Should have 'Second Caption'");
}

#[test]
fn test_listtext_ignorable_ordered_detection() {
    let rtf_content = r#"{\rtf1\ansi
\pard\ilvl0\ls1{\*\listtext 1.\tab}First item\par
\pard\ilvl0\ls1{\*\listtext 2.\tab}Second item\par
}"#;
    let (_text, _, _, metas, _) = extract_text_from_rtf(rtf_content, true);
    let ordered_items: Vec<_> = metas.iter().filter(|m| m.ordered && m.list_level.is_some()).collect();
    assert!(
        !ordered_items.is_empty(),
        "Should have ordered list items with \\* prefix, metas: {:?}",
        metas
    );
}
