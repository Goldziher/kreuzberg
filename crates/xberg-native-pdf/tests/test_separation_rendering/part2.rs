/// Spot-colour text on a plate. The test font is the standard
/// Helvetica (built-in PDF font), so a system font with the same
/// metrics is needed — we just verify *some* ink appears on the
/// plate inside the text region. Without text rendering this
/// region would be all-zero.
fn build_text_in_separation_pdf() -> Vec<u8> {
    let content = "BT\n/F1 24 Tf\n20 50 Td\n/CS1 cs\n0.9 scn\n(SPOT) Tj\nET\n";
    let content_obj = format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content);
    assemble_pdf(vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << /Font << /F1 6 0 R >> /ColorSpace << /CS1 5 0 R >> >> >>".to_string(),
            content_obj,
            "[/Separation /TextSpot /DeviceGray 7 0 R]".to_string(),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
            "<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>".to_string(),
        ])
}

#[test]
fn text_with_spot_color_appears_on_plate() {
    let pdf_bytes = build_text_in_separation_pdf();
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");
    let plate = render_separation(&doc, 0, "TextSpot", 144).expect("render plate");

    let inked = plate.data.iter().filter(|&&v| v > 180).count();
    assert!(
        inked > 100,
        "Spot-colour text should leave at least some inked pixels on the plate, got {}",
        inked
    );

    let cyan = render_separation(&doc, 0, "Cyan", 144).expect("render Cyan");
    let cyan_inked = cyan.data.iter().filter(|&&v| v > 50).count();
    assert_eq!(
        cyan_inked, 0,
        "Spot-only text should not appear on Cyan plate, got {}",
        cyan_inked
    );
}

#[test]
fn cmyk_text_routes_to_process_plates() {
    // Pure cyan text via `k` operator. The Cyan plate must show
    // ink in the text region; Black plate must remain empty. ~keep
    let content = "BT\n/F1 24 Tf\n20 50 Td\n0.8 0.0 0.0 0.0 k\n(CYAN) Tj\nET\n";
    let content_obj = format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content);
    let pdf_bytes = assemble_pdf(vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
            content_obj,
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        ]);
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

    let cyan = render_separation(&doc, 0, "Cyan", 144).expect("render Cyan");
    let cyan_inked = cyan.data.iter().filter(|&&v| v > 150).count();
    assert!(
        cyan_inked > 100,
        "CMYK text should paint to Cyan plate (got {} bright pixels)",
        cyan_inked
    );

    let black = render_separation(&doc, 0, "Black", 144).expect("render Black");
    let black_inked = black.data.iter().filter(|&&v| v > 50).count();
    assert_eq!(
        black_inked, 0,
        "Pure-cyan text should not appear on Black plate, got {}",
        black_inked
    );
}

// =====================================================================
// §8.6.6.4: Reserved colorant names /All and /None
// ===================================================================== ~keep

/// Build a PDF with a Separation `/All` colorant (registration marks).
/// Paints a small rect with `/All` at full tint, plus a separate Pantone
/// rect on a known spot ink so the test can verify CMYK plates exist.
fn build_pdf_with_all_separation() -> Vec<u8> {
    let content = b"/CS1 cs 1.0 scn 10 10 30 30 re f \
                        /CS2 cs 1.0 scn 60 60 30 30 re f";
    build_pdf_two_separations(b"All", b"PANTONE", content)
}

fn build_pdf_with_none_separation() -> Vec<u8> {
    let content = b"/CS1 cs 1.0 scn 10 10 30 30 re f \
                        /CS2 cs 1.0 scn 60 60 30 30 re f";
    build_pdf_two_separations(b"None", b"PANTONE", content)
}

fn build_pdf_two_separations(ink_a: &[u8], ink_b: &[u8], content_stream: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut offsets = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    offsets.push(buf.len());
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
              /Resources << /ColorSpace << /CS1 5 0 R /CS2 6 0 R >> >> >>\nendobj\n",
    );

    offsets.push(buf.len());
    let stream_header = format!("4 0 obj\n<< /Length {} >>\nstream\n", content_stream.len());
    buf.extend_from_slice(stream_header.as_bytes());
    buf.extend_from_slice(content_stream);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    offsets.push(buf.len());
    let cs1 = format!(
        "5 0 obj\n[/Separation /{} /DeviceGray 7 0 R]\nendobj\n",
        std::str::from_utf8(ink_a).unwrap()
    );
    buf.extend_from_slice(cs1.as_bytes());

    offsets.push(buf.len());
    let cs2 = format!(
        "6 0 obj\n[/Separation /{} /DeviceGray 7 0 R]\nendobj\n",
        std::str::from_utf8(ink_b).unwrap()
    );
    buf.extend_from_slice(cs2.as_bytes());

    offsets.push(buf.len());
    buf.extend_from_slice(b"7 0 obj\n<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>\nendobj\n");

    let xref_offset = buf.len();
    buf.extend_from_slice(b"xref\n");
    let line = format!("0 {}\n", offsets.len() + 1);
    buf.extend_from_slice(line.as_bytes());
    buf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets {
        let entry = format!("{:010} 00000 n \n", offset);
        buf.extend_from_slice(entry.as_bytes());
    }
    let trailer = format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        offsets.len() + 1,
        xref_offset
    );
    buf.extend_from_slice(trailer.as_bytes());
    buf
}

/// Helper: sample the center of a 30x30 rect at (10,10) at 144 DPI.
/// Page is 100x100 pt -> 200x200 px. Rect at (10,10)-(40,40) pt ->
/// (20,20)-(80,80) px, center ~ (50,50). PDF y-axis is bottom-up so we
/// sample at (height - py) on the buffer.
fn sample_plate_at(plate: &xberg_native_pdf::rendering::SeparationPlate, x_pt: f32, y_pt: f32) -> u8 {
    let scale = plate.width as f32 / 100.0;
    let px = (x_pt * scale) as u32;
    let py = (y_pt * scale) as u32;
    let idx = ((plate.height - 1 - py) * plate.width + px) as usize;
    plate.data[idx]
}

#[test]
fn test_all_separation_paints_every_plate() {
    // §8.6.6.4: /All is a reserved colorant — content painted with
    // /Separation /All should appear on every output plate (CMYK + spots). ~keep
    let doc = PdfDocument::from_bytes(build_pdf_with_all_separation()).unwrap();

    // get_page_inks must not list "All" as a plate name. ~keep
    let inks = doc.get_page_inks(0).unwrap();
    assert!(!inks.contains(&"All".to_string()), "got: {:?}", inks);
    assert!(inks.contains(&"PANTONE".to_string()), "got: {:?}", inks);

    let plates = render_separations(&doc, 0, 144).unwrap();
    let names: Vec<&str> = plates.iter().map(|p| p.ink_name.as_str()).collect();
    assert!(!names.contains(&"All"), "no plate named 'All' should be materialized");

    for plate in &plates {
        let v = sample_plate_at(plate, 25.0, 25.0);
        assert!(
            v > 200,
            "/All content missing from {} plate (got value {})",
            plate.ink_name,
            v
        );
    }

    let pantone = plates.iter().find(|p| p.ink_name == "PANTONE").unwrap();
    assert!(sample_plate_at(pantone, 75.0, 75.0) > 200);
    let cyan = plates.iter().find(|p| p.ink_name == "Cyan").unwrap();
    assert!(
        sample_plate_at(cyan, 75.0, 75.0) < 50,
        "PANTONE-only rect should not be on Cyan plate"
    );
}

#[test]
fn test_none_separation_paints_nothing() {
    // §8.6.6.4: /None is a reserved colorant — produces no marks anywhere
    // and never names a plate. ~keep
    let doc = PdfDocument::from_bytes(build_pdf_with_none_separation()).unwrap();

    let inks = doc.get_page_inks(0).unwrap();
    assert!(!inks.contains(&"None".to_string()), "got: {:?}", inks);

    let plates = render_separations(&doc, 0, 144).unwrap();
    let names: Vec<&str> = plates.iter().map(|p| p.ink_name.as_str()).collect();
    assert!(!names.contains(&"None"), "no plate named 'None' should be materialized");

    // The /None rect must not appear on any plate. ~keep
    for plate in &plates {
        let v = sample_plate_at(plate, 25.0, 25.0);
        assert!(
            v < 50,
            "/None content leaked onto {} plate (got value {})",
            plate.ink_name,
            v
        );
    }

    let pantone = plates.iter().find(|p| p.ink_name == "PANTONE").unwrap();
    assert!(sample_plate_at(pantone, 75.0, 75.0) > 200);
}

// =====================================================================
// SPEC GAPS — foundation tests for the next iteration.
//
// These tests document spec-compliance gaps in the v1 separation
// renderer. They are #[ignore]'d so the suite passes; each one names
// the relevant ISO 32000-1 section and what real-world PDF feature it
// covers. Removing the #[ignore] after the underlying gap is fixed
// turns them into passing regression tests.
// ===================================================================== ~keep

/// §11.7.4 — Overprint. With OP=true painting a Separation ink on top of
/// CMYK content adds to the spot plate while leaving the underlying CMYK
/// plates unchanged. Without overprint the spot paint knocks out the CMYK
/// at those pixels (covered by the spec-default tests in
/// `tests/test_separation_overprint.rs`).
#[test]
fn test_overprint_preserves_underlying_ink() {
    // K rect 50% under, Separation rect on top with OP=true via /GS1.
    // Spec: spot paint with overprint preserves underlying K plate.
    // Current: spot paint knocks out K (last-writer-wins per pixel). ~keep
    let content = "0 0 0 0.5 k\n20 20 60 60 re f\n\
                       /GS1 gs /CS1 cs 1.0 scn\n30 30 40 40 re f\n";
    let content_obj = format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content);
    let pdf = assemble_pdf(vec![
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
              /Resources << /ColorSpace << /CS1 5 0 R >> /ExtGState << /GS1 7 0 R >> >> >>"
            .into(),
        content_obj,
        "[/Separation /SpotInk /DeviceGray 6 0 R]".into(),
        "<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>".into(),
        "<< /Type /ExtGState /OP true /op true /OPM 1 >>".into(),
    ]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let plates = render_separations(&doc, 0, 144).unwrap();

    let black = plates.iter().find(|p| p.ink_name == "Black").unwrap();
    let bw = black.width as usize;
    let center = black.data[(100 * bw) + 100];
    assert!(
        (100..=140).contains(&center),
        "Black plate at overlap should retain ~50% tint under overprinted spot, got {}",
        center
    );

    let spot = plates.iter().find(|p| p.ink_name == "SpotInk").unwrap();
    let sw = spot.width as usize;
    assert!(
        spot.data[(100 * sw) + 100] > 200,
        "SpotInk plate should show the overprinted ink"
    );
}

/// §9.3.6 — Text rendering mode 3 = "neither fill nor stroke (invisible)".
/// Advances the text matrix but contributes no pixels to any plate.
/// Common in tagged/searchable PDFs where invisible text underlies an
/// image of scanned content. Regression test for already-working behavior.
#[test]
fn test_render_mode_3_invisible_text_no_plate_pixels() {
    let content = "/CS1 cs 1.0 scn\n\
                       BT /F1 24 Tf 3 Tr 20 50 Td (HIDDEN) Tj ET\n";
    let content_obj = format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content);
    let pdf = assemble_pdf(vec![
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R \
              /Resources << /ColorSpace << /CS1 5 0 R >> \
                           /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> >>"
            .into(),
        content_obj,
        "[/Separation /Watermark /DeviceGray 6 0 R]".into(),
        "<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>".into(),
    ]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let watermark = render_separation(&doc, 0, "Watermark", 144).unwrap();
    let inked = watermark.data.iter().filter(|&&v| v > 50).count();
    assert_eq!(
        inked, 0,
        "Tr 3 (invisible) text must not contribute pixels to the Watermark plate, got {} inked",
        inked
    );
}

/// §9.3.6 — Text rendering modes 1, 2, 5, 6 stroke the glyphs. Stroke
/// color comes from the stroke color space + components, not fill.
/// A separation with `Tr 1 (text)` should route to the stroke ink's plate.
#[test]
#[ignore = "spec gap: stroke-mode text routes to fill color, not stroke"]
fn test_stroke_mode_text_uses_stroke_color() {
    let content = "/CS1 cs 1.0 scn\n/CS2 CS 1.0 SCN\n\
                       BT /F1 24 Tf 1 Tr 20 50 Td (X) Tj ET\n";
    let content_obj = format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content);
    let pdf = assemble_pdf(vec![
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
              /Resources << /ColorSpace << /CS1 5 0 R /CS2 6 0 R >> \
                           /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> >>"
            .into(),
        content_obj,
        "[/Separation /FillInk /DeviceGray 7 0 R]".into(),
        "[/Separation /StrokeInk /DeviceGray 7 0 R]".into(),
        "<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>".into(),
    ]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let stroke_plate = render_separation(&doc, 0, "StrokeInk", 144).unwrap();
    let stroke_inked = stroke_plate.data.iter().filter(|&&v| v > 50).count();
    assert!(
        stroke_inked > 100,
        "StrokeInk plate should receive the Tr 1 stroked glyph, got {} inked",
        stroke_inked
    );
    let fill_plate = render_separation(&doc, 0, "FillInk", 144).unwrap();
    let fill_inked = fill_plate.data.iter().filter(|&&v| v > 50).count();
    assert_eq!(
        fill_inked, 0,
        "FillInk plate should NOT receive Tr 1 text, got {} inked",
        fill_inked
    );
}

/// §8.6.5.6 — DefaultRGB remap. References to DeviceRGB resolve through
/// /Resources/ColorSpace/DefaultRGB when present.
#[test]
fn test_default_rgb_remap() {
    let content = "1.0 0.0 0.0 rg\n25 25 50 50 re f\n";
    let content_obj = format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content);
    let pdf = assemble_pdf(vec![
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
              /Resources << /ColorSpace << /DefaultRGB 5 0 R >> >> >>"
            .into(),
        content_obj,
        "[/DeviceN [/SpotR /SpotG /SpotB] /DeviceRGB 6 0 R]".into(),
        "<< /FunctionType 2 /Domain [0 1] /N 3 /C0 [0 0 0] /C1 [1 0 0] >>".into(),
    ]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let plate = render_separation(&doc, 0, "SpotR", 144).unwrap();
    let center = plate.data[(plate.height as usize / 2) * plate.width as usize + (plate.width as usize / 2)];
    assert!(
        center > 200,
        "DefaultRGB should remap `rg` -> Separation; SpotR plate center got {}",
        center
    );
}

/// §8.6.5.6 — DefaultGray remap. References to DeviceGray resolve through
/// /Resources/ColorSpace/DefaultGray when present.
#[test]
fn test_default_gray_remap() {
    let content = "0.5 g\n25 25 50 50 re f\n";
    let content_obj = format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content);
    let pdf = assemble_pdf(vec![
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
              /Resources << /ColorSpace << /DefaultGray 5 0 R >> >> >>"
            .into(),
        content_obj,
        "[/Separation /GraySpot /DeviceGray 6 0 R]".into(),
        "<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>".into(),
    ]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let plate = render_separation(&doc, 0, "GraySpot", 144).unwrap();
    let center = plate.data[(plate.height as usize / 2) * plate.width as usize + (plate.width as usize / 2)];
    assert!(
        center > 100 && center < 160,
        "DefaultGray should remap `g 0.5` -> Separation; GraySpot center got {}",
        center
    );
}

/// §8.10.1 — Form XObject inherits the COMPLETE graphics state at the
/// time of invocation, not just color. Line width, line cap/join,
/// dash pattern, font, ExtGState alpha/blend mode are all inherited.
/// Currently only color state is propagated into the recursive call.
#[test]
#[ignore = "spec gap: Form XObject inherits only color, not full GS (line width, etc.)"]
fn test_form_xobject_inherits_line_width() {
    // Caller sets line width to 10pt and a Separation color, then invokes
    // a Form XObject that strokes a line WITHOUT setting line width.
    // Per §8.10.1, the form should inherit width=10. Currently inherits
    // only color, so the stroke renders at default 1pt (much thinner). ~keep
    let form_content = "50 30 m 50 70 l S\n";
    let form_obj = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Length {} >>\n\
             stream\n{}\nendstream",
        form_content.len(),
        form_content
    );
    let page_content = "/CS1 cs 1.0 scn /CS1 CS 1.0 SCN 10 w\n/Fm1 Do\n";
    let page_obj = format!(
        "<< /Length {} >>\nstream\n{}\nendstream",
        page_content.len(),
        page_content
    );
    let pdf = assemble_pdf(vec![
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
              /Resources << /XObject << /Fm1 5 0 R >> \
                           /ColorSpace << /CS1 6 0 R >> >> >>"
            .into(),
        page_obj,
        form_obj,
        "[/Separation /InheritInk /DeviceGray 7 0 R]".into(),
        "<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>".into(),
    ]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let plate = render_separation(&doc, 0, "InheritInk", 144).unwrap();
    let row = plate.height as usize / 2;
    let start = row * plate.width as usize;
    let stroke_width: usize = (start..start + plate.width as usize)
        .filter(|&i| plate.data[i] > 50)
        .count();
    assert!(
        stroke_width >= 15,
        "Stroke should inherit 10pt line width (≈20 px at 144 DPI); got {} inked px",
        stroke_width
    );
}

/// §8.6.6.5 — DeviceN can declare /Subtype /NChannel with /Process and
/// /Colorants entries describing additional per-component blending into
/// process inks. Plain DeviceN currently routes each named component to
/// its own plate without honoring /Process additive contributions.
#[test]
#[ignore = "spec gap: §8.6.6.5 NChannel /Process and /Colorants routing"]
fn test_nchannel_process_attributes() {
    // NChannel DeviceN with /Subtype /NChannel and /Process declaring a
    // per-process-component contribution into CMYK. Painting with the
    // NChannel space should ALSO add tint to the named process plates.
    // Currently the renderer routes only to the named components. ~keep
    let content = "/CS1 cs 1.0 scn\n25 25 50 50 re f\n";
    let content_obj = format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content);
    let pdf = assemble_pdf(vec![
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
              /Resources << /ColorSpace << /CS1 5 0 R >> >> >>"
            .into(),
        content_obj,
        "[/DeviceN [/SpotNC] /DeviceCMYK 6 0 R \
              << /Subtype /NChannel \
                 /Process << /ColorSpace /DeviceCMYK /Components [/Cyan] >> >>]"
            .into(),
        "<< /FunctionType 2 /Domain [0 1] /N 4 /C0 [0 0 0 0] /C1 [0.5 0 0 0] >>".into(),
    ]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let spot = render_separation(&doc, 0, "SpotNC", 144).unwrap();
    let sw = spot.width as usize;
    assert!(
        spot.data[(spot.height as usize / 2) * sw + sw / 2] > 200,
        "SpotNC plate should receive tint"
    );
    let cyan = render_separation(&doc, 0, "Cyan", 144).unwrap();
    let cw = cyan.width as usize;
    let cyan_center = cyan.data[(cyan.height as usize / 2) * cw + cw / 2];
    assert!(
        cyan_center > 100,
        "NChannel /Process should route additional tint to Cyan, got {}",
        cyan_center
    );
}

/// §8.6.6 — Negative and >1.0 tint values. Spec permits them; some
/// prepress files use them for "more than 100% ink" effects. Currently
/// silently clamped to [0, 1] with no diagnostic.
#[test]
#[ignore = "spec gap: §8.6.6 tint values outside [0,1] silently clamped"]
fn test_out_of_range_tint_values_documented() {
    // PDF allows tint values outside [0,1] (e.g. for "150% ink" effects).
    // Current renderer clamps to [0,1] silently. The right behavior is
    // either to (a) clamp and emit a diagnostic, or (b) preserve the
    // out-of-range data via a wider pixel format. This test pins the
    // expectation: 1.5 tint produces > 1.0 ink coverage — currently
    // capped at 255, indistinguishable from 1.0. ~keep
    let content = "/CS1 cs 1.5 scn\n25 25 50 50 re f\n";
    let content_obj = format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content);
    let pdf = assemble_pdf(vec![
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
              /Resources << /ColorSpace << /CS1 5 0 R >> >> >>"
            .into(),
        content_obj,
        "[/Separation /OverInk /DeviceGray 6 0 R]".into(),
        "<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>".into(),
    ]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let plate = render_separation(&doc, 0, "OverInk", 144).unwrap();
    let center = plate.data[(plate.height as usize / 2) * plate.width as usize + (plate.width as usize / 2)];
    assert_ne!(
        center, 255,
        "tint value 1.5 should be detectable as out-of-range (currently clamped to 255)"
    );
}

/// §12.5.4 — Annotations rendered via /AP appearance streams may contain
/// spot-color content. The composite renderer paints them; the separation
/// renderer currently does not, so spot-colored stamps/watermarks delivered
/// via annotation streams are dropped from plates.
#[test]
#[ignore = "spec gap: §12.5.4 annotations not rendered to plates"]
fn test_annotation_appearance_streams_contribute_to_plates() {
    let appearance = "/CS1 cs 1.0 scn\n0 0 50 50 re f\n";
    let appearance_obj = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 50 50] \
              /Resources << /ColorSpace << /CS1 7 0 R >> >> /Length {} >>\n\
             stream\n{}\nendstream",
        appearance.len(),
        appearance
    );
    let pdf = assemble_pdf(vec![
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
              /Annots [5 0 R] >>"
            .into(),
        "<< /Length 0 >>\nstream\n\nendstream".into(),
        "<< /Type /Annot /Subtype /Stamp /Rect [25 25 75 75] /AP << /N 6 0 R >> >>".into(),
        appearance_obj,
        "[/Separation /StampInk /DeviceGray 8 0 R]".into(),
        "<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>".into(),
    ]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let plate = render_separation(&doc, 0, "StampInk", 144).unwrap();
    let inked = plate.data.iter().filter(|&&v| v > 50).count();
    assert!(
        inked > 1000,
        "StampInk plate should receive annotation appearance content, got {} inked",
        inked
    );
}

/// §8.7 — Tiling and shading patterns. Patterns can use Separation/DeviceN
/// content. The current renderer drops /Pattern colors and shading
/// operators (`sh`) entirely.
#[test]
#[ignore = "spec gap: §8.7 Pattern color spaces and `sh` operator not supported"]
fn test_pattern_color_space() {
    // Tiling pattern in a Separation ink. Spec says the pattern's
    // contents are painted (tiled) to cover the filled area; that
    // tiled content should contribute to the Separation plate. ~keep
    let pattern = "/CS1 cs 1.0 scn\n0 0 10 10 re f\n";
    let pattern_obj = format!(
        "<< /Type /Pattern /PatternType 1 /PaintType 1 /TilingType 1 \
              /BBox [0 0 10 10] /XStep 10 /YStep 10 \
              /Resources << /ColorSpace << /CS1 7 0 R >> >> /Length {} >>\n\
             stream\n{}\nendstream",
        pattern.len(),
        pattern
    );
    let page_content = "/Pattern cs /P1 scn\n25 25 50 50 re f\n";
    let page_obj = format!(
        "<< /Length {} >>\nstream\n{}\nendstream",
        page_content.len(),
        page_content
    );
    let pdf = assemble_pdf(vec![
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
              /Resources << /Pattern << /P1 5 0 R >> /ColorSpace << /CS1 7 0 R >> >> >>"
            .into(),
        page_obj,
        pattern_obj,
        "<< >>".into(),
        "[/Separation /PatternInk /DeviceGray 8 0 R]".into(),
        "<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>".into(),
    ]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let plate = render_separation(&doc, 0, "PatternInk", 144).unwrap();
    let inked = plate.data.iter().filter(|&&v| v > 50).count();
    assert!(
        inked > 1000,
        "Pattern fill in Separation ink should populate the plate, got {} inked",
        inked
    );
}

/// §8.9 — Inline images (BI/ID/EI) can carry DeviceN samples. Currently
/// dropped by the operator walker.
#[test]
#[ignore = "spec gap: §8.9 inline images dropped"]
fn test_inline_image_devicen() {
    let mut content: Vec<u8> = Vec::new();
    content.extend_from_slice(b"/CS1 cs\nBI /W 10 /H 10 /BPC 8 /CS /DeviceGray ID\n");
    content.extend(std::iter::repeat_n(0xFFu8, 100));
    content.extend_from_slice(b"\nEI\n");

    let mut buf: Vec<u8> = Vec::new();
    let mut offsets: Vec<usize> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
              /Resources << /ColorSpace << /CS1 5 0 R >> >> >>\nendobj\n",
    );
    offsets.push(buf.len());
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(&content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"5 0 obj\n[/Separation /InlineInk /DeviceGray 6 0 R]\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"6 0 obj\n<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>\nendobj\n");

    let xref = buf.len();
    let n = offsets.len() + 1;
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", n).as_bytes());
    for off in &offsets {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", n, xref).as_bytes());

    let doc = PdfDocument::from_bytes(buf).unwrap();
    let plate = render_separation(&doc, 0, "InlineInk", 144).unwrap();
    let inked = plate.data.iter().filter(|&&v| v > 50).count();
    assert!(
        inked > 0,
        "Inline image in Separation should contribute to plate, got {} inked",
        inked
    );
}
