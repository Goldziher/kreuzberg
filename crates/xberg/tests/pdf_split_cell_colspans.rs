//! Split table bands retain their logical columns (#2165).
#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::{ExtractionConfig, NodeContent, OutputFormat};

const PDF: &[u8] = include_bytes!("../../../fixtures/repros/bidder-column-alignment/sfport-p1-repro.pdf");

#[test]
fn split_bidder_names_align_with_ids_and_amounts() {
    for output_format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let result = extract_bytes_document_blocking(
            PDF,
            "application/pdf",
            &ExtractionConfig {
                disable_ocr: true,
                use_cache: false,
                enable_quality_processing: false,
                include_document_structure: true,
                output_format,
                ..Default::default()
            },
        )
        .expect("public bid tabulation must extract");
        let table = result
            .tables
            .iter()
            .find(|table| table.cells.iter().flatten().any(|cell| cell == "BIDDER ID #"))
            .expect("bidder table must be present");
        let cells = &table.cells;
        let ids = cells.iter().find(|row| row.iter().any(|s| s == "BIDDER ID #")).unwrap();
        let item_one = cells.iter().find(|row| row[0] == "1").unwrap();
        for (name, id, amount) in [
            ("YERBA BUENA ENGINEERING &", "01", "(1)$205,000.00"),
            ("WICKMAN DEVELOPMENT AND", "02", "$68,000.00"),
            ("CAZADORO CONSTRUCTION INC", "03", "$150,000.00"),
        ] {
            let column = ids.iter().position(|s| s == id).unwrap();
            let name_row = cells.iter().find(|row| row.iter().any(|s| s == name)).unwrap();
            assert_eq!(name_row[column], name, "bidder {id} must keep its column");
            assert_eq!(item_one[column], amount);
        }

        // All 17 amount rows are unaffected by subdivision of the company-name band.
        let monetary_rows: Vec<_> = cells.iter().filter(|row| row.iter().any(|s| s.contains('$'))).collect();
        let expected: Vec<Vec<&str>> =
            include_str!("../../../fixtures/repros/bidder-column-alignment/monetary-rows.tsv")
                .lines()
                .map(|line| line.split('\t').collect())
                .collect();
        assert_eq!(monetary_rows.len(), 17);
        assert_eq!(monetary_rows, expected.iter().collect::<Vec<_>>());

        let grid = result
            .document
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find_map(|node| {
                if let NodeContent::Table { grid } = &node.content {
                    grid.cells
                        .iter()
                        .any(|cell| cell.content == "BIDDER ID #")
                        .then_some(grid)
                } else {
                    None
                }
            })
            .expect("hierarchical table must be present");
        let mut hierarchical_cells = vec![vec![String::new(); grid.cols as usize]; grid.rows as usize];
        for cell in &grid.cells {
            hierarchical_cells[cell.row as usize][cell.col as usize] = cell.content.clone();
        }
        assert_eq!(&hierarchical_cells, cells);

        let markdown_rows: Vec<Vec<_>> = table
            .markdown
            .lines()
            .enumerate()
            .filter(|(index, _)| *index != 1)
            .map(|(_, line)| line.trim().trim_matches('|').split('|').map(str::trim).collect())
            .collect();
        assert_eq!(markdown_rows, *cells);
    }
}
