//! Native PDF page tables must describe the final selected tables (#2167).
#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::{ExtractedDocument, ExtractionConfig};

const WILDOMAR: &[u8] =
    include_bytes!("../../../fixtures/repros/table-snapshot-consistency/fixture/wildomar-p17-repro.pdf");

fn config(format: &str, structure: bool, pages: bool, tables: bool) -> ExtractionConfig {
    serde_json::from_value(serde_json::json!({
        "use_cache": false, "disable_ocr": true, "enable_quality_processing": false,
        "output_format": format, "include_document_structure": structure,
        "pages": {"extract_pages": pages, "insert_page_markers": false},
        "pdf_options": {"extract_tables": tables, "hierarchy": {"enabled": false}},
        "concurrency": {"max_threads": 1}
    }))
    .unwrap()
}

fn assert_page_tables(result: &ExtractedDocument) {
    for page in result.pages.iter().flatten() {
        let canonical: Vec<_> = result
            .tables
            .iter()
            .filter(|t| t.page_number == page.page_number)
            .collect();
        assert_eq!(
            serde_json::to_value(page.tables.iter().map(|t| t.as_ref()).collect::<Vec<_>>()).unwrap(),
            serde_json::to_value(canonical).unwrap(),
            "page {} must preserve canonical membership, order, cells, IDs and columns",
            page.page_number
        );
    }
}

#[test]
fn wildomar_page_tables_match_final_selection_in_all_output_modes() {
    for format in ["plain", "markdown"] {
        for structure in [false, true] {
            for pages in [true, false] {
                for tables in [false, true] {
                    let result = extract_bytes_document_blocking(
                        WILDOMAR,
                        "application/pdf",
                        &config(format, structure, pages, tables),
                    )
                    .unwrap();
                    let structured = structure || format == "markdown";
                    assert_eq!(
                        result.tables.len(),
                        if !tables {
                            0
                        } else if structured {
                            2
                        } else {
                            3
                        },
                        "format={format}, structure={structure}, pages={pages}, tables={tables}"
                    );
                    if pages || structured {
                        assert_eq!(result.pages.as_ref().unwrap().len(), 1);
                        assert_page_tables(&result);
                    }
                    if tables && structured {
                        assert_eq!(result.tables[0].table_id.as_deref(), Some("table-1"));
                        assert_eq!(result.tables[1].table_id.as_deref(), Some("table-2"));
                        assert_eq!(result.tables[0].cells.len(), 5);
                        assert_eq!(result.tables[1].cells.len(), 10);
                        assert!(result.tables.iter().all(|t| t.columns.is_some()));
                    }
                }
            }
        }
    }
}

#[test]
fn real_extracted_image_indices_remain_associated_with_their_pages() {
    let bytes = std::fs::read(helpers::get_test_file_path("pdf/with_images.pdf"))
        .expect("fetch pdf/with_images.pdf with test_documents/scripts/fetch_corpus.py");
    let mut config = config("markdown", true, true, true);
    config.images = Some(serde_json::from_value(serde_json::json!({"extract_images": true})).unwrap());
    let result = extract_bytes_document_blocking(&bytes, "application/pdf", &config).unwrap();
    let images = result.images.as_ref().expect("real PDF images must be extracted");
    assert!(!images.is_empty());
    let pages = result.pages.as_ref().unwrap();
    assert!(!pages.iter().all(|page| page.image_indices.is_empty()));
    for page in pages {
        for index in &page.image_indices {
            assert_eq!(images[*index as usize].page_number, Some(page.page_number));
        }
    }
    assert_page_tables(&result);
}

fn tables_and_blank_pages_pdf() -> Vec<u8> {
    use lopdf::{Document, Object, Stream, dictionary};
    let mut pdf = Document::with_version("1.5");
    let pages_id = pdf.new_object_id();
    let font = pdf.add_object(dictionary! {"Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica"});
    let mut table_content = String::new();
    for (top, label) in [(700, "Apples"), (400, "Oranges")] {
        for row in 0..=3 {
            table_content.push_str(&format!(
                "72 {} m 582 {} l S\n",
                top + 15 - row * 25,
                top + 15 - row * 25
            ));
        }
        for column in 0..=3 {
            table_content.push_str(&format!(
                "{} {} m {} {} l S\n",
                72 + column * 170,
                top + 15,
                72 + column * 170,
                top - 60
            ));
        }
        for (row, values) in [
            ["Item", "Quantity", "Price"],
            [label, "12", "45"],
            ["Total", "12", "45"],
        ]
        .iter()
        .enumerate()
        {
            for (column, value) in values.iter().enumerate() {
                table_content.push_str(&format!(
                    "BT /F1 12 Tf {} {} Td ({value}) Tj ET\n",
                    72 + column * 170,
                    top - row * 25
                ));
            }
        }
    }
    let mut kids = Vec::new();
    for content in [
        table_content,
        "BT /F1 12 Tf 72 700 Td (A page containing ordinary prose without a table.) Tj ET".to_string(),
        String::new(),
    ] {
        let stream = pdf.add_object(Stream::new(dictionary! {}, content.into_bytes()));
        let page = pdf.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Resources" => dictionary! {"Font" => dictionary! {"F1" => font}}, "Contents" => stream
        });
        kids.push(Object::Reference(page));
    }
    pdf.objects.insert(
        pages_id,
        dictionary! {"Type" => "Pages", "Kids" => kids, "Count" => 3}.into(),
    );
    let catalog = pdf.add_object(dictionary! {"Type" => "Catalog", "Pages" => pages_id});
    pdf.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
fn distinct_tables_and_pages_without_tables_are_retained() {
    let bytes = tables_and_blank_pages_pdf();
    for (format, structure) in [("plain", false), ("markdown", true)] {
        let result =
            extract_bytes_document_blocking(&bytes, "application/pdf", &config(format, structure, true, true)).unwrap();
        assert_eq!(result.tables.len(), 2, "both physically separate tables survive");
        let pages = result.pages.as_ref().unwrap();
        assert_eq!(pages.iter().map(|p| p.page_number).collect::<Vec<_>>(), vec![1, 2, 3]);
        assert!(pages[1].content.contains("ordinary prose"));
        assert!(pages[1].tables.is_empty());
        assert!(pages[2].tables.is_empty());
        assert_eq!(pages[2].is_blank, Some(true));
        assert_page_tables(&result);
    }
}
