//! End-to-end integration test for XLSX metadata extraction

use kreuzberg::extraction::excel::read_excel_file;

#[test]
fn test_xlsx_full_metadata_extraction() {
    // Compute path from workspace root (crates/kreuzberg -> workspace root)
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/office/excel.xlsx");

    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let result = read_excel_file(test_file.to_str().unwrap()).expect("Should extract XLSX successfully");

    // Verify content extraction
    assert!(!result.sheets.is_empty(), "Should have at least one sheet");

    // Verify basic metadata
    assert!(result.metadata.contains_key("sheet_count"), "Should have sheet count");

    // Office metadata should be present (if available in test file)
    // Note: Not all XLSX files have comprehensive metadata
    println!("✅ XLSX metadata extraction test passed!");
    println!("   Found {} metadata fields", result.metadata.len());
    for (key, value) in &result.metadata {
        println!("   {}: {}", key, value);
    }
}

#[test]
fn test_xlsx_multi_sheet_metadata() {
    // Compute path from workspace root
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/spreadsheets/excel_multi_sheet.xlsx");

    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let result = read_excel_file(test_file.to_str().unwrap()).expect("Should extract multi-sheet XLSX successfully");

    // Verify multiple sheets
    assert!(
        result.sheets.len() > 1,
        "Should have multiple sheets, got {}",
        result.sheets.len()
    );

    // Verify sheet names in metadata
    assert!(
        result.metadata.contains_key("sheet_names"),
        "Should have sheet_names metadata"
    );

    println!("✅ XLSX multi-sheet metadata extraction test passed!");
    println!("   Found {} sheets", result.sheets.len());
}

#[test]
fn test_xlsx_minimal_metadata_extraction() {
    // Compute path from workspace root
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/spreadsheets/test_01.xlsx");

    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let result = read_excel_file(test_file.to_str().unwrap()).expect("Should extract XLSX successfully");

    // Verify content extraction works even with minimal metadata
    assert!(!result.sheets.is_empty(), "Content should not be empty");
    assert!(
        result.metadata.contains_key("sheet_count"),
        "Should have basic sheet metadata"
    );

    println!("✅ XLSX minimal metadata extraction test passed!");
}
