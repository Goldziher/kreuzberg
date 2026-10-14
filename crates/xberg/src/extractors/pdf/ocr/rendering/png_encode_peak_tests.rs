use super::*;

#[test]
fn parallel_batch_peak_counts_every_live_page_conversion() {
    let images = [
        image::DynamicImage::ImageRgb8(image::RgbImage::new(10, 10)),
        image::DynamicImage::ImageRgb8(image::RgbImage::new(10, 10)),
    ];
    let limits = crate::extractors::security::SecurityLimits {
        max_content_size: 526_000,
        ..Default::default()
    };

    validate_png_encode_batch_peak(images.iter(), false, &limits)
        .expect("the same pages encoded sequentially must fit the between-threshold budget");
    let error = validate_png_encode_batch_peak(images.iter(), true, &limits)
        .expect_err("parallel page conversions must be budgeted together");

    assert!(matches!(error, crate::XbergError::Validation { .. }));
}

/// #1748: a batch wider than any single page's own allowance must still validate every
/// page, because [`validate_png_encode_pages_individually`] charges each one on its own
/// rather than summing the batch. Eight 10x10 pages charged in one
/// `validate_png_encode_batch_peak` call (the bug this helper replaces) peak at
/// 2,400 source bytes + 300 conversion bytes + 2,100,352 output bytes = 2,103,052,
/// which trips the 526,000 limit below and rejects every page in the batch; charged one
/// at a time each page peaks at 300 + 300 + 262,544 = 263,144, comfortably under it. The
/// three production call sites (`extract_mixed_ocr_native`'s raster-capture and
/// single-backend paths, `extract_with_ocr_for_page`'s pre-rendered-images path) all
/// delegate to this one helper now, so a fourth call site written against
/// `validate_png_encode_batch_peak` directly is the only way to reintroduce the sum.
#[test]
fn validate_pages_individually_accepts_a_batch_wider_than_one_page_allows() {
    const PAGE_COUNT: usize = 8;
    let images: Vec<image::DynamicImage> = (0..PAGE_COUNT)
        .map(|_| image::DynamicImage::ImageRgb8(image::RgbImage::new(10, 10)))
        .collect();
    let limits = crate::extractors::security::SecurityLimits {
        max_content_size: 526_000,
        ..Default::default()
    };

    // A literal batch-wide sum would trip here (peaks at 2,103,052 bytes, see above).
    validate_png_encode_pages_individually(images.iter(), &limits).expect(
        "each page must be charged against max_content_size on its own; a batch summed \
         wholesale would reject every page here, not just the ones that are actually too big",
    );
}

/// #1748: the per-page allowance must not grow just because the batch happens to be
/// wide. A single page over the limit still fails even inside an otherwise-small batch,
/// which a whole-batch-average accounting could mask.
#[test]
fn validate_pages_individually_still_rejects_one_oversized_page() {
    let images = [
        image::DynamicImage::ImageRgb8(image::RgbImage::new(10, 10)),
        image::DynamicImage::ImageRgb8(image::RgbImage::new(1000, 1000)),
    ];
    let limits = crate::extractors::security::SecurityLimits {
        max_content_size: 526_000,
        ..Default::default()
    };

    let error = validate_png_encode_pages_individually(images.iter(), &limits)
        .expect_err("the oversized second page must still be rejected on its own");
    assert!(matches!(error, crate::XbergError::Validation { .. }));
}
