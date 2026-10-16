//! Adobe YCCK CMYK JPEG (APP14 `color_transform = 2`) decode.
//!
//! Per Adobe convention, YCCK JPEGs store YCbCr derived from the inverted
//! CMY (i.e. `rgb_to_ycbcr(255-C, 255-M, 255-Y)`) plus an inverted K. When
//! `jpeg-decoder` 0.3 reverses the transform via `color_convert_line_ycck`,
//! its output is the **inverted** CMYK form: `255 - actual_value` per
//! channel for the first three components, and straight K. xberg-native-pdf must
//! apply `255 - x` to the first three channels (the K channel was already
//! inverted by jpeg-decoder) to recover straight CMYK.
//!
//! This differs from the APP14 `transform = 0` (plain CMYK) case, where
//! `jpeg-decoder`'s `color_convert_line_cmyk` already returns straight
//! CMYK — see `tests/test_jpeg_decoder_cmyk_contract.rs`.

use xberg_native_pdf::extractors::images::decode_cmyk_jpeg_to_rgb;

/// Build a minimal Adobe-style CMYK JPEG: a 1×1 image whose stored CMYK
/// values are all zero (so with Adobe inversion applied → pure white) and
/// whose APP14 marker flags `color_transform = 0` (Unknown = inverted CMYK).
///
/// Building a valid baseline JPEG by hand would be prohibitive, so the test
/// instead takes a tiny fixture JPEG and feeds it through
/// `decode_cmyk_jpeg_to_rgb`. The data below is a hand-written sequence of
/// JPEG markers for a single-MCU 8×8 CMYK image with all DC coefficients
/// set to 0 and Adobe APP14 color_transform = 0.
fn adobe_all_zero_cmyk_jpeg() -> Vec<u8> {
    // Rather than hand-rolling a decoder-valid bitstream, embed a
    // pre-captured Adobe CMYK JPEG from our corpus. If a future refactor
    // needs to reproduce this fixture: take any CMYK-JPEG-carrying PDF,
    // extract one `/Filter /DCTDecode /ColorSpace [/ICCBased ...]` stream,
    // and inline its bytes here.
    //
    // The fixture is a 10×11 CMYK JPEG lifted from a LaTeX-authored PDF
    // that used WPS 演示 as its producer — i.e. a real Adobe-convention
    // CMYK JPEG (APP14 color_transform = 2, inverted CMYK encoding). ~keep
    include_bytes!("fixtures/adobe_cmyk_10x11_white.jpg").to_vec()
}

#[test]
fn cmyk_jpeg_with_adobe_marker_decodes_to_bright_rgb() {
    let jpeg = adobe_all_zero_cmyk_jpeg();
    let rgb = decode_cmyk_jpeg_to_rgb(&jpeg).expect("CMYK JPEG should decode");

    for chunk in rgb.as_chunks::<3>().0 {
        assert!(
            chunk[0] > 200 && chunk[1] > 200 && chunk[2] > 200,
            "expected bright RGB from Adobe-inverted CMYK white, got {:?}",
            chunk
        );
    }
}
