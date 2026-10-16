use super::super::*;
use super::common::*;

/// Helper: build a minimal PDF whose single character maps to U+FB01 (LATIN SMALL
/// LIGATURE FI) via a ToUnicode CMap. This exercises the path where pdfium hands us
/// U+FB01 from the font's ToUnicode map and we must NOT expand it to "fi".
pub(super) fn build_ligature_fi_pdf() -> Vec<u8> {
    let cmap = "/CIDInit /ProcSet findresource begin\n\
                    12 dict begin\n\
                    begincmap\n\
                    /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
                    /CMapName /Adobe-Identity-UCS def\n\
                    /CMapType 2 def\n\
                    1 begincodespacerange\n\
                    <01> <01>\n\
                    endcodespacerange\n\
                    1 beginbfchar\n\
                    <01> <FB01>\n\
                    endbfchar\n\
                    endcmap\n\
                    CMapName currentdict /CMap defineresource pop\n\
                    end\n\
                    end\n";

    // Content stream: BT /F1 12 Tf 100 500 Td (\001) Tj ET ~keep
    let content = "BT /F1 12 Tf 100 500 Td (\\001) Tj ET\n";

    let mut out: Vec<u8> = Vec::new();
    let mut off: Vec<usize> = vec![0];

    out.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");

    macro_rules! push {
        ($body:expr) => {{
            off.push(out.len());
            let id = off.len() - 1;
            out.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", id, $body).as_bytes());
        }};
    }

    push!("<< /Type /Catalog /Pages 2 0 R >>");
    push!("<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    push!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
             /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"
    );
    push!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
             /Encoding << /Type /Encoding /Differences [1 /fi] >> \
             /ToUnicode 6 0 R >>"
    );
    push!(format!("<< /Length {} >>\nstream\n{}endstream", content.len(), content));
    push!(format!("<< /Length {} >>\nstream\n{}endstream", cmap.len(), cmap));

    let xref_offset = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", off.len()).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for &o in &off[1..] {
        out.extend_from_slice(format!("{:010} 00000 n \n", o).as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            off.len(),
            xref_offset
        )
        .as_bytes(),
    );
    out
}

pub(super) fn build_pdf_with_annotations(annot_objects: Vec<(usize, Vec<u8>)>) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets: Vec<(usize, usize)> = Vec::new();

    let off1 = pdf.len();
    offsets.push((1, off1));
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    offsets.push((2, off2));
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let annot_refs: String = annot_objects
        .iter()
        .map(|(num, _)| format!("{} 0 R", num))
        .collect::<Vec<_>>()
        .join(" ");

    let off3 = pdf.len();
    offsets.push((3, off3));
    let page_str = format!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> /Annots [{}] >>\nendobj\n",
        annot_refs
    );
    pdf.extend_from_slice(page_str.as_bytes());

    for (obj_num, obj_data) in &annot_objects {
        let off = pdf.len();
        offsets.push((*obj_num, off));
        pdf.extend_from_slice(obj_data);
    }

    let max_obj = offsets.iter().map(|(n, _)| *n).max().unwrap_or(0);
    let xref_off = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", max_obj + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for obj_num in 1..=max_obj {
        if let Some((_, off)) = offsets.iter().find(|(n, _)| *n == obj_num) {
            pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
        } else {
            pdf.extend_from_slice(b"0000000000 65535 f \n");
        }
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            max_obj + 1,
            xref_off
        )
        .as_bytes(),
    );
    pdf
}

pub(super) fn build_catalog_test_pdf(catalog: &[u8], pages: &[u8], extra_objects: &[(u32, &[u8])]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let max_object_id = extra_objects.iter().map(|(id, _)| *id).max().unwrap_or(2).max(2);
    let mut offsets = vec![None; max_object_id as usize + 1];
    for (object_id, body) in std::iter::once((1u32, catalog))
        .chain(std::iter::once((2u32, pages)))
        .chain(extra_objects.iter().copied())
    {
        offsets[object_id as usize] = Some(pdf.len());
        pdf.extend_from_slice(format!("{object_id} 0 obj\n").as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes());
    for offset in offsets.into_iter().skip(1) {
        match offset {
            Some(offset) => pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes()),
            None => pdf.extend_from_slice(b"0000000000 00000 f \n"),
        }
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            max_object_id + 1
        )
        .as_bytes(),
    );
    pdf
}

pub(super) fn corrupt_third_object_header(mut pdf: Vec<u8>) -> Vec<u8> {
    const HEADER: &[u8] = b"3 0 obj";
    let position = pdf
        .windows(HEADER.len())
        .position(|window| window == HEADER)
        .expect("test object header must exist");
    pdf[position..position + HEADER.len()].copy_from_slice(b"X 0 bad");
    pdf
}

/// Build a span with explicit width and text (for corridor-geometry tests).
#[cfg(test)]
pub(super) fn corridor_span(text: &str, x: f32, y: f32, w: f32) -> crate::layout::TextSpan {
    use crate::geometry::Rect;
    use crate::layout::{Color, FontWeight, TextSpan};
    TextSpan {
        provenance: None,
        text_rise: 0.0,
        artifact_type: None,
        text: text.to_string(),
        bbox: Rect::new(x, y, w, 10.0),
        font_size: 10.0,
        font_name: "Test".to_string(),
        font_weight: FontWeight::Normal,
        is_italic: false,
        is_monospace: false,
        color: Color { r: 0.0, g: 0.0, b: 0.0 },
        mcid: None,
        mcid_scope: None,
        sequence: 0,
        split_boundary_before: false,
        offset_semantic: false,
        char_spacing: 0.0,
        word_spacing: 0.0,
        horizontal_scaling: 100.0,
        primary_detected: false,
        char_widths: vec![],
        char_x_offsets: Vec::new(),
        heading_level: None,
        rotation_degrees: 0.0,
        wmode: 0,
        rtl_draw_logical: false,
        mirrored: false,
        page_rotation_applied: 0,
    }
}

pub(super) fn oc_test_doc() -> PdfDocument {
    PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap()
}

pub(super) fn ocg_dict(name: Object) -> Object {
    let mut d = std::collections::HashMap::new();
    d.insert("Type".to_string(), Object::Name("OCG".to_string()));
    d.insert("Name".to_string(), name);
    Object::Dictionary(d)
}

pub(super) fn ocmd_dict(ocgs: Object) -> Object {
    let mut d = std::collections::HashMap::new();
    d.insert("Type".to_string(), Object::Name("OCMD".to_string()));
    d.insert("OCGs".to_string(), ocgs);
    Object::Dictionary(d)
}

pub(super) fn utf16_string(s: &str, big_endian: bool) -> Object {
    let mut bytes = if big_endian { vec![0xFE, 0xFF] } else { vec![0xFF, 0xFE] };
    for u in s.encode_utf16() {
        if big_endian {
            bytes.extend_from_slice(&u.to_be_bytes());
        } else {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
    }
    Object::String(bytes)
}

/// Like [`build_minimal_pdf`] but with a caller-supplied `/MediaBox` array
/// literal (e.g. `"10 -100 622 692"`), for GH#1653 extent-vs-corner tests. ~keep
pub(super) fn build_minimal_pdf_with_media_box(media_box: &str, content: &[u8]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();

    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off3 = pdf.len();
    pdf.extend_from_slice(
        format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [{media_box}] /Contents 4 0 R /Resources << >> >>\nendobj\n"
        )
        .as_bytes(),
    );

    let off4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 5\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off4).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    pdf
}

/// Builds a single-page PDF whose `/Rotate` entry is the caller-supplied raw
/// PDF token (`"90"`, `"-90"`, `"135"`, `"90.5"`, `"(bogus)"`, ...), placed on
/// the page dict (`inherited = false`) or the parent `/Pages` node
/// (`inherited = true`). Mirrors `xberg::pdf::render::build_pdf_with_rotate`'s
/// fixture shape (out of scope to reuse directly - that file is owned by a
/// concurrent change), built with this crate's own manual PDF writer instead
/// of `lopdf`, which this crate does not depend on. ~keep
pub(super) fn build_pdf_with_rotate_token(rotate_token: Option<&str>, inherited: bool) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();

    let page_rotate = if inherited {
        String::new()
    } else {
        rotate_token.map_or(String::new(), |t| format!(" /Rotate {t}"))
    };
    let pages_rotate = if inherited {
        rotate_token.map_or(String::new(), |t| format!(" /Rotate {t}"))
    } else {
        String::new()
    };

    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    pdf.extend_from_slice(
        format!("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1{pages_rotate} >>\nendobj\n").as_bytes(),
    );

    let off3 = pdf.len();
    pdf.extend_from_slice(
        format!("3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100]{page_rotate} >>\nendobj\n").as_bytes(),
    );

    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 4\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in [off1, off2, off3] {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    pdf
}

/// Build a minimal TrueType font with a cmap format 4 table mapping each
/// `(char_code, glyph_id)` pair.
///
/// Mirrors the builder in `fonts::truetype_cmap`'s own test module;
/// duplicated here on purpose — that copy pins cmap TABLE PARSING, this one
/// exists only to give `donate_truetype_cmaps_within_set` a font with a real,
/// lazily-parseable `truetype_cmap()` to donate. ~keep
pub(super) fn build_truetype_with_cmap_format4(mappings: &[(u16, u16)]) -> Vec<u8> {
    use byteorder::{BigEndian, WriteBytesExt};

    let mut data = Vec::new();

    data.write_u32::<BigEndian>(0x00010000).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(16).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();

    let cmap_offset: u32 = 12 + 16;
    data.write_u32::<BigEndian>(0x636D_6170).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>(cmap_offset).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();

    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();

    let subtable_offset: u32 = 4 + 8;
    data.write_u16::<BigEndian>(3).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u32::<BigEndian>(subtable_offset).unwrap();

    let mut segments: Vec<(u16, u16, i16)> = Vec::new();
    for &(char_code, gid) in mappings {
        let delta = gid as i16 - char_code as i16;
        segments.push((char_code, char_code, delta));
    }
    segments.push((0xFFFF, 0xFFFF, 1));

    let seg_count = segments.len();
    let seg_count_x2 = (seg_count * 2) as u16;

    data.write_u16::<BigEndian>(4).unwrap();
    let length_pos = data.len();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();

    data.write_u16::<BigEndian>(seg_count_x2).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();

    for seg in &segments {
        data.write_u16::<BigEndian>(seg.1).unwrap();
    }
    data.write_u16::<BigEndian>(0).unwrap();
    for seg in &segments {
        data.write_u16::<BigEndian>(seg.0).unwrap();
    }
    for seg in &segments {
        data.write_i16::<BigEndian>(seg.2).unwrap();
    }
    for _ in &segments {
        data.write_u16::<BigEndian>(0).unwrap();
    }

    let fmt4_start = length_pos - 2;
    let fmt4_len = data.len() - fmt4_start;
    let len_bytes = (fmt4_len as u16).to_be_bytes();
    data[length_pos] = len_bytes[0];
    data[length_pos + 1] = len_bytes[1];

    data
}

/// Build the object graph for a `/Font` dictionary holding a donor
/// (embedded TrueType with a real cmap) and a recipient (Type0, Identity-H,
/// same stripped base name, no cmap of its own) directly in `doc`'s object
/// cache, and return the `/Font` dict's own `ObjectRef` plus a `Resources`
/// object pointing at it.
pub(super) fn donor_and_recipient_font_resources(doc: &PdfDocument) -> (ObjectRef, Object) {
    let donor_ref = ObjectRef::new(11, 0);
    let descriptor_ref = ObjectRef::new(12, 0);
    let font_file_ref = ObjectRef::new(13, 0);
    let recipient_ref = ObjectRef::new(14, 0);
    let font_dict_ref = ObjectRef::new(15, 0);

    let font_program = build_truetype_with_cmap_format4(&[(0x41, 3)]);

    doc.object_cache.lock_or_recover().insert(
        font_file_ref,
        Object::Stream {
            dict: HashMap::new(),
            data: bytes::Bytes::from(font_program),
        },
    );
    doc.object_cache.lock_or_recover().insert(
        descriptor_ref,
        Object::Dictionary(HashMap::from([
            ("Flags".to_string(), Object::Integer(32)),
            ("FontFile2".to_string(), Object::Reference(font_file_ref)),
        ])),
    );
    doc.object_cache.lock_or_recover().insert(
        donor_ref,
        Object::Dictionary(HashMap::from([
            ("Type".to_string(), Object::Name("Font".to_string())),
            ("Subtype".to_string(), Object::Name("TrueType".to_string())),
            (
                "BaseFont".to_string(),
                Object::Name("ABCDEF+DonationTest1746".to_string()),
            ),
            ("FontDescriptor".to_string(), Object::Reference(descriptor_ref)),
        ])),
    );
    doc.object_cache.lock_or_recover().insert(
        recipient_ref,
        Object::Dictionary(HashMap::from([
            ("Type".to_string(), Object::Name("Font".to_string())),
            ("Subtype".to_string(), Object::Name("Type0".to_string())),
            (
                "BaseFont".to_string(),
                Object::Name("GHIJKL+DonationTest1746".to_string()),
            ),
            ("Encoding".to_string(), Object::Name("Identity-H".to_string())),
        ])),
    );
    doc.object_cache.lock_or_recover().insert(
        font_dict_ref,
        Object::Dictionary(HashMap::from([
            ("F1".to_string(), Object::Reference(donor_ref)),
            ("F2".to_string(), Object::Reference(recipient_ref)),
        ])),
    );

    let resources = Object::Dictionary(HashMap::from([("Font".to_string(), Object::Reference(font_dict_ref))]));
    (font_dict_ref, resources)
}

/// Build a PDF whose page tree is a chain of `levels` `/Pages` nodes, each with a
/// single kid, ending in one `/Page`. The root node carries the inheritable
/// `/MediaBox` and `/Resources`, so a walk that reaches the leaf can be told apart
/// from one that recovered it by scanning: only the former merges those in.
///
/// No node carries `/Count`. That is what forces every walker to actually descend
/// — `get_page_count_standard` bails without it, and `collect_page_refs` skips its
/// flat-subtree fast path — which is the shape GH#1755 reports.
pub(super) fn build_page_tree_chain_pdf(levels: usize) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets: Vec<usize> = Vec::new();

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    for level in 0..levels {
        let id = 2 + level;
        offsets.push(pdf.len());
        let inheritable = if level == 0 {
            " /MediaBox [0 0 612 792] /Resources << >>"
        } else {
            ""
        };
        pdf.extend_from_slice(
            format!(
                "{} 0 obj\n<< /Type /Pages /Kids [{} 0 R]{} >>\nendobj\n",
                id,
                id + 1,
                inheritable
            )
            .as_bytes(),
        );
    }

    let page_id = 2 + levels;
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!(
            "{} 0 obj\n<< /Type /Page /Parent {} 0 R >>\nendobj\n",
            page_id,
            page_id - 1
        )
        .as_bytes(),
    );

    let size = page_id + 1;
    let xref_off = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", size).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            size, xref_off
        )
        .as_bytes(),
    );
    pdf
}
