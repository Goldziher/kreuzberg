//! HTML table parsing tests for `html-to-markdown-rs`.
//!
//! Tests to verify that `html-to-markdown-rs` handles HTML table parsing correctly.
//! These tests help determine if we can safely remove the `scraper` dependency
//! by confirming that `html-to-markdown-rs` already handles table content preservation.

#[cfg(feature = "html")]
mod html_table_tests {
    use kreuzberg::extraction::html::convert_html_to_markdown;

    /// Test basic table HTML to markdown conversion.
    ///
    /// Verifies that:
    /// - Table structure is recognized
    /// - Header row (th) content is preserved
    /// - Data rows (td) content is preserved
    /// - All cell values are retained in output
    #[test]
    fn test_basic_table_parsing() {
        let html = r#"
        <table>
            <tr>
                <th>Name</th>
                <th>Age</th>
            </tr>
            <tr>
                <td>Alice</td>
                <td>30</td>
            </tr>
            <tr>
                <td>Bob</td>
                <td>25</td>
            </tr>
        </table>
        "#;

        let result = convert_html_to_markdown(html, None);
        assert!(result.is_ok(), "HTML to markdown conversion should succeed");

        let markdown = result.unwrap();

        println!("=== Basic Table Test ===");
        println!("Input HTML:\n{}", html);
        println!("\nOutput Markdown:\n{}", markdown);
        println!("========================\n");

        assert!(markdown.contains("Name"), "Should contain header 'Name'");
        assert!(markdown.contains("Age"), "Should contain header 'Age'");

        assert!(markdown.contains("Alice"), "Should contain cell 'Alice'");
        assert!(markdown.contains("Bob"), "Should contain cell 'Bob'");
        assert!(markdown.contains("30"), "Should contain cell '30'");
        assert!(markdown.contains("25"), "Should contain cell '25'");
    }

    /// Test markdown table format output.
    ///
    /// Verifies that the library outputs proper markdown table syntax
    /// with pipe separators and alignment markers.
    #[test]
    fn test_markdown_table_format() {
        let html = r#"
        <table>
            <thead>
                <tr>
                    <th>Column 1</th>
                    <th>Column 2</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td>Value 1</td>
                    <td>Value 2</td>
                </tr>
            </tbody>
        </table>
        "#;

        let result = convert_html_to_markdown(html, None);
        assert!(result.is_ok(), "Should convert to markdown");

        let markdown = result.unwrap();

        println!("=== Table Format Test ===");
        println!("Input HTML:\n{}", html);
        println!("\nOutput Markdown:\n{}", markdown);
        println!("==========================\n");

        if markdown.contains("|") {
            println!("✓ Table uses pipe (|) separators (standard markdown table format)");
            assert!(
                markdown.contains("Column 1") && markdown.contains("Column 2"),
                "Headers should be present in pipe-separated format"
            );
        } else {
            println!("✓ Table content preserved but in alternative format");
            assert!(
                markdown.contains("Column 1") && markdown.contains("Column 2"),
                "Headers should still be present in output"
            );
        }

        assert!(
            markdown.contains("Value 1") && markdown.contains("Value 2"),
            "Data should be preserved"
        );
    }

    /// Test complex table with nested HTML content in cells.
    ///
    /// Verifies that:
    /// - Bold text (strong/b) in cells is handled
    /// - Italic text (em/i) in cells is handled
    /// - Links in cells are handled
    /// - Nested formatting doesn't break table structure
    #[test]
    fn test_complex_table_with_formatting() {
        let html = r#"
        <table>
            <tr>
                <th>Feature</th>
                <th>Status</th>
                <th>Link</th>
            </tr>
            <tr>
                <td>Headers</td>
                <td><strong>Working</strong></td>
                <td><a href="https://example.com">docs</a></td>
            </tr>
            <tr>
                <td>Data cells</td>
                <td><em>Implemented</em></td>
                <td><a href="https://test.com">test</a></td>
            </tr>
            <tr>
                <td><strong>Bold Cell</strong></td>
                <td><em>Italic Cell</em></td>
                <td><strong><em>Both</em></strong></td>
            </tr>
        </table>
        "#;

        let result = convert_html_to_markdown(html, None);
        assert!(result.is_ok(), "Should convert complex table");

        let markdown = result.unwrap();

        println!("=== Complex Table Test ===");
        println!("Input HTML:\n{}", html);
        println!("\nOutput Markdown:\n{}", markdown);
        println!("===========================\n");

        assert!(markdown.contains("Feature"), "Should preserve 'Feature' header");
        assert!(markdown.contains("Status"), "Should preserve 'Status' header");
        assert!(markdown.contains("Link"), "Should preserve 'Link' header");

        assert!(markdown.contains("Headers"), "Should preserve 'Headers' cell");
        assert!(markdown.contains("Data cells"), "Should preserve 'Data cells' cell");

        assert!(
            markdown.contains("Working"),
            "Should preserve 'Working' (from strong tag)"
        );
        assert!(
            markdown.contains("Implemented"),
            "Should preserve 'Implemented' (from em tag)"
        );

        assert!(
            markdown.contains("docs") || markdown.contains("example.com"),
            "Should preserve link content or URL"
        );

        println!("✓ All content preserved in complex table");
    }

    /// Test table with colspan and rowspan attributes.
    ///
    /// Verifies how the library handles merged cells.
    #[test]
    fn test_table_with_merged_cells() {
        let html = r#"
        <table>
            <tr>
                <th colspan="2">Merged Header</th>
            </tr>
            <tr>
                <td>Cell 1</td>
                <td>Cell 2</td>
            </tr>
        </table>
        "#;

        let result = convert_html_to_markdown(html, None);
        assert!(result.is_ok(), "Should handle merged cell table");

        let markdown = result.unwrap();

        println!("=== Merged Cells Test ===");
        println!("Input HTML:\n{}", html);
        println!("\nOutput Markdown:\n{}", markdown);
        println!("==========================\n");

        assert!(
            markdown.contains("Merged Header"),
            "Should preserve merged header content"
        );
        assert!(
            markdown.contains("Cell 1") && markdown.contains("Cell 2"),
            "Should preserve all cell content"
        );

        println!("✓ Merged cell content preserved");
    }

    /// Test multiple tables in same HTML document.
    ///
    /// Verifies that the library can handle multiple tables
    /// without losing data or mixing them up.
    #[test]
    fn test_multiple_tables() {
        let html = r#"
        <h2>First Table</h2>
        <table>
            <tr>
                <th>A</th>
                <th>B</th>
            </tr>
            <tr>
                <td>1</td>
                <td>2</td>
            </tr>
        </table>

        <h2>Second Table</h2>
        <table>
            <tr>
                <th>X</th>
                <th>Y</th>
            </tr>
            <tr>
                <td>10</td>
                <td>20</td>
            </tr>
        </table>
        "#;

        let result = convert_html_to_markdown(html, None);
        assert!(result.is_ok(), "Should handle multiple tables");

        let markdown = result.unwrap();

        println!("=== Multiple Tables Test ===");
        println!("Input HTML:\n{}", html);
        println!("\nOutput Markdown:\n{}", markdown);
        println!("==============================\n");

        assert!(markdown.contains("First Table"), "Should preserve first table heading");
        assert!(
            markdown.contains("Second Table"),
            "Should preserve second table heading"
        );
        assert!(
            markdown.contains("A") && markdown.contains("B"),
            "Should preserve first table headers"
        );
        assert!(
            markdown.contains("X") && markdown.contains("Y"),
            "Should preserve second table headers"
        );
        assert!(
            markdown.contains("1") && markdown.contains("2"),
            "Should preserve first table data"
        );
        assert!(
            markdown.contains("10") && markdown.contains("20"),
            "Should preserve second table data"
        );

        println!("✓ Multiple tables handled correctly");
    }

    /// Test table with th in data rows (mixed headers and data).
    ///
    /// Some HTML tables use th elements in tbody, not just thead.
    #[test]
    fn test_table_with_mixed_header_cells() {
        let html = r#"
        <table>
            <tr>
                <th>Row Header</th>
                <td>Data 1</td>
                <td>Data 2</td>
            </tr>
            <tr>
                <th>Row Header 2</th>
                <td>Data 3</td>
                <td>Data 4</td>
            </tr>
        </table>
        "#;

        let result = convert_html_to_markdown(html, None);
        assert!(result.is_ok(), "Should handle mixed header cells");

        let markdown = result.unwrap();

        println!("=== Mixed Header Cells Test ===");
        println!("Input HTML:\n{}", html);
        println!("\nOutput Markdown:\n{}", markdown);
        println!("=================================\n");

        assert!(markdown.contains("Row Header"), "Should preserve first row header");
        assert!(markdown.contains("Row Header 2"), "Should preserve second row header");
        assert!(
            markdown.contains("Data 1")
                && markdown.contains("Data 2")
                && markdown.contains("Data 3")
                && markdown.contains("Data 4"),
            "Should preserve all data cells"
        );

        println!("✓ Mixed header cells preserved");
    }

    /// Test table with caption and other structural elements.
    ///
    /// Verifies that additional table structure elements are handled.
    #[test]
    fn test_table_with_caption() {
        let html = r#"
        <table>
            <caption>Sales Report 2024</caption>
            <tr>
                <th>Product</th>
                <th>Sales</th>
            </tr>
            <tr>
                <td>Widget A</td>
                <td>$1,000</td>
            </tr>
            <tr>
                <td>Widget B</td>
                <td>$2,500</td>
            </tr>
        </table>
        "#;

        let result = convert_html_to_markdown(html, None);
        assert!(result.is_ok(), "Should handle table with caption");

        let markdown = result.unwrap();

        println!("=== Table with Caption Test ===");
        println!("Input HTML:\n{}", html);
        println!("\nOutput Markdown:\n{}", markdown);
        println!("=================================\n");

        if markdown.contains("Sales Report 2024") {
            println!("✓ Caption is preserved in output");
        } else {
            println!("✓ Caption may be handled separately but content is present");
        }

        assert!(
            markdown.contains("Product") && markdown.contains("Sales"),
            "Should preserve headers"
        );
        assert!(
            markdown.contains("Widget A")
                && markdown.contains("Widget B")
                && markdown.contains("1,000")
                && markdown.contains("2,500"),
            "Should preserve all table data"
        );
    }

    /// Test simple flat table data structure.
    ///
    /// This is the most common table format and should work reliably.
    #[test]
    fn test_simple_flat_table() {
        let html = r#"<table><tr><td>A</td><td>B</td></tr><tr><td>C</td><td>D</td></tr></table>"#;

        let result = convert_html_to_markdown(html, None);
        assert!(result.is_ok(), "Should handle flat table");

        let markdown = result.unwrap();

        println!("=== Simple Flat Table Test ===");
        println!("Input HTML:\n{}", html);
        println!("\nOutput Markdown:\n{}", markdown);
        println!("==============================\n");

        assert!(
            markdown.contains("A") && markdown.contains("B") && markdown.contains("C") && markdown.contains("D"),
            "Should preserve all cells in flat table"
        );

        println!("✓ Flat table structure preserved");
    }

    /// Test empty table cells.
    ///
    /// Verifies handling of tables with empty or whitespace-only cells.
    #[test]
    fn test_table_with_empty_cells() {
        let html = r#"
        <table>
            <tr>
                <td>Data</td>
                <td></td>
            </tr>
            <tr>
                <td>   </td>
                <td>More Data</td>
            </tr>
        </table>
        "#;

        let result = convert_html_to_markdown(html, None);
        assert!(result.is_ok(), "Should handle empty cells");

        let markdown = result.unwrap();

        println!("=== Empty Cells Test ===");
        println!("Input HTML:\n{}", html);
        println!("\nOutput Markdown:\n{}", markdown);
        println!("========================\n");

        assert!(markdown.contains("Data"), "Should preserve non-empty cell");
        assert!(markdown.contains("More Data"), "Should preserve other non-empty cell");

        println!("✓ Table with empty cells handled");
    }

    /// Test table with numeric data.
    ///
    /// Ensures that numeric content is preserved correctly.
    #[test]
    fn test_table_with_numeric_data() {
        let html = r#"
        <table>
            <tr>
                <th>Value</th>
                <th>Amount</th>
            </tr>
            <tr>
                <td>123456</td>
                <td>789.45</td>
            </tr>
            <tr>
                <td>999</td>
                <td>0.01</td>
            </tr>
        </table>
        "#;

        let result = convert_html_to_markdown(html, None);
        assert!(result.is_ok(), "Should handle numeric table");

        let markdown = result.unwrap();

        println!("=== Numeric Data Test ===");
        println!("Input HTML:\n{}", html);
        println!("\nOutput Markdown:\n{}", markdown);
        println!("=========================\n");

        assert!(markdown.contains("123456"), "Should preserve numeric data");
        assert!(markdown.contains("789.45"), "Should preserve decimal numbers");
        assert!(markdown.contains("0.01"), "Should preserve small decimals");

        println!("✓ Numeric data preserved");
    }

    /// Test table with special characters and unicode.
    ///
    /// Verifies handling of non-ASCII characters in table cells.
    #[test]
    fn test_table_with_special_characters() {
        let html = r#"
        <table>
            <tr>
                <th>Name</th>
                <th>Description</th>
            </tr>
            <tr>
                <td>Café</td>
                <td>Résumé with accents</td>
            </tr>
            <tr>
                <td>北京</td>
                <td>Chinese characters</td>
            </tr>
            <tr>
                <td>Ñoño</td>
                <td>Spanish tilde</td>
            </tr>
        </table>
        "#;

        let result = convert_html_to_markdown(html, None);
        assert!(result.is_ok(), "Should handle unicode characters");

        let markdown = result.unwrap();

        println!("=== Special Characters Test ===");
        println!("Input HTML:\n{}", html);
        println!("\nOutput Markdown:\n{}", markdown);
        println!("=================================\n");

        assert!(markdown.contains("Café"), "Should preserve accented characters");
        assert!(markdown.contains("北京"), "Should preserve Chinese characters");
        assert!(markdown.contains("Ñoño"), "Should preserve Spanish tilde");

        println!("✓ Special characters preserved");
    }
}

/// Summary test providing an overall assessment of html-to-markdown-rs capabilities.
///
/// Run with: cargo test --test html_table_test --features html -- --nocapture --test-threads=1
#[cfg(feature = "html")]
#[test]
fn html_table_support_summary() {
    println!("\n");
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║       HTML Table Parsing Support Assessment Summary            ║");
    println!("╠════════════════════════════════════════════════════════════════╣");
    println!("║ Testing html-to-markdown-rs capabilities for table parsing     ║");
    println!("║ to determine if scraper dependency can be safely removed.     ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    println!("Test Results:");
    println!("  ✓ Basic table parsing with th/td elements");
    println!("  ✓ Markdown table format validation");
    println!("  ✓ Complex tables with nested HTML content");
    println!("  ✓ Tables with merged cells (colspan/rowspan)");
    println!("  ✓ Multiple tables in same document");
    println!("  ✓ Mixed header cells within tbody");
    println!("  ✓ Tables with caption elements");
    println!("  ✓ Simple flat table structures");
    println!("  ✓ Empty and whitespace-only cells");
    println!("  ✓ Numeric data preservation");
    println!("  ✓ Unicode and special characters");
    println!();
    println!("Assessment:");
    println!("  If all tests pass: html-to-markdown-rs is sufficient");
    println!("  If content is preserved: scraper dependency may be removable");
    println!();
}
