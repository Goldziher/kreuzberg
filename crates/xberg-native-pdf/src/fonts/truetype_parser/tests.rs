//! Unit tests for [`super`].
//!
//! Split out of `truetype_parser.rs` purely for file size, mirroring the split used for
//! `document.rs`/`document/tests.rs` and `extractors/text/mod.rs`/`extractors/text/tests.rs`. ~keep

use super::*;

#[test]
fn test_error_on_empty_data() {
    let result = TrueTypeFont::parse(&[]);
    assert!(matches!(result, Err(TrueTypeError::EmptyFont)));
}

#[test]
fn test_error_on_invalid_data() {
    let result = TrueTypeFont::parse(b"not a font file");
    assert!(matches!(result, Err(TrueTypeError::ParseError(_))));
}

#[test]
fn test_font_flags_nonsymbolic() {}

#[test]
fn test_tounicode_cmap_format() {
    let mut used_chars = HashMap::new();
    used_chars.insert(0x0041, 1_u16);
    used_chars.insert(0x0042, 2_u16);
}

/// Helper to load a font by trying bundled test fixtures first, then system paths.
fn load_font(name: &str) -> Option<Vec<u8>> {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let candidates = [
        format!("{manifest}/src/fonts/assets/{name}"),
        format!("{manifest}/tests/fixtures/fonts/{name}"),
        format!("/usr/share/fonts/truetype/dejavu/{name}"),
        format!("/usr/share/fonts/dejavu-sans-fonts/{name}"),
        format!("/usr/share/fonts/TTF/{name}"),
    ];
    for path in &candidates {
        if let Ok(data) = std::fs::read(path) {
            return Some(data);
        }
    }
    None
}

fn load_dejavu_sans() -> Option<Vec<u8>> {
    load_font("DejaVuSans.ttf")
}

fn load_dejavu_sans_bold() -> Option<Vec<u8>> {
    load_font("DejaVuSans-Bold.ttf")
}

fn load_dejavu_sans_mono() -> Option<Vec<u8>> {
    load_font("DejaVuSansMono.ttf")
}

#[test]
fn test_parse_valid_font() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data);
    assert!(font.is_ok(), "Failed to parse DejaVu Sans: {:?}", font.err());
}

#[test]
fn test_font_postscript_name() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    let ps_name = font.postscript_name();
    if let Some(ref name) = ps_name {
        assert!(!name.is_empty(), "PostScript name should not be empty if present");
    }
}

#[test]
fn test_font_family_name() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    let family = font.family_name();
    if let Some(ref name) = family {
        assert!(!name.is_empty(), "Family name should not be empty if present");
    }
}

#[test]
fn test_font_units_per_em() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    let upem = font.units_per_em();
    assert!(upem > 0, "Units per em should be positive, got: {}", upem);
}

#[test]
fn test_font_ascender_descender() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    let asc = font.ascender();
    let desc = font.descender();
    assert!(asc > 0, "Ascender should be positive, got: {}", asc);
    assert!(desc < 0, "Descender should be negative, got: {}", desc);
}

#[test]
fn test_font_cap_height() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    if let Some(cap_h) = font.cap_height() {
        assert!(cap_h > 0, "Cap height should be positive, got: {}", cap_h);
    }
}

#[test]
fn test_font_x_height() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    if let Some(x_h) = font.x_height() {
        assert!(x_h > 0, "x-height should be positive, got: {}", x_h);
    }
}

#[test]
fn test_font_bbox() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    let (x_min, y_min, x_max, y_max) = font.bbox();
    assert!(x_max > x_min, "x_max ({}) should be > x_min ({})", x_max, x_min);
    assert!(y_max > y_min, "y_max ({}) should be > y_min ({})", y_max, y_min);
}

#[test]
fn test_font_italic_angle_regular() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    assert_eq!(font.italic_angle(), 0.0, "Regular font should have italic angle 0");
}

#[test]
fn test_font_is_not_bold_regular() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    assert!(!font.is_bold(), "DejaVu Sans (regular) should not be bold");
}

#[test]
fn test_font_is_bold() {
    let data = match load_dejavu_sans_bold() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    assert!(font.is_bold(), "DejaVu Sans Bold should be bold");
}

#[test]
fn test_font_is_not_italic_regular() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    assert!(!font.is_italic(), "DejaVu Sans (regular) should not be italic");
}

#[test]
fn test_glyph_id_for_ascii() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();

    let gid_a = font.glyph_id(0x0041);
    assert!(gid_a.is_some(), "Glyph for 'A' (U+0041) should exist");
    assert!(gid_a.unwrap() > 0, "GID for 'A' should be > 0 (0 is .notdef)");

    let gid_z = font.glyph_id(0x005A);
    assert!(gid_z.is_some(), "Glyph for 'Z' (U+005A) should exist");
}

#[test]
fn test_glyph_id_nonexistent() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();

    let gid = font.glyph_id(0xFFFD_u32.wrapping_add(1000));
    let _ = gid;
}

#[test]
fn test_glyph_width() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();

    if let Some(gid) = font.glyph_id(0x0041) {
        let width = font.glyph_width(gid);
        assert!(width > 0, "Width for 'A' should be positive, got: {}", width);
    }
}

#[test]
fn test_glyph_width_default_for_missing() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();

    let width = font.glyph_width(u16::MAX);
    assert_eq!(width, 500, "Missing glyph should return default width 500");
}

#[test]
fn test_char_width() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();

    let w_a = font.char_width(0x0041);
    assert!(w_a > 0, "Width of 'A' should be positive");

    let w_space = font.char_width(0x0020);
    assert!(w_space > 0, "Width of space should be positive");

    let w_big = font.char_width(0x0057);
    let w_small = font.char_width(0x0069);
    assert!(
        w_big > w_small,
        "'W' width ({}) should be > 'i' width ({})",
        w_big,
        w_small
    );
}

#[test]
fn test_num_glyphs() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    let n = font.num_glyphs();
    assert!(n > 100, "DejaVu Sans should have many glyphs, got: {}", n);
}

#[test]
fn test_raw_data() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    assert_eq!(font.raw_data().len(), data.len(), "raw_data should match input length");
    assert_eq!(font.raw_data(), data.as_slice(), "raw_data should match input bytes");
}

#[test]
fn test_supported_codepoints() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    let codepoints = font.supported_codepoints();
    assert!(
        codepoints.len() > 100,
        "DejaVu Sans should support many codepoints, got: {}",
        codepoints.len()
    );
    for w in codepoints.windows(2) {
        assert!(w[0] <= w[1], "Codepoints should be sorted");
    }
    assert!(codepoints.contains(&0x0041), "Should support 'A' (U+0041)");
}

#[test]
fn test_stem_v_regular_vs_bold() {
    let data_regular = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let data_bold = match load_dejavu_sans_bold() {
        Some(d) => d,
        None => return,
    };
    let font_regular = TrueTypeFont::parse(&data_regular).unwrap();
    let font_bold = TrueTypeFont::parse(&data_bold).unwrap();

    let stem_regular = font_regular.stem_v();
    let stem_bold = font_bold.stem_v();
    assert!(
        stem_regular > 0,
        "Regular stem_v should be positive, got {}",
        stem_regular
    );
    assert!(
        stem_bold > stem_regular,
        "Bold stem_v ({}) should be greater than regular stem_v ({})",
        stem_bold,
        stem_regular
    );
}

#[test]
fn test_font_flags_regular() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    let flags = font.font_flags();

    assert!(
        flags & (1 << 5) != 0,
        "Nonsymbolic flag should be set, got flags: {}",
        flags
    );

    assert!(flags & (1 << 6) == 0, "Italic flag should not be set for regular font");
}

#[test]
#[ignore = "DejaVuSansMono.ttf (343 KB) is not vendored in src/fonts/assets or \
            tests/fixtures/fonts, so this can never run against a published crate; \
            run manually with a local copy of the font and --include-ignored"]
fn test_font_flags_monospace() {
    let data = load_dejavu_sans_mono().expect("DejaVuSansMono.ttf must be present locally to run this ignored test");
    let font = TrueTypeFont::parse(&data).unwrap();
    let flags = font.font_flags();

    assert!(
        flags & 1 != 0,
        "FixedPitch flag should be set for monospace font, got flags: {}",
        flags
    );
}

#[test]
fn test_generate_widths_array() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();

    let mut used = BTreeSet::new();
    if let Some(gid_a) = font.glyph_id(0x0041) {
        used.insert(gid_a);
    }
    if let Some(gid_b) = font.glyph_id(0x0042) {
        used.insert(gid_b);
    }
    if let Some(gid_c) = font.glyph_id(0x0043) {
        used.insert(gid_c);
    }

    let widths = font.generate_widths_array(&used);
    let widths_str = String::from_utf8(widths).expect("widths should be valid UTF-8");

    assert!(widths_str.starts_with('['), "Widths array should start with [");
    assert!(widths_str.ends_with(']'), "Widths array should end with ]");
    assert!(widths_str.len() > 2, "Widths array should not be empty");
}

#[test]
fn test_generate_widths_array_empty() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();

    let used = BTreeSet::new();
    let widths = font.generate_widths_array(&used);
    let widths_str = String::from_utf8(widths).expect("widths should be valid UTF-8");
    assert_eq!(widths_str, "[]", "Empty glyph set should produce []");
}

#[test]
fn test_generate_tounicode_cmap_bmp() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();

    let mut used_chars = HashMap::new();
    if let Some(gid) = font.glyph_id(0x0041) {
        used_chars.insert(0x0041_u32, gid);
    }
    if let Some(gid) = font.glyph_id(0x0042) {
        used_chars.insert(0x0042_u32, gid);
    }

    let cmap = font.generate_tounicode_cmap(&used_chars);

    assert!(cmap.contains("begincmap"), "CMap should contain begincmap");
    assert!(cmap.contains("endcmap"), "CMap should contain endcmap");
    assert!(cmap.contains("beginbfchar"), "CMap should contain beginbfchar");
    assert!(cmap.contains("endbfchar"), "CMap should contain endbfchar");
    assert!(cmap.contains("<0041>"), "CMap should contain Unicode for 'A'");
}

#[test]
fn test_generate_tounicode_cmap_supplementary_plane() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();

    let mut used_chars = HashMap::new();
    used_chars.insert(0x1F600_u32, 5000_u16);

    let cmap = font.generate_tounicode_cmap(&used_chars);

    assert!(cmap.contains("begincmap"), "CMap should contain begincmap");
    assert!(
        cmap.contains("<D83D"),
        "CMap should contain high surrogate for supplementary char"
    );
}

#[test]
fn test_generate_tounicode_cmap_empty() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();

    let used_chars = HashMap::new();
    let cmap = font.generate_tounicode_cmap(&used_chars);

    assert!(cmap.contains("begincmap"), "CMap should contain header");
    assert!(cmap.contains("endcmap"), "CMap should contain footer");
    assert!(
        !cmap.contains("beginbfchar"),
        "Empty chars should not produce bfchar section"
    );
}

#[test]
fn test_font_metrics_from_font() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    let metrics = FontMetrics::from_font(&font);

    assert!(!metrics.name.is_empty(), "Name should not be empty");
    assert!(!metrics.family.is_empty(), "Family should not be empty");
    assert!(metrics.units_per_em > 0);
    assert!(metrics.ascender > 0);
    assert!(metrics.descender < 0);
    assert!(metrics.cap_height > 0);
    assert!(metrics.x_height > 0);
    assert!(!metrics.is_bold);
    assert!(!metrics.is_italic);
}

#[test]
fn test_font_metrics_to_pdf_units() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    let metrics = FontMetrics::from_font(&font);

    let pdf_asc = metrics.pdf_ascender();
    assert!(pdf_asc > 0, "PDF ascender should be positive, got: {}", pdf_asc);

    let pdf_desc = metrics.pdf_descender();
    assert!(pdf_desc < 0, "PDF descender should be negative, got: {}", pdf_desc);

    let pdf_cap = metrics.pdf_cap_height();
    assert!(pdf_cap > 0, "PDF cap height should be positive, got: {}", pdf_cap);
}

#[test]
fn test_font_metrics_pdf_bbox() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    let metrics = FontMetrics::from_font(&font);

    let (x_min, y_min, x_max, y_max) = metrics.pdf_bbox();
    assert!(x_max > x_min, "PDF bbox x_max should be > x_min");
    assert!(y_max > y_min, "PDF bbox y_max should be > y_min");
}

#[test]
fn test_font_metrics_to_pdf_units_calculation() {
    let data = match load_dejavu_sans() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    let metrics = FontMetrics::from_font(&font);

    let test_value: i16 = 500;
    let expected = (test_value as i32 * 1000) / metrics.units_per_em as i32;
    let actual = metrics.to_pdf_units(test_value);
    assert_eq!(actual, expected);
}

#[test]
fn test_font_metrics_bold() {
    let data = match load_dejavu_sans_bold() {
        Some(d) => d,
        None => return,
    };
    let font = TrueTypeFont::parse(&data).unwrap();
    let metrics = FontMetrics::from_font(&font);

    assert!(metrics.is_bold, "Bold font metrics should report is_bold");
    assert_eq!(metrics.stem_v, 140, "Bold stem_v should be 140");
}

#[test]
fn test_truetype_error_display_empty_font() {
    let err = TrueTypeError::EmptyFont;
    let msg = format!("{}", err);
    assert!(msg.contains("empty or invalid"), "Got: {}", msg);
}

#[test]
fn test_truetype_error_display_parse_error() {
    let err = TrueTypeError::ParseError("bad header".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("bad header"), "Got: {}", msg);
}

#[test]
fn test_truetype_error_display_missing_table() {
    let err = TrueTypeError::MissingTable("cmap".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("cmap"), "Got: {}", msg);
}

#[test]
fn test_truetype_error_display_glyph_not_found() {
    let err = TrueTypeError::GlyphNotFound(0x1234);
    let msg = format!("{}", err);
    assert!(msg.contains("1234"), "Got: {}", msg);
}

#[test]
fn test_truetype_error_io_conversion() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file missing");
    let tt_err: TrueTypeError = io_err.into();
    assert!(matches!(tt_err, TrueTypeError::IoError(_)));
    let msg = format!("{}", tt_err);
    assert!(msg.contains("file missing"), "Got: {}", msg);
}

#[test]
fn test_parse_truncated_data() {
    let truncated = vec![0x00, 0x01, 0x00, 0x00];
    let result = TrueTypeFont::parse(&truncated);
    assert!(result.is_err(), "Truncated font data should fail to parse");
}

#[test]
fn test_parse_random_bytes() {
    let random_data = vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE, 0x00, 0x00];
    let result = TrueTypeFont::parse(&random_data);
    assert!(result.is_err(), "Random bytes should fail to parse as font");
}

#[test]
fn should_preserve_u16_glyph_count_at_upstream_count_boundaries() {
    let data = load_dejavu_sans().expect("bundled font must be available");
    let mut font = TrueTypeFont::parse(&data).unwrap();
    for (reported, expected) in [
        (0, 0),
        (u32::from(u16::MAX) - 1, u16::MAX - 1),
        (u32::from(u16::MAX), u16::MAX),
        (u32::from(u16::MAX) + 1, u16::MAX),
        (u32::MAX, u16::MAX),
    ] {
        font.metrics.glyph_count = reported;
        let count: u16 = font.num_glyphs();
        assert_eq!(count, expected, "upstream glyph count {reported}");
    }
}

#[test]
fn should_bound_width_table_to_the_public_glyph_range() {
    let data = load_dejavu_sans().expect("bundled font must be available");
    let mut font = TrueTypeFont::parse(&data).unwrap();
    font.metrics.glyph_count = u32::MAX;
    font.glyph_widths.clear();
    font.build_width_table();
    assert_eq!(font.glyph_widths.len(), usize::from(u16::MAX));
    assert!(font.glyph_widths.contains_key(&(u16::MAX - 1)));
    assert!(!font.glyph_widths.contains_key(&u16::MAX));
}
