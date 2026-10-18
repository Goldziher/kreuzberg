use super::*;

#[test]
fn should_extract_shared_strings_when_xlsx_dimension_omits_columns() {
    let worksheet = br#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <dimension ref="1:2"/>
  <sheetData>
    <row r="1"><c r="A1" t="s"><v>0</v></c></row>
    <row r="2"><c r="A2" t="s"><v>1</v></c></row>
  </sheetData>
</worksheet>"#;
    let shared_strings = br#"<?xml version="1.0" encoding="UTF-8"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="2" uniqueCount="2">
  <si><t>Budget</t></si><si><t>Ordinary Income/Expense</t></si>
</sst>"#;
    let bytes = make_xlsx(
        r#"<sheets><sheet name="Budget" sheetId="1" r:id="rId1"/></sheets>"#,
        WORKSHEET_REL,
        &[
            ("xl/worksheets/sheet1.xml", worksheet.to_vec()),
            ("xl/sharedStrings.xml", shared_strings.to_vec()),
        ],
    );

    let (workbook, warnings) = read_excel_bytes(&bytes, ".xlsx", &test_limits(10_000))
        .expect("row-only dimensions are valid producer output and must be normalized");

    assert_eq!(workbook.sheets.len(), 1, "warnings: {warnings:?}");
    assert_eq!(
        workbook.sheets[0].table_cells,
        Some(vec![
            vec!["Budget".to_owned()],
            vec!["Ordinary Income/Expense".to_owned()],
        ])
    );

    let directory = tempfile::tempdir().expect("create temporary directory");
    let path = directory.path().join("row-only-dimension.xlsx");
    std::fs::write(&path, &bytes).expect("write XLSX");
    let (file_workbook, file_warnings) = read_excel_file(path.to_str().expect("UTF-8 path"), &test_limits(10_000))
        .expect("the file path must use the same normalization");
    assert_eq!(file_workbook.sheets[0].table_cells, workbook.sheets[0].table_cells);
    assert!(file_warnings.is_empty(), "warnings: {file_warnings:?}");
}

fn make_ods_with_content(content_xml: &[u8]) -> Vec<u8> {
    use std::io::Write as _;

    let mut bytes = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut bytes));
        zip.start_file(
            "mimetype",
            zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
        )
        .expect("start mimetype");
        zip.write_all(b"application/vnd.oasis.opendocument.spreadsheet")
            .expect("write mimetype");
        zip.start_file("META-INF/manifest.xml", zip::write::SimpleFileOptions::default())
            .expect("start manifest");
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0">
 <manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.spreadsheet"/>
 <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
</manifest:manifest>"#,
        )
        .expect("write manifest");
        zip.start_file("content.xml", zip::write::SimpleFileOptions::default())
            .expect("start content.xml");
        zip.write_all(content_xml).expect("write content.xml");
        zip.finish().expect("finish ODS");
    }
    bytes
}

#[test]
fn should_ignore_legal_whitespace_between_ods_cells() {
    let bytes = make_ods_with_content(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
 xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 office:version="1.2">
 <office:body><office:spreadsheet><table:table table:name="Sheet1">
  <table:table-row>
   <table:table-cell office:value-type="string"><text:p>Header</text:p></table:table-cell>
   <table:table-cell office:value-type="string"><text:p>Value</text:p></table:table-cell>
  </table:table-row>
 </table:table></office:spreadsheet></office:body>
</office:document-content>"#,
    );

    let (workbook, warnings) = read_excel_bytes(&bytes, ".ods", &test_limits(10_000))
        .expect("ODF permits indentation whitespace between table cells");

    assert_eq!(workbook.sheets.len(), 1, "warnings: {warnings:?}");
    assert_eq!(
        workbook.sheets[0].table_cells,
        Some(vec![vec!["Header".to_owned(), "Value".to_owned()]])
    );

    let directory = tempfile::tempdir().expect("create temporary directory");
    let path = directory.path().join("indented.ods");
    std::fs::write(&path, &bytes).expect("write ODS");
    let (file_workbook, file_warnings) = read_excel_file(path.to_str().expect("UTF-8 path"), &test_limits(10_000))
        .expect("the file path must use the same normalization");
    assert_eq!(file_workbook.sheets[0].table_cells, workbook.sheets[0].table_cells);
    assert!(file_warnings.is_empty(), "warnings: {file_warnings:?}");
}

fn xls_with_oversized_declared_workbook_stream(stream_len: u64) -> Vec<u8> {
    use std::io::Write as _;

    let mut compound = cfb::CompoundFile::create(Cursor::new(Vec::new())).expect("create compound file");
    compound
        .create_stream("/Workbook")
        .expect("create workbook stream")
        .write_all(b"BIFF")
        .expect("write workbook stream");
    let mut bytes = compound.into_inner().into_inner();

    let workbook_name: Vec<u8> = "Workbook\0".encode_utf16().flat_map(u16::to_le_bytes).collect();
    let name_offset = bytes
        .windows(workbook_name.len())
        .position(|window| window == workbook_name)
        .expect("find Workbook directory entry");
    let entry_offset = name_offset - (name_offset % 128);
    bytes[entry_offset + 120..entry_offset + 128].copy_from_slice(&stream_len.to_le_bytes());
    bytes
}

#[test]
fn should_reject_xls_stream_larger_than_security_limit_before_parsing() {
    let bytes = xls_with_oversized_declared_workbook_stream(64 * 1024);
    let limits = SecurityLimits {
        max_archive_size: 1024,
        ..Default::default()
    };

    let error = read_excel_bytes(&bytes, ".xls", &limits)
        .expect_err("a declared stream larger than the configured limit must be rejected");
    assert!(
        error.to_string().contains("65536") && error.to_string().contains("1024"),
        "the error must report the declared stream size and configured limit: {error}"
    );

    let directory = tempfile::tempdir().expect("create temporary directory");
    let path = directory.path().join("oversized-stream.xls");
    std::fs::write(&path, &bytes).expect("write XLS");
    let file_error = read_excel_file(path.to_str().expect("UTF-8 path"), &limits)
        .expect_err("the file path must enforce the same stream limit");
    assert!(
        file_error.to_string().contains("65536") && file_error.to_string().contains("1024"),
        "the error must report the declared stream size and configured limit: {file_error}"
    );
}

#[test]
fn should_reject_xls_stream_larger_than_physical_container_before_parsing() {
    let bytes = xls_with_oversized_declared_workbook_stream(64 * 1024);

    let error = read_excel_bytes(&bytes, ".xls", &SecurityLimits::default())
        .expect_err("a declared stream larger than its physical container must be rejected");
    assert_eq!(
        error.to_string(),
        format!(
            "Validation error: XLS stream declares 65536 bytes, which exceeds the {}-byte container",
            bytes.len()
        )
    );
}

fn forge_zip_uncompressed_sizes(mut bytes: Vec<u8>, declared_size: u32) -> Vec<u8> {
    let mut offset = 0;
    let mut patched_entries = 0;
    while offset + 46 <= bytes.len() {
        if bytes[offset..].starts_with(b"PK\x01\x02") {
            bytes[offset + 24..offset + 28].copy_from_slice(&declared_size.to_le_bytes());
            let name_length = u16::from_le_bytes([bytes[offset + 28], bytes[offset + 29]]) as usize;
            let extra_length = u16::from_le_bytes([bytes[offset + 30], bytes[offset + 31]]) as usize;
            let comment_length = u16::from_le_bytes([bytes[offset + 32], bytes[offset + 33]]) as usize;
            offset += 46 + name_length + extra_length + comment_length;
            patched_entries += 1;
        } else {
            offset += 1;
        }
    }
    assert!(patched_entries > 1, "fixture must contain multiple ZIP entries");
    bytes
}

#[test]
fn should_reject_forged_zip_sizes_before_rewriting_contents() {
    let entry = vec![b'x'; 700];
    let worksheet = br#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <dimension ref="1:1"/>
  <sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Value</t></is></c></row></sheetData>
</worksheet>"#;
    let bytes = make_xlsx(
        r#"<sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>"#,
        WORKSHEET_REL,
        &[
            ("xl/worksheets/sheet1.xml", worksheet.to_vec()),
            ("xl/media/filler-1.bin", entry.clone()),
            ("xl/media/filler-2.bin", entry),
        ],
    );
    let bytes = forge_zip_uncompressed_sizes(bytes, 1);
    let limits = SecurityLimits {
        max_archive_size: 1024,
        ..Default::default()
    };

    let error = read_excel_bytes(&bytes, ".xlsx", &limits)
        .expect_err("forged central-directory sizes must fail strict header validation");
    let XbergError::Security {
        message,
        source: Some(source),
    } = error
    else {
        panic!("expected a sourced ZIP security error, received {error}");
    };
    assert_eq!(
        message,
        "Invalid spreadsheet ZIP header consistency: invalid Zip archive: header inconsistency detected: \
         uncompressed size mismatch (does not match central directory)"
    );
    assert!(matches!(
        source.downcast_ref::<zip::result::ZipError>(),
        Some(zip::result::ZipError::InvalidArchive(_))
    ));
}
