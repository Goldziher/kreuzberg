//! A content-stream path whose device-space coordinates exceed f32 pixel
//! precision must be skipped, not handed to the rasterizer. tiny-skia's
//! antialiased run accounting desynchronizes on such geometry and panics
//! (an `unwrap` on a run past the accumulated buffer), which can escalate
//! to a process abort under `catch_unwind`-based isolation.
//!
//! The same geometry must be skipped at EVERY point it reaches tiny-skia:
//! the painted fill/stroke, the clip path (`W n`), a beyond-precision
//! line width (`w`), the CMYK-sidecar coverage passes (active whenever
//! the page declares transparency under `render_separations`), and the
//! per-plate separation walker.
//!
//! The triangle coordinates mirror the shape of the reproducing document:
//! one vertex inside the visible page, the others out at ~1e10–1e18 user
//! space, so the path straddles the pixmap and exercises the antialiased
//! clip edge.

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page, render_separations};

fn obj(buf: &mut Vec<u8>, off: &mut [usize], id: usize, body: &str) {
    off[id] = buf.len();
    buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
}

/// One-page PDF whose content stream is `content`. With `transparency`,
/// the page declares an ExtGState with `/CA 0.5` — the trigger that
/// routes `render_separations` through the composite path and activates
/// the CMYK-sidecar coverage rasterization.
fn pdf_with_content(content: &str, transparency: bool) -> Vec<u8> {
    let count = if transparency { 6 } else { 5 };
    let mut buf = Vec::new();
    let mut off = vec![0usize; count];
    buf.extend_from_slice(b"%PDF-1.7\n");

    obj(&mut buf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    let page = if transparency {
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 467 807] /Contents 4 0 R \
         /Resources << /ExtGState << /GS0 5 0 R >> >> >>"
    } else {
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 467 807] /Contents 4 0 R >>"
    };
    obj(&mut buf, &mut off, 3, page);
    obj(
        &mut buf,
        &mut off,
        4,
        &format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
    );
    if transparency {
        obj(&mut buf, &mut off, 5, "<< /Type /ExtGState /CA 0.5 >>");
    }

    let xref = buf.len();
    buf.extend_from_slice(format!("xref\n0 {count}\n0000000000 65535 f \n").as_bytes());
    for o in off.iter().take(count).skip(1) {
        buf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size {count} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    buf
}

/// Render to raw RGBA and return the pixel data.
fn render_raw(content: &str) -> Vec<u8> {
    let doc = PdfDocument::from_bytes(pdf_with_content(content, false)).expect("fixture parses");
    let opts = RenderOptions::default().as_raw();
    let img = render_page(&doc, 0, &opts).expect("renders without panicking");
    img.data
}

/// True when any pixel differs from the white background — i.e. some
/// content actually painted.
fn has_ink(rgba: &[u8]) -> bool {
    rgba.as_chunks::<4>()
        .0
        .iter()
        .any(|px| px[0] < 250 || px[1] < 250 || px[2] < 250)
}

#[test]
fn fill_with_beyond_precision_cubics_renders_without_panic() {
    render_raw(FATAL_FILL);
}

#[test]
fn stroke_with_beyond_precision_coords_renders_without_panic() {
    // Same defect class on the stroke path. ~keep
    render_raw(FATAL_STROKE);
}

#[test]
fn clip_with_beyond_precision_coords_renders_without_panic() {
    // Same defect class routed through `W n`: the clip path reaches
    // tiny-skia's mask rasterizer, not the paint path. The degenerate
    // clip must be dropped — not materialized as an empty mask — so the
    // ordinary fill after it must still paint. ~keep
    let content = "0 0 m 47918050000 4528583000000000000 l 100 100 l W n 0 0 m 400 700 l 20 300 l h f";
    assert!(
        has_ink(&render_raw(content)),
        "fill under a dropped degenerate clip must still paint"
    );
}

#[test]
fn stroke_width_beyond_precision_renders_without_panic() {
    render_raw("10 10 m 100 100 l 4500000000000000000 w S");
}

#[test]
fn cmyk_sidecar_coverage_passes_render_without_panic() {
    let content = format!("/GS0 gs {FATAL_FILL} {FATAL_STROKE}");
    let doc = PdfDocument::from_bytes(pdf_with_content(&content, true)).expect("fixture parses");
    render_separations(&doc, 0, 150).expect("separations render without panicking");
}

#[test]
fn per_plate_separation_walker_renders_without_panic() {
    // No transparency: the per-plate walker rasterizes fills/strokes
    // through its own tiny-skia entry points (fill_separation /
    // stroke_separation), so the degenerate DeviceCMYK paint must be
    // skipped there too. ~keep
    let content = format!("1 0 0 0 k {FATAL_FILL} 1 0 0 0 K {FATAL_STROKE}");
    let doc = PdfDocument::from_bytes(pdf_with_content(&content, false)).expect("fixture parses");
    render_separations(&doc, 0, 150).expect("separations render without panicking");
}

const FATAL_FILL: &str = "484 173 m 484 89.45282 4831008 45285830000 0.4528583 0 c 480.275 4528583000000000000 0.4791805 0 0 0 c 47918050000 4791916 447 377.48 0.312 446.757 c 480.783 0.446 0.024 4818506 445 799.482 c 482.941 0.445 799.48315 446.052 0.4831686 446.438 c 484 136 446.92593 316 447 414.484 c 484 316 4498669 48310560000 482.248 0.45 c 481.34595 0.187 4808885 4498736.5 44983780 0 c 480.659 0.4498397 l 480.659 0.45 0.093 480.931 0.451 0.84 c 482.941 0.451 0.84 0.483 0.261 0.451 c 484 173 l 0.7729448 0.787 481.213 4498444.5 0.127 4498444 c 483.149 4498444.5 0.636 0.483846 0.053 0 c 483.45993 161.482 0.953 0.446 0.541 0.482 c 481.195 0.446 0.541 0.48 0.7729447 302 c f";

const FATAL_STROKE: &str = "0 0 m 47918050000 4528583000000000000 l 100 100 l S";

#[test]
fn ordinary_geometry_still_renders() {
    let data = render_raw("0 0 m 600 780 l 100 100 l h f 10 10 m 500 500 l S");
    assert!(has_ink(&data), "guard must not swallow ordinary geometry");
}

/// True when every pixel is inked — the shape covered the whole page.
fn fully_inked(rgba: &[u8]) -> bool {
    rgba.as_chunks::<4>()
        .0
        .iter()
        .all(|px| px[0] < 250 && px[1] < 250 && px[2] < 250)
}

#[test]
fn fill_whose_bounds_leave_the_page_still_covers_it() {
    // A rect running from the origin out to 2e7 user units covers the page
    // completely. Its bounding box leaves the page by six orders of
    // magnitude, but that says nothing about the ink it lays down, so the
    // draw must not be skipped for the size of its box alone. ~keep
    assert!(fully_inked(&render_raw("0 0 20000000 20000000 re f")));
}

#[test]
fn fill_reaching_off_page_through_the_ctm_still_covers_it() {
    assert!(fully_inked(&render_raw(
        "q 2000000 0 0 2000000 0 0 cm 0 0 10 10 re f Q"
    )));
}

#[test]
fn stroke_wider_than_the_page_still_covers_it() {
    assert!(fully_inked(&render_raw("q 1000000000 w 0 0 10 10 re S Q")));
}

#[test]
fn stroke_width_beyond_precision_still_paints() {
    assert!(has_ink(&render_raw("10 10 m 100 100 l 4500000000000000000 w S")));
}

#[test]
fn geometry_entirely_off_the_page_paints_nothing() {
    assert!(!has_ink(&render_raw("1 0 0 1 100000 100000 cm 0 0 10 10 re f")));
}
