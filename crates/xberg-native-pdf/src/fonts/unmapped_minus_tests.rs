use super::*;

// ~keep These independently generated PDFs embed synthetic Type2 outlines, not corpus fonts.
// The unknown opaque17 glyph varies between a math-axis bar and seven rejection controls.
const MINUS: &[u8] = include_bytes!("test_data/unmapped_minus/minus.pdf");

fn font_from_pdf(bytes: &[u8]) -> FontInfo {
    let document = PdfDocument::from_bytes(bytes.to_vec()).unwrap();
    let resources = document.get_page_resources(0).unwrap();
    let font_reference = resources
        .as_dict()
        .unwrap()
        .get("Font")
        .unwrap()
        .as_dict()
        .unwrap()
        .get("FM")
        .unwrap()
        .as_reference()
        .unwrap();
    FontInfo::from_dict(&document.load_object(font_reference).unwrap(), &document).unwrap()
}

#[test]
fn issue_2130_should_recover_unmapped_math_axis_bar_from_embedded_outline() {
    let font = font_from_pdf(MINUS);
    assert_eq!(font.base_font, "SyntheticUnmapped");
    assert_eq!(font.diff_glyph_names.get(&3).map(String::as_str), Some("opaque17"));
    assert_eq!(font.cff_gid_map.as_ref().unwrap().get(&3), Some(&1));
    assert_eq!(font.char_to_unicode(3).as_deref(), Some("−"));
    assert_eq!(font.get_byte_to_char_table()[3], '−');
}

#[test]
fn issue_2130_should_recover_other_unmapped_control_codes_without_name_assumptions() {
    let mut font = font_from_pdf(MINUS);
    font.cff_gid_map = Some(HashMap::from([(7, 1)]));
    font.diff_glyph_names = HashMap::from([(7, "another_opaque_glyph".to_string())]);
    assert_eq!(font.char_to_unicode(7).as_deref(), Some("−"));
}

#[test]
fn issue_2130_should_preserve_sign_in_native_text_and_character_extraction() {
    let document = PdfDocument::from_bytes(MINUS.to_vec()).unwrap();
    assert_eq!(document.extract_text(0).unwrap(), "−0.9393");
    let chars: String = document.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
    assert_eq!(chars, "−0.9393");
}

#[test]
fn issue_2130_should_reject_seven_nonminus_outlines() {
    let controls: [(&str, &[u8]); 7] = [
        ("underscore", include_bytes!("test_data/unmapped_minus/underscore.pdf")),
        ("macron", include_bytes!("test_data/unmapped_minus/macron.pdf")),
        ("square", include_bytes!("test_data/unmapped_minus/square.pdf")),
        ("vertical", include_bytes!("test_data/unmapped_minus/vertical.pdf")),
        ("equals", include_bytes!("test_data/unmapped_minus/equals.pdf")),
        ("plus", include_bytes!("test_data/unmapped_minus/plus.pdf")),
        ("curve", include_bytes!("test_data/unmapped_minus/curve.pdf")),
    ];
    for (shape, bytes) in controls {
        let font = font_from_pdf(bytes);
        assert_eq!(font.cff_gid_map.as_ref().unwrap().get(&3), Some(&1), "{shape}");
        assert_eq!(font.char_to_unicode(3), None, "must not invent a minus for {shape}");
    }
}

#[test]
fn issue_2130_should_preserve_explicit_tounicode_mappings() {
    for (encoded, expected) in [("002B", "+"), ("003D", "="), ("0003", "\u{FFFD}")] {
        let mut font = font_from_pdf(MINUS);
        let cmap =
            format!("1 begincodespacerange <00> <ff> endcodespacerange 1 beginbfchar <03> <{encoded}> endbfchar");
        font.to_unicode = Some(LazyCMap::new(cmap.into_bytes()));
        assert_eq!(font.char_to_unicode(3).as_deref(), Some(expected));
    }
}

#[test]
fn issue_2130_should_preserve_explicit_encoding_mappings() {
    for mapped in ['+', '=', '\u{0003}'] {
        let mut font = font_from_pdf(MINUS);
        font.encoding = Encoding::Custom(HashMap::from([(3, mapped)]));
        assert_eq!(font.char_to_unicode(3), Some(mapped.to_string()));
    }
}

#[test]
fn issue_2130_should_require_valid_embedded_font_data() {
    for data in [None, Some(Vec::new()), Some(vec![0, 1, 2, 3])] {
        let mut font = font_from_pdf(MINUS);
        font.embedded_font_data = data.map(Arc::new);
        assert_eq!(font.char_to_unicode(3), None);
    }
}

#[test]
fn issue_2130_should_require_authoritative_nonzero_cff_glyph_mapping() {
    for mapping in [
        None,
        Some(HashMap::new()),
        Some(HashMap::from([(3, 0)])),
        Some(HashMap::from([(3, u16::MAX)])),
    ] {
        let mut font = font_from_pdf(MINUS);
        font.cff_gid_map = mapping;
        assert_eq!(font.char_to_unicode(3), None);
    }
}

#[test]
fn issue_2130_should_not_override_unmapped_composite_fonts() {
    let mut font = font_from_pdf(MINUS);
    font.subtype = "Type0".to_string();
    assert_eq!(font.char_to_unicode(3).as_deref(), Some("\u{FFFD}"));
}

#[test]
fn issue_2130_should_require_explicit_unambiguous_symbolic_font_flags() {
    for flags in [None, Some(0), Some(32), Some(36)] {
        let mut font = font_from_pdf(MINUS);
        font.flags = flags;
        assert_eq!(font.char_to_unicode(3), None, "font flags {flags:?}");
    }
}

#[test]
fn issue_2130_should_not_infer_minus_for_printable_codes_or_whitespace() {
    for code in [9, 10, 13, 0x21, 0x41, 0x7f, 0x85] {
        let mut font = font_from_pdf(MINUS);
        font.encoding = Encoding::Custom(HashMap::new());
        font.cff_gid_map = Some(HashMap::from([(code, 1)]));
        assert_eq!(font.char_to_unicode(code.into()), None, "code{code}");
    }
}
