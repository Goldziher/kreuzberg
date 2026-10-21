use super::extract_text_from_rtf;

/// #86: `\shptxt` (drawing-object/text-box text) is a plain destination,
/// but real producers nest it inside an ignorable `{\*\shp{\*\shpinst
/// ...}}` ancestor. Before the fix, the outer ignorable-and-unrecognized
/// destination set `skip_depth` for the whole subtree, and the nested
/// `\shptxt` group -- despite being recognized -- had no way to escape
/// that already-active skip, so its text was dropped.
#[test]
fn test_shptxt_survives_nested_ignorable_ancestor() {
    let rtf = r"{\rtf1\ansi
{\*\shp{\*\shpinst{\sp{\sn shapeType}{\sv202}}{\shptxt Text box content}}}
Body text after the shape.\par
}";
    let (text, _tables, _images, _para_metas, _fmt) = extract_text_from_rtf(rtf, false);

    assert!(
        text.contains("Text box content"),
        "text-box content should be extracted, got: {text:?}"
    );
    assert!(
        text.contains("Body text after the shape."),
        "ordinary body text should be unaffected, got: {text:?}"
    );
}

/// #86: `\annotation` (Word comment text) was previously treated as an
/// unrecognized ignorable destination and skipped whole. `\atnid` carries
/// the comment's numeric id and should label the extracted comment.
#[test]
fn test_annotation_and_atnid_are_extracted_as_labeled_comment() {
    let rtf = r"{\rtf1\ansi
Body text.\par
{\annotation{\*\atnid7}Reviewer comment here}
}";
    let (text, _tables, _images, _para_metas, _fmt) = extract_text_from_rtf(rtf, false);

    assert!(
        text.contains("[Comment 7]: Reviewer comment here"),
        "comment should be extracted and labeled with its atnid, got: {text:?}"
    );
    assert!(
        text.contains("Body text."),
        "ordinary body text should be unaffected, got: {text:?}"
    );
}

/// A comment with no `\atnid` falls back to a running 1-based counter
/// rather than being dropped or mislabeled.
#[test]
fn test_annotation_without_atnid_falls_back_to_counter_label() {
    let rtf = r"{\rtf1\ansi
{\annotation First comment}
{\annotation Second comment}
}";
    let (text, _tables, _images, _para_metas, _fmt) = extract_text_from_rtf(rtf, false);

    assert!(text.contains("[Comment 1]: First comment"), "got: {text:?}");
    assert!(text.contains("[Comment 2]: Second comment"), "got: {text:?}");
}

/// `\~` (non-breaking space) and `\_` (non-breaking hyphen) are control
/// symbols, not control words. The control-word parser found no letters in
/// them and left the `~` or `_` to be read as text, so `5\~kg` came out as
/// `5~kg`. `\-` (optional hyphen) only marks a possible break.
#[test]
fn test_control_symbols_read_as_the_characters_they_stand_for() {
    let rtf = r"{\rtf1\ansi 5\~kg, non\_stop, co\-operate\par}";
    let (text, _tables, _images, _para_metas, _fmt) = extract_text_from_rtf(rtf, false);

    assert_eq!(text.trim(), "5\u{a0}kg, non\u{2011}stop, cooperate");
}

/// An escaped `\{`, `\}` or `\\` inside a table row went to the body text
/// instead of the cell, so `set \{x\} here` filled the cell as `set x here`.
#[test]
fn test_escaped_literals_stay_in_their_table_cell() {
    let rtf = r"{\rtf1\ansi\trowd\cellx1000\cellx2000 set \{x\} here\cell C:\\tmp, 5\~kg\cell\row}";
    let (text, tables, _images, _para_metas, _fmt) = extract_text_from_rtf(rtf, false);

    assert_eq!(tables.len(), 1, "expected one table, got: {tables:?}");
    assert_eq!(
        tables[0].cells,
        vec![vec!["set {x} here".to_string(), "C:\\tmp, 5\u{a0}kg".to_string()]]
    );
    assert!(
        !text.contains('{') && !text.contains("C:\\"),
        "cell text leaked into the body: {text:?}"
    );
}
