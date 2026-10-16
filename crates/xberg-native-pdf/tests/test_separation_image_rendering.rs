//! Tests for raster image XObject routing in the separation-plate renderer
//! (ISO 32000-1 §8.9 image XObjects, §11.7.4 overprint placement).
//!
//! Spatial convention: 100×100 page rendered at 72 DPI = 100×100 pixel
//! plates. Images are placed with `50 0 0 50 25 25 cm` — a 50×50 user-space
//! square centred on the page, occupying image rows 25..75, cols 25..75.

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::rendering::{SeparationPlate, render_separations};

fn sample(plate: &SeparationPlate, x: u32, y: u32) -> u8 {
    plate.data[(y * plate.width + x) as usize]
}

fn plate<'a>(plates: &'a [SeparationPlate], name: &str) -> &'a SeparationPlate {
    plates.iter().find(|p| p.ink_name == name).unwrap_or_else(|| {
        panic!(
            "missing plate {name:?}; have {:?}",
            plates.iter().map(|p| p.ink_name.as_str()).collect::<Vec<_>>()
        )
    })
}

fn finalize_pdf(mut buf: Vec<u8>, offsets: Vec<usize>) -> Vec<u8> {
    let xref_offset = buf.len();
    buf.extend_from_slice(b"xref\n");
    buf.extend_from_slice(format!("0 {}\n", offsets.len() + 1).as_bytes());
    buf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            offsets.len() + 1,
            xref_offset
        )
        .as_bytes(),
    );
    buf
}

/// Build a single-page PDF where /Im1 is a `width × height` DeviceCMYK image
/// painted at the unit square via `50 0 0 50 25 25 cm`. `cmyk_samples` is
/// interleaved 8-bpc CMYK (W*H*4 bytes).
fn build_pdf_with_cmyk_image(cmyk_samples: &[u8], width: u32, height: u32) -> Vec<u8> {
    let content = b"q\n50 0 0 50 25 25 cm\n/Im1 Do\nQ\n";
    let mut buf = Vec::new();
    let mut offsets = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
           /Contents 4 0 R /Resources << /XObject << /Im1 5 0 R >> >> >>\nendobj\n",
    );
    offsets.push(buf.len());
    let hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(hdr.as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    let img_hdr = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width {w} /Height {h} \
         /ColorSpace /DeviceCMYK /BitsPerComponent 8 /Length {len} >>\nstream\n",
        w = width,
        h = height,
        len = cmyk_samples.len()
    );
    buf.extend_from_slice(img_hdr.as_bytes());
    buf.extend_from_slice(cmyk_samples);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    finalize_pdf(buf, offsets)
}

#[test]
fn cmyk_image_routes_channels_to_process_plates() {
    let cmyk: Vec<u8> = vec![255, 0, 0, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 0, 0, 255];
    let doc = PdfDocument::from_bytes(build_pdf_with_cmyk_image(&cmyk, 2, 2)).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let c = plate(&plates, "Cyan");
    let m = plate(&plates, "Magenta");
    let y = plate(&plates, "Yellow");
    let k = plate(&plates, "Black");

    // Sample inside each quadrant of the painted region (25..75 × 25..75).
    // Image top-left cell (Cyan) → plate top-left within the region. ~keep
    assert!(
        sample(c, 35, 35) > 200,
        "Cyan channel lands on Cyan plate (top-left quadrant); got {}",
        sample(c, 35, 35)
    );
    assert!(
        sample(m, 60, 35) > 200,
        "Magenta channel lands on Magenta plate (top-right quadrant); got {}",
        sample(m, 60, 35)
    );
    assert!(
        sample(y, 35, 60) > 200,
        "Yellow channel lands on Yellow plate (bottom-left quadrant); got {}",
        sample(y, 35, 60)
    );
    assert!(
        sample(k, 60, 60) > 200,
        "Black channel lands on Black plate (bottom-right quadrant); got {}",
        sample(k, 60, 60)
    );

    assert_eq!(sample(c, 5, 5), 0, "Cyan untouched outside image bbox");
    assert_eq!(sample(k, 5, 5), 0, "Black untouched outside image bbox");
}

#[test]
fn cmyk_image_inside_form_xobject_routes_to_plates() {
    let content = b"/F1 Do\n";
    let form_content = b"q\n50 0 0 50 25 25 cm\n/Im1 Do\nQ\n";
    let cmyk: Vec<u8> = vec![0, 0, 0, 255];

    let mut buf = Vec::new();
    let mut offsets = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
           /Contents 4 0 R /Resources << /XObject << /F1 5 0 R >> >> >>\nendobj\n",
    );
    offsets.push(buf.len());
    let hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(hdr.as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    let form_hdr = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
            /Resources << /XObject << /Im1 6 0 R >> >> /Length {} >>\nstream\n",
        form_content.len()
    );
    buf.extend_from_slice(form_hdr.as_bytes());
    buf.extend_from_slice(form_content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    let img_hdr = format!(
        "6 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 \
         /ColorSpace /DeviceCMYK /BitsPerComponent 8 /Length {} >>\nstream\n",
        cmyk.len()
    );
    buf.extend_from_slice(img_hdr.as_bytes());
    buf.extend_from_slice(&cmyk);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let pdf = finalize_pdf(buf, offsets);

    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let k = plate(&plates, "Black");
    assert!(
        sample(k, 50, 50) > 200,
        "K channel of nested-form CMYK image reaches the Black plate; got {}",
        sample(k, 50, 50)
    );
}

#[test]
fn separation_image_routes_to_spot_plate() {
    // 1×1 Separation /Pantone-185 image at full tint.
    // /ColorSpace is declared at page-level as /CS1 → indirect ref to
    // [/Separation /Pantone-185 /DeviceCMYK <tint transform>]. ~keep
    let content = b"q\n50 0 0 50 25 25 cm\n/Im1 Do\nQ\n";
    let samples: Vec<u8> = vec![255];

    let mut buf = Vec::new();
    let mut offsets = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
           /Contents 4 0 R \
           /Resources << /XObject << /Im1 5 0 R >> \
                        /ColorSpace << /CS1 6 0 R >> >> >>\nendobj\n",
    );
    offsets.push(buf.len());
    let hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(hdr.as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    let img_hdr = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 \
         /ColorSpace /CS1 /BitsPerComponent 8 /Length {} >>\nstream\n",
        samples.len()
    );
    buf.extend_from_slice(img_hdr.as_bytes());
    buf.extend_from_slice(&samples);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"6 0 obj\n[/Separation /Pantone-185 /DeviceCMYK 7 0 R]\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"7 0 obj\n<< /FunctionType 2 /Domain [0 1] /N 1 \
            /C0 [0 0 0 0] /C1 [0 0.85 0.45 0] >>\nendobj\n",
    );
    let pdf = finalize_pdf(buf, offsets);

    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let pantone = plate(&plates, "Pantone-185");
    assert!(
        sample(pantone, 50, 50) > 200,
        "Separation image lands on its named plate; got {}",
        sample(pantone, 50, 50)
    );
    let cyan = plate(&plates, "Cyan");
    assert_eq!(sample(cyan, 50, 50), 0, "Spot image leaves process plates untouched");
}

/// Build a single-page PDF with a 16-bpc DeviceCMYK image.
///
/// `samples` is interleaved big-endian 16-bit CMYK
/// (`width * height * 4 * 2` bytes).
fn build_pdf_with_16bpc_cmyk_image(samples: &[u8], width: u32, height: u32) -> Vec<u8> {
    let content = b"q\n50 0 0 50 25 25 cm\n/Im1 Do\nQ\n";
    let mut buf = Vec::new();
    let mut offsets = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
           /Contents 4 0 R /Resources << /XObject << /Im1 5 0 R >> >> >>\nendobj\n",
    );
    offsets.push(buf.len());
    let hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(hdr.as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    let img_hdr = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width {w} /Height {h} \
         /ColorSpace /DeviceCMYK /BitsPerComponent 16 /Length {len} >>\nstream\n",
        w = width,
        h = height,
        len = samples.len()
    );
    buf.extend_from_slice(img_hdr.as_bytes());
    buf.extend_from_slice(samples);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    finalize_pdf(buf, offsets)
}

/// §8.9.5: BitsPerComponent ∈ {1, 2, 4, 8, 16}. A 16-bpc image is reduced to
/// one byte per component during extraction, so by the time routing sees it
/// the samples are 8-bit and it is routed rather than skipped. What must hold
/// is that the reduction preserved channel identity: reading the high byte of
/// each 16-bit sample as a separate channel would smear the C channel across
/// M and Y.
///
/// The name this carried — `non_8bpc_image_is_skipped_not_mis_routed` — stopped
/// being true once extraction reduced 16-bpc samples: nothing is skipped here,
/// and asserting only that M/Y/K stay clean passes whether the image routes or
/// is dropped entirely. The Cyan assertion is what distinguishes them.
#[test]
fn sixteen_bpc_cmyk_image_routes_with_channels_intact() {
    let mut samples = Vec::with_capacity(2 * 2 * 4 * 2);
    for _ in 0..(2 * 2) {
        samples.extend_from_slice(&[0xFFu8, 0xFFu8]);
        samples.extend_from_slice(&[0x00u8, 0x00u8]);
        samples.extend_from_slice(&[0x00u8, 0x00u8]);
        samples.extend_from_slice(&[0x00u8, 0x00u8]);
    }
    let doc = PdfDocument::from_bytes(build_pdf_with_16bpc_cmyk_image(&samples, 2, 2)).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");

    let cyan = plate(&plates, "Cyan");
    assert!(
        sample(cyan, 50, 50) > 200,
        "16-bpc C=0xFFFF must reduce to full ink on the Cyan plate; got {}",
        sample(cyan, 50, 50)
    );

    // M / Y / K plates must stay clean even though the 8-bpc misread would
    // pick up nonzero bytes from the 16-bit C channel. ~keep
    for ink in ["Magenta", "Yellow", "Black"] {
        let p = plate(&plates, ink);
        assert_eq!(
            sample(p, 50, 50),
            0,
            "16-bpc CMYK image must not paint stray ink on the {ink} plate; \
             got {}",
            sample(p, 50, 50)
        );
    }
}

/// Build a single-page PDF with a Separation image carrying a `/Decode`
/// array. `decode` is two floats representing the [dmin, dmax] mapping
/// applied per channel.
fn build_pdf_with_separation_image_decode(samples: &[u8], width: u32, height: u32, decode: [f32; 2]) -> Vec<u8> {
    let content = b"q\n50 0 0 50 25 25 cm\n/Im1 Do\nQ\n";
    let mut buf = Vec::new();
    let mut offsets = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
           /Contents 4 0 R \
           /Resources << /XObject << /Im1 5 0 R >> \
                        /ColorSpace << /CS1 6 0 R >> >> >>\nendobj\n",
    );
    offsets.push(buf.len());
    let hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(hdr.as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    let img_hdr = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width {w} /Height {h} \
         /ColorSpace /CS1 /BitsPerComponent 8 \
         /Decode [{d0} {d1}] /Length {len} >>\nstream\n",
        w = width,
        h = height,
        d0 = decode[0],
        d1 = decode[1],
        len = samples.len()
    );
    buf.extend_from_slice(img_hdr.as_bytes());
    buf.extend_from_slice(samples);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"6 0 obj\n[/Separation /Pantone-185 /DeviceCMYK 7 0 R]\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"7 0 obj\n<< /FunctionType 2 /Domain [0 1] /N 1 \
            /C0 [0 0 0 0] /C1 [0 0.85 0.45 0] >>\nendobj\n",
    );
    finalize_pdf(buf, offsets)
}

/// §8.9.5.2: `/Decode [1 0]` inverts the sample-to-tint mapping. With raw
/// sample 0 and `/Decode [1 0]`, the decoded tint is 1.0 (full ink).
#[test]
fn separation_image_decode_array_inverts_routing() {
    let samples = vec![0u8]; // 1×1 raw sample of 0; under /Decode [1 0] → full tint ~keep
    let doc =
        PdfDocument::from_bytes(build_pdf_with_separation_image_decode(&samples, 1, 1, [1.0, 0.0])).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let pantone = plate(&plates, "Pantone-185");
    assert!(
        sample(pantone, 50, 50) > 200,
        "Inverted Separation image: raw 0 + /Decode [1 0] → full tint on the spot plate; got {}",
        sample(pantone, 50, 50)
    );
}

/// A 16-bpc Separation image whose `/Decode` array is longer than the colour
/// space's component count. The extractor's parser is exact (`len == ncomp*2`)
/// and rejects it, so nothing is folded into the samples; this path's parser is
/// lenient (`len >= ncomp*2`) and still owes the image the inversion.
fn build_pdf_with_16bpc_separation_overlong_decode(samples: &[u8], width: u32, height: u32) -> Vec<u8> {
    let content = b"q\n50 0 0 50 25 25 cm\n/Im1 Do\nQ\n";
    let mut buf = Vec::new();
    let mut offsets = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
           /Contents 4 0 R \
           /Resources << /XObject << /Im1 5 0 R >> \
                        /ColorSpace << /CS1 6 0 R >> >> >>\nendobj\n",
    );
    offsets.push(buf.len());
    let hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(hdr.as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    let img_hdr = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width {w} /Height {h} \
         /ColorSpace /CS1 /BitsPerComponent 16 \
         /Decode [1 0 1 0] /Length {len} >>\nstream\n",
        w = width,
        h = height,
        len = samples.len()
    );
    buf.extend_from_slice(img_hdr.as_bytes());
    buf.extend_from_slice(samples);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"6 0 obj\n[/Separation /Pantone-185 /DeviceCMYK 7 0 R]\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"7 0 obj\n<< /FunctionType 2 /Domain [0 1] /N 1 \
            /C0 [0 0 0 0] /C1 [0 0.85 0.45 0] >>\nendobj\n",
    );
    finalize_pdf(buf, offsets)
}

/// §8.9.5.2: plate routing applies `/Decode` itself, so it must key off "was a
/// `/Decode` folded in", not the wider "were these samples rescaled at all".
/// Reducing 16-bit samples to 8 rescales them without applying any map, so
/// reading the wider fact here drops an inversion the image is still owed.
#[test]
fn plate_routing_applies_decode_the_extractor_rejected() {
    let samples: Vec<u8> = vec![0xFF, 0xFF, 0x00, 0x00];
    let doc = PdfDocument::from_bytes(build_pdf_with_16bpc_separation_overlong_decode(&samples, 2, 1)).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let pantone = plate(&plates, "Pantone-185");
    assert!(
        sample(pantone, 30, 50) < 16,
        "raw-full-ink pixel inverts to no ink under /Decode [1 0]; got {}",
        sample(pantone, 30, 50)
    );
    assert!(
        sample(pantone, 70, 50) > 200,
        "raw-no-ink pixel inverts to full ink under /Decode [1 0]; got {}",
        sample(pantone, 70, 50)
    );
}

/// §8.9.6.2: ImageMask is a 1-bpc stencil. Sample value 0 marks the page
/// with the current non-stroking colour; value 1 leaves the pixel
/// transparent. With a Separation fill colour, the named spot plate must
/// show the marked area and other plates must stay untouched.
#[test]
fn image_mask_painted_with_spot_colour_routes_to_spot_plate() {
    // 8×8 stencil, all 0s. Per §8.9.6.2 every pixel is "painted" (alpha=255)
    // and the spot fill colour is composited through it. Packed 1-bit row
    // form is 1 byte per row = 0x00, 8 rows = 8 bytes. ~keep
    let stencil: Vec<u8> = vec![0x00; 8];
    let content = b"q\n/CS1 cs\n1 scn\n50 0 0 50 25 25 cm\n/Im1 Do\nQ\n";

    let mut buf = Vec::new();
    let mut offsets = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
           /Contents 4 0 R \
           /Resources << /XObject << /Im1 5 0 R >> \
                        /ColorSpace << /CS1 6 0 R >> >> >>\nendobj\n",
    );
    offsets.push(buf.len());
    let hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(hdr.as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    let img_hdr = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 8 /Height 8 \
         /ImageMask true /BitsPerComponent 1 /Length {} >>\nstream\n",
        stencil.len()
    );
    buf.extend_from_slice(img_hdr.as_bytes());
    buf.extend_from_slice(&stencil);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"6 0 obj\n[/Separation /Pantone-185 /DeviceCMYK 7 0 R]\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"7 0 obj\n<< /FunctionType 2 /Domain [0 1] /N 1 \
            /C0 [0 0 0 0] /C1 [0 0.85 0.45 0] >>\nendobj\n",
    );
    let pdf = finalize_pdf(buf, offsets);

    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");

    let pantone = plate(&plates, "Pantone-185");
    assert!(
        sample(pantone, 50, 50) > 200,
        "ImageMask painted with Separation colour reaches the spot plate; got {}",
        sample(pantone, 50, 50)
    );
    let cyan = plate(&plates, "Cyan");
    assert_eq!(
        sample(cyan, 50, 50),
        0,
        "ImageMask with spot fill does not paint the process plates"
    );
}

/// DeviceN with two named spot inks: each channel routes to its named plate.
#[test]
fn devicen_image_routes_each_channel_to_its_named_plate() {
    let samples: Vec<u8> = vec![255, 0, 0, 255];
    let content = b"q\n50 0 0 50 25 25 cm\n/Im1 Do\nQ\n";

    let mut buf = Vec::new();
    let mut offsets = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
           /Contents 4 0 R \
           /Resources << /XObject << /Im1 5 0 R >> \
                        /ColorSpace << /CS1 6 0 R >> >> >>\nendobj\n",
    );
    offsets.push(buf.len());
    let hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(hdr.as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    let img_hdr = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 2 /Height 1 \
         /ColorSpace /CS1 /BitsPerComponent 8 /Length {} >>\nstream\n",
        samples.len()
    );
    buf.extend_from_slice(img_hdr.as_bytes());
    buf.extend_from_slice(&samples);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"6 0 obj\n[/DeviceN [/SpotRed /SpotBlue] /DeviceCMYK 7 0 R]\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"7 0 obj\n<< /FunctionType 2 /Domain [0 1 0 1] /N 1 \
            /C0 [0 0 0 0] /C1 [0 1 1 0] >>\nendobj\n",
    );
    let pdf = finalize_pdf(buf, offsets);

    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");

    let red = plate(&plates, "SpotRed");
    let blue = plate(&plates, "SpotBlue");
    assert!(
        sample(red, 30, 50) > 200,
        "DeviceN channel 0 lands on SpotRed plate (left half); got {}",
        sample(red, 30, 50)
    );
    assert!(
        sample(blue, 70, 50) > 200,
        "DeviceN channel 1 lands on SpotBlue plate (right half); got {}",
        sample(blue, 70, 50)
    );
    assert!(
        sample(red, 70, 50) < 16,
        "SpotRed plate does not pick up channel-1 ink in the right half; got {}",
        sample(red, 70, 50)
    );
    assert!(
        sample(blue, 30, 50) < 16,
        "SpotBlue plate does not pick up channel-0 ink in the left half; got {}",
        sample(blue, 30, 50)
    );
}

/// Encode an 8x8 CMYK image as a JPEG whose RAW DCT sample plane equals
/// `raw_cmyk` per channel (straight CMYK: 0 = no ink, 255 = full ink - the
/// convention real Distiller / Quark PDFs write and poppler / Ghostscript
/// decode).
///
/// `jpeg-encoder`'s `ColorType::Cmyk` path inverts on encode (stores
/// `255 - input` in the entropy stream) and writes an Adobe APP14
/// `color_transform = 0` marker, so we feed it the per-channel complement to
/// land the intended raw samples in the stream. On the read side jpeg-decoder
/// applies its own `255 - x` for the Adobe marker, which
/// `decode_cmyk_jpeg_to_raw_cmyk` undoes, yielding `raw_cmyk` back.
fn encode_cmyk_jpeg(raw_cmyk: &[u8], width: u16, height: u16) -> Vec<u8> {
    use jpeg_encoder::{ColorType, Encoder};
    let inverted: Vec<u8> = raw_cmyk.iter().map(|b| 255 - *b).collect();
    let mut out = Vec::new();
    let encoder = Encoder::new(&mut out, 95);
    encoder
        .encode(&inverted, width, height, ColorType::Cmyk)
        .expect("encode CMYK JPEG");
    out
}

/// Build a single-page PDF with a JPEG-encoded DeviceCMYK image.
fn build_pdf_with_cmyk_jpeg(jpeg: &[u8], width: u32, height: u32) -> Vec<u8> {
    let content = b"q\n50 0 0 50 25 25 cm\n/Im1 Do\nQ\n";
    let mut buf = Vec::new();
    let mut offsets = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
           /Contents 4 0 R /Resources << /XObject << /Im1 5 0 R >> >> >>\nendobj\n",
    );
    offsets.push(buf.len());
    let hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(hdr.as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    let img_hdr = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width {w} /Height {h} \
         /ColorSpace /DeviceCMYK /BitsPerComponent 8 \
         /Filter /DCTDecode /Length {len} >>\nstream\n",
        w = width,
        h = height,
        len = jpeg.len()
    );
    buf.extend_from_slice(img_hdr.as_bytes());
    buf.extend_from_slice(jpeg);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    finalize_pdf(buf, offsets)
}

/// CMYK JPEG with an Adobe APP14 marker (color_transform = 0). jpeg-decoder
/// applies a `255 - x` inversion to every Adobe-marked CMYK JPEG; poppler /
/// Ghostscript instead treat the raw DCT samples as straight CMYK ink, so
/// `decode_cmyk_jpeg_to_raw_cmyk` undoes that inversion. This test builds a
/// straight-CMYK pure-cyan image (raw DCT samples C=255, M=Y=K=0 - the
/// real-world Distiller / Quark convention) and checks it lands on the Cyan
/// plate. Without the inversion-undo the samples would read M = Y = K high
/// and Cyan low - the near-black failure mode this test guards against.
#[test]
fn cmyk_jpeg_app14_inversion_round_trips_to_correct_plate() {
    let mut cmyk = Vec::with_capacity(8 * 8 * 4);
    for _ in 0..(8 * 8) {
        cmyk.extend_from_slice(&[255, 0, 0, 0]);
    }
    let jpeg = encode_cmyk_jpeg(&cmyk, 8, 8);
    let doc = PdfDocument::from_bytes(build_pdf_with_cmyk_jpeg(&jpeg, 8, 8)).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");

    let cyan_v = sample(plate(&plates, "Cyan"), 50, 50);
    let magenta_v = sample(plate(&plates, "Magenta"), 50, 50);
    let yellow_v = sample(plate(&plates, "Yellow"), 50, 50);
    let black_v = sample(plate(&plates, "Black"), 50, 50);

    assert!(
        cyan_v > 200,
        "Cyan plate after APP14 inversion; got {cyan_v} (M={magenta_v} Y={yellow_v} K={black_v})"
    );
    assert!(
        magenta_v < 50 && yellow_v < 50 && black_v < 50,
        "M / Y / K plates must stay near zero for a pure-cyan source after APP14 \
         inversion; got M={magenta_v} Y={yellow_v} K={black_v}"
    );
}

/// A four-component JPEG 2000 image on a `/DeviceCMYK` XObject paints the process plates.
///
/// The module used to document a JPX skip path ("no pure-Rust JP2 decoder is bundled") guarded by
/// a predicate that returned `false` unconditionally, so the branch was unreachable and the
/// statement false: `hayro-jpeg2000` is a non-optional dependency and a JPX image is decoded and
/// routed by its colour space like any other. This pins that, so the claim cannot come back
/// undetected. See GH#1855.
///
/// The fixture is a 16x16 lossless codestream with one channel saturated per quadrant, so each
/// plate is checked at a sample point no other channel can reach. It is a bare codestream rather
/// than a JP2 container because a four-component JP2 has no enumerated colour-space value to
/// declare, and the PDF's `/ColorSpace` is what decides routing regardless. Built with
/// `opj_compress -i quad.raw -o out.j2k -F 16,16,4,8,u -r 1 -n 3`; note that OpenJPEG's raw
/// reader is **component-planar**, so interleaved input silently produces a valid codestream of
/// the wrong image -- and it round-trips through `opj_decompress` byte-identical either way,
/// because the writer shares the convention.
const CMYK_QUADRANTS_J2K: &[u8] = include_bytes!("fixtures/jpx/gh1855_cmyk_quadrants.j2k");

fn build_pdf_with_jpx_image(codestream: &[u8], width: u32, height: u32, color_space: Option<&str>) -> Vec<u8> {
    let content = b"q\n50 0 0 50 25 25 cm\n/Im1 Do\nQ\n";
    let mut buf = Vec::new();
    let mut offsets = Vec::new();
    buf.extend_from_slice(b"%PDF-1.5\n");

    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
           /Contents 4 0 R /Resources << /XObject << /Im1 5 0 R >> >> >>\nendobj\n",
    );
    offsets.push(buf.len());
    let hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(hdr.as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    let color_space = color_space.map_or(String::new(), |space| format!(" /ColorSpace {space}"));
    let img_hdr = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width {w} /Height {h} \
         {color_space} /BitsPerComponent 8 /Filter /JPXDecode /Length {len} >>\nstream\n",
        w = width,
        h = height,
        len = codestream.len()
    );
    buf.extend_from_slice(img_hdr.as_bytes());
    buf.extend_from_slice(codestream);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    finalize_pdf(buf, offsets)
}

/// Wrap a bare JPEG 2000 codestream in the minimal JP2 box structure, declaring CMYK through an
/// enumerated `colr` box.
///
/// Load-bearing, not cosmetic: `hayro-jpeg2000` reads a *bare* codestream's colour space by
/// assuming `Srgb` for any image with three or more components, ignoring the `Csiz` field, and
/// then reconciles the 4-vs-3 mismatch by treating the fourth channel as alpha. So this fixture,
/// whose codestream does declare `Csiz=4`, decodes as 3-component RGB unless the colour space is
/// stated in a JP2 box, where `jp2::parse` reads it directly. `/JPXDecode` accepts either form.
/// ~keep
fn wrap_as_cmyk_jp2(codestream: &[u8]) -> Vec<u8> {
    fn jp2_box(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + data.len());
        out.extend_from_slice(&((8 + data.len()) as u32).to_be_bytes());
        out.extend_from_slice(tag);
        out.extend_from_slice(data);
        out
    }

    const ENUM_CS_CMYK: u32 = 12;
    let signature_box = jp2_box(b"jP  ", &[0x0D, 0x0A, 0x87, 0x0A]);
    let mut ftyp_data = b"jp2 ".to_vec();
    ftyp_data.extend_from_slice(&0u32.to_be_bytes());
    ftyp_data.extend_from_slice(b"jp2 ");
    let ftyp_box = jp2_box(b"ftyp", &ftyp_data);
    let mut colr_data = vec![1u8, 0, 0];
    colr_data.extend_from_slice(&ENUM_CS_CMYK.to_be_bytes());
    let colr_box = jp2_box(b"colr", &colr_data);
    let jp2h_box = jp2_box(b"jp2h", &colr_box);
    let jp2c_box = jp2_box(b"jp2c", codestream);

    [signature_box, ftyp_box, jp2h_box, jp2c_box].concat()
}

#[test]
fn jpx_cmyk_image_routes_channels_to_process_plates() {
    let doc = PdfDocument::from_bytes(build_pdf_with_jpx_image(
        &wrap_as_cmyk_jp2(CMYK_QUADRANTS_J2K),
        16,
        16,
        Some("/DeviceCMYK"),
    ))
    .expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");

    let quadrant_samples = [
        ("Cyan", 35u32, 35u32),
        ("Magenta", 60, 35),
        ("Yellow", 35, 60),
        ("Black", 60, 60),
    ];
    for (ink, x, y) in quadrant_samples {
        let value = sample(plate(&plates, ink), x, y);
        assert!(
            value > 200,
            "the {ink} quadrant of a JPXDecode DeviceCMYK image must reach the {ink} plate; \
             got {value} at ({x}, {y})"
        );
        for (other, _, _) in quadrant_samples.iter().filter(|(other, _, _)| *other != ink) {
            let bleed = sample(plate(&plates, other), x, y);
            assert!(
                bleed < 50,
                "the {ink} quadrant must leave the {other} plate near zero; got {bleed}"
            );
        }
    }

    assert_eq!(
        sample(plate(&plates, "Cyan"), 5, 5),
        0,
        "plates stay untouched outside the image bbox"
    );
}

#[test]
fn a_gray_jpx_without_a_colorspace_routes_to_the_black_plate() {
    const GRAY_JP2: &[u8] = include_bytes!("fixtures/jpx/gh1885_indices_grey.jp2");
    let doc = PdfDocument::from_bytes(build_pdf_with_jpx_image(GRAY_JP2, 120, 40, None)).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let black = plate(&plates, "Black");
    assert!(
        black.data.as_chunks::<4>().0.iter().any(|pixel| pixel[0] > 200),
        "the grayscale image's dark glyphs must reach the Black plate"
    );
    for ink in ["Cyan", "Magenta", "Yellow"] {
        assert!(
            plate(&plates, ink)
                .data
                .as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| pixel[0] < 50),
            "the grayscale image must not reach the {ink} plate"
        );
    }
}

/// GH#1898: the separation classifier had no arm for `/Indexed`, so an
/// Indexed image always resolved to `Unknown` and was skipped before
/// extraction, whatever ink intent its base colour space carried.
///
/// Build a single-page PDF with a 2x2 `/Indexed /DeviceCMYK` image whose
/// palette's only entry (index 0, `hival` 1) is pure cyan; every pixel's raw
/// sample is index 0. The palette stream is a separate indirect object, the
/// same pattern `test_indexed_palette_starts_with_cr.rs` uses.
fn build_pdf_with_indexed_cmyk_image(indices: &[u8], width: u32, height: u32) -> Vec<u8> {
    let content = b"q\n50 0 0 50 25 25 cm\n/Im1 Do\nQ\n";
    // hival=1 needs (1+1)*4 = 8 palette bytes: index 0 = pure cyan
    // CMYK(255,0,0,0), index 1 is unused by this fixture. ~keep
    let palette: [u8; 8] = [255, 0, 0, 0, 0, 0, 0, 0];

    let mut buf = Vec::new();
    let mut offsets = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
           /Contents 4 0 R /Resources << /XObject << /Im1 5 0 R >> >> >>\nendobj\n",
    );
    offsets.push(buf.len());
    let hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(hdr.as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    let img_hdr = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width {w} /Height {h} \
         /ColorSpace 6 0 R /BitsPerComponent 8 /Length {len} >>\nstream\n",
        w = width,
        h = height,
        len = indices.len()
    );
    buf.extend_from_slice(img_hdr.as_bytes());
    buf.extend_from_slice(indices);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"6 0 obj\n[/Indexed /DeviceCMYK 1 7 0 R]\nendobj\n");
    offsets.push(buf.len());
    let pal_hdr = format!("7 0 obj\n<< /Length {} >>\nstream\n", palette.len());
    buf.extend_from_slice(pal_hdr.as_bytes());
    buf.extend_from_slice(&palette);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    finalize_pdf(buf, offsets)
}

#[test]
fn indexed_cmyk_image_routes_base_ink_to_cyan_plate() {
    // 2x2 image, every raw sample is palette index 0 = pure cyan CMYK(255,0,0,0). ~keep
    let indices: Vec<u8> = vec![0, 0, 0, 0];
    let doc = PdfDocument::from_bytes(build_pdf_with_indexed_cmyk_image(&indices, 2, 2)).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");

    let cyan = plate(&plates, "Cyan");
    assert!(
        sample(cyan, 50, 50) > 200,
        "an /Indexed /DeviceCMYK image's pure-cyan palette entry must reach the \
         Cyan plate; got {}",
        sample(cyan, 50, 50)
    );
    let magenta = plate(&plates, "Magenta");
    assert_eq!(
        sample(magenta, 50, 50),
        0,
        "the palette entry carries no magenta; the Magenta plate must stay untouched"
    );
}
