use crate::extraction::image::{decode_image_to_rgb8_with_security_limits, resolve_known_source_dpi};
use std::io::Cursor;
use tiff::decoder::{Decoder, ifd::Value};
use tiff::encoder::{Rational, TiffEncoder, colortype::Gray8};
use tiff::tags::Tag;

const WIDTH: u32 = 1300;
const HEIGHT: u32 = 3000;
const DPI: f64 = 300.0;
const DPI_TOLERANCE: f64 = 0.000_001;
const PIXEL_LEVEL: u8 = 211;

fn tagged_tiff(width: u32, height: u32, x: (u32, u32), y: (u32, u32), unit: u16) -> Vec<u8> {
    let mut output = Cursor::new(Vec::new());
    {
        let mut encoder = TiffEncoder::new(&mut output).expect("valid test TIFF encoder");
        let mut frame = encoder
            .new_image::<Gray8>(width, height)
            .expect("valid test TIFF image");
        let tags = frame.encoder();
        tags.write_tag(Tag::XResolution, Rational { n: x.0, d: x.1 })
            .expect("test XResolution tag");
        tags.write_tag(Tag::YResolution, Rational { n: y.0, d: y.1 })
            .expect("test YResolution tag");
        tags.write_tag(Tag::ResolutionUnit, unit)
            .expect("test ResolutionUnit tag");
        frame
            .write_data(&vec![PIXEL_LEVEL; width as usize * height as usize])
            .expect("test grayscale pixels");
    }
    output.into_inner()
}

fn assert_declared_density(bytes: &[u8], expected_x: (u32, u32), expected_y: (u32, u32), unit: u16) {
    let mut decoder = Decoder::new(Cursor::new(bytes)).expect("encoded fixture is readable TIFF");
    assert!(matches!(
        decoder.get_tag(Tag::XResolution).expect("fixture has XResolution"),
        Value::Rational(n, d) if (n, d) == expected_x
    ));
    assert!(matches!(
        decoder.get_tag(Tag::YResolution).expect("fixture has YResolution"),
        Value::Rational(n, d) if (n, d) == expected_y
    ));
    assert_eq!(decoder.get_tag_unsigned::<u16>(Tag::ResolutionUnit).unwrap(), unit);
}

#[test]
fn should_resolve_primary_tiff_density_for_nonstandard_dimensions() {
    let bytes = tagged_tiff(WIDTH, HEIGHT, (300, 1), (300, 1), 2);
    assert_declared_density(&bytes, (300, 1), (300, 1), 2);
    let limits = crate::extractors::security::SecurityLimits::default();
    let decoded = decode_image_to_rgb8_with_security_limits(&bytes, &limits).expect("valid synthetic TIFF");
    assert_eq!(decoded.dimensions(), (WIDTH, HEIGHT));
    assert_eq!(resolve_known_source_dpi(None, &bytes, WIDTH, HEIGHT), Some(DPI));
}

#[test]
fn should_convert_tiff_centimeter_resolution_to_dpi() {
    let bytes = tagged_tiff(130, 300, (15000, 127), (15000, 127), 3);
    assert_declared_density(&bytes, (15000, 127), (15000, 127), 3);
    let actual = resolve_known_source_dpi(None, &bytes, 130, 300).expect("absolute TIFF density");
    assert!((actual - DPI).abs() < DPI_TOLERANCE, "expected 300 DPI, got {actual}");
}

#[test]
fn should_accept_equivalent_tiff_axis_rationals() {
    let bytes = tagged_tiff(130, 300, (300, 1), (600, 2), 2);
    assert_eq!(resolve_known_source_dpi(None, &bytes, 130, 300), Some(DPI));
}

#[test]
fn should_preserve_explicit_source_dpi_precedence_over_tiff_density() {
    let bytes = tagged_tiff(130, 300, (300, 1), (300, 1), 2);
    assert_eq!(resolve_known_source_dpi(Some(150.0), &bytes, 130, 300), Some(150.0));
}

#[test]
fn should_prefer_tiff_density_to_standard_page_size_inference() {
    let bytes = tagged_tiff(2550, 3300, (150, 1), (150, 1), 2);
    assert_eq!(resolve_known_source_dpi(None, &bytes, 2550, 3300), Some(150.0));
}

#[test]
fn should_leave_invalid_or_anisotropic_tiff_density_unknown() {
    let cases = [
        ((300, 1), (300, 1), 1),
        ((300, 1), (300, 1), 0),
        ((300, 1), (300, 1), 4),
        ((0, 1), (300, 1), 2),
        ((300, 0), (300, 1), 2),
        ((300, 1), (300, 0), 2),
        ((300, 1), (600, 1), 2),
    ];
    assert_eq!(cases.len(), 7);
    for (x, y, unit) in cases {
        let bytes = tagged_tiff(130, 300, x, y, unit);
        assert_eq!(
            resolve_known_source_dpi(None, &bytes, 130, 300),
            None,
            "x={x:?} y={y:?} unit={unit}"
        );
    }
}

#[test]
fn should_leave_truncated_tiff_density_unknown() {
    let bytes = tagged_tiff(130, 300, (300, 1), (300, 1), 2);
    let cuts = [0, 2, 4, 8];
    assert_eq!(cuts.len(), 4);
    for cut in cuts {
        assert_eq!(
            resolve_known_source_dpi(None, &bytes[..cut], 130, 300),
            None,
            "cut={cut}"
        );
    }
    let invalid_offset = b"II\x2a\x00\xff\xff\xff\xff";
    assert_eq!(resolve_known_source_dpi(None, invalid_offset, 130, 300), None);
}

#[test]
fn should_preserve_tagged_tiff_pixels_when_target_matches_source_density() {
    let bytes = tagged_tiff(WIDTH, HEIGHT, (300, 1), (300, 1), 2);
    let limits = crate::extractors::security::SecurityLimits::default();
    let decoded = decode_image_to_rgb8_with_security_limits(&bytes, &limits).expect("valid synthetic TIFF");
    let original = decoded.into_raw();
    let config = crate::types::ImageDpiConfig {
        target_dpi: 300,
        ..Default::default()
    };
    let result = crate::image::preprocessing::normalize_image_dpi_owned(
        original.clone(),
        WIDTH as usize,
        HEIGHT as usize,
        &config,
        resolve_known_source_dpi(None, &bytes, WIDTH, HEIGHT),
    )
    .map_err(|(error, _original)| error)
    .expect("DPI normalization succeeds");
    assert_eq!(result.dimensions, (WIDTH as usize, HEIGHT as usize));
    assert_eq!(result.metadata.original_dpi.horizontal, DPI);
    assert_eq!(result.metadata.original_dpi.vertical, DPI);
    assert_eq!(result.metadata.final_dpi, 300);
    assert_eq!(result.metadata.scale_factor, 1.0);
    assert!(result.metadata.skipped_resize);
    assert_eq!(result.rgb_data, original);
}

#[test]
fn should_scale_tagged_tiff_by_its_physical_density_at_a_lower_target() {
    let bytes = tagged_tiff(130, 300, (300, 1), (300, 1), 2);
    let limits = crate::extractors::security::SecurityLimits::default();
    let decoded = decode_image_to_rgb8_with_security_limits(&bytes, &limits).expect("valid synthetic TIFF");
    let config = crate::types::ImageDpiConfig {
        target_dpi: 150,
        max_image_dimension: 4096,
        auto_adjust_dpi: false,
        min_dpi: 72,
        max_dpi: 600,
    };
    let result = crate::image::preprocessing::normalize_image_dpi_owned(
        decoded.into_raw(),
        130,
        300,
        &config,
        resolve_known_source_dpi(None, &bytes, 130, 300),
    )
    .map_err(|(error, _original)| error)
    .expect("DPI normalization succeeds");
    assert_eq!(result.dimensions, (65, 150));
    assert_eq!(result.metadata.original_dpi.horizontal, DPI);
    assert_eq!(result.metadata.original_dpi.vertical, DPI);
    assert_eq!(result.metadata.final_dpi, 150);
    assert_eq!(result.metadata.scale_factor, 0.5);
    assert!(!result.metadata.skipped_resize);
}
