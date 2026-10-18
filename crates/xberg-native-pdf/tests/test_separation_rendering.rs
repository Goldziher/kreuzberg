//! Tests for separation plate rendering.
//!
//! Verifies that individual ink separation plates are rendered correctly
//! as grayscale images where pixel intensity = tint percentage.

mod tests {
    use xberg_native_pdf::document::PdfDocument;
    use xberg_native_pdf::rendering::{render_separation, render_separations};

    /// Build a minimal PDF with a Separation color space and a filled rectangle.
    ///
    /// The page is 100x100 pt with a 50x50 pt rectangle centered at (25,25)
    /// filled with the given ink at the given tint.
    fn build_separation_pdf(ink_name: &str, tint: f32) -> Vec<u8> {
        let content = format!("/CS1 cs\n{} scn\n25 25 50 50 re f\n", tint);
        let content_bytes = content.as_bytes();

        let mut buf = Vec::new();
        let mut offsets = Vec::new();

        buf.extend_from_slice(b"%PDF-1.4\n");

        offsets.push(buf.len());
        buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        offsets.push(buf.len());
        buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

        offsets.push(buf.len());
        buf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << /ColorSpace << /CS1 5 0 R >> >> >>\nendobj\n",
        );

        offsets.push(buf.len());
        let stream_header = format!("4 0 obj\n<< /Length {} >>\nstream\n", content_bytes.len());
        buf.extend_from_slice(stream_header.as_bytes());
        buf.extend_from_slice(content_bytes);
        buf.extend_from_slice(b"\nendstream\nendobj\n");

        offsets.push(buf.len());
        let cs = format!("5 0 obj\n[/Separation /{} /DeviceGray 6 0 R]\nendobj\n", ink_name);
        buf.extend_from_slice(cs.as_bytes());

        offsets.push(buf.len());
        buf.extend_from_slice(b"6 0 obj\n<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>\nendobj\n");

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

    /// Build a PDF with DeviceCMYK content (a filled rectangle).
    fn build_cmyk_pdf(c: f32, m: f32, y: f32, k: f32) -> Vec<u8> {
        let content = format!("{} {} {} {} k\n25 25 50 50 re f\n", c, m, y, k);
        let content_bytes = content.as_bytes();

        let mut buf = Vec::new();
        let mut offsets = Vec::new();

        buf.extend_from_slice(b"%PDF-1.4\n");

        offsets.push(buf.len());
        buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        offsets.push(buf.len());
        buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

        offsets.push(buf.len());
        buf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << >> >>\nendobj\n",
        );

        offsets.push(buf.len());
        let stream_header = format!("4 0 obj\n<< /Length {} >>\nstream\n", content_bytes.len());
        buf.extend_from_slice(stream_header.as_bytes());
        buf.extend_from_slice(content_bytes);
        buf.extend_from_slice(b"\nendstream\nendobj\n");

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

    /// Build a PDF with a DeviceN color space containing multiple inks.
    fn build_devicen_pdf(ink_names: &[&str], tints: &[f32]) -> Vec<u8> {
        assert_eq!(ink_names.len(), tints.len());

        let tint_str: String = tints.iter().map(|t| format!("{} ", t)).collect();
        let content = format!("/CS1 cs\n{}scn\n25 25 50 50 re f\n", tint_str);
        let content_bytes = content.as_bytes();

        let inks_str: String = ink_names.iter().map(|n| format!("/{} ", n)).collect();

        let mut buf = Vec::new();
        let mut offsets = Vec::new();

        buf.extend_from_slice(b"%PDF-1.4\n");

        offsets.push(buf.len());
        buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        offsets.push(buf.len());
        buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

        offsets.push(buf.len());
        buf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << /ColorSpace << /CS1 5 0 R >> >> >>\nendobj\n",
        );

        offsets.push(buf.len());
        let stream_header = format!("4 0 obj\n<< /Length {} >>\nstream\n", content_bytes.len());
        buf.extend_from_slice(stream_header.as_bytes());
        buf.extend_from_slice(content_bytes);
        buf.extend_from_slice(b"\nendstream\nendobj\n");

        offsets.push(buf.len());
        let cs = format!("5 0 obj\n[/DeviceN [{}] /DeviceGray 6 0 R]\nendobj\n", inks_str.trim());
        buf.extend_from_slice(cs.as_bytes());

        offsets.push(buf.len());
        buf.extend_from_slice(b"6 0 obj\n<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>\nendobj\n");

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

    #[test]
    fn separation_ink_appears_in_plate() {
        let pdf_bytes = build_separation_pdf("Dieline", 0.8);
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        let plate = render_separation(&doc, 0, "Dieline", 72).expect("render Dieline plate");

        assert_eq!(plate.ink_name, "Dieline");
        assert_eq!(plate.width, 100);
        assert_eq!(plate.height, 100);
        assert_eq!(plate.data.len(), 100 * 100);

        // The rectangle is from (25,25) to (75,75) in PDF coords.
        // At 72 DPI, 1pt = 1px, so check pixel at center of rectangle.
        // PDF y=50 -> image y = 100 - 50 = 50 (flipped y) ~keep
        let center_x = 50usize;
        let center_y = 50usize;
        let center_val = plate.data[center_y * plate.width as usize + center_x];

        // Tint 0.8 should give ~204 (0.8 * 255) ~keep
        assert!(
            center_val > 180,
            "Expected tint ~204 at rectangle center, got {}",
            center_val
        );

        let outside_val = plate.data[5 * plate.width as usize + 5];
        assert_eq!(
            outside_val, 0,
            "Expected zero tint outside rectangle, got {}",
            outside_val
        );
    }

    #[test]
    fn cmyk_content_appears_in_process_plates() {
        let pdf_bytes = build_cmyk_pdf(0.5, 0.0, 0.0, 0.0);
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        let plates = render_separations(&doc, 0, 72).expect("render separations");

        let ink_names: Vec<&str> = plates.iter().map(|p| p.ink_name.as_str()).collect();
        assert!(ink_names.contains(&"Cyan"), "Expected Cyan plate, got {:?}", ink_names);
        assert!(
            ink_names.contains(&"Magenta"),
            "Expected Magenta plate, got {:?}",
            ink_names
        );
        assert!(
            ink_names.contains(&"Yellow"),
            "Expected Yellow plate, got {:?}",
            ink_names
        );
        assert!(
            ink_names.contains(&"Black"),
            "Expected Black plate, got {:?}",
            ink_names
        );

        let cyan_plate = plates.iter().find(|p| p.ink_name == "Cyan").unwrap();
        let center_val = cyan_plate.data[50 * cyan_plate.width as usize + 50];
        assert!(
            center_val > 100 && center_val < 160,
            "Expected ~128 for 50% cyan, got {}",
            center_val
        );

        let magenta_plate = plates.iter().find(|p| p.ink_name == "Magenta").unwrap();
        let magenta_center = magenta_plate.data[50 * magenta_plate.width as usize + 50];
        assert_eq!(magenta_center, 0, "Expected zero magenta, got {}", magenta_center);
    }

    #[test]
    fn empty_plate_for_missing_ink() {
        let pdf_bytes = build_separation_pdf("Varnish", 1.0);
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        let plate = render_separation(&doc, 0, "Dieline", 72).expect("render Dieline plate");

        assert_eq!(plate.ink_name, "Dieline");
        let non_zero = plate.data.iter().filter(|&&v| v > 0).count();
        assert_eq!(
            non_zero, 0,
            "Expected all-zero plate for missing ink, got {} non-zero pixels",
            non_zero
        );
    }

    #[test]
    fn devicen_ink_routing() {
        let pdf_bytes = build_devicen_pdf(&["SpotRed", "SpotBlue"], &[0.7, 0.3]);
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        let red_plate = render_separation(&doc, 0, "SpotRed", 72).expect("render SpotRed plate");
        let blue_plate = render_separation(&doc, 0, "SpotBlue", 72).expect("render SpotBlue plate");

        let red_center = red_plate.data[50 * red_plate.width as usize + 50];
        let blue_center = blue_plate.data[50 * blue_plate.width as usize + 50];

        assert!(red_center > 150, "Expected SpotRed tint ~179, got {}", red_center);
        assert!(
            blue_center > 50 && blue_center < 110,
            "Expected SpotBlue tint ~77, got {}",
            blue_center
        );
    }

    #[test]
    fn render_separations_returns_all_inks() {
        let pdf_bytes = build_separation_pdf("Dieline", 1.0);
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        let plates = render_separations(&doc, 0, 72).expect("render all separations");

        let ink_names: Vec<&str> = plates.iter().map(|p| p.ink_name.as_str()).collect();
        assert!(
            ink_names.contains(&"Dieline"),
            "Expected Dieline in plates, got {:?}",
            ink_names
        );
    }

    #[test]
    fn full_tint_separation_plate() {
        let pdf_bytes = build_separation_pdf("FullInk", 1.0);
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        let plate = render_separation(&doc, 0, "FullInk", 72).expect("render plate");
        let center_val = plate.data[50 * plate.width as usize + 50];

        assert!(center_val > 240, "Expected ~255 for full tint, got {}", center_val);
    }

    #[test]
    fn zero_tint_separation_plate() {
        let pdf_bytes = build_separation_pdf("ZeroInk", 0.0);
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        let plate = render_separation(&doc, 0, "ZeroInk", 72).expect("render plate");
        let center_val = plate.data[50 * plate.width as usize + 50];

        assert_eq!(center_val, 0, "Expected 0 for zero tint, got {}", center_val);
    }

    /// Hand-rolled PDF builder used by the regression tests below.
    /// `objects[i]` becomes object number `i + 1`.
    fn assemble_pdf(objects: Vec<String>) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        let mut offsets: Vec<usize> = Vec::new();
        buf.extend_from_slice(b"%PDF-1.4\n");
        for (i, body) in objects.iter().enumerate() {
            offsets.push(buf.len());
            let header = format!("{} 0 obj\n", i + 1);
            buf.extend_from_slice(header.as_bytes());
            buf.extend_from_slice(body.as_bytes());
            buf.extend_from_slice(b"\nendobj\n");
        }
        let xref = buf.len();
        buf.extend_from_slice(b"xref\n");
        let header = format!("0 {}\n", offsets.len() + 1);
        buf.extend_from_slice(header.as_bytes());
        buf.extend_from_slice(b"0000000000 65535 f \n");
        for offset in &offsets {
            let entry = format!("{:010} 00000 n \n", offset);
            buf.extend_from_slice(entry.as_bytes());
        }
        let trailer = format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            offsets.len() + 1,
            xref
        );
        buf.extend_from_slice(trailer.as_bytes());
        buf
    }

    /// Build a page whose CMYK content is inside a Form XObject.
    /// Regression for RED #1: the previous shallow scan missed
    /// CMYK content nested inside Form XObjects and returned no
    /// process plates for those pages.
    fn build_cmyk_in_form_xobject_pdf() -> Vec<u8> {
        let form_stream = "0.6 0.0 0.0 0.0 k\n25 25 50 50 re f\n";
        let content_stream = "/F1 Do\n";
        let form_header = format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Length {} >>\nstream\n{}\nendstream",
            form_stream.len(),
            form_stream
        );
        let content_header = format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content_stream.len(),
            content_stream
        );
        assemble_pdf(vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << /XObject << /F1 5 0 R >> >> >>".to_string(),
            content_header,
            form_header,
        ])
    }

    #[test]
    fn cmyk_inside_form_xobject_produces_plates() {
        let pdf_bytes = build_cmyk_in_form_xobject_pdf();
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        let plates = render_separations(&doc, 0, 72).expect("render separations");
        let cyan = plates
            .iter()
            .find(|p| p.ink_name == "Cyan")
            .expect("Cyan plate must be returned even when content is inside a Form XObject");
        let center = cyan.data[50 * cyan.width as usize + 50];
        assert!(
            center > 120 && center < 180,
            "Form XObject CMYK content missing from Cyan plate (got {})",
            center
        );
    }

    /// Build a page that paints with DeviceCMYK (k operator) but overrides
    /// CMYK via /Resources/ColorSpace/DefaultCMYK pointing at a 4-channel
    /// DeviceN with renamed process colorants. Per ISO 32000-1 §8.6.5.6 a
    /// DefaultCMYK remap must preserve arity, so we use 4 named inks.
    fn build_default_cmyk_remap_pdf() -> Vec<u8> {
        let content_stream = "0.7 0.0 0.0 0.0 k\n25 25 50 50 re f\n";
        let content_obj = format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content_stream.len(),
            content_stream
        );
        assemble_pdf(vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << /ColorSpace << /DefaultCMYK 5 0 R >> >> >>".to_string(),
            content_obj,
            "[/DeviceN [/OverrideCyan /OverrideMagenta /OverrideYellow /OverrideBlack] /DeviceCMYK 6 0 R]".to_string(),
            "<< /FunctionType 2 /Domain [0 1] /N 4 /C0 [0 0 0 0] /C1 [1 0 0 0] >>".to_string(),
        ])
    }

    /// §8.6.5.6 DefaultCMYK currently has only partial support: the lookup
    /// resolves but content from `k`/`K` operators is not routed through the
    /// override color space. The override plate stays empty.
    #[test]
    #[ignore = "spec gap: §8.6.5.6 DefaultCMYK lookup works but content routing through override target is incomplete"]
    fn default_cmyk_remap_redirects_to_separation() {
        let pdf_bytes = build_default_cmyk_remap_pdf();
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");
        let plates = render_separations(&doc, 0, 72).expect("render separations");

        let override_cyan = plates
            .iter()
            .find(|p| p.ink_name == "OverrideCyan")
            .expect("OverrideCyan plate should exist when DefaultCMYK remaps to it");
        let override_center = override_cyan.data[50 * override_cyan.width as usize + 50];
        assert!(
            override_center > 150,
            "OverrideCyan plate should receive the remapped cyan content (got {})",
            override_center
        );
    }

    /// Two overlapping rectangles in the same ink. Per PDF opaque
    /// painting model the later fill replaces the earlier one in the
    /// overlapping region.
    fn build_overlapping_separation_pdf() -> Vec<u8> {
        let content = "/CS1 cs\n0.3 scn\n10 10 50 50 re f\n0.9 scn\n40 40 50 50 re f\n";
        let content_obj = format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content);
        assemble_pdf(vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << /ColorSpace << /CS1 5 0 R >> >> >>".to_string(),
            content_obj,
            "[/Separation /OverlapInk /DeviceGray 6 0 R]".to_string(),
            "<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>".to_string(),
        ])
    }

    #[test]
    fn overlapping_same_ink_last_writer_wins() {
        let pdf_bytes = build_overlapping_separation_pdf();
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        let plate = render_separation(&doc, 0, "OverlapInk", 72).expect("render plate");
        let overlap_val = plate.data[50 * plate.width as usize + 50];
        assert!(
            overlap_val > 220,
            "Overlap region should be the later tint (~230), got {}",
            overlap_val
        );

        let only_a = plate.data[80 * plate.width as usize + 20];
        assert!(
            only_a > 60 && only_a < 100,
            "Non-overlap rect A region should be ~77 (0.3 tint), got {}",
            only_a
        );

        let only_b = plate.data[20 * plate.width as usize + 80];
        assert!(
            only_b > 220,
            "Non-overlap rect B region should be ~230 (0.9 tint), got {}",
            only_b
        );
    }

    /// Anti-aliased edge produces intermediate plate values.
    /// Uses a rotated rectangle (via cm) so the edges cannot fall on
    /// integer pixel boundaries; AA must produce intermediate values.
    #[test]
    fn antialiased_edge_produces_intermediate_values() {
        let content = "/CS1 cs\n1.0 scn\nq\n0.978 0.208 -0.208 0.978 30 30 cm\n0 0 40 40 re f\nQ\n";
        let content_obj = format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content);
        let pdf_bytes = assemble_pdf(vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << /ColorSpace << /CS1 5 0 R >> >> >>".to_string(),
            content_obj,
            "[/Separation /AAEdge /DeviceGray 6 0 R]".to_string(),
            "<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>".to_string(),
        ]);
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        let plate = render_separation(&doc, 0, "AAEdge", 144).expect("render plate");
        let mut has_partial = false;
        for &v in plate.data.iter() {
            if v > 10 && v < 245 {
                has_partial = true;
                break;
            }
        }
        assert!(
            has_partial,
            "Expected at least one anti-aliased pixel value between 10 and 245"
        );
    }

    /// DeviceRGB content must NOT be converted into CMYK plates —
    /// the renderer intentionally skips RGB/Gray paths.
    fn build_rgb_only_pdf() -> Vec<u8> {
        let content = "0.5 0.5 0.5 rg\n25 25 50 50 re f\n";
        let content_obj = format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content);
        assemble_pdf(vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << >> >>".to_string(),
            content_obj,
        ])
    }

    #[test]
    fn devicergb_content_does_not_contribute_to_plates() {
        let pdf_bytes = build_rgb_only_pdf();
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");
        let plates = render_separations(&doc, 0, 72).expect("render separations");
        for plate in &plates {
            let nonzero = plate.data.iter().filter(|&&v| v > 0).count();
            assert_eq!(
                nonzero, 0,
                "Plate {} should be empty for RGB-only artwork, got {} non-zero pixels",
                plate.ink_name, nonzero
            );
        }
    }

    /// q/Q must save and restore the colour state including the
    /// chosen colour space and ink components.
    fn build_save_restore_color_pdf() -> Vec<u8> {
        let content = "/CS1 cs\n0.9 scn\n10 10 20 20 re f\nq\n0.1 scn\n40 40 20 20 re f\nQ\n70 70 20 20 re f\n";
        let content_obj = format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content);
        assemble_pdf(vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << /ColorSpace << /CS1 5 0 R >> >> >>".to_string(),
            content_obj,
            "[/Separation /SR /DeviceGray 6 0 R]".to_string(),
            "<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>".to_string(),
        ])
    }

    #[test]
    fn q_restore_preserves_color_state() {
        let pdf_bytes = build_save_restore_color_pdf();
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        let plate = render_separation(&doc, 0, "SR", 72).expect("render plate");
        let outer1 = plate.data[80 * plate.width as usize + 20];
        let outer2 = plate.data[20 * plate.width as usize + 80];
        assert!(outer1 > 220, "First rect tint 0.9 expected, got {}", outer1);
        assert!(
            outer2 > 220,
            "After Q the color must restore to outer 0.9 tint, got {}",
            outer2
        );
    }

    /// Build a Form XObject whose initial colour state inherits from
    /// the caller (PDF §8.10.1). Caller sets ink + tint, form just
    /// fills a rect without re-stating colour — the inherited tint
    /// must end up on the plate.
    fn build_form_inherits_color_pdf() -> Vec<u8> {
        let form_stream = "30 30 40 40 re f\n";
        let content_stream = "/CS1 cs\n0.8 scn\n/F1 Do\n";
        let form_header = format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Length {} >>\nstream\n{}\nendstream",
            form_stream.len(),
            form_stream
        );
        let content_obj = format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content_stream.len(),
            content_stream
        );
        assemble_pdf(vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << /ColorSpace << /CS1 6 0 R >> /XObject << /F1 5 0 R >> >> >>".to_string(),
            content_obj,
            form_header,
            "[/Separation /Inherit /DeviceGray 7 0 R]".to_string(),
            "<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>".to_string(),
        ])
    }

    #[test]
    fn form_xobject_inherits_caller_color_state() {
        let pdf_bytes = build_form_inherits_color_pdf();
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        let plate = render_separation(&doc, 0, "Inherit", 72).expect("render plate");
        let center = plate.data[50 * plate.width as usize + 50];
        assert!(
            center > 170,
            "Form XObject must inherit caller's spot tint 0.8 (~204), got {}",
            center
        );
    }

    /// Render at 90-degree rotation. The fill rect (25,25 50x50) should
    /// land somewhere inside the rotated page.
    fn build_rotated_separation_pdf(rotation: u16) -> Vec<u8> {
        let content = "/CS1 cs\n1.0 scn\n25 25 50 50 re f\n";
        let content_obj = format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content);
        let page_obj = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Rotate {} /Contents 4 0 R /Resources << /ColorSpace << /CS1 5 0 R >> >> >>",
            rotation
        );
        assemble_pdf(vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            page_obj,
            content_obj,
            "[/Separation /RotInk /DeviceGray 6 0 R]".to_string(),
            "<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>".to_string(),
        ])
    }

    #[test]
    fn rotated_pages_render_separations() {
        for rotation in [90u16, 180, 270] {
            let pdf_bytes = build_rotated_separation_pdf(rotation);
            let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");
            let plate = render_separation(&doc, 0, "RotInk", 72).expect("render rotated separation plate");
            let nonzero = plate.data.iter().filter(|&&v| v > 200).count();
            assert!(
                nonzero > 1000,
                "Rotation {} should still paint the rect somewhere on the plate, got {} bright pixels",
                rotation,
                nonzero
            );
        }
    }

    /// cs without setting a colour must reset to the space's initial
    /// value. For Separation that initial is full tint (1.0). Drawing
    /// after `/CS1 cs` with no `scn` should still produce ink at the
    /// max tint.
    fn build_cs_initial_value_pdf() -> Vec<u8> {
        // `/CS1 cs` then directly fill — no scn between them. Per
        // ISO 32000-1 §8.6.4.2 the initial Separation value is 1.0. ~keep
        let content = "/CS1 cs\n25 25 50 50 re f\n";
        let content_obj = format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content);
        assemble_pdf(vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << /ColorSpace << /CS1 5 0 R >> >> >>".to_string(),
            content_obj,
            "[/Separation /CsInit /DeviceGray 6 0 R]".to_string(),
            "<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>".to_string(),
        ])
    }

    #[test]
    fn cs_resets_to_initial_color_value() {
        let pdf_bytes = build_cs_initial_value_pdf();
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");
        let plate = render_separation(&doc, 0, "CsInit", 72).expect("render plate");
        let center = plate.data[50 * plate.width as usize + 50];
        assert!(
            center > 240,
            "cs without scn should leave initial tint 1.0 (~255), got {}",
            center
        );
    }

    /// `K` (uppercase) sets stroke CMYK; without an explicit `cs` the
    /// initial DeviceCMYK colour is [0,0,0,1] — full Black on the K plate.
    /// We verify the cs-resets-to-initial behaviour by switching to
    /// DeviceCMYK and immediately filling.
    #[test]
    fn cmyk_cs_initial_is_full_black() {
        let content = "/DeviceCMYK cs\n25 25 50 50 re f\n";
        let content_obj = format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content);
        let pdf_bytes = assemble_pdf(vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << >> >>".to_string(),
            content_obj,
        ]);
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");
        let plate = render_separation(&doc, 0, "Black", 72).expect("render K plate");
        let center = plate.data[50 * plate.width as usize + 50];
        assert!(
            center > 240,
            "DeviceCMYK cs initial is [0,0,0,1] -> full K plate, got {}",
            center
        );
        let cyan = render_separation(&doc, 0, "Cyan", 72).expect("render C plate");
        let cyan_center = cyan.data[50 * cyan.width as usize + 50];
        assert_eq!(cyan_center, 0, "Cyan must be 0 (initial CMYK is K-only)");
    }

    // Split for file-too-long (#1567): `include!` keeps one module scope, and a subdirectory
    // file is not auto-discovered as its own test binary the way a sibling would be. ~keep
    include!("test_separation_rendering/part2.rs");
}
