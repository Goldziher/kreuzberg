//! Press-accurate OutputIntent CMYK ICC integration tests.
//!
//! Builds synthetic PDFs that declare an `/OutputIntents` array with a
//! CMYK `DestOutputProfile`, renders them through the composite path,
//! and pins that the resulting RGB values come from the qcms-driven
//! ICC conversion rather than the §10.3.5 additive-clamp fallback.
//!
//! The minimal CMYK ICC profile used here is synthesised in-test (see
//! `build_minimal_cmyk_to_rgb_lut8_profile` and the README in
//! `tests/fixtures/icc/`). It maps every CMYK input to a constant
//! `RGB(128, 128, 128)` so the pin is unambiguous: an OutputIntent-
//! driven render gives ~128 grey; an additive-clamp fallback gives the
//! §10.3.5 value for the input CMYK.

#![allow(dead_code)]

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page};

// `#[ignore = OUTPUT_INTENT_DEFER_*]` so a future engineer running the

/// Caching of `Transform::new_srgb_target` calls. Each `k` / `K` operator
/// rebuilds the qcms transform today; the plan defers this to phase 7.
const OUTPUT_INTENT_DEFER_PHASE_7_CACHING: &str = "OUTPUT_INTENT_DEFER_PHASE_7_CACHING: plan phase 7 will cache compiled qcms transforms; \
     until then per-paint transform construction is the baseline";

/// Form XObject inheritance of /OutputIntents was originally tagged
/// against the same marker as /DefaultCMYK (round 1); the placeholder
/// probe lives below and stays ignored until a Form-XObject fixture
/// helper lands.
const OUTPUT_INTENT_DEFER_FORM_XOBJECT_INHERITANCE: &str = "OUTPUT_INTENT_DEFER_FORM_XOBJECT_INHERITANCE: needs a Form XObject test-fixture helper \
     before the inheritance probe can be wired";

// ===========================================================================
// Minimal CMYK ICC profile synthesis
// ===========================================================================
//
// ICC profile structure (per ICC.1:2004-10 §7 for v2; ICC.1:2010 §7
// for v4 — the layout is identical, only the version byte at offset
// 8..12 differs):
//   - 128-byte header
//   - 4-byte tag count
//   - tag table: N × 12 bytes (signature, offset, size)
//   - tag data: each section 4-byte aligned
//
// Minimum tags qcms's CMYK→RGB transform path needs:
//   - A2B0 (mft1 LUT8 type): CMYK→PCS lookup
// qcms reads the LUT8 (entry-size 1, fixed 256-entry input/output tables)
// per ICC.1 §10.8. Layout inside the LUT8 tag data:
//   bytes 0..4    type signature 'mft1' (0x6d667431)
//   bytes 4..8    reserved zero
//   bytes 8       input channels (4 for CMYK)
//   bytes 9       output channels (3 for RGB)
//   bytes 10      grid points per dimension
//   bytes 11      padding
//   bytes 12..48  9 × s15Fixed16 matrix entries (identity for CMYK)
//   bytes 48..    input tables (input_channels × 256 bytes)
//   then          CLUT (grid_points^input_channels × output_channels bytes)
//   then          output tables (output_channels × 256 bytes) ~keep

/// Build a minimal valid ICC v2 CMYK→Lab profile whose A2B0 LUT8 maps
/// every CMYK input to a fixed Lab tuple. The PCS is `Lab ` rather
/// than `XYZ ` because qcms's Lab→XYZ→sRGB chain decodes the 8-bit
/// LUT8 outputs as `L = byte/255*100`, `a = byte - 128`, `b = byte -
/// 128` — easier to point at "neutral grey" than to compute the
/// matching XYZ tuple and round it into a LUT8 byte.
///
/// The constant CLUT makes the test pin unambiguous: whichever CMYK
/// quadruple the renderer feeds the profile, the qcms-converted RGB
/// is the same near-neutral grey that Lab(target_L, 0, 0) projects to
/// through sRGB. That's distinct from the §10.3.5 additive-clamp
/// value for any non-degenerate CMYK input, so a fallback to
/// additive-clamp is immediately visible.
///
/// `target_l_byte` is the LUT8 byte for the L* channel — e.g. 135 ≈
/// L*53, which projects through sRGB to roughly mid-grey
/// `RGB(~128, ~128, ~128)`. a* and b* are pinned at 128 (decoded as
/// 0, the achromatic axis).
fn build_minimal_cmyk_to_rgb_lut8_profile(target_l_byte: u8) -> Vec<u8> {
    build_minimal_cmyk_to_rgb_lut8_profile_with_version(target_l_byte, IccProfileVersion::V2)
}

/// ICC profile header version byte (bytes 8..12 of the 128-byte header).
///
/// ICC.1:2004-10 §7.2.3 (Table 14): the first byte is the major
/// revision, the second the minor, bytes 10..12 are reserved (must be
/// zero). qcms 0.3.0's `check_profile_version` (iccread.rs:274) reads
/// the reserved bytes and rejects anything non-zero, but the version
/// comparison itself is commented out — both v2 (0x02400000) and v4
/// (0x04000000) profile headers parse provided the tag-data the qcms
/// CMM consumes is itself well-formed.
///
/// LUT8 (`mft1`) tag bodies are an ICC v2-era construct. ICC v4
/// introduces the `mAB ` tag form; qcms parses both for the `A2B0`
/// transform-direction tag, so a v4-versioned profile whose A2B0 body
/// is still an mft1 LUT8 is parseable end-to-end. A true v4 profile
/// with mAB tag bodies needs richer tag construction (curve sets,
/// matrices, a CLUT) — synthesising one in-test for the constant-CLUT
/// fixture trick gains nothing the version-byte flip already proves;
/// the LUT8 body is intent-invariant whether the header advertises v2
/// or v4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IccProfileVersion {
    /// 2.4.0.0 — the version qcms 0.3.0's iccread treats as the LUT8
    /// path.
    V2,
    /// 4.0.0.0 — the modern major revision. qcms 0.3.0 accepts the
    /// header (no major-revision check) and reads whatever A2B0 tag
    /// body is present; a v4 header with an mft1 LUT8 body is a
    /// legitimate forward-compatible encoding.
    V4,
}

impl IccProfileVersion {
    fn header_bytes(self) -> [u8; 4] {
        // ICC.1:2004-10 §7.2.3: major.minor with bytes 10..12 reserved
        // (must be zero per the spec; qcms enforces this in
        // check_profile_version). ~keep
        match self {
            Self::V2 => 0x0240_0000u32.to_be_bytes(),
            Self::V4 => 0x0400_0000u32.to_be_bytes(),
        }
    }
}

/// Like [`build_minimal_cmyk_to_rgb_lut8_profile`] but with an
/// explicit ICC version-byte choice in the 128-byte header. Used by
/// the phase-6 ICC v4 verification probe to assert qcms 0.3.0 accepts
/// the v4 header at parse time and drives the same constant-CLUT body
/// to the same byte-exact RGB reference.
fn build_minimal_cmyk_to_rgb_lut8_profile_with_version(target_l_byte: u8, version: IccProfileVersion) -> Vec<u8> {
    // LUT8 tag body for in=4 out=3 grid=2.
    // Sizes:
    //   header: 48
    //   input tables: 4 * 256 = 1024
    //   CLUT: 2^4 * 3 = 48
    //   output tables: 3 * 256 = 768
    //   total: 1888 bytes ~keep
    let in_chan: u8 = 4;
    let out_chan: u8 = 3;
    let grid: u8 = 2;
    let mut lut = Vec::with_capacity(1888);

    lut.extend_from_slice(&0x6d66_7431u32.to_be_bytes());
    lut.extend_from_slice(&0u32.to_be_bytes());
    lut.push(in_chan);
    lut.push(out_chan);
    lut.push(grid);
    lut.push(0);

    let identity: [i32; 9] = [0x0001_0000, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x0001_0000];
    for v in identity {
        lut.extend_from_slice(&(v as u32).to_be_bytes());
    }

    for _ in 0..in_chan {
        for i in 0..256u16 {
            lut.push(i as u8);
        }
    }

    // CLUT: 2^4 × 3 = 16 grid points × 3 output channels.
    // Every grid point outputs Lab(target_L, 0, 0) — neutral grey at the
    // requested lightness. qcms decodes LUT8 outputs through the chain
    //   L = byte/255 * 100
    //   a = byte - 128
    //   b = byte - 128
    // so target_l_byte directly controls L*; a* and b* are pinned at
    // 128 (decoded as the achromatic axis 0). ~keep
    let grid_size = (grid as usize).pow(in_chan as u32);
    for _ in 0..grid_size {
        lut.push(target_l_byte);
        lut.push(128);
        lut.push(128);
    }

    for _ in 0..out_chan {
        for i in 0..256u16 {
            lut.push(i as u8);
        }
    }

    debug_assert_eq!(lut.len(), 1888, "LUT8 body size mismatch");

    // ICC profile envelope: 128-byte header + tag table + tag data.
    // Total profile size: 128 (header) + 4 (count) + 12 (one tag entry)
    // + 1888 (A2B0 data) = 2032 bytes, with the A2B0 data starting at
    // offset 144. ~keep
    let mut profile = vec![0u8; 128];
    let total_size: u32 = 128 + 4 + 12 + lut.len() as u32;

    profile[0..4].copy_from_slice(&total_size.to_be_bytes());
    profile[8..12].copy_from_slice(&version.header_bytes());
    profile[12..16].copy_from_slice(b"prtr");
    profile[16..20].copy_from_slice(b"CMYK");
    profile[20..24].copy_from_slice(b"Lab ");
    profile[36..40].copy_from_slice(b"acsp");
    profile[64..68].copy_from_slice(&0u32.to_be_bytes());
    profile[68..72].copy_from_slice(&0x0000_F6D6u32.to_be_bytes());
    profile[72..76].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    profile[76..80].copy_from_slice(&0x0000_D32Du32.to_be_bytes());

    profile.extend_from_slice(&1u32.to_be_bytes());
    profile.extend_from_slice(&0x4132_4230u32.to_be_bytes());
    profile.extend_from_slice(&144u32.to_be_bytes());
    profile.extend_from_slice(&(lut.len() as u32).to_be_bytes());

    profile.extend_from_slice(&lut);

    profile
}

/// Build a minimal valid ICC v2 RGB→Lab profile whose A2B0 LUT8 maps
/// every RGB input to a fixed Lab tuple. Mirrors the CMYK builder's
/// constant-CLUT trick at `in_chan=3` so the qcms reference is a
/// stable point: whichever RGB the renderer feeds through the profile,
/// the output is the same near-neutral grey at L*=target_l_byte/255*100.
///
/// Used by the Phase 9 `/DefaultRGB` precedence probe: the override
/// declared as `[/ICCBased <stream>]` against bare /DeviceRGB paint
/// routes the components through this profile; the rendered pixel
/// matches the qcms reference RGB regardless of the painted RGB value.
///
/// qcms 0.3.0's LUT8 parser at `iccread.rs:760` accepts `in_chan ∈ {3,
/// 4}` only; this builder targets the `in_chan=3` slot. The synthesised
/// device class is `prtr` and the PCS is `Lab ` so the same Lab→sRGB
/// decoder runs for both CMYK and RGB fixture profiles — only the
/// input-channel count and CLUT-grid power differ.
fn build_minimal_rgb_to_lab_lut8_profile(target_l_byte: u8) -> Vec<u8> {
    // LUT8 tag body for in=3 out=3 grid=2.
    // Sizes:
    //   header: 48
    //   input tables: 3 * 256 = 768
    //   CLUT: 2^3 * 3 = 24
    //   output tables: 3 * 256 = 768
    //   total: 1608 bytes ~keep
    let in_chan: u8 = 3;
    let out_chan: u8 = 3;
    let grid: u8 = 2;
    let mut lut = Vec::with_capacity(1608);

    lut.extend_from_slice(&0x6d66_7431u32.to_be_bytes());
    lut.extend_from_slice(&0u32.to_be_bytes());
    lut.push(in_chan);
    lut.push(out_chan);
    lut.push(grid);
    lut.push(0);

    let identity: [i32; 9] = [0x0001_0000, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x0001_0000];
    for v in identity {
        lut.extend_from_slice(&(v as u32).to_be_bytes());
    }

    for _ in 0..in_chan {
        for i in 0..256u16 {
            lut.push(i as u8);
        }
    }

    let grid_size = (grid as usize).pow(in_chan as u32);
    for _ in 0..grid_size {
        lut.push(target_l_byte);
        lut.push(128);
        lut.push(128);
    }

    for _ in 0..out_chan {
        for i in 0..256u16 {
            lut.push(i as u8);
        }
    }

    debug_assert_eq!(lut.len(), 1608, "RGB LUT8 body size mismatch");

    let mut profile = vec![0u8; 128];
    let total_size: u32 = 128 + 4 + 12 + lut.len() as u32;
    profile[0..4].copy_from_slice(&total_size.to_be_bytes());
    profile[8..12].copy_from_slice(&IccProfileVersion::V2.header_bytes());
    profile[12..16].copy_from_slice(b"prtr");
    profile[16..20].copy_from_slice(b"RGB ");
    profile[20..24].copy_from_slice(b"Lab ");
    profile[36..40].copy_from_slice(b"acsp");
    profile[64..68].copy_from_slice(&0u32.to_be_bytes());
    profile[68..72].copy_from_slice(&0x0000_F6D6u32.to_be_bytes());
    profile[72..76].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    profile[76..80].copy_from_slice(&0x0000_D32Du32.to_be_bytes());

    profile.extend_from_slice(&1u32.to_be_bytes());
    profile.extend_from_slice(&0x4132_4230u32.to_be_bytes());
    profile.extend_from_slice(&144u32.to_be_bytes());
    profile.extend_from_slice(&(lut.len() as u32).to_be_bytes());
    profile.extend_from_slice(&lut);
    profile
}

/// Build a minimal valid ICC v2 Gray TRC profile.
///
/// Unlike the LUT8-based CMYK and RGB fixtures, Gray ICC profiles use
/// the simpler `kTRC` (Tone Reproduction Curve) tag — a single curve
/// mapping the device byte into the linear PCS. qcms 0.3.0's
/// `iccread.rs:1712-1714` reads only the `kTRC` tag for GRAY-signed
/// profiles; no A2B0 / B2A0 / colorant / matrix tags are needed.
///
/// The curve emitted here is a 256-entry `curv` table that linearly
/// ramps from 0 to 65535, corresponding to a gamma of 1.0 (the linear
/// identity). qcms's gray transform path then drives the destination
/// sRGB profile's output_gamma_lut_{r,g,b}: a linear input byte
/// becomes a linear PCS-Y value (`device/255`), which the sRGB
/// inverse-gamma encoding then converts to a perceptual sRGB byte.
/// The result is the canonical sRGB encoding of the linear gray:
/// `byte → sRGB_inv_gamma(byte/255) → sRGB byte`.
///
/// Using gamma 1.0 keeps the profile honest (a deliberate, not
/// accidental, identity in linear space) and produces a distinctive
/// reference value through the sRGB encoder that's nowhere near the
/// raw input byte for mid-tones — making a no-ICC fall-through
/// failure mode immediately visible.
fn build_minimal_gray_trc_profile() -> Vec<u8> {
    // ICC v2 `curveType` tag body shape (ICC.1:2004-10 §10.5):
    //   bytes 0..4   type signature 'curv' (0x63757276)
    //   bytes 4..8   reserved zero
    //   bytes 8..12  count (number of entries)
    //   bytes 12..   count × u16 entries (big-endian)
    //
    // 256-entry linear ramp 0..65535 — qcms reads this as the input
    // gamma table for the gray channel. ~keep
    let entry_count: u32 = 256;
    let mut curv = Vec::with_capacity(12 + (entry_count as usize) * 2);
    curv.extend_from_slice(&0x6375_7276u32.to_be_bytes());
    curv.extend_from_slice(&0u32.to_be_bytes());
    curv.extend_from_slice(&entry_count.to_be_bytes());
    for i in 0..entry_count {
        let v = ((i * 65535) / (entry_count - 1)) as u16;
        curv.extend_from_slice(&v.to_be_bytes());
    }

    // Envelope: 128-byte header + 4 (tag count) + 12 (one tag entry) +
    // curveType body. Tag data offset = 144. ~keep
    let mut profile = vec![0u8; 128];
    let total_size: u32 = 128 + 4 + 12 + curv.len() as u32;
    profile[0..4].copy_from_slice(&total_size.to_be_bytes());
    profile[8..12].copy_from_slice(&IccProfileVersion::V2.header_bytes());
    profile[12..16].copy_from_slice(b"mntr");
    profile[16..20].copy_from_slice(b"GRAY");
    profile[20..24].copy_from_slice(b"XYZ ");
    profile[36..40].copy_from_slice(b"acsp");
    profile[68..72].copy_from_slice(&0x0000_F6D6u32.to_be_bytes());
    profile[72..76].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    profile[76..80].copy_from_slice(&0x0000_D32Du32.to_be_bytes());

    profile.extend_from_slice(&1u32.to_be_bytes());
    profile.extend_from_slice(&0x6b54_5243u32.to_be_bytes());
    profile.extend_from_slice(&144u32.to_be_bytes());
    profile.extend_from_slice(&(curv.len() as u32).to_be_bytes());
    profile.extend_from_slice(&curv);
    profile
}

/// Build a one-page PDF with the given catalog entries and content
/// stream. The catalog entries string is spliced into the catalog
/// dictionary so callers can add `/OutputIntents [...]` without
/// reconstructing the whole envelope.
///
/// MediaBox is fixed at `[0 0 100 100]`; rendering at 72 DPI gives a
/// 100×100 pixel canvas so callers can pin pixels at known offsets.
fn build_pdf_with_catalog_entries_and_content(
    catalog_entries: &str,
    content_ops: &str,
    icc_profile_bytes: Option<&[u8]>,
) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    let cat_off = buf.len();
    let catalog = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {} >>\nendobj\n",
        catalog_entries
    );
    buf.extend_from_slice(catalog.as_bytes());

    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let page_off = buf.len();
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R >>\nendobj\n",
    );

    let stream_off = buf.len();
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content_ops.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content_ops.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let icc_off;
    let obj_count;
    if let Some(icc) = icc_profile_bytes {
        icc_off = buf.len();
        let icc_hdr = format!("5 0 obj\n<< /N 4 /Length {} >>\nstream\n", icc.len());
        buf.extend_from_slice(icc_hdr.as_bytes());
        buf.extend_from_slice(icc);
        buf.extend_from_slice(b"\nendstream\nendobj\n");
        obj_count = 6;
    } else {
        icc_off = 0;
        obj_count = 5;
    }

    let xref_off = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", obj_count).as_bytes());
    for off in [cat_off, pages_off, page_off, stream_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    if icc_profile_bytes.is_some() {
        buf.extend_from_slice(format!("{:010} 00000 n \n", icc_off).as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            obj_count, xref_off
        )
        .as_bytes(),
    );
    buf
}

/// Build a PDF whose page paints CMYK(0.25, 0, 0, 0) into a 60×60
/// rect centred on the canvas and whose catalog declares
/// `/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX
/// /OutputCondition (Synthetic CMYK) /DestOutputProfile 5 0 R >>]`.
fn build_pdf_cmyk_with_output_intent(icc_profile_bytes: &[u8]) -> Vec<u8> {
    let catalog_entries = "/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (Synthetic CMYK) /DestOutputProfile 5 0 R >>]";
    let content_ops = "0.25 0 0 0 k\n20 20 60 60 re\nf\n";
    build_pdf_with_catalog_entries_and_content(catalog_entries, content_ops, Some(icc_profile_bytes))
}

/// Same paint operator as `build_pdf_cmyk_with_output_intent` but with
/// no `/OutputIntents` in the catalog. Pins the §10.3.5 fallback.
fn build_pdf_cmyk_without_output_intent() -> Vec<u8> {
    let content_ops = "0.25 0 0 0 k\n20 20 60 60 re\nf\n";
    build_pdf_with_catalog_entries_and_content("", content_ops, None)
}

/// Build a PDF whose page paints a `/Separation` colour space (with a
/// Type-4 PostScript tint transform that produces CMYK(0, tint, 0, 0))
/// against a document-level `/OutputIntents` CMYK profile.
///
/// Object layout:
///   1 — Catalog (with /OutputIntents → 5 0 R)
///   2 — Pages
///   3 — Page (with Resources /ColorSpace /CS1 →
///       [/Separation /MagentaSpot /DeviceCMYK 6 0 R])
///   4 — Content stream
///   5 — OutputIntent profile stream
///   6 — Tint-transform Type-4 stream
///
/// The Type-4 program `{ 0.0 exch 0.0 0.0 }` lifts the input tint into
/// the M position so the alternate-space output is CMYK(0, tint, 0, 0).
fn build_pdf_separation_type4_devicecmyk_with_output_intent(output_intent_profile: &[u8]) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    let cat_off = buf.len();
    let catalog = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (Synthetic CMYK) /DestOutputProfile 5 0 R >>] >>\nendobj\n";
    buf.extend_from_slice(catalog.as_bytes());

    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let page_off = buf.len();
    let page = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /ColorSpace << /CS1 [/Separation /MagentaSpot /DeviceCMYK 6 0 R] >> >> /Contents 4 0 R >>\nendobj\n";
    buf.extend_from_slice(page.as_bytes());

    let stream_off = buf.len();
    let content = "/CS1 cs\n1.0 scn\n20 20 60 60 re\nf\n";
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let icc_off = buf.len();
    let icc_hdr = format!("5 0 obj\n<< /N 4 /Length {} >>\nstream\n", output_intent_profile.len());
    buf.extend_from_slice(icc_hdr.as_bytes());
    buf.extend_from_slice(output_intent_profile);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let tint_off = buf.len();
    let tint_program: &[u8] = b"{ 0.0 exch 0.0 0.0 }";
    let tint_hdr = format!(
        "6 0 obj\n<< /FunctionType 4 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] /Length {} >>\nstream\n",
        tint_program.len()
    );
    buf.extend_from_slice(tint_hdr.as_bytes());
    buf.extend_from_slice(tint_program);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = buf.len();
    let obj_count = 7;
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", obj_count).as_bytes());
    for off in [cat_off, pages_off, page_off, stream_off, icc_off, tint_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            obj_count, xref_off
        )
        .as_bytes(),
    );
    buf
}

/// Same shape as `build_pdf_separation_type4_devicecmyk_with_output_intent`
/// but the colour space is a 2-colorant `/DeviceN` whose alternate is
/// `/DeviceCMYK` and whose Type-4 tint transform consumes the two input
/// tints and emits CMYK(0, tint0, 0, 0) — i.e. only the first input
/// drives the magenta component, the second is dropped. With content
/// `[1.0 0.5] scn` the input is (tint0=1.0, tint1=0.5) and the output is
/// CMYK(0, 1, 0, 0).
fn build_pdf_devicen_type4_devicecmyk_with_output_intent(output_intent_profile: &[u8]) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    let cat_off = buf.len();
    let catalog = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (Synthetic CMYK) /DestOutputProfile 5 0 R >>] >>\nendobj\n";
    buf.extend_from_slice(catalog.as_bytes());

    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let page_off = buf.len();
    let page = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /ColorSpace << /CS1 [/DeviceN [/Magenta /Cyan] /DeviceCMYK 6 0 R] >> >> /Contents 4 0 R >>\nendobj\n";
    buf.extend_from_slice(page.as_bytes());

    let stream_off = buf.len();
    let content = "/CS1 cs\n1.0 0.5 scn\n20 20 60 60 re\nf\n";
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let icc_off = buf.len();
    let icc_hdr = format!("5 0 obj\n<< /N 4 /Length {} >>\nstream\n", output_intent_profile.len());
    buf.extend_from_slice(icc_hdr.as_bytes());
    buf.extend_from_slice(output_intent_profile);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let tint_off = buf.len();
    let tint_program: &[u8] = b"{ pop 0.0 exch 0.0 0.0 }";
    let tint_hdr = format!(
        "6 0 obj\n<< /FunctionType 4 /Domain [0 1 0 1] /Range [0 1 0 1 0 1 0 1] /Length {} >>\nstream\n",
        tint_program.len()
    );
    buf.extend_from_slice(tint_hdr.as_bytes());
    buf.extend_from_slice(tint_program);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = buf.len();
    let obj_count = 7;
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", obj_count).as_bytes());
    for off in [cat_off, pages_off, page_off, stream_off, icc_off, tint_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            obj_count, xref_off
        )
        .as_bytes(),
    );
    buf
}

/// Build a PDF that declares BOTH an `/OutputIntents` CMYK profile A and
/// a page-resources `/ColorSpace /CS1 [/ICCBased <stream>]` colour space
/// whose embedded N=4 profile B is a DIFFERENT minimal CMYK profile. The
/// content stream sets fill colour space to `/CS1` and paints with
/// `0.25 0 0 0 scn`.
///
/// Object layout:
///   1 — Catalog (with /OutputIntents → 5 0 R)
///   2 — Pages
///   3 — Page (with Resources /ColorSpace /CS1 → ICCBased referencing 6 0 R)
///   4 — Content stream
///   5 — OutputIntent profile A stream
///   6 — ICCBased embedded profile B stream
fn build_pdf_embedded_iccbased_with_different_output_intent(
    output_intent_profile_a: &[u8],
    embedded_iccbased_profile_b: &[u8],
) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    let cat_off = buf.len();
    let catalog = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (Synthetic CMYK A) /DestOutputProfile 5 0 R >>] >>\nendobj\n";
    buf.extend_from_slice(catalog.as_bytes());

    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let page_off = buf.len();
    let page = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /ColorSpace << /CS1 [/ICCBased 6 0 R] >> >> /Contents 4 0 R >>\nendobj\n";
    buf.extend_from_slice(page.as_bytes());

    let stream_off = buf.len();
    let content = "/CS1 cs\n0.25 0 0 0 scn\n20 20 60 60 re\nf\n";
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let icc_a_off = buf.len();
    let icc_a_hdr = format!(
        "5 0 obj\n<< /N 4 /Length {} >>\nstream\n",
        output_intent_profile_a.len()
    );
    buf.extend_from_slice(icc_a_hdr.as_bytes());
    buf.extend_from_slice(output_intent_profile_a);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let icc_b_off = buf.len();
    let icc_b_hdr = format!(
        "6 0 obj\n<< /N 4 /Length {} >>\nstream\n",
        embedded_iccbased_profile_b.len()
    );
    buf.extend_from_slice(icc_b_hdr.as_bytes());
    buf.extend_from_slice(embedded_iccbased_profile_b);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = buf.len();
    let obj_count = 7;
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", obj_count).as_bytes());
    for off in [cat_off, pages_off, page_off, stream_off, icc_a_off, icc_b_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            obj_count, xref_off
        )
        .as_bytes(),
    );
    buf
}

/// Build a PDF whose page declares BOTH `/OutputIntents` profile A and
/// page Resources `/ColorSpace /DefaultCMYK [/ICCBased <profile B>]`,
/// then paints a bare `/DeviceCMYK` (`0.25 0 0 0 k`) rectangle.
///
/// ISO 32000-1:2008 §8.6.5.6: the `/DefaultCMYK` override redirects
/// bare /DeviceCMYK paint through the override's colour space —
/// independently of any document-level /OutputIntents profile. The
/// override therefore wins on the rendered pixel.
///
/// Object layout:
///   1 — Catalog (with /OutputIntents → 5 0 R)
///   2 — Pages
///   3 — Page (with Resources /ColorSpace /DefaultCMYK → ICCBased
///       referencing 6 0 R)
///   4 — Content stream
///   5 — OutputIntent profile A stream
///   6 — DefaultCMYK embedded profile B stream
fn build_pdf_default_cmyk_overrides_output_intent(
    output_intent_profile_a: &[u8],
    default_cmyk_profile_b: &[u8],
) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    let cat_off = buf.len();
    let catalog = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (Synthetic CMYK A) /DestOutputProfile 5 0 R >>] >>\nendobj\n";
    buf.extend_from_slice(catalog.as_bytes());

    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let page_off = buf.len();
    // /DefaultCMYK is a named entry in /Resources /ColorSpace per
    // §8.6.5.6. Its value is an ICCBased colour space wrapping profile B. ~keep
    let page = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /ColorSpace << /DefaultCMYK [/ICCBased 6 0 R] >> >> /Contents 4 0 R >>\nendobj\n";
    buf.extend_from_slice(page.as_bytes());

    let stream_off = buf.len();
    // Bare /DeviceCMYK paint via `k` — the canonical case §8.6.5.6
    // redirects through /DefaultCMYK. ~keep
    let content = "0.25 0 0 0 k\n20 20 60 60 re\nf\n";
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let icc_a_off = buf.len();
    let icc_a_hdr = format!(
        "5 0 obj\n<< /N 4 /Length {} >>\nstream\n",
        output_intent_profile_a.len()
    );
    buf.extend_from_slice(icc_a_hdr.as_bytes());
    buf.extend_from_slice(output_intent_profile_a);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let icc_b_off = buf.len();
    let icc_b_hdr = format!("6 0 obj\n<< /N 4 /Length {} >>\nstream\n", default_cmyk_profile_b.len());
    buf.extend_from_slice(icc_b_hdr.as_bytes());
    buf.extend_from_slice(default_cmyk_profile_b);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = buf.len();
    let obj_count = 7;
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", obj_count).as_bytes());
    for off in [cat_off, pages_off, page_off, stream_off, icc_a_off, icc_b_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            obj_count, xref_off
        )
        .as_bytes(),
    );
    buf
}

/// Build a PDF whose page declares `/Resources /ColorSpace /DefaultRGB
/// [/ICCBased <stream>]` and paints bare `/DeviceRGB` with `rg`.
/// §8.6.5.6 redirects the bare paint through the /DefaultRGB override.
///
/// No /OutputIntents declared — RGB OutputIntents aren't carried by the
/// pipeline at all (only CMYK /N=4), so the only thing that can
/// influence bare-DeviceRGB rendering through this fixture is the
/// /DefaultRGB consumer the resolver gains in this phase.
///
/// Object layout:
///   1 — Catalog
///   2 — Pages
///   3 — Page (with Resources /ColorSpace /DefaultRGB → ICCBased
///       referencing 5 0 R)
///   4 — Content stream
///   5 — DefaultRGB embedded N=3 profile stream
fn build_pdf_default_rgb_overrides_bare_device_rgb(default_rgb_profile: &[u8]) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let page_off = buf.len();
    let page = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /ColorSpace << /DefaultRGB [/ICCBased 5 0 R] >> >> /Contents 4 0 R >>\nendobj\n";
    buf.extend_from_slice(page.as_bytes());

    let stream_off = buf.len();
    let content = "0.8 0.2 0.5 rg\n20 20 60 60 re\nf\n";
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let icc_off = buf.len();
    let icc_hdr = format!("5 0 obj\n<< /N 3 /Length {} >>\nstream\n", default_rgb_profile.len());
    buf.extend_from_slice(icc_hdr.as_bytes());
    buf.extend_from_slice(default_rgb_profile);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = buf.len();
    let obj_count = 6;
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", obj_count).as_bytes());
    for off in [cat_off, pages_off, page_off, stream_off, icc_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            obj_count, xref_off
        )
        .as_bytes(),
    );
    buf
}

/// Build a PDF whose page declares `/Resources /ColorSpace /DefaultGray
/// [/Separation /MagentaSpot /DeviceCMYK <Type-4>]` and paints bare
/// `/DeviceGray` with `g`. §8.6.5.6 redirects the bare paint through the
/// /DefaultGray override.
///
/// The Separation tint transform `{ 0.0 exch 0.0 0.0 }` consumes the
/// single gray input and emits CMYK(0, gray, 0, 0). For gray=0.5 the
/// alternate is CMYK(0, 0.5, 0, 0); §10.3.5 projects that to
/// RGB(255, 127, 255) — a magenta that's clearly distinct from the
/// literal grey RGB(127, 127, 127) the bare paint would produce
/// without the override.
///
/// Object layout:
///   1 — Catalog
///   2 — Pages
///   3 — Page (with Resources /ColorSpace /DefaultGray → Separation
///       array referencing 5 0 R)
///   4 — Content stream
///   5 — Tint-transform Type-4 stream
fn build_pdf_default_gray_routes_bare_device_gray() -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let page_off = buf.len();
    // /DefaultGray is a Separation that lifts the gray input into the
    // M channel; the alternate is /DeviceCMYK so §10.3.5 yields a
    // magenta-channel-only pixel. ~keep
    let page = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /ColorSpace << /DefaultGray [/Separation /MagentaSpot /DeviceCMYK 5 0 R] >> >> /Contents 4 0 R >>\nendobj\n";
    buf.extend_from_slice(page.as_bytes());

    let stream_off = buf.len();
    let content = "0.5 g\n20 20 60 60 re\nf\n";
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let tint_off = buf.len();
    let tint_program: &[u8] = b"{ 0.0 exch 0.0 0.0 }";
    let tint_hdr = format!(
        "5 0 obj\n<< /FunctionType 4 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] /Length {} >>\nstream\n",
        tint_program.len()
    );
    buf.extend_from_slice(tint_hdr.as_bytes());
    buf.extend_from_slice(tint_program);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = buf.len();
    let obj_count = 6;
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", obj_count).as_bytes());
    for off in [cat_off, pages_off, page_off, stream_off, tint_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            obj_count, xref_off
        )
        .as_bytes(),
    );
    buf
}

// Split for file-too-long (#1567): `include!` keeps one module scope, and a subdirectory
// file is not auto-discovered as its own test binary the way a sibling would be. ~keep
include!("test_render_output_intent/part2.rs");
include!("test_render_output_intent/part3.rs");
include!("test_render_output_intent/part4.rs");
include!("test_render_output_intent/part5.rs");
