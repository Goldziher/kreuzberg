use super::extract_text_from_rtf;

#[test]
fn should_restore_formats_at_normalized_unicode_byte_offsets() {
    let (text, tables, images, metadata, formatting) = extract_text_from_rtf(r"{\rtf1 \b A{\i \u233?}B\b0}", true);
    assert_eq!(text, "Aé B");
    assert_eq!((tables.len(), images.len(), metadata.len()), (0, 0, 1));
    assert_eq!(
        formatting
            .spans
            .iter()
            .map(|span| (span.start, span.end, span.bold, span.italic))
            .collect::<Vec<_>>(),
        vec![(0, 1, true, false), (1, 3, true, true), (3, 5, true, false)]
    );
}

#[test]
fn should_restore_hidden_and_unicode_fallback_scopes() {
    let (text, tables, images, metadata, formatting) =
        extract_text_from_rtf(r"{\rtf1\uc1 A{\v Hidden}{\uc2\u1040??}B}", true);
    assert_eq!(text, "AА B");
    assert_eq!(
        (tables.len(), images.len(), metadata.len(), formatting.spans.len()),
        (0, 0, 1, 0)
    );
}

#[test]
fn should_append_footnotes_text_boxes_and_labeled_comments_in_order() {
    for plain in [false, true] {
        let (text, tables, images, metadata, formatting) = extract_text_from_rtf(
            r"{\rtf1 A{\footnote Note}{\shptxt Box}{\annotation{\*\atnid7}Comment}}",
            plain,
        );
        assert_eq!(text, "A\n\n[^1]: Note\n\nBox\n\n[Comment 7]: Comment\n\n");
        assert_eq!(
            (tables.len(), images.len(), metadata.len(), formatting.hyperlinks.len()),
            (0, 0, 1, 0)
        );
    }
}

#[test]
fn should_remap_hyperlink_and_format_spans_after_whitespace_normalization() {
    let (text, _, _, _, formatting) = extract_text_from_rtf(
        r#"{\rtf1 A {\field{\*\fldinst HYPERLINK "https://example.test"}{\fldrslt \b \u233?}} Z}"#,
        true,
    );
    assert_eq!(text, "A é Z");
    assert_eq!(formatting.hyperlinks, vec![(2, 4, "https://example.test".into())]);
    assert_eq!(
        formatting
            .spans
            .iter()
            .map(|span| (span.start, span.end, span.bold))
            .collect::<Vec<_>>(),
        vec![(0, 2, false), (2, 4, true)]
    );
}

#[test]
fn should_preserve_table_cells_placeholders_and_paragraph_metadata_in_both_modes() {
    for (plain, expected_table) in [(true, "A\tB\n"), (false, "| A | B |\n| --- | --- |\n")] {
        let (text, tables, images, metadata, _) =
            extract_text_from_rtf(r"{\rtf1 Before\par\trowd\intbl A\cell B\cell\row After}", plain);
        assert_eq!(text, "Before\n\n[TABLE_ROW]\n\nAfter");
        assert_eq!((tables.len(), images.len()), (1, 0));
        assert_eq!(tables[0].cells, vec![vec!["A", "B"]]);
        assert_eq!(tables[0].markdown, expected_table);
        assert_eq!(
            metadata
                .iter()
                .map(|paragraph| (
                    paragraph.heading_level,
                    paragraph.list_level,
                    paragraph.list_id,
                    paragraph.is_table,
                    paragraph.ordered
                ))
                .collect::<Vec<_>>(),
            vec![
                (0, None, None, false, false),
                (0, None, None, true, false),
                (0, None, None, false, false)
            ]
        );
    }
}

#[test]
fn should_preserve_primary_image_bytes_and_suppress_paired_fallback_in_both_modes() {
    for (plain, expected_text) in [(true, ""), (false, "![image](image.png)")] {
        let (text, tables, images, _, _) = extract_text_from_rtf(
            concat!(
                r"{\rtf1{\*\shppict{\pict\pngblip 89504e47}}",
                r"{\nonshppict{\pict\jpegblip ffd8ffe0}}}"
            ),
            plain,
        );
        assert_eq!(text, expected_text);
        assert_eq!(tables.len(), 0);
        assert_eq!(
            images
                .iter()
                .map(|image| (image.format, image.data.as_slice()))
                .collect::<Vec<_>>(),
            vec![("png", b"\x89PNG".as_slice())]
        );
    }
}
