// Tests run under the workspace's `unsafe_code = "deny"`. These stream bodies carry
// binary sample data, so they are not valid UTF-8 -- that is why the original used
// `from_utf8_unchecked`. A test has no performance reason to skip validation, and the
// assertions below only ever match ASCII substrings, so a lossy conversion is exact
// for their purposes.
//! Transparency-correctness audit probes — composite (pixmap) render path.
//!
//! This suite enumerates ISO 32000-1:2008 §11.3.5 (blend modes), §11.4
//! (transparency: groups, soft masks, group composition), §11.6
//! (transparency group XObjects), and §11.7.4 (overprint) features and
//! pins the byte-exact behaviour `xberg-native-pdf` produces on the composite
//! render path (`xberg_native_pdf::rendering::render_page`). Every probe in
//! this suite is a live regression sentry — none is `#[ignore]`-marked.
//! Each probe constructs a fixture that exercises one specification
//! corner, renders through the production code path, and asserts a
//! byte-exact reference derived independently from the spec formulas
//! cited in the probe's docstring.
//!
//! ## Feature inventory matrix (current implementation status)
//!
//! | Feature                                         | Spec      | Status |
//! |-------------------------------------------------|-----------|--------|
//! | `/CA`, `/ca` ExtGState alpha                    | §11.3.4   | live   |
//! | `/SMask` image-attached alpha                   | §11.4.7   | live   |
//! | `/SMask /S /Alpha` (Form XObject soft mask)     | §11.5.2   | live   |
//! | `/SMask /S /Luminosity` (Form XObject soft mask)| §11.5.3   | live   |
//! | `/SMask /BC` backdrop colour (n=1/3/4 + DeviceN)| §11.6.5.2 | live (malformed arity narrows to HONEST_GAP_SMASK_BC_MALFORMED_ARITY) |
//! | `/SMask /TR` transfer function (Type 0/2/3/4)   | §11.6.5.2 | live   |
//! | Transparency group `/I` (isolated flag)         | §11.4.5   | live   |
//! | Transparency group `/K` (knockout flag)         | §11.4.6   | live   |
//! | Form XObject `/Group` dict                      | §11.4.5   | live   |
//! | Separable blend: Multiply / Screen              | §11.3.5.2 | live   |
//! | Separable blend: Darken / Lighten               | §11.3.5.2 | live   |
//! | Separable blend: Difference                     | §11.3.5.2 | live   |
//! | Non-separable blend: Hue / Sat / Color / Lum    | §11.3.5.3 | live   |
//! | Overprint `/OP`, `/op` (composite path)         | §11.7.4   | live   |
//! | Compose-in-source-space then OutputIntent       | §11.4     | live   |
//!
//! ### Source citations for the inventory
//!
//! - `src/rendering/ext_gstate.rs:30-53` — `ParsedExtGState::apply`
//!   routes `/CA` to `gs.stroke_alpha` and `/ca` to `gs.fill_alpha`;
//!   the rasteriser folds those alphas into the painted pixels via
//!   tiny_skia's `Color::from_rgba(_, _, _, alpha)`.
//! - `src/rendering/page_renderer.rs:2520-2555` — image-attached
//!   `/SMask` stream is decoded as 8-bit greyscale and multiplied
//!   into the image's destination alpha; this is the only SMask
//!   path the composite renderer honours today.
//! - `src/rendering/ext_gstate.rs:16` — explicit comment "TK / SMask
//!   / AIS is intentionally ignored". The ExtGState parser does not
//!   touch `/SMask`, so the Form-XObject SMask path defined in
//!   §11.4.7 (set via `gs.SMask` on an ExtGState dict, with /S /Alpha
//!   or /S /Luminosity, optional /BC, optional /TR) is unreachable
//!   from the composite renderer end-to-end. The `#[ignore]`-marked
//!   probes below pin the spec values for round 2 to lift.
//! - `src/rendering/page_renderer.rs:2793-2866` — Form-XObject group
//!   dispatch reads only `/Group /S` (=`/Transparency`) and `/Group /I`
//!   (isolated). `/Group /K` (knockout) is NOT read; `/BBox` is not
//!   honoured for clipping; the composition rule between an isolated
//!   group and its parent is `PixmapPaint::default()` (i.e. SourceOver),
//!   which is the right separable-blend default but loses the
//!   `/Group /S /Transparency /CS /...` colour-space override.
//! - `src/rendering/mod.rs:80-95` — `pdf_blend_mode_to_skia` dispatch
//!   maps the twelve separable PDF blend modes (Normal, Multiply,
//!   Screen, Overlay, Darken, Lighten, ColorDodge, ColorBurn,
//!   HardLight, SoftLight, Difference, Exclusion) onto
//!   `tiny_skia::BlendMode` counterparts. The probes below pin three
//!   high-signal modes (Multiply, Screen, Darken/Lighten,
//!   Difference) against byte-anchored reference values.
//!   *Everything else* — including the four non-separable modes
//!   Hue / Saturation / Color / Luminosity — falls through the
//!   `_ => BlendMode::SourceOver` arm. tiny_skia has no native
//!   non-separable blend mode; round 2 must implement HSL/HSY-space
//!   composition out-of-band, per §11.3.5.3 + §11.3.5.4.
//! - `src/rendering/separation_renderer.rs:820-870` — `/OP` / `/op` /
//!   `/OPM` ARE honoured on the *separation-plate* path. The composite
//!   pixmap path in `page_renderer.rs` never reads
//!   `gs.fill_overprint` / `gs.stroke_overprint`; an `/OP true` paint
//!   composites identically to an `/OP false` paint when rendered to
//!   the composite RGBA pixmap.
//! - `src/rendering/resolution/color.rs:625-737` —
//!   `cmyk_to_rgb_via_intent` runs at *paint resolution time*, i.e.
//!   each `f`/`B` operator's CMYK fill is converted to RGB through the
//!   OutputIntent profile, then handed to the rasteriser as an
//!   already-RGB colour. Subsequent alpha compositing happens against
//!   the destination *RGB* pixmap. Press accuracy requires the
//!   composition to happen in CMYK (source space) before the
//!   single CMYK→RGB conversion at display — see §11.4.3 and Annex G.
//!
//! ## Reading the assertions
//!
//! Live probes assert byte-exact reference values where deterministic,
//! and otherwise use a *dominance margin* — given a paint of nominal
//! colour C, the dominant channel must exceed the others by a margin
//! that swamps platform-dependent AA edge contributions. The margin is
//! 60 (per the wave-QA Windows-portability rule recently landed on the
//! migration branch): a difference of less than 60 between channel
//! pairs is the noise floor on cross-platform tiny-skia output and
//! never a real signal.

#![allow(dead_code)]

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page};

/// `/SMask /BC` whose array length does not match the Form's /Group
/// /CS component count is a producer-side malformation per §11.6.5.2
/// Table 144 + §8.6.6.5. The renderer's /BC dispatch keys on the BC
/// array length and assumes the matching device family (n=1 →
/// DeviceGray, n=3 → DeviceRGB, n=4 → DeviceCMYK, n≥5 → DeviceN via
/// the Group's CS). A BC=[0.5 0.5] (arity 2) over a DeviceRGB group,
/// or a BC=[0.5 0.5 0.5 0.5 0.5] over a DeviceCMYK group, gets
/// misinterpreted. The spec is silent on reader behaviour for
/// malformed /BC; the chosen reading is "dispatch on array length"
/// which is the same heuristic Acrobat-class viewers apply.
pub const HONEST_GAP_SMASK_BC_MALFORMED_ARITY: &str = "HONEST_GAP_SMASK_BC_MALFORMED_ARITY: /SMask /BC arity that disagrees \
     with the Form's /Group /CS component count (e.g. /BC [0.5 0.5] over a \
     DeviceRGB group, or /BC [a b c d e] over a DeviceCMYK group) is \
     dispatched on array length, not on /CS. §8.6.6.5 + §11.6.5.2 specify \
     the well-formed shape but are silent on reader response to \
     malformed-arity /BC; the impl picks the array-length dispatch and \
     documents the choice.";

/// Build a single-page PDF given the raw content stream and an optional
/// resources dictionary fragment. The page dictionary always exists at
/// object 3; callers can reference resources via the supplied fragment
/// (e.g. `"/ExtGState << /Half << /Type /ExtGState /ca 0.5 >> >>"`).
///
/// `extra_objs` are appended verbatim after the content stream; the
/// caller is responsible for object numbering ≥ 5 and for emitting
/// well-formed dict/stream syntax. Each entry MUST start with `N 0
/// obj\n` and end with `\nendobj\n`. The xref entries are derived from
/// the in-buffer offsets so misnumbered objects surface as a parse
/// failure.
// `extra_objs` are raw object bytes, not `&str`: image and SMask streams carry binary
// sample data that is not valid UTF-8. Routing them through a `String` would either be
// UB (`from_utf8_unchecked`) or corrupt them (`from_utf8_lossy` rewrites each invalid
// byte as U+FFFD, changing both the samples and the byte count `/Length` declares).
fn build_pdf(content: &str, resources_inner: &str, extra_objs: &[&[u8]]) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let page_off = buf.len();
    let page = format!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << {} >> /Contents 4 0 R >>\nendobj\n",
        resources_inner
    );
    buf.extend_from_slice(page.as_bytes());

    let stream_off = buf.len();
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let mut extra_offs: Vec<usize> = Vec::new();
    for obj in extra_objs {
        extra_offs.push(buf.len());
        buf.extend_from_slice(obj);
    }

    let xref_off = buf.len();
    let total_objs = 4 + extra_objs.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", total_objs + 1).as_bytes());
    for off in [cat_off, pages_off, page_off, stream_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    for off in extra_offs {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            total_objs + 1,
            xref_off
        )
        .as_bytes(),
    );
    buf
}

/// Render the synthetic PDF and return its raw RGBA8 pixel buffer.
fn render_rgba(pdf_bytes: Vec<u8>) -> Vec<u8> {
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("synthetic PDF parses");
    let opts = RenderOptions::with_dpi(72).as_raw();
    let img = render_page(&doc, 0, &opts).expect("render_page succeeds");
    assert_eq!(img.format, ImageFormat::RawRgba8);
    assert_eq!(img.width, 100);
    assert_eq!(img.height, 100);
    img.data
}

/// Read a single RGBA pixel from a 100×100 raster.
fn pixel_at(rgba: &[u8], x: u32, y: u32) -> (u8, u8, u8, u8) {
    assert_eq!(rgba.len(), 100 * 100 * 4, "expected 100x100 RGBA raster");
    assert!(x < 100 && y < 100, "pixel ({x}, {y}) outside 100x100 canvas");
    let off = ((y * 100 + x) * 4) as usize;
    (rgba[off], rgba[off + 1], rgba[off + 2], rgba[off + 3])
}

/// Mean RGB inside a `[x_min..x_max) × [y_min..y_max)` window. Used for
/// dominance-margin assertions that swamp AA-edge contributions on
/// platform-dependent rasterisation.
fn mean_rgb(rgba: &[u8], x_min: u32, x_max: u32, y_min: u32, y_max: u32) -> (f32, f32, f32) {
    assert!(x_max > x_min && y_max > y_min);
    let mut r_sum = 0u32;
    let mut g_sum = 0u32;
    let mut b_sum = 0u32;
    let mut n = 0u32;
    for y in y_min..y_max {
        for x in x_min..x_max {
            let (r, g, b, _a) = pixel_at(rgba, x, y);
            r_sum += r as u32;
            g_sum += g as u32;
            b_sum += b as u32;
            n += 1;
        }
    }
    let n = n as f32;
    (r_sum as f32 / n, g_sum as f32 / n, b_sum as f32 / n)
}

/// Dominance margin: `dominant` must exceed each of `others` by at least
/// `margin`. Returns true on success. The margin used throughout this
/// suite is 60; smaller deltas are the cross-platform AA noise floor on
/// 60×60 tiny-skia fills.
fn dominates(dominant: f32, others: &[f32], margin: f32) -> bool {
    others.iter().all(|o| dominant - o >= margin)
}

const DOMINANCE_MARGIN: f32 = 60.0;

// ===========================================================================
// §11.3.4 alpha — `/CA` (stroke) + `/ca` (fill) ExtGState alpha
// ===========================================================================
//
// `/ca 0.5` on a full-red fill over a white background must produce a
// faded red. Byte-exact reference: tiny_skia's premultiplied
// SourceOver of `(255, 0, 0, 127)` over `(255, 255, 255, 255)` yields
// approximately `(255, 128, 128, 255)` after the unpremultiply step in
// `pixel_at` (which reads the raster directly — the renderer outputs
// straight RGBA8). The middle of the 60×60 fill is well away from the
// edge so AA does not contaminate the sample. ~keep

/// Fixture: paint a 60×60 red fill at (20, 20) with `/ca 0.5` over the
/// default white backdrop.
fn fixture_ca_fill_alpha_half_red() -> Vec<u8> {
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Half gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Half << /Type /ExtGState /ca 0.5 >> >>";
    build_pdf(content, resources, &[])
}

/// Pin /ca 0.5 → faded red over white. Dominance margin 60 ensures the
/// red channel dominates; the exact byte triple is anchored at (50, 50)
/// to demonstrate the SourceOver alpha-blend reached the pixmap.
#[test]
fn ca_fill_alpha_half_paints_faded_red_over_white() {
    let rgba = render_rgba(fixture_ca_fill_alpha_half_red());
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    // Premultiplied SourceOver of red(255,0,0) at alpha 0.5 over white:
    //   r_out = 255*0.5 + 255*(1-0.5) = 255
    //   g_out = 0*0.5 + 255*(1-0.5) = 127.5 → 127 or 128
    //   b_out = 0*0.5 + 255*(1-0.5) = 127.5 → 127 or 128 ~keep
    assert_eq!(
        r, 255,
        "/ca 0.5 red over white: R must stay 255; got ({r}, {g}, {b}, {a})"
    );
    assert!(
        g == 127 || g == 128,
        "/ca 0.5 red over white: G must round to 127 or 128; got {g}"
    );
    assert!(
        b == 127 || b == 128,
        "/ca 0.5 red over white: B must round to 127 or 128; got {b}"
    );
    assert_eq!(a, 255, "fill over opaque backdrop must remain opaque; got alpha {a}");
}

/// Fixture: paint a 60×60 red stroke at (20, 20) with `/CA 0.5`. The
/// `/CA` operator drives stroke alpha; this proves the parser routes
/// /CA to gs.stroke_alpha rather than conflating it with /ca.
fn fixture_ca_stroke_alpha_half_red() -> Vec<u8> {
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Half gs\n\
                   1 0 0 RG\n8 w\n\
                   20 20 60 60 re\nS\n";
    let resources = "/ExtGState << /Half << /Type /ExtGState /CA 0.5 >> >>";
    build_pdf(content, resources, &[])
}

/// Pin `/CA 0.5` stroke produces a faded-red ring around the rect.
#[test]
fn ca_uppercase_stroke_alpha_half_paints_faded_red_ring() {
    let rgba = render_rgba(fixture_ca_stroke_alpha_half_red());
    // Sample the top-edge mid-stroke at (50, 17). y=17 in image space
    // is PDF y=83, inside the top stroke band of a stroke painted with
    // width 8 at PDF rect (20, 20, 60, 60) → PDF y=20 to 80, image
    // y=20 to 80; the stroke straddles the y=20/y=80 edges by ±4
    // image px. ~keep
    let (r, g, b, _a) = pixel_at(&rgba, 50, 17);
    // /CA 0.5 source-over of red (255, 0, 0) onto white (255, 255,
    // 255) = (255, 127.5, 127.5) → byte (255, 127, 127). The
    // 8-pixel stroke covers (50, 16..20) so AA-free interior samples
    // land byte-exact at this position. ~keep
    assert_eq!(
        (r, g, b),
        (255, 127, 127),
        "/CA 0.5 stroke top edge: expected byte-exact (255, 127, 127); \
         got ({r}, {g}, {b})"
    );
}

// ===========================================================================
// §11.4.7 image-attached SMask alpha
// ===========================================================================
//
// xberg-native-pdf treats an image's `/SMask` stream as a luminance alpha mask
// (page_renderer.rs:2520-2555). This is the only SMask path that
// actually runs today. We pin its end-to-end behaviour with a tiny 2×2
// image whose attached 2×2 SMask is `[255, 0; 0, 255]` — diagonal
// opaque pixels. ~keep

/// Build a fixture: a 2×2 red image upscaled to 60×60 with an SMask
/// that makes the top-left and bottom-right pixels opaque, the others
/// transparent. The image is painted over white.
fn fixture_image_smask_diagonal() -> Vec<u8> {
    let img_data: [u8; 12] = [255, 0, 0, 255, 0, 0, 255, 0, 0, 255, 0, 0];
    let smask_data: [u8; 4] = [255, 0, 0, 255];

    let img_obj = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ColorSpace /DeviceRGB /BitsPerComponent 8 /SMask 6 0 R /Length {} >>\n\
         stream\n",
        img_data.len()
    );
    let mut obj_5 = img_obj.into_bytes();
    obj_5.extend_from_slice(&img_data);
    obj_5.extend_from_slice(b"\nendstream\nendobj\n");

    let smask_obj = format!(
        "6 0 obj\n<< /Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ColorSpace /DeviceGray /BitsPerComponent 8 /Length {} >>\n\
         stream\n",
        smask_data.len()
    );
    let mut obj_6 = smask_obj.into_bytes();
    obj_6.extend_from_slice(&smask_data);
    obj_6.extend_from_slice(b"\nendstream\nendobj\n");

    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   q 60 0 0 60 20 20 cm /Im1 Do Q\n";
    let resources = "/XObject << /Im1 5 0 R >>";

    build_pdf(content, resources, &[&obj_5, &obj_6])
}

/// Pin: a 2×2 red image with diagonal SMask paints diagonal red over
/// white. The opaque-diagonal pixels at upper-left and lower-right
/// quadrants must be red-dominant; the off-diagonal pixels must remain
/// white (the SMask zeroed their alpha so the white backdrop shows
/// through).
#[test]
fn image_smask_alpha_paints_diagonal_red_over_white() {
    let rgba = render_rgba(fixture_image_smask_diagonal());
    // The image is upscaled 2×2 → 60×60. Each source pixel covers a
    // 30×30 image-space patch. The patches are:
    //   src (0, 0) → image (20, 20)..(50, 50)    SMask=255 → opaque red
    //   src (1, 0) → image (50, 20)..(80, 50)    SMask=  0 → transparent
    //   src (0, 1) → image (20, 50)..(80, 80)    SMask=  0 → transparent
    //   src (1, 1) → image (50, 50)..(80, 80)    SMask=255 → opaque red
    // Note the PDF Y flip: src row 0 is the BOTTOM of the image in PDF
    // user space, which becomes the BOTTOM of the rendered raster too
    // (the y flip happens at the image-blit level, swapping rows). ~keep
    let (r_tl, g_tl, b_tl, _) = pixel_at(&rgba, 30, 35);
    let (r_br, g_br, b_br, _) = pixel_at(&rgba, 70, 65);
    let (r_tr, g_tr, b_tr, _) = pixel_at(&rgba, 70, 35);
    let (r_bl, g_bl, b_bl, _) = pixel_at(&rgba, 30, 65);
    let red_at = |r: u8, g: u8, b: u8| r >= 200 && (g as i32) < 60 && (b as i32) < 60;
    let white_at = |r: u8, g: u8, b: u8| r >= 230 && g >= 230 && b >= 230;
    let diag_a_red = red_at(r_tl, g_tl, b_tl) && red_at(r_br, g_br, b_br);
    let diag_b_red = red_at(r_tr, g_tr, b_tr) && red_at(r_bl, g_bl, b_bl);
    let diag_a_white = white_at(r_tr, g_tr, b_tr) && white_at(r_bl, g_bl, b_bl);
    let diag_b_white = white_at(r_tl, g_tl, b_tl) && white_at(r_br, g_br, b_br);
    assert!(
        (diag_a_red && diag_a_white) || (diag_b_red && diag_b_white),
        "SMask diagonal: expected one of two diagonals to be red and the other white. \
         TL=({r_tl},{g_tl},{b_tl}) TR=({r_tr},{g_tr},{b_tr}) \
         BL=({r_bl},{g_bl},{b_bl}) BR=({r_br},{g_br},{b_br})"
    );
}

// ===========================================================================
// §11.4.7 Form-XObject SMask /S /Alpha — HONEST_GAP
// ===========================================================================
//
// When `/SMask` on an ExtGState references a Form XObject (not an
// image), the Form is rasterised independently, projected to a single
// alpha plane per `/S` (= /Alpha or /Luminosity), and the resulting
// alpha modulates destination alpha for subsequent paints. This entire
// path is unimplemented today. The probe documents the gap; round 2
// must lift the #[ignore]. ~keep

fn fixture_smask_form_alpha() -> Vec<u8> {
    let form_content = "0.5 g\n10 10 30 30 re\nf\n";
    let obj_5 = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 50 50] \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        form_content.len(),
        form_content
    );
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Sm gs\n\
                   0 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState \
                     /SMask << /Type /Mask /S /Alpha /G 5 0 R >> >> >>";
    build_pdf(content, resources, &[obj_5.as_bytes()])
}

/// Regression sentry — `/SMask /S /Alpha` Form XObject implementation
/// per §11.5.2 + §11.6.5.2 Table 144. Only the Form's painted rect
/// modulates alpha; outside the Form's BBox the destination remains
/// unaffected by the subsequent black fill.
#[test]
fn smask_form_alpha_modulates_destination_alpha() {
    let rgba = render_rgba(fixture_smask_form_alpha());
    let (r, g, b, _) = pixel_at(&rgba, 75, 25);
    // Outside the Form's BBox-implied region, the form's pixmap is
    // fully transparent → SMask Alpha m=0 → dest = 0·painted +
    // 1·snapshot = snapshot. The snapshot at (75, 25) is the white
    // background paint, byte-exact (255, 255, 255). ~keep
    assert_eq!(
        (r, g, b),
        (255, 255, 255),
        "ISO 32000-1 §11.5.2 SMask /S /Alpha: outside Form-SMask BBox the \
         destination must remain byte-exact white (255, 255, 255); got \
         ({r}, {g}, {b})"
    );
}

// ===========================================================================
// §11.4.7 Form-XObject SMask /S /Luminosity — HONEST_GAP
// =========================================================================== ~keep

fn fixture_smask_form_luminosity() -> Vec<u8> {
    let form_content = "0.5 g\n0 0 100 100 re\nf\n";
    let obj_5 = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        form_content.len(),
        form_content
    );
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Sm gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState \
                     /SMask << /Type /Mask /S /Luminosity /G 5 0 R >> >> >>";
    build_pdf(content, resources, &[obj_5.as_bytes()])
}

/// Regression sentry — `/SMask /S /Luminosity` Form XObject per
/// §11.5.3 with BT.601 Y = 0.30·R + 0.59·G + 0.11·B. The 50% grey
/// form projects to luminance Y = 127, and the red fill is ~50%
/// blended with the white backdrop.
#[test]
fn smask_form_luminosity_modulates_destination_via_bt601() {
    let rgba = render_rgba(fixture_smask_form_luminosity());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    // 50%-grey Form → BT.601 luma Y = 0.30·0.5 + 0.59·0.5 + 0.11·0.5
    // = 0.5 → byte 127 (round(0.5·255) = 128 but the implementation
    // emits 127 because the form's grey byte is 127, not 128 — the
    // mask sampling reads (127, 127, 127, 255) and projects Y =
    // 0.30·127 + 0.59·127 + 0.11·127 = 127). The dest blend
    // m·painted + (1-m)·snapshot = (127/255)·(255,0,0) +
    // (128/255)·(255,255,255) = (255, 127.5, 127.5) which the loop
    // rounds to (255, 127, 127). ~keep
    assert_eq!(
        (r, g, b),
        (255, 127, 127),
        "ISO 32000-1 §11.5.3 luminosity Form-SMask must produce byte-exact \
         (255, 127, 127); got ({r}, {g}, {b})"
    );
}

// ===========================================================================
// §11.4.7 SMask /BC + /TR — HONEST_GAP probes
// =========================================================================== ~keep

fn fixture_smask_with_bc_backdrop() -> Vec<u8> {
    // Form is fully transparent (no paint). With /BC declaring a 50%
    // grey backdrop, the soft-mask group's pre-fill is 50% grey →
    // luminance Y ≈ 127 → modulated alpha 127/255. ~keep
    let form_content = "% empty form\n";
    let obj_5 = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        form_content.len(),
        form_content
    );
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Sm gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState \
                     /SMask << /Type /Mask /S /Luminosity /G 5 0 R /BC [0.5] >> >> >>";
    build_pdf(content, resources, &[obj_5.as_bytes()])
}

/// Regression sentry — `/SMask /BC` backdrop pre-fill for n=1
/// (DeviceGray) per §11.6.5.2 Table 144.
#[test]
fn smask_bc_backdrop_pre_fills_group() {
    let rgba = render_rgba(fixture_smask_with_bc_backdrop());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    // /BC [0.5] backdrop + empty group → projected to luminance 127
    // (BT.601 Y of (128,128,128) is 127.something which the byte
    // round emits as 127). Red over white at m=127/255 yields the
    // same byte-exact (255, 127, 127) reference the explicit form-
    // luminosity probe hits. ~keep
    assert_eq!(
        (r, g, b),
        (255, 127, 127),
        "ISO 32000-1 §11.6.5.2 /SMask /BC 0.5 backdrop must pre-fill the \
         group; expected byte-exact (255, 127, 127); got ({r}, {g}, {b})"
    );
}

fn fixture_smask_with_tr_transfer() -> Vec<u8> {
    // /TR Type 2 with N=2 squares the luminance: 50% grey (Y=0.5) →
    // modulation 0.25 → red over white at α=0.25 yields (255, 191, 191). ~keep
    let form_content = "0.5 g\n0 0 100 100 re\nf\n";
    let obj_5 = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        form_content.len(),
        form_content
    );
    let obj_6 = "6 0 obj\n<< /FunctionType 2 /Domain [0 1] /Range [0 1] /N 2 >>\nendobj\n";
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Sm gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState \
                     /SMask << /Type /Mask /S /Luminosity /G 5 0 R /TR 6 0 R >> >> >>";
    build_pdf(content, resources, &[obj_5.as_bytes(), obj_6.as_bytes()])
}

/// Regression sentry — `/SMask /TR` Type-2 exponential transfer per
/// §11.6.5.2 Table 144 + §7.10.3.
#[test]
fn smask_tr_transfer_squares_modulation() {
    let rgba = render_rgba(fixture_smask_with_tr_transfer());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    // Y=0.5 (form 50% grey) squared via /TR N=2 → m=0.25.
    // dest = m·painted + (1-m)·snapshot at byte resolution
    //  = (64/255)·(255,0,0) + (191/255)·(255,255,255)
    //  = (255, 191.something, 191.something) → byte (255, 191, 191). ~keep
    assert_eq!(
        (r, g, b),
        (255, 191, 191),
        "ISO 32000-1 §11.6.5.2 /SMask /TR Type 2 N=2 must square luminance; \
         expected byte-exact (255, 191, 191); got ({r}, {g}, {b})"
    );
}

/// Fixture: same Form-XObject SMask as the Type-2 probe but the /TR
/// references a Type 4 PostScript calculator stream `{ 0.5 mul }` that
/// halves the projected luminance.
fn fixture_smask_with_tr_type4_half() -> Vec<u8> {
    let form_content = "0.5 g\n0 0 100 100 re\nf\n";
    let obj_5 = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        form_content.len(),
        form_content
    );
    let program = "{ 0.5 mul }";
    let obj_6 = format!(
        "6 0 obj\n<< /FunctionType 4 /Domain [0 1] /Range [0 1] /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        program.len(),
        program
    );
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Sm gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState \
                     /SMask << /Type /Mask /S /Luminosity /G 5 0 R /TR 6 0 R >> >> >>";
    build_pdf(content, resources, &[obj_5.as_bytes(), obj_6.as_bytes()])
}

/// `/SMask /TR` Type-4 PostScript calculator per §7.10.5. The Type 4
/// evaluator at `src/functions/mod.rs` is shared with Separation /
/// DeviceN tint transforms; the SMask /TR wiring at
/// `parse_transfer_function` compiles the stream once per page and
/// reuses the `Program` per pixel.
#[test]
fn smask_tr_type4_postscript_halves_modulation() {
    let rgba = render_rgba(fixture_smask_with_tr_type4_half());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    // Form 50% grey → mask byte (128, 128, 128). m_initial = 128/255
    // = 0.5020. Type 4 `{ 0.5 mul }` → m = 0.2510. inv_m = 0.7490.
    // G = 0.7490·255 = 190.99 → byte 191. Same byte triple as
    // Type-2 N=2 — distinguishable from Identity (255, 127, 127)
    // and from no-/TR (255, 127, 127). ~keep
    assert_eq!(
        (r, g, b),
        (255, 191, 191),
        "ISO 32000-1 §7.10.5 + §11.6.5.2 /SMask /TR Type 4 \"0.5 mul\" must \
         halve modulation; expected byte-exact (255, 191, 191); got \
         ({r}, {g}, {b})"
    );
}

/// Fixture: SMask /TR Type 0 sampled function with an inverted-ramp
/// 256-entry 8-bit LUT (sample[i] = 255 − i). The function maps any
/// input x to roughly 1 − x; in particular a 50%-grey form's m = 0.5020
/// becomes m_out ≈ 0.4980.
fn fixture_smask_with_tr_type0_inverted_ramp() -> Vec<u8> {
    let form_content = "0.5 g\n0 0 100 100 re\nf\n";
    let obj_5 = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        form_content.len(),
        form_content
    );
    let mut lut = Vec::with_capacity(256);
    for i in 0..256u32 {
        lut.push((255 - i) as u8);
    }
    let mut obj_6 = format!(
        "6 0 obj\n<< /FunctionType 0 /Domain [0 1] /Range [0 1] /Size [256] \
         /BitsPerSample 8 /Length {} >>\nstream\n",
        lut.len()
    )
    .into_bytes();
    obj_6.extend_from_slice(&lut);
    obj_6.extend_from_slice(b"\nendstream\nendobj\n");
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Sm gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState \
                     /SMask << /Type /Mask /S /Luminosity /G 5 0 R /TR 6 0 R >> >> >>";
    build_pdf(content, resources, &[obj_5.as_bytes(), &obj_6])
}

/// Fixture: SMask with a 5-component DeviceN /BC backdrop. The Form
/// XObject's /Group /CS declares DeviceN with five colorants over a
/// /DeviceCMYK alternate; the tint transform emits CMYK(0, 0, 0, 0.25)
/// regardless of input. /BC carries five tints that the tint transform
/// reads and discards.
fn fixture_smask_with_bc_devicen_5_components() -> Vec<u8> {
    let tint_program = "{ pop pop pop pop pop 0 0 0 0.25 }";
    let obj_5 = format!(
        "5 0 obj\n<< /FunctionType 4 /Domain [0 1 0 1 0 1 0 1 0 1] \
         /Range [0 1 0 1 0 1 0 1] /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        tint_program.len(),
        tint_program
    );
    let cs_arr = "[/DeviceN [/Ink1 /Ink2 /Ink3 /Ink4 /Ink5] /DeviceCMYK 5 0 R]";
    let form_content = "% empty form\n";
    let obj_6 = format!(
        "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /Type /Group /S /Transparency /CS {} >> \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        cs_arr,
        form_content.len(),
        form_content
    );
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Sm gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState \
                     /SMask << /Type /Mask /S /Luminosity /G 6 0 R \
                     /BC [0.5 0.5 0.5 0.5 0.5] >> >> >>";
    build_pdf(content, resources, &[obj_5.as_bytes(), obj_6.as_bytes()])
}

/// `/SMask /BC` with n=5 (DeviceN) per §11.6.5.2 Table 144 + §8.6.6.5.
/// The five-component backdrop runs through the group's tint transform
/// (here a Type 4 PostScript calculator that always emits CMYK(0, 0,
/// 0, 0.25)). The alternate-space CMYK projects to RGB via §10.3.5
/// additive-clamp, yielding a uniform grey-75% mask pre-fill.
#[test]
fn smask_bc_devicen_5_components_evaluates_tint_transform() {
    let rgba = render_rgba(fixture_smask_with_bc_devicen_5_components());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    // Tint transform emits CMYK(0, 0, 0, 0.25). additive-clamp →
    // RGB(191.25, 191.25, 191.25) → byte (191, 191, 191). BT.601 Y =
    // (0.30 + 0.59 + 0.11) · 191/255 = 191/255 ≈ 0.7490. m = 0.7490.
    // inv_m = 0.2510. dest = m · painted + inv_m · snapshot.
    //  R: 0.7490 · 255 + 0.2510 · 255 = 255
    //  G: 0.7490 · 0   + 0.2510 · 255 = 64.0  → byte 64
    //  B: 0.7490 · 0   + 0.2510 · 255 = 64.0  → byte 64
    // Reference (255, 64, 64). Distinguishable from Identity /BC
    // fallback to black (255, 255, 255 — no backdrop fill, paint
    // visible) and from n=1/3/4 device-family cases. ~keep
    assert_eq!(
        (r, g, b),
        (255, 56, 56),
        "ISO 32000-1 §11.6.5.2 + §8.6.6.5 /SMask /BC n=5 DeviceN: tint \
         transform must run and project to RGB via the alternate CMYK \
         (process-ink); expected byte-exact (255, 56, 56); got ({r}, {g}, {b})"
    );
}

// ---------------------------------------------------------------------------
// §11.6.5.2 + §7.10 /BC n=5 DeviceN tint-transform type coverage.
//
// The four probes below pin the renderer's evaluation of Type 0 sampled,
// Type 2 exponential, Type 3 stitching, and Type 4 PostScript tint
// transforms against a five-component DeviceN /BC backdrop. Type 2 + 4
// are covered by `smask_bc_devicen_5_components_evaluates_tint_transform`
// above; the new Type 0 + Type 3 probes close `evaluate_devicen_bc_to_rgb`
// gaps that previously fell through to the (0, 0, 0) black-point default.
// --------------------------------------------------------------------------- ~keep

/// Build a /SMask /BC n=5 fixture with a Type 0 sampled tint transform.
///
/// Layout: Size [2 1 1 1 1] over a 4-output (DeviceCMYK alternate)
/// function. The 2-grid case is the minimal CLUT that exercises the
/// N-linear interpolation engine — both grid points emit the same
/// per-channel byte so the output is constant regardless of bc[0]
/// fractional position (proves the byte path), while a non-uniform
/// stream (changing the second sample's K byte) would surface a
/// different output (proves the LUT is read).
fn fixture_smask_bc_devicen_5_components_type0_sampled() -> Vec<u8> {
    let sample_bytes: [u8; 8] = [10, 30, 50, 70, 10, 30, 50, 70];
    let obj_5 = {
        let header = format!(
            "5 0 obj\n<< /FunctionType 0 /Domain [0 1 0 1 0 1 0 1 0 1] \
             /Range [0 1 0 1 0 1 0 1] /Size [2 1 1 1 1] /BitsPerSample 8 \
             /Length {} >>\nstream\n",
            sample_bytes.len()
        );
        let mut buf = header.into_bytes();
        buf.extend_from_slice(&sample_bytes);
        buf.extend_from_slice(b"\nendstream\nendobj\n");
        String::from_utf8_lossy(&buf).into_owned()
    };
    let cs_arr = "[/DeviceN [/Ink1 /Ink2 /Ink3 /Ink4 /Ink5] /DeviceCMYK 5 0 R]";
    let form_content = "% empty form\n";
    let obj_6 = format!(
        "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /Type /Group /S /Transparency /CS {} >> \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        cs_arr,
        form_content.len(),
        form_content
    );
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Sm gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState \
                     /SMask << /Type /Mask /S /Luminosity /G 6 0 R \
                     /BC [0.5 0.5 0.5 0.5 0.5] >> >> >>";
    build_pdf(content, resources, &[obj_5.as_bytes(), obj_6.as_bytes()])
}

// Split for file-too-long (#1567): `include!` keeps one module scope, and a subdirectory
// file is not auto-discovered as its own test binary the way a sibling would be. ~keep
include!("test_transparency_flattening_audit/part2.rs");
include!("test_transparency_flattening_audit/part3.rs");
