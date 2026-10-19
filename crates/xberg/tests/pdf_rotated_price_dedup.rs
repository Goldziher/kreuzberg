//! Repeated native prices must survive hierarchy redraw removal (#2164).
#![cfg(feature = "pdf")]
mod helpers;
use helpers::extract_bytes_document_blocking;
use xberg::{ExtractionConfig, NodeContent, OutputFormat};

fn config(output_format: OutputFormat) -> ExtractionConfig {
    ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        include_document_structure: true,
        output_format,
        ..Default::default()
    }
}

#[test]
fn wildomar_prices_retain_multiplicity_within_their_item_rows() {
    let bytes = include_bytes!("../../../fixtures/repros/rotated-price-loss/wildomar-bid-tab.pdf");
    let repeated_price = regex::Regex::new(r"(?:^|[^\d,])50,000\.00").unwrap();
    for format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let result = extract_bytes_document_blocking(bytes, "application/pdf", &config(format)).unwrap();
        let row = |start: &str, end: &str| {
            let start = result.content.find(start).expect("item row start");
            let end = result.content[start..].find(end).expect("next item row") + start;
            &result.content[start..end]
        };
        assert_eq!(row("A-4", "A-5").matches("50,000.00").count(), 2);
        let a10 = row("A-10", "A-11");
        for (price, count) in [
            ("125.00", 2),
            ("130.00", 1),
            ("111.00", 1),
            ("115.00", 1),
            ("120.00", 1),
        ] {
            assert_eq!(a10.matches(price).count(), count, "A-10: {price}");
        }
        assert_eq!(repeated_price.find_iter(&result.content).count(), 4);
    }
}

#[test]
fn concord_retains_all_270_prices_in_source_row_order() {
    let bytes = include_bytes!("../../../fixtures/repros/rotated-price-loss/concord-airport-page29.pdf");
    let money = regex::Regex::new(r"\$[\d,]+\.\d{2}").unwrap();
    for format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let result = extract_bytes_document_blocking(bytes, "application/pdf", &config(format)).unwrap();
        let expected = include_str!("../../../fixtures/repros/rotated-price-loss/concord-monetary-rows.tsv");
        let mut count = 0;
        for line in expected.lines() {
            let cells: Vec<_> = line.split('\t').collect();
            let row = result
                .tables
                .iter()
                .flat_map(|table| &table.cells)
                .find(|row| row.first().is_some_and(|id| id == cells[0]))
                .unwrap()
                .join(" ");
            let actual: Vec<_> = money.find_iter(&row).map(|m| m.as_str()).collect();
            assert_eq!(actual, cells[1..], "item {}", cells[0]);
            count += actual.len();
        }
        assert_eq!(count, 270);
    }
}

fn build_pdf_with_content(content: &str, font: &str, page_rotation: i32) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 900 900] \
          /Rotate {page_rotation} /Contents 4 0 R /Resources << /Font << /Helvetica 5 0 R >> >> >>\nendobj\n"
        )
        .as_bytes(),
    );

    offsets.push(pdf.len());
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!(
            "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /{font} \
          /Encoding /WinAnsiEncoding >>\nendobj\n"
        )
        .as_bytes(),
    );

    let xref_pos = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for &off in &offsets[1..] {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            offsets.len(),
            xref_pos
        )
        .as_bytes(),
    );
    pdf
}

fn price_pdf(
    angle: f32,
    font: &str,
    scale: f32,
    page_rotation: i32,
    overlay: Option<(f32, f32)>,
    price: &str,
) -> Vec<u8> {
    let (sin, cos) = angle.to_radians().sin_cos();
    let mut content = format!(
        "BT /Helvetica 10 Tf 1 0 0 1 25 865 Tm (Three distinct rows with nine repeated prices) Tj ET\nq {a:.6} {b:.6} {c:.6} {d:.6} 420 420 cm\nBT /Helvetica 6 Tf 7.2 TL ET\n",
        a = cos * scale,
        b = sin * scale,
        c = -sin * scale,
        d = cos * scale
    );
    for row in 0..3 {
        let y = 25.0 - row as f32 * 12.0;
        content.push_str(&format!("BT 1 0 0 1 -122.5 {y} Tm (R0{row}) Tj T* ET\n"));
        for col in 0..3 {
            let x = -97.5 + col as f32 * 65.0;
            content.push_str(&format!("BT 1 0 0 1 {x} {y} Tm ({price}) Tj T* ET\n"));
            if let Some((dx, dy)) = overlay {
                content.push_str(&format!(
                    "BT 1 0 0 1 {x} {y} Tm ({price}) Tj T* ET\n",
                    x = x + dx,
                    y = y + dy
                ));
            }
        }
    }
    content.push_str("Q\n");
    build_pdf_with_content(&content, font, page_rotation)
}

#[test]
fn rotated_prices_survive_with_fonts_scales_page_rotation_and_real_redraws() {
    let mut cases = Vec::new();
    for angle in [0.0, 30.0, 45.0, 60.0, 80.0, 90.0, 180.0, 270.0, -90.0] {
        cases.push((angle, "Helvetica", 1.0, 0, None, "$  1,234,567.89  ", "1,234,567.89"));
    }
    for font in ["Courier", "Times-Roman"] {
        cases.push((90.0, font, 1.0, 0, None, "$  1,234,567.89  ", "1,234,567.89"));
    }
    for scale in [0.5, 2.0] {
        cases.push((90.0, "Helvetica", scale, 0, None, "$  1,234,567.89  ", "1,234,567.89"));
    }
    for rotation in [90, 180, 270] {
        cases.push((
            90.0,
            "Helvetica",
            1.0,
            rotation,
            None,
            "$  1,234,567.89  ",
            "1,234,567.89",
        ));
    }
    for overlay in [(0.0, 0.0), (0.4, 0.2)] {
        cases.push((
            90.0,
            "Helvetica",
            1.0,
            0,
            Some(overlay),
            "$  1,234,567.89  ",
            "1,234,567.89",
        ));
    }
    for (price, token) in [
        ("0                  ", "0"),
        ("-100.00            ", "-100.00"),
        ("$44.00             ", "$44.00"),
    ] {
        cases.push((90.0, "Helvetica", 1.0, 0, None, price, token));
    }
    for (angle, font, scale, rotation, overlay, price, token) in cases {
        let bytes = price_pdf(angle, font, scale, rotation, overlay, price);
        let mut options = config(OutputFormat::Plain);
        options.pages = Some(xberg::PageConfig {
            extract_pages: true,
            ..Default::default()
        });
        let result = extract_bytes_document_blocking(&bytes, "application/pdf", &options).unwrap();
        // Canonical text leaves exercise hierarchy deduplication. Rendered content
        // may also append a heuristic table containing the same text.
        let content = result
            .document
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .filter_map(|node| match &node.content {
                NodeContent::Paragraph { text }
                | NodeContent::Heading { text, .. }
                | NodeContent::Code { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        let count = content.split_whitespace().filter(|text| *text == token).count();
        assert_eq!(
            count, 9,
            "angle={angle}, font={font}, scale={scale}, rotation={rotation}, overlay={overlay:?}, price={price:?}: {}",
            result.content
        );
    }
}
