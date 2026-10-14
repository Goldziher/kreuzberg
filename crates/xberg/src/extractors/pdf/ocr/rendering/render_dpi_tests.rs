use super::*;

/// Without an `ImageExtractionConfig`, a Letter page renders at the unchanged historical
/// default of 150 DPI: 8.5in * 150 = 1275px wide.
#[test]
fn render_full_pdf_ocr_batch_defaults_to_150_dpi_without_images_config() {
    let pdf = crate::pdf::render::build_minimal_pdf_with_mediabox(612.0, 792.0);
    let (doc, _page_count, page_rotations) = open_pdf_for_full_ocr(&pdf).unwrap();

    let batch = render_full_pdf_ocr_batch(
        &doc,
        &page_rotations,
        0..1,
        &crate::extractors::security::SecurityLimits::default(),
        None,
    )
    .expect("blank Letter page must render");

    assert_eq!(batch.len(), 1);
    let (_, _, width, height) = &batch[0];
    assert_eq!(*width, 1275, "8.5in at 150 DPI is 1275px wide");
    assert_eq!(*height, 1650, "11in at 150 DPI is 1650px tall");
}

/// #1786: a page that is one full-page raster (a scan) renders at the raster's own density,
/// not at the 150 default that downsamples it before the OCR preprocessor upscales it again.
/// A 400 px raster painted over a 100 pt page is 288 dpi, so the page renders 400 px wide.
#[test]
fn render_full_pdf_ocr_batch_renders_a_full_page_raster_at_its_own_density() {
    let pdf = crate::pdf::render::build_full_page_raster_pdf((100.0, 100.0), (400, 400), 1.0, 0);
    let (doc, _page_count, page_rotations) = open_pdf_for_full_ocr(&pdf).unwrap();

    let batch = render_full_pdf_ocr_batch(
        &doc,
        &page_rotations,
        0..1,
        &crate::extractors::security::SecurityLimits::default(),
        None,
    )
    .expect("a full-page raster must render");

    let (_, _, width, height) = &batch[0];
    assert_eq!(
        *width, 400,
        "a 288 dpi scan of a 100 pt page renders 400 px wide, not 208"
    );
    assert_eq!(*height, 400);
}

/// The control for the test above: the same raster covering a quarter of a page that
/// carries 20 lines of native text is a figure on a text page, and the page keeps the 150
/// dpi default (100 pt at 150 dpi is 208.3 px, which the renderer rounds up to 209).
#[test]
fn render_full_pdf_ocr_batch_keeps_the_default_dpi_for_a_page_with_a_figure() {
    let pdf = crate::pdf::render::build_full_page_raster_pdf((100.0, 100.0), (400, 400), 0.25, 20);
    let (doc, _page_count, page_rotations) = open_pdf_for_full_ocr(&pdf).unwrap();

    let batch = render_full_pdf_ocr_batch(
        &doc,
        &page_rotations,
        0..1,
        &crate::extractors::security::SecurityLimits::default(),
        None,
    )
    .expect("a figure page must render");

    let (_, _, width, _) = &batch[0];
    assert_eq!(
        *width, 209,
        "a figure covering a quarter of the page must not change the render dpi"
    );
}

/// The ceiling holds on a real page size: a Letter page carrying a 600 dpi scan renders at
/// `SCAN_PAGE_MAX_RENDER_DPI` (2550 x 3300 px) and stays inside the default security limits
/// through the raster normalization step, which is the claim the ceiling's comment makes.
#[test]
fn render_full_pdf_ocr_batch_caps_a_letter_scan_at_the_ceiling_within_default_limits() {
    let pdf = crate::pdf::render::build_full_page_raster_pdf((612.0, 792.0), (5100, 6600), 1.0, 0);
    let (doc, _page_count, page_rotations) = open_pdf_for_full_ocr(&pdf).unwrap();

    let batch = render_full_pdf_ocr_batch(
        &doc,
        &page_rotations,
        0..1,
        &crate::extractors::security::SecurityLimits::default(),
        None,
    )
    .expect("a Letter scan at the ceiling must render inside the default limits");

    let (_, _, width, height) = &batch[0];
    assert_eq!((*width, *height), (2550, 3300), "8.5 x 11 in at the 300 dpi ceiling");
}

/// #1577 still holds on a scan: a configured `target_dpi` decides, not the raster's density.
#[test]
fn render_full_pdf_ocr_batch_lets_a_configured_dpi_win_over_the_raster_density() {
    let pdf = crate::pdf::render::build_full_page_raster_pdf((100.0, 100.0), (400, 400), 1.0, 0);
    let (doc, _page_count, page_rotations) = open_pdf_for_full_ocr(&pdf).unwrap();
    let images_config = crate::core::config::ImageExtractionConfig {
        target_dpi: 72,
        auto_adjust_dpi: false,
        min_dpi: 72,
        max_dpi: 600,
        ..Default::default()
    };

    let batch = render_full_pdf_ocr_batch(
        &doc,
        &page_rotations,
        0..1,
        &crate::extractors::security::SecurityLimits::default(),
        Some(&images_config),
    )
    .expect("a full-page raster must render");

    let (_, _, width, _) = &batch[0];
    assert_eq!(
        *width, 100,
        "100 pt at the configured 72 dpi is 100 px, whatever the raster's density"
    );
}

/// The #1577 behavior: a configured `target_dpi` inside the security budget must actually
/// change the rendered pixel dimensions, not be silently ignored. Before the fix, this
/// page rendered identically regardless of `images_config`.
#[test]
fn render_full_pdf_ocr_batch_honours_configured_target_dpi() {
    let pdf = crate::pdf::render::build_minimal_pdf_with_mediabox(612.0, 792.0);
    let (doc, _page_count, page_rotations) = open_pdf_for_full_ocr(&pdf).unwrap();
    let images_config = crate::core::config::ImageExtractionConfig {
        target_dpi: 300,
        auto_adjust_dpi: false,
        min_dpi: 72,
        max_dpi: 600,
        ..Default::default()
    };

    let batch = render_full_pdf_ocr_batch(
        &doc,
        &page_rotations,
        0..1,
        &crate::extractors::security::SecurityLimits::default(),
        Some(&images_config),
    )
    .expect("blank Letter page must render");

    assert_eq!(batch.len(), 1);
    let (_, _, width, height) = &batch[0];
    assert_eq!(*width, 2550, "8.5in at 300 DPI is 2550px wide");
    assert_eq!(*height, 3300, "11in at 300 DPI is 3300px tall");
}

#[test]
fn render_full_pdf_ocr_batch_caps_dpi_to_the_png_encode_limit() {
    let _ = crate::pdf::render::take_xberg_native_pdf_render_warnings();
    let pdf = crate::pdf::render::build_minimal_pdf_with_mediabox(612.0, 792.0);
    let (doc, _page_count, page_rotations) = open_pdf_for_full_ocr(&pdf).unwrap();
    let images_config = crate::core::config::ImageExtractionConfig {
        target_dpi: 400,
        auto_adjust_dpi: false,
        min_dpi: 72,
        max_dpi: 600,
        ..Default::default()
    };

    let batch = render_full_pdf_ocr_batch(
        &doc,
        &page_rotations,
        0..1,
        &crate::extractors::security::SecurityLimits::default(),
        Some(&images_config),
    )
    .expect("the configured DPI must be lowered before the render exceeds the PNG encode limit");

    let (_, _, width, height) = &batch[0];
    assert_eq!((*width, *height), (2703, 3498), "Letter at the safe 318 DPI cap");
    let warnings = crate::pdf::render::take_xberg_native_pdf_render_warnings();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].source, "pdf-render");
    assert_eq!(
        warnings[0].message,
        "Page 1 OCR render DPI was reduced from 400 to 318 to fit security_limits.max_content_size"
    );
}

#[test]
fn render_full_pdf_ocr_batch_caps_fractional_media_box_with_renderer_dimensions() {
    let pdf = crate::pdf::render::build_minimal_pdf_with_mediabox(72.1, 72.1);
    let (doc, _page_count, page_rotations) = open_pdf_for_full_ocr(&pdf).unwrap();
    let images_config = crate::core::config::ImageExtractionConfig {
        target_dpi: 100,
        auto_adjust_dpi: false,
        min_dpi: 72,
        max_dpi: 600,
        ..Default::default()
    };
    let security_limits = crate::extractors::security::SecurityLimits {
        // At 100 DPI the renderer's ceil rule produces 101 x 101 pixels and exceeds
        // this limit. At the capped 99 DPI it produces 100 x 100 and fits exactly. ~keep
        max_content_size: 100 * 100 * 11 + 256 * 1024,
        ..Default::default()
    };

    let batch = render_full_pdf_ocr_batch(&doc, &page_rotations, 0..1, &security_limits, Some(&images_config))
        .expect("the cap must use the renderer's ceil dimensions before validating the PNG encode");

    assert_eq!(batch.len(), 1);
    let (_, _, width, height) = &batch[0];
    assert_eq!((*width, *height), (100, 100));
}

#[test]
fn render_full_pdf_ocr_batch_caps_a_legal_scan_under_default_limits() {
    let pdf = crate::pdf::render::build_full_page_raster_pdf((612.0, 1008.0), (2550, 4200), 1.0, 0);
    let (doc, _page_count, page_rotations) = open_pdf_for_full_ocr(&pdf).unwrap();

    let batch = render_full_pdf_ocr_batch(
        &doc,
        &page_rotations,
        0..1,
        &crate::extractors::security::SecurityLimits::default(),
        None,
    )
    .expect("the scan-density DPI must be lowered before a legal page exceeds the default limit");

    let (_, _, width, height) = &batch[0];
    assert_eq!((*width, *height), (2397, 3948), "Legal at the safe 282 DPI cap");
}
