//! Tests for the image extraction module, split out of `image.rs` to keep that file
//! under the line-count limit.

#[cfg(feature = "ocr")]
#[path = "tests/tagged_tiff_source_dpi_tests.rs"]
mod tagged_tiff_source_dpi_tests;

use super::*;
use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
use std::io::Cursor;

fn create_test_image(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
    let img: RgbImage = ImageBuffer::from_fn(width, height, |x, y| {
        let r = ((x as f32 / width as f32) * 255.0) as u8;
        let g = ((y as f32 / height as f32) * 255.0) as u8;
        let b = 128;
        Rgb([r, g, b])
    });

    let mut bytes: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(&mut bytes);
    img.write_to(&mut cursor, format).unwrap();
    bytes
}

fn image_decode_limits(max_content_size: usize) -> crate::extractors::security::SecurityLimits {
    crate::extractors::security::SecurityLimits {
        max_content_size,
        ..Default::default()
    }
}

#[cfg(feature = "ocr")]
#[test]
fn should_reject_oversized_declared_dimensions_before_ocr_decode() {
    let bytes = crate::extraction::image_decode::bmp_with_declared_dimensions(100, 100);
    let limits = image_decode_limits(1024);

    let error = decode_image_with_security_limits(&bytes, &limits)
        .expect_err("oversized decoded dimensions must be rejected from the header probe");

    assert!(matches!(error, XbergError::Validation { .. }));
    assert!(error.to_string().contains("100x100"));
    assert!(error.to_string().contains("security_limits.max_content_size"));
}

#[cfg(feature = "ocr")]
#[test]
fn should_decode_normal_image_within_security_budget() {
    let bytes = create_test_image(2, 2, ImageFormat::Png);
    let limits = image_decode_limits(1024);

    let image = decode_image_with_security_limits(&bytes, &limits)
        .expect("normal image within the decoded-byte budget should load");

    assert_eq!((image.width(), image.height()), (2, 2));
}

/// GH#1554 regression: `load_image_for_ocr` hardcoded `SecurityLimits::default()`
/// instead of taking the caller's configured limits, so a legitimate high-resolution
/// scan the caller had explicitly permitted was refused anyway. A 6100x6100 solid-color
/// image decodes to 6100 * 6100 * 3 = 111,630,000 bytes, which exceeds the default
/// `max_content_size` of 100 MiB (104,857,600 bytes) but fits comfortably under a
/// caller-configured 200 MiB limit. PNG compresses a solid color to a few hundred bytes,
/// so the encoded fixture stays small even though the decoded budget does not. ~keep
#[cfg(feature = "ocr")]
#[test]
fn should_permit_high_resolution_scan_under_configured_limit_default_rejects() {
    let width = 6100;
    let height = 6100;
    let img: RgbImage = ImageBuffer::from_pixel(width, height, Rgb([200u8, 100, 50]));
    let mut bytes: Vec<u8> = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png).unwrap();

    let default_limits = SecurityLimits::default();
    let default_error = load_image_for_ocr(&bytes, &default_limits)
        .expect_err("a 111,630,000-byte decode must be refused under the 100 MiB default");
    assert!(matches!(default_error, XbergError::Validation { .. }));
    let default_message = default_error.to_string();
    assert!(
        default_message.contains("security_limits.max_content_size (104857600 bytes)"),
        "the refusal must identify the default field and ceiling actually enforced: {default_message}"
    );

    let configured_limits = SecurityLimits {
        max_content_size: 200 * 1024 * 1024,
        ..Default::default()
    };
    let image = load_image_for_ocr(&bytes, &configured_limits)
        .expect("a caller-configured 200 MiB limit must permit the same 111,630,000-byte decode");
    assert_eq!((image.width(), image.height()), (width, height));
}

#[test]
fn metadata_rejects_corrupt_pixels_within_security_budget() {
    let bytes = crate::extraction::image_decode::bmp_with_declared_dimensions(10, 10);
    let limits = image_decode_limits(10_000);

    let error = extract_image_metadata_with_security_limits(&bytes, &limits)
        .expect_err("metadata extraction must validate bounded pixel data, not only the header");

    assert!(matches!(error, XbergError::Parsing { .. }));
}

#[test]
fn test_extract_png_image_returns_correct_metadata() {
    let bytes = create_test_image(100, 80, ImageFormat::Png);
    let result = extract_image_metadata(&bytes);

    assert!(result.is_ok());
    let metadata = result.unwrap();
    assert_eq!(metadata.width, 100);
    assert_eq!(metadata.height, 80);
    assert_eq!(metadata.format, "PNG");
}

#[test]
fn test_extract_jpeg_image_returns_correct_metadata() {
    let bytes = create_test_image(200, 150, ImageFormat::Jpeg);
    let result = extract_image_metadata(&bytes);

    assert!(result.is_ok());
    let metadata = result.unwrap();
    assert_eq!(metadata.width, 200);
    assert_eq!(metadata.height, 150);
    assert_eq!(metadata.format, "JPEG");
}

#[test]
fn test_extract_webp_image_returns_correct_metadata() {
    let bytes = create_test_image(120, 90, ImageFormat::WebP);
    let result = extract_image_metadata(&bytes);

    assert!(result.is_ok());
    let metadata = result.unwrap();
    assert_eq!(metadata.width, 120);
    assert_eq!(metadata.height, 90);
    assert_eq!(metadata.format, "WEBP");
}

#[test]
fn test_extract_bmp_image_returns_correct_metadata() {
    let bytes = create_test_image(50, 50, ImageFormat::Bmp);
    let result = extract_image_metadata(&bytes);

    assert!(result.is_ok());
    let metadata = result.unwrap();
    assert_eq!(metadata.width, 50);
    assert_eq!(metadata.height, 50);
    assert_eq!(metadata.format, "BMP");
}

#[test]
fn test_extract_tiff_image_returns_correct_metadata() {
    let bytes = create_test_image(180, 120, ImageFormat::Tiff);
    let result = extract_image_metadata(&bytes);

    assert!(result.is_ok());
    let metadata = result.unwrap();
    assert_eq!(metadata.width, 180);
    assert_eq!(metadata.height, 120);
    assert_eq!(metadata.format, "TIFF");
}

#[test]
fn test_extract_gif_image_returns_correct_metadata() {
    let bytes = create_test_image(64, 64, ImageFormat::Gif);
    let result = extract_image_metadata(&bytes);

    assert!(result.is_ok());
    let metadata = result.unwrap();
    assert_eq!(metadata.width, 64);
    assert_eq!(metadata.height, 64);
    assert_eq!(metadata.format, "GIF");
}

#[test]
fn test_extract_image_extreme_aspect_ratio() {
    let bytes = create_test_image(1000, 10, ImageFormat::Png);
    let result = extract_image_metadata(&bytes);

    assert!(result.is_ok());
    let metadata = result.unwrap();
    assert_eq!(metadata.width, 1000);
    assert_eq!(metadata.height, 10);
    assert!(metadata.width / metadata.height >= 100);
}

#[test]
fn test_extract_image_dimensions_correctly() {
    let bytes = create_test_image(640, 480, ImageFormat::Png);
    let result = extract_image_metadata(&bytes);

    assert!(result.is_ok());
    let metadata = result.unwrap();
    assert_eq!(metadata.width, 640);
    assert_eq!(metadata.height, 480);
}

#[test]
fn test_extract_image_format_correctly() {
    let png_bytes = create_test_image(100, 100, ImageFormat::Png);
    let jpeg_bytes = create_test_image(100, 100, ImageFormat::Jpeg);

    let png_metadata = extract_image_metadata(&png_bytes).unwrap();
    let jpeg_metadata = extract_image_metadata(&jpeg_bytes).unwrap();

    assert_eq!(png_metadata.format, "PNG");
    assert_eq!(jpeg_metadata.format, "JPEG");
}

#[test]
fn test_extract_image_without_exif_returns_empty_map() {
    let bytes = create_test_image(100, 100, ImageFormat::Png);
    let result = extract_image_metadata(&bytes);

    assert!(result.is_ok());
    let metadata = result.unwrap();
    assert!(metadata.exif_data.is_empty());
}

#[test]
fn test_extract_exif_data_from_jpeg_with_exif() {
    let bytes = create_test_image(100, 100, ImageFormat::Jpeg);
    let result = extract_image_metadata(&bytes);

    assert!(result.is_ok());
    let metadata = result.unwrap();
    assert_eq!(metadata.exif_data.len(), 0);
}

#[test]
fn test_extract_image_metadata_invalid_returns_error() {
    let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
    let result = extract_image_metadata(&invalid_bytes);
    assert!(result.is_err());
}

#[test]
fn test_extract_image_corrupted_data_returns_error() {
    let mut bytes = create_test_image(100, 100, ImageFormat::Png);
    if bytes.len() > 50 {
        for byte in bytes.iter_mut().take(50).skip(20) {
            *byte = 0xFF;
        }
    }

    let _result = extract_image_metadata(&bytes);
}

#[test]
fn test_extract_image_empty_bytes_returns_error() {
    let empty_bytes: Vec<u8> = Vec::new();
    let result = extract_image_metadata(&empty_bytes);
    assert!(result.is_err());
}

#[test]
fn test_extract_image_unsupported_format_returns_error() {
    let unsupported_bytes = vec![0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20, 0x0D, 0x0A, 0x87, 0x0A];
    let result = extract_image_metadata(&unsupported_bytes);
    assert!(result.is_err());
}

#[test]
fn test_extract_very_small_image_1x1_pixel() {
    let bytes = create_test_image(1, 1, ImageFormat::Png);
    let result = extract_image_metadata(&bytes);

    assert!(result.is_ok());
    let metadata = result.unwrap();
    assert_eq!(metadata.width, 1);
    assert_eq!(metadata.height, 1);
    assert_eq!(metadata.format, "PNG");
}

#[test]
fn test_extract_large_image_dimensions() {
    let bytes = create_test_image(2048, 1536, ImageFormat::Png);
    let result = extract_image_metadata(&bytes);

    assert!(result.is_ok());
    let metadata = result.unwrap();
    assert_eq!(metadata.width, 2048);
    assert_eq!(metadata.height, 1536);
}

#[test]
fn test_extract_image_with_no_metadata_has_empty_exif() {
    let bytes = create_test_image(100, 100, ImageFormat::Png);
    let result = extract_image_metadata(&bytes);

    assert!(result.is_ok());
    let metadata = result.unwrap();
    assert!(metadata.exif_data.is_empty());
}

#[cfg(feature = "heic")]
#[test]
fn test_extract_image_metadata_handles_heic() {
    for (label, relative) in [
        ("heic", "images/test.heic"),
        ("heif", "images/test.heif"),
        ("avif", "images/test.avif"),
    ] {
        let Some(bytes) = crate::utils::read_test_fixture(relative) else {
            continue;
        };
        let meta = extract_image_metadata(&bytes).unwrap_or_else(|e| panic!("{label}: {e}"));
        assert!(meta.width > 0, "{label}: width should be > 0");
        assert!(meta.height > 0, "{label}: height should be > 0");
        assert_eq!(meta.format, "HEIF", "{label}: unexpected format tag");
    }
}

#[cfg(not(feature = "heic"))]
#[test]
fn test_extract_image_metadata_heic_without_feature_errors() {
    let mut heic_stub = Vec::from(&b"\x00\x00\x00\x18ftypheicheic"[..]);
    heic_stub.extend_from_slice(&[0u8; 12]);
    let err = extract_image_metadata(&heic_stub).expect_err("heic without feature should error");
    let msg = err.to_string();
    assert!(msg.contains("heic"), "expected `heic` mention in error: {msg}");
}

#[test]
fn test_extract_exif_data_returns_empty_map_for_non_jpeg() {
    let png_bytes = create_test_image(100, 100, ImageFormat::Png);
    let exif_data = extract_exif_data(&png_bytes);
    assert!(exif_data.is_empty());
}

#[test]
fn test_extract_rectangular_image_portrait_orientation() {
    let bytes = create_test_image(400, 800, ImageFormat::Jpeg);
    let result = extract_image_metadata(&bytes);

    assert!(result.is_ok());
    let metadata = result.unwrap();
    assert_eq!(metadata.width, 400);
    assert_eq!(metadata.height, 800);
    assert!(metadata.height > metadata.width);
}

#[test]
fn test_extract_rectangular_image_landscape_orientation() {
    let bytes = create_test_image(800, 400, ImageFormat::Png);
    let result = extract_image_metadata(&bytes);

    assert!(result.is_ok());
    let metadata = result.unwrap();
    assert_eq!(metadata.width, 800);
    assert_eq!(metadata.height, 400);
    assert!(metadata.width > metadata.height);
}

#[test]
fn test_extract_square_image_equal_dimensions() {
    let bytes = create_test_image(512, 512, ImageFormat::Png);
    let result = extract_image_metadata(&bytes);

    assert!(result.is_ok());
    let metadata = result.unwrap();
    assert_eq!(metadata.width, 512);
    assert_eq!(metadata.height, 512);
    assert_eq!(metadata.width, metadata.height);
}

#[test]
fn test_extract_metadata_preserves_format_case() {
    let png_bytes = create_test_image(100, 100, ImageFormat::Png);
    let jpeg_bytes = create_test_image(100, 100, ImageFormat::Jpeg);
    let webp_bytes = create_test_image(100, 100, ImageFormat::WebP);

    let png_meta = extract_image_metadata(&png_bytes).unwrap();
    let jpeg_meta = extract_image_metadata(&jpeg_bytes).unwrap();
    let webp_meta = extract_image_metadata(&webp_bytes).unwrap();

    assert_eq!(png_meta.format, "PNG");
    assert_eq!(jpeg_meta.format, "JPEG");
    assert_eq!(webp_meta.format, "WEBP");
}

#[test]
fn test_jp2_magic_detection() {
    assert!(is_jp2(&[0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20, 0x0D, 0x0A]));
    assert!(!is_jp2(&[0x89, 0x50, 0x4E, 0x47]));
    assert!(!is_jp2(&[0x00, 0x00]));
    assert!(!is_jp2(&[]));
}

#[test]
fn test_extract_jp2_rust_logo_metadata() {
    let Some(bytes) = crate::utils::read_test_fixture("images/rust-logo-512x512-blk.jp2") else {
        return;
    };
    let result = extract_image_metadata(&bytes);
    assert!(result.is_ok(), "Failed to extract JP2 metadata: {:?}", result.err());
    let metadata = result.unwrap();
    assert_eq!(metadata.width, 512);
    assert_eq!(metadata.height, 512);
    assert_eq!(metadata.format, "JPEG2000");
}

#[test]
fn test_extract_jp2_hadley_crater_metadata() {
    let Some(bytes) = crate::utils::read_test_fixture("images/Hadley_Crater.jp2") else {
        return;
    };
    let result = extract_image_metadata(&bytes);
    assert!(result.is_ok(), "Failed to extract JP2 metadata: {:?}", result.err());
    let metadata = result.unwrap();
    assert!(metadata.width > 0);
    assert!(metadata.height > 0);
    assert_eq!(metadata.format, "JPEG2000");
}

#[test]
fn test_parse_jp2_boxes_invalid_data() {
    let invalid = vec![0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20, 0x0D, 0x0A, 0x87, 0x0A];
    let result = decode_jp2_metadata(&invalid);
    assert!(result.is_err());
}

#[test]
fn test_jp2_magic_detection_comprehensive() {
    assert!(is_jp2(&[
        0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20, 0x0D, 0x0A, 0x87, 0x0A
    ]));
    assert!(!is_jp2(&[0xFF, 0x4F, 0xFF, 0x51]));
    assert!(!is_jp2(&[0x89, 0x50, 0x4E, 0x47]));
    assert!(!is_jp2(&[]));
}

// GH#1630: PNG pHYs density detection.
#[cfg(feature = "ocr")]
mod png_density {
    use super::*;
    use image::ImageFormat;

    /// Standard PNG CRC-32 (polynomial 0xEDB88320), needed to splice a well-formed `pHYs`
    /// chunk into a real encoded PNG for the test fixtures below.
    fn png_crc32(data: &[u8]) -> u32 {
        let mut crc: u32 = 0xFFFF_FFFF;
        for &byte in data {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
            }
        }
        !crc
    }

    fn png_chunk(chunk_type: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut body = Vec::with_capacity(4 + data.len());
        body.extend_from_slice(chunk_type);
        body.extend_from_slice(data);
        let mut out = Vec::with_capacity(4 + body.len() + 4);
        out.extend_from_slice(&(u32::try_from(data.len()).unwrap()).to_be_bytes());
        out.extend_from_slice(&body);
        out.extend_from_slice(&png_crc32(&body).to_be_bytes());
        out
    }

    /// Splice a `pHYs` chunk expressing `ppu` pixels-per-metre (both axes) into a real
    /// encoded PNG, immediately after IHDR (signature 8 bytes + IHDR chunk 25 bytes), which
    /// is where an encoder conventionally places it and always before IDAT.
    fn png_with_phys(png_bytes: &[u8], ppu: u32) -> Vec<u8> {
        const SIGNATURE_AND_IHDR_LEN: usize = 8 + 25;
        let mut data = Vec::with_capacity(9);
        data.extend_from_slice(&ppu.to_be_bytes());
        data.extend_from_slice(&ppu.to_be_bytes());
        data.push(1); // unit = meter
        let phys = png_chunk(b"pHYs", &data);

        let mut out = Vec::with_capacity(png_bytes.len() + phys.len());
        out.extend_from_slice(&png_bytes[..SIGNATURE_AND_IHDR_LEN]);
        out.extend_from_slice(&phys);
        out.extend_from_slice(&png_bytes[SIGNATURE_AND_IHDR_LEN..]);
        out
    }

    /// 11811 px/m is the reporter's fixture value: 11811 * 0.0254 = 299.9994 DPI, not an
    /// exact 300 — the detector and any consumer must tolerate that rather than requiring an
    /// exact integer match.
    const ELEVEN_THOUSAND_EIGHT_HUNDRED_ELEVEN_PPU: u32 = 11811;
    const EXPECTED_DPI_FROM_11811_PPU: f64 = 299.999_4;
    const DPI_TOLERANCE: f64 = 0.001;

    #[test]
    fn should_decode_source_density_from_png_phys_chunk() {
        let png = create_test_image(4, 4, ImageFormat::Png);
        let with_phys = png_with_phys(&png, ELEVEN_THOUSAND_EIGHT_HUNDRED_ELEVEN_PPU);

        let dpi = png_pixel_density_dpi(&with_phys).expect("pHYs chunk must be detected");

        assert!(
            (dpi - EXPECTED_DPI_FROM_11811_PPU).abs() < DPI_TOLERANCE,
            "expected ~{EXPECTED_DPI_FROM_11811_PPU} DPI, got {dpi}"
        );
    }

    #[test]
    fn should_return_none_for_png_without_phys_chunk() {
        let png = create_test_image(4, 4, ImageFormat::Png);

        assert_eq!(
            png_pixel_density_dpi(&png),
            None,
            "a PNG with no embedded density metadata must not report a fabricated DPI"
        );
    }

    #[test]
    fn should_return_none_for_non_png_bytes() {
        assert_eq!(png_pixel_density_dpi(b"not a png"), None);
        assert_eq!(png_pixel_density_dpi(&[]), None);
    }

    #[test]
    fn should_return_none_for_unspecified_phys_unit() {
        let png = create_test_image(4, 4, ImageFormat::Png);
        let mut with_phys = png_with_phys(&png, ELEVEN_THOUSAND_EIGHT_HUNDRED_ELEVEN_PPU);
        // Flip the unit byte (last byte of the 9-byte pHYs data) from meter (1) to
        // unspecified (0): signature(8) + IHDR(25) + pHYs length/type(8) + xppu/yppu(8).
        let unit_byte_offset = 8 + 25 + 8 + 8;
        with_phys[unit_byte_offset] = 0;

        assert_eq!(png_pixel_density_dpi(&with_phys), None);
    }

    // #1788: infer a source DPI from pixel dimensions matching a standard page size when no
    // embedded density tag exists, instead of always assuming 72 DPI.
    mod page_size_dpi_inference {
        use super::*;

        /// The reporter's own dimensions: US Letter (8.5in x 11in) at exactly 300 DPI.
        const LETTER_300_DPI_WIDTH: u32 = 2550;
        const LETTER_300_DPI_HEIGHT: u32 = 3300;

        #[test]
        fn should_infer_300_dpi_for_letter_sized_pixel_dimensions() {
            let dpi = infer_dpi_from_standard_page_size(LETTER_300_DPI_WIDTH, LETTER_300_DPI_HEIGHT)
                .expect("2550x3300 must match US Letter at 300 DPI");

            assert!((dpi - 300.0).abs() < 0.5, "expected ~300 DPI, got {dpi}");
        }

        #[test]
        fn should_infer_the_same_dpi_in_landscape_orientation() {
            let dpi = infer_dpi_from_standard_page_size(LETTER_300_DPI_HEIGHT, LETTER_300_DPI_WIDTH)
                .expect("a rotated (landscape) letter page must still match");

            assert!((dpi - 300.0).abs() < 0.5, "expected ~300 DPI, got {dpi}");
        }

        /// Negative control: pixel dimensions that match no standard page size at any plausible
        /// scan resolution must not have a DPI invented for them.
        #[test]
        fn should_return_none_for_dimensions_matching_no_standard_page_size() {
            assert_eq!(
                infer_dpi_from_standard_page_size(4, 4),
                None,
                "a tiny fixture image must not be misread as a page at some implausible DPI"
            );
            assert_eq!(
                infer_dpi_from_standard_page_size(1920, 1080),
                None,
                "a 16:9 screenshot's aspect ratio does not match any standard page size"
            );
        }

        #[test]
        fn should_return_none_for_zero_dimensions() {
            assert_eq!(infer_dpi_from_standard_page_size(0, 100), None);
            assert_eq!(infer_dpi_from_standard_page_size(100, 0), None);
        }

        /// End-to-end through `resolve_known_source_dpi`: a PNG with no `pHYs` chunk, whose
        /// caller-supplied pixel dimensions are letter-at-300-DPI, must resolve to ~300 DPI
        /// instead of falling through to the historical 72 DPI assumption (#1788).
        #[test]
        fn resolve_known_source_dpi_infers_letter_size_when_no_metadata_tag_is_present() {
            let untagged_png = create_test_image(4, 4, ImageFormat::Png);

            let resolved = resolve_known_source_dpi(None, &untagged_png, LETTER_300_DPI_WIDTH, LETTER_300_DPI_HEIGHT)
                .expect("letter-sized pixel dimensions must resolve a DPI even with no pHYs chunk");

            assert!((resolved - 300.0).abs() < 0.5, "expected ~300 DPI, got {resolved}");
        }

        /// Negative control: a PNG that DOES carry a resolution tag must be unaffected by the
        /// new inference tier — the embedded tag still wins, exactly as before this fix, even
        /// when it disagrees with what page-size inference would have guessed.
        #[test]
        fn resolve_known_source_dpi_prefers_an_embedded_density_tag_over_page_size_inference() {
            let png = create_test_image(4, 4, ImageFormat::Png);
            // 72 DPI tag (2835 px/m, rounded), deliberately mismatched from the letter-at-300
            // pixel dimensions passed below, so a regression that ignores the tag and falls
            // through to inference is observable.
            const SEVENTY_TWO_DPI_PPU: u32 = 2835;
            let tagged = png_with_phys(&png, SEVENTY_TWO_DPI_PPU);

            let resolved = resolve_known_source_dpi(None, &tagged, LETTER_300_DPI_WIDTH, LETTER_300_DPI_HEIGHT)
                .expect("pHYs chunk must be detected");

            assert!(
                (resolved - 72.0).abs() < 1.0,
                "an explicit resolution tag must win over page-size inference, got {resolved}"
            );
        }

        /// Dimensions matching no standard page size must still fall through cleanly to `None`
        /// (the historical 72 DPI assumption downstream), not an invented DPI.
        #[test]
        fn resolve_known_source_dpi_stays_none_for_non_page_shaped_dimensions() {
            let screenshot_shaped_png = create_test_image(4, 4, ImageFormat::Png);

            assert_eq!(
                resolve_known_source_dpi(None, &screenshot_shaped_png, 1920, 1080),
                None,
                "dimensions matching no standard page size must not have a DPI invented for them"
            );
        }
    }
}
