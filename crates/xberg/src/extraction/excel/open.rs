//! Format dispatch and read entry points (file/bytes -> parsed workbook) for the
//! Excel/ODS extraction module, split out of `excel.rs` to keep that file under the
//! line-count limit.

use super::*;

#[cfg(any(feature = "excel", feature = "excel-wasm"))]
fn validate_legacy_xls_container<R: Read + Seek>(mut reader: R, limits: &SecurityLimits) -> Result<()> {
    let container_size = reader.seek(std::io::SeekFrom::End(0))?;
    reader.seek(std::io::SeekFrom::Start(0))?;
    let compound = cfb::OpenOptions::new()
        .open_with(reader)
        .map_err(|error| XbergError::validation(format!("Invalid XLS compound container: {error}")))?;
    let configured_limit = limits.max_archive_size as u64;

    for entry in compound.walk().filter(|entry| entry.is_stream()) {
        let declared_size = entry.len();
        if declared_size > configured_limit {
            return Err(XbergError::validation(format!(
                "XLS stream declares {declared_size} bytes, which exceeds the configured limit of \
                 {configured_limit} bytes (SecurityLimits::max_archive_size)"
            )));
        }
        if declared_size > container_size {
            return Err(XbergError::validation(format!(
                "XLS stream declares {declared_size} bytes, which exceeds the {container_size}-byte container"
            )));
        }
    }

    Ok(())
}

#[cfg(any(feature = "excel", feature = "excel-wasm"))]
fn is_row_only_dimension_ref(value: &[u8]) -> bool {
    let mut parts = value.split(|byte| *byte == b':');
    let Some(start) = parts.next() else { return false };
    let Some(end) = parts.next() else { return false };
    parts.next().is_none()
        && [start, end]
            .iter()
            .all(|part| !part.is_empty() && part.iter().all(|byte| byte.is_ascii_digit() || *byte == b'$'))
}

#[cfg(any(feature = "excel", feature = "excel-wasm"))]
fn row_only_dimension(event: &quick_xml::events::BytesStart<'_>) -> Result<bool> {
    if event.local_name().as_ref() != "dimension" {
        return Ok(false);
    }
    for attribute in event.attributes().with_checks(false) {
        let attribute = attribute.map_err(|error| XbergError::parsing(format!("Invalid worksheet XML: {error}")))?;
        if attribute.key.as_ref() == "ref" {
            return Ok(is_row_only_dimension_ref(attribute.value.as_ref().as_bytes()));
        }
    }
    Ok(false)
}

#[cfg(any(feature = "excel", feature = "excel-wasm"))]
fn remove_row_only_xlsx_dimensions(xml: &[u8]) -> Result<(Vec<u8>, bool)> {
    use quick_xml::events::Event;

    let mut reader = quick_xml::Reader::from_reader(xml);
    let mut writer = quick_xml::Writer::new(Vec::with_capacity(xml.len()));
    let mut buffer = Vec::new();
    let mut changed = false;

    loop {
        buffer.clear();
        let event = reader
            .read_event_into(&mut buffer)
            .map_err(|error| XbergError::parsing(format!("Invalid worksheet XML: {error}")))?;
        match event {
            Event::Empty(ref start) if row_only_dimension(start)? => changed = true,
            Event::Start(ref start) if row_only_dimension(start)? => {
                let name = start.name().into_inner().to_owned();
                reader
                    .read_to_end(quick_xml::name::QName(&name))
                    .map_err(|error| XbergError::parsing(format!("Invalid worksheet XML: {error}")))?;
                changed = true;
            }
            Event::Eof => break,
            event => writer
                .write_event(event.into_owned())
                .map_err(|error| XbergError::parsing(format!("Failed to normalize worksheet XML: {error}")))?,
        }
    }

    Ok((writer.into_inner(), changed))
}

#[cfg(any(feature = "excel", feature = "excel-wasm"))]
fn remove_ods_row_whitespace(xml: &[u8]) -> Result<(Vec<u8>, bool)> {
    use quick_xml::events::Event;

    let mut reader = quick_xml::Reader::from_reader(xml);
    let mut writer = quick_xml::Writer::new(Vec::with_capacity(xml.len()));
    let mut buffer = Vec::new();
    let mut depth = 0usize;
    let mut row_depth = None;
    let mut changed = false;

    loop {
        buffer.clear();
        let event = reader
            .read_event_into(&mut buffer)
            .map_err(|error| XbergError::parsing(format!("Invalid ODS content XML: {error}")))?;
        match &event {
            Event::Start(start) => {
                depth += 1;
                if start.local_name().as_ref() == "table-row" {
                    row_depth = Some(depth);
                }
            }
            Event::End(end) => {
                if end.local_name().as_ref() == "table-row" && row_depth == Some(depth) {
                    row_depth = None;
                }
                depth = depth.saturating_sub(1);
            }
            Event::Text(text)
                if row_depth == Some(depth) && text.as_ref().bytes().all(|byte| byte.is_ascii_whitespace()) =>
            {
                changed = true;
                continue;
            }
            Event::Eof => break,
            _ => {}
        }
        writer
            .write_event(event.into_owned())
            .map_err(|error| XbergError::parsing(format!("Failed to normalize ODS content XML: {error}")))?;
    }

    Ok((writer.into_inner(), changed))
}

#[cfg(any(feature = "excel", feature = "excel-wasm"))]
fn read_zip_member_bounded<R: Read + Seek>(
    reader: &mut R,
    header_start: u64,
    compressed_size: u64,
    crc32: u32,
    limit: usize,
    member_name: &str,
) -> Result<Vec<u8>> {
    let read_limit = u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1);
    reader.seek(std::io::SeekFrom::Start(header_start))?;
    let options = zip::read::ZipReadOptions::new()
        .override_compressed_size(compressed_size)
        .override_uncompressed_size(read_limit)
        .override_crc(crc32);
    let mut member = zip::read::read_zipfile_from_stream_with_options(reader, options)
        .map_err(|error| XbergError::parsing(format!("Invalid spreadsheet ZIP entry '{member_name}': {error}")))?
        .ok_or_else(|| XbergError::parsing(format!("Missing spreadsheet ZIP entry '{member_name}'")))?;
    let mut bytes = Vec::new();
    member.by_ref().take(read_limit).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(XbergError::validation(format!(
            "Spreadsheet ZIP member '{member_name}' exceeds the configured {limit}-byte archive limit"
        )));
    }
    Ok(bytes)
}

#[cfg(any(feature = "excel", feature = "excel-wasm"))]
fn rewrite_spreadsheet_zip(
    data: &[u8],
    limits: &SecurityLimits,
    mut transform: impl FnMut(&str, &[u8]) -> Result<(Vec<u8>, bool)>,
) -> Result<Option<Vec<u8>>> {
    let archive = zip::ZipArchive::new(Cursor::new(data))
        .map_err(|error| XbergError::parsing(format!("Invalid spreadsheet ZIP: {error}")))?;
    let mut entry_metadata = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let entry = archive
            .by_index_data(index)
            .map_err(|error| XbergError::parsing(format!("Invalid spreadsheet ZIP entry: {error}")))?;
        let name = entry
            .name()
            .map_err(|error| XbergError::parsing(format!("Invalid spreadsheet ZIP entry name: {error}")))?
            .into_owned();
        entry_metadata.push((
            name,
            entry.is_dir(),
            entry.header_start(),
            entry.compressed_size(),
            entry.crc32(),
        ));
    }
    let mut reader = archive.into_inner();
    let mut entries = Vec::with_capacity(entry_metadata.len());
    let mut actual_contents_size = 0usize;
    let mut changed = false;
    for (name, is_dir, header_start, compressed_size, crc32) in entry_metadata {
        if is_dir {
            entries.push((name, None));
            continue;
        }
        let contents = read_zip_member_bounded(
            &mut reader,
            header_start,
            compressed_size,
            crc32,
            limits.max_archive_size,
            &name,
        )?;
        actual_contents_size = actual_contents_size
            .checked_add(contents.len())
            .ok_or_else(|| XbergError::validation("Spreadsheet ZIP aggregate size overflow".to_owned()))?;
        if actual_contents_size > limits.max_archive_size {
            return Err(XbergError::validation(format!(
                "Spreadsheet ZIP actual contents total {actual_contents_size} bytes, which exceeds the configured \
                 aggregate limit of {} bytes (SecurityLimits::max_archive_size)",
                limits.max_archive_size
            )));
        }
        let (contents, entry_changed) = transform(&name, &contents)?;
        changed |= entry_changed;
        entries.push((name, Some(contents)));
    }
    if !changed {
        return Ok(None);
    }

    write_spreadsheet_zip(entries, data.len()).map(Some)
}

#[cfg(any(feature = "excel", feature = "excel-wasm"))]
fn write_spreadsheet_zip(entries: Vec<(String, Option<Vec<u8>>)>, capacity: usize) -> Result<Vec<u8>> {
    use std::io::Write as _;

    let mut rewritten = Vec::with_capacity(capacity);
    {
        let mut writer = zip::ZipWriter::new(Cursor::new(&mut rewritten));
        for (name, contents) in entries {
            let options = zip::write::SimpleFileOptions::default().compression_method(if name == "mimetype" {
                zip::CompressionMethod::Stored
            } else {
                zip::CompressionMethod::Deflated
            });
            match contents {
                Some(contents) => {
                    writer.start_file(name, options).map_err(|error| {
                        XbergError::parsing(format!("Failed to rewrite spreadsheet ZIP entry: {error}"))
                    })?;
                    writer.write_all(&contents)?;
                }
                None => writer.add_directory(name, options).map_err(|error| {
                    XbergError::parsing(format!("Failed to rewrite spreadsheet ZIP directory: {error}"))
                })?,
            }
        }
        writer
            .finish()
            .map_err(|error| XbergError::parsing(format!("Failed to finish spreadsheet ZIP: {error}")))?;
    }
    Ok(rewritten)
}

#[cfg(any(feature = "excel", feature = "excel-wasm"))]
fn normalize_xlsx_dimensions(data: &[u8], limits: &SecurityLimits) -> Result<Option<Vec<u8>>> {
    rewrite_spreadsheet_zip(data, limits, |name, contents| {
        if name.starts_with("xl/worksheets/") && name.ends_with(".xml") {
            remove_row_only_xlsx_dimensions(contents)
        } else {
            Ok((contents.to_vec(), false))
        }
    })
}

#[cfg(any(feature = "excel", feature = "excel-wasm"))]
fn has_row_only_xlsx_dimensions<R: Read + Seek>(reader: R, limits: &SecurityLimits) -> Result<bool> {
    let archive =
        zip::ZipArchive::new(reader).map_err(|error| XbergError::parsing(format!("Invalid XLSX ZIP: {error}")))?;
    let mut worksheets = Vec::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index_data(index)
            .map_err(|error| XbergError::parsing(format!("Invalid XLSX ZIP entry: {error}")))?;
        let name = entry
            .name()
            .map_err(|error| XbergError::parsing(format!("Invalid XLSX ZIP entry name: {error}")))?
            .into_owned();
        if !name.starts_with("xl/worksheets/") || !name.ends_with(".xml") {
            continue;
        }
        worksheets.push((name, entry.header_start(), entry.compressed_size(), entry.crc32()));
    }
    let mut reader = archive.into_inner();
    for (name, header_start, compressed_size, crc32) in worksheets {
        let contents = read_zip_member_bounded(
            &mut reader,
            header_start,
            compressed_size,
            crc32,
            limits.max_archive_size,
            &name,
        )?;
        if remove_row_only_xlsx_dimensions(&contents)?.1 {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(any(feature = "excel", feature = "excel-wasm"))]
fn normalize_ods_row_whitespace(data: &[u8], limits: &SecurityLimits) -> Result<Option<Vec<u8>>> {
    rewrite_spreadsheet_zip(data, limits, |name, contents| {
        if name == "content.xml" {
            remove_ods_row_whitespace(contents)
        } else {
            Ok((contents.to_vec(), false))
        }
    })
}

/// Reject a ZIP-backed spreadsheet (XLSX/XLSM/XLTM/XLAM/XLSB/ODS) whose ZIP central
/// directory declares more entries than the caller's configured
/// `SecurityLimits::max_files_in_archive`, or whose declared aggregate uncompressed
/// size or compression ratio exceeds `SecurityLimits::max_archive_size` /
/// `max_compression_ratio` (`ZipBombValidator`).
///
/// `calamine::Xlsx`/`Xlsb`/`Ods` own the `zip::ZipArchive` internally end-to-end
/// (`Reader::new` opens it and immediately reads workbook/style/shared-string parts
/// before returning) and never expose the archive, an entry count, or per-entry
/// sizes, so none of these limits can be enforced from inside calamine's read path.
/// This reads the archive's central directory once, ahead of the calamine open,
/// purely to enforce them — parsing the central directory does not decompress any
/// entry, so this is not "after the bomb has already been expanded." Errors out
/// rather than truncating, matching the DOCX/PPTX top-level containers
/// (`extraction::docx::parser::validate_archive_security`,
/// `extraction::pptx::container::check_entry_count` +
/// `extractors::security::ZipBombValidator`): a workbook depends on specific named
/// parts (`xl/workbook.xml`, `xl/_rels/workbook.xml.rels`, per-sheet XML) that cannot
/// survive an arbitrary truncation of the ZIP's central directory.
///
/// The entry-count check runs first and keeps its own specific error message
/// (existing tests assert on it); `ZipBombValidator::validate` also re-checks entry
/// count using the same `limits.max_files_in_archive`, which is a no-op here since
/// the explicit check above already returned on that condition, but keeps this
/// function equivalent to calling the validator directly.
///
/// Skips ZIP validation only when a legacy `.xls`/`.xla` input has the OLE compound-file
/// signature. An embedded ZIP can otherwise make `ZipArchive` accept the outer OLE bytes,
/// then fail while resolving a member against the wrong container (#1938). Modern spreadsheet
/// extensions retain ZIP validation even when their bytes begin with an OLE signature.
#[cfg(any(feature = "excel", feature = "excel-wasm"))]
fn validate_zip_container<R: Read + Seek>(mut reader: R, file_extension: &str, limits: &SecurityLimits) -> Result<()> {
    const OLE_SIGNATURE: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

    let initial_position = reader.stream_position()?;
    let mut signature = [0u8; OLE_SIGNATURE.len()];
    let has_ole_signature = reader.read_exact(&mut signature).is_ok() && signature == OLE_SIGNATURE;
    reader.seek(std::io::SeekFrom::Start(initial_position))?;

    let extension = file_extension.to_ascii_lowercase();
    if matches!(extension.as_str(), ".xls" | ".xla") && has_ole_signature {
        return validate_legacy_xls_container(reader, limits);
    }

    let mut archive = match zip::ZipArchive::new(reader) {
        Ok(archive) => archive,
        Err(_) => return Ok(()),
    };
    if archive.len() > limits.max_files_in_archive {
        return Err(XbergError::validation(format!(
            "Spreadsheet ZIP archive declares {} entries, which exceeds the configured limit of {} \
             (SecurityLimits::max_files_in_archive); reduce the archive's entry count or raise the limit",
            archive.len(),
            limits.max_files_in_archive
        )));
    }
    crate::extractors::security::ZipBombValidator::new(limits.clone()).validate(&mut archive)?;
    Ok(())
}

pub(crate) fn read_excel_file(file_path: &str, limits: &SecurityLimits) -> Result<ExcelReadResult> {
    let lower_path = file_path.to_lowercase();
    let mut warnings: Vec<ProcessingWarning> = Vec::new();

    #[cfg(any(feature = "excel", feature = "excel-wasm"))]
    {
        let check_file = std::fs::File::open(file_path)?;
        let file_extension = Path::new(file_path)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| format!(".{extension}"))
            .unwrap_or_default();
        validate_zip_container(std::io::BufReader::new(check_file), &file_extension, limits)?;
    }
    #[cfg(not(any(feature = "excel", feature = "excel-wasm")))]
    let _ = limits;

    #[cfg(feature = "office")]
    let office_metadata = if lower_path.ends_with(".xlsx")
        || lower_path.ends_with(".xlsm")
        || lower_path.ends_with(".xlam")
        || lower_path.ends_with(".xltm")
    {
        extract_xlsx_office_metadata_from_file(file_path).ok()
    } else if lower_path.ends_with(".ods") {
        extract_ods_office_metadata_from_file(file_path).ok()
    } else {
        None
    };

    #[cfg(not(feature = "office"))]
    let office_metadata: Option<HashMap<String, String>> = None;

    if lower_path.ends_with(".xlsx") || lower_path.ends_with(".xlsm") || lower_path.ends_with(".xltm") {
        return read_xlsx_family_file(file_path, office_metadata, warnings, limits);
    }
    if lower_path.ends_with(".xlam") {
        return read_xlam_file(file_path, office_metadata, warnings);
    }
    if lower_path.ends_with(".xla") {
        return read_xla_file(file_path, office_metadata, warnings);
    }
    if lower_path.ends_with(".xlsb") {
        return read_xlsb_file(file_path, office_metadata, warnings);
    }
    if lower_path.ends_with(".ods") {
        return read_ods_file(file_path, office_metadata, warnings, limits);
    }

    let workbook = match open_workbook_auto(Path::new(file_path)) {
        Ok(wb) => wb,
        Err(calamine::Error::Io(io_err)) => {
            if io_err.kind() == std::io::ErrorKind::InvalidData {
                return Err(XbergError::parsing(format!(
                    "Cannot detect Excel file format: {}",
                    io_err
                )));
            }
            // Real IO error - bubble up unchanged ~keep
            return Err(io_err.into());
        }
        Err(e) => return Err(XbergError::parsing(format!("Failed to parse Excel file: {}", e))),
    };

    let result = process_workbook(workbook, office_metadata, &mut warnings)?;
    Ok((result, warnings))
}

/// Read an XLSX/XLSM/XLTM file: parse via calamine's XLSX reader, then merge sheet
/// revisions and comments — the extra sidecar data only this primary XLSX-family format
/// carries (XLAM shares the same reader but not this sidecar data, see [`read_xlam_file`]).
fn read_xlsx_family_file(
    file_path: &str,
    office_metadata: Option<HashMap<String, String>>,
    mut warnings: Vec<ProcessingWarning>,
    limits: &SecurityLimits,
) -> Result<ExcelReadResult> {
    let check_file = std::fs::File::open(file_path)?;
    if has_row_only_xlsx_dimensions(std::io::BufReader::new(check_file), limits)? {
        let bytes = std::fs::read(file_path)?;
        let normalized = normalize_xlsx_dimensions(&bytes, limits)?
            .ok_or_else(|| XbergError::parsing("Failed to normalize row-only XLSX worksheet dimensions".to_owned()))?;
        let workbook = calamine::Xlsx::new(Cursor::new(normalized))
            .map_err(|e| XbergError::parsing(format!("Failed to parse XLSX: {}", e)))?;
        let mut result = process_xlsx_workbook(workbook, office_metadata, &mut warnings)?;
        result.revisions = extract_xlsx_revisions_from_file(file_path);
        if let Some(comments) = extract_xlsx_comments_from_file(file_path) {
            result.metadata.insert("comments".to_owned(), comments);
        }
        return Ok((result, warnings));
    }
    let file = std::fs::File::open(file_path)?;
    let workbook = calamine::Xlsx::new(std::io::BufReader::new(file))
        .map_err(|e| XbergError::parsing(format!("Failed to parse XLSX: {}", e)))?;
    let mut result = process_xlsx_workbook(workbook, office_metadata, &mut warnings)?;
    result.revisions = extract_xlsx_revisions_from_file(file_path);
    if let Some(comments) = extract_xlsx_comments_from_file(file_path) {
        result.metadata.insert("comments".to_owned(), comments);
    }
    Ok((result, warnings))
}

/// Read a `.xlam` add-in file via the XLSX reader; on failure return an empty workbook with
/// a warning instead of failing the whole read (an add-in commonly carries no sheet data
/// worth extracting, so a parse failure here is not fatal).
fn read_xlam_file(
    file_path: &str,
    office_metadata: Option<HashMap<String, String>>,
    mut warnings: Vec<ProcessingWarning>,
) -> Result<ExcelReadResult> {
    let file = std::fs::File::open(file_path)?;
    match calamine::Xlsx::new(std::io::BufReader::new(file)) {
        Ok(workbook) => {
            let result = process_xlsx_workbook(workbook, office_metadata, &mut warnings)?;
            Ok((result, warnings))
        }
        Err(e) => {
            push_warning(
                &mut warnings,
                "excel",
                format!("Workbook could not be parsed as XLSX and no sheets were extracted ({e})"),
            );
            Ok((
                ExcelWorkbook {
                    sheets: vec![],
                    metadata: office_metadata.unwrap_or_default(),
                    revisions: None,
                },
                warnings,
            ))
        }
    }
}

/// Read a legacy `.xla` add-in file via the XLS reader, with the same
/// fall-back-to-empty-workbook-on-parse-failure behavior as [`read_xlam_file`].
fn read_xla_file(
    file_path: &str,
    office_metadata: Option<HashMap<String, String>>,
    mut warnings: Vec<ProcessingWarning>,
) -> Result<ExcelReadResult> {
    let file = std::fs::File::open(file_path)?;
    match calamine::Xls::new(std::io::BufReader::new(file)) {
        Ok(workbook) => {
            let result = process_workbook(workbook, office_metadata, &mut warnings)?;
            Ok((result, warnings))
        }
        Err(e) => {
            push_warning(
                &mut warnings,
                "excel",
                format!("Workbook could not be parsed as XLS and no sheets were extracted ({e})"),
            );
            Ok((
                ExcelWorkbook {
                    sheets: vec![],
                    metadata: office_metadata.unwrap_or_default(),
                    revisions: None,
                },
                warnings,
            ))
        }
    }
}

/// Read a `.xlsb` binary workbook file.
fn read_xlsb_file(
    file_path: &str,
    office_metadata: Option<HashMap<String, String>>,
    mut warnings: Vec<ProcessingWarning>,
) -> Result<ExcelReadResult> {
    let file = std::fs::File::open(file_path)?;
    let workbook = calamine::Xlsb::new(std::io::BufReader::new(file))
        .map_err(|e| XbergError::parsing(format!("Failed to parse XLSB: {}", e)))?;
    let result = process_workbook(workbook, office_metadata, &mut warnings)?;
    Ok((result, warnings))
}

pub(crate) fn read_excel_bytes(data: &[u8], file_extension: &str, limits: &SecurityLimits) -> Result<ExcelReadResult> {
    let warnings: Vec<ProcessingWarning> = Vec::new();

    #[cfg(any(feature = "excel", feature = "excel-wasm"))]
    validate_zip_container(Cursor::new(data), file_extension, limits)?;
    #[cfg(not(any(feature = "excel", feature = "excel-wasm")))]
    let _ = limits;

    #[cfg(feature = "office")]
    let office_metadata = match file_extension.to_lowercase().as_str() {
        ".xlsx" | ".xlsm" | ".xlam" | ".xltm" => extract_xlsx_office_metadata_from_bytes(data).ok(),
        ".ods" => extract_ods_office_metadata_from_bytes(data).ok(),
        _ => None,
    };

    #[cfg(not(feature = "office"))]
    let office_metadata: Option<HashMap<String, String>> = None;

    match file_extension.to_lowercase().as_str() {
        ".xlsx" | ".xlsm" | ".xltm" => read_xlsx_family_bytes(data, office_metadata, warnings, limits),
        ".xlam" => read_xlam_bytes(data, office_metadata, warnings),
        ".xls" => read_xls_bytes(data, office_metadata, warnings),
        ".xla" => read_xla_bytes(data, office_metadata, warnings),
        ".xlsb" => read_xlsb_bytes(data, office_metadata, warnings),
        ".ods" => read_ods_bytes(data, office_metadata, warnings, limits),
        _ => Err(XbergError::parsing(format!(
            "Unsupported file extension: {}",
            file_extension
        ))),
    }
}

/// Read XLSX/XLSM/XLTM bytes: the byte-slice counterpart of [`read_xlsx_family_file`].
fn read_xlsx_family_bytes(
    data: &[u8],
    office_metadata: Option<HashMap<String, String>>,
    mut warnings: Vec<ProcessingWarning>,
    limits: &SecurityLimits,
) -> Result<ExcelReadResult> {
    let normalized = if has_row_only_xlsx_dimensions(Cursor::new(data), limits)? {
        normalize_xlsx_dimensions(data, limits)?
    } else {
        None
    };
    let cursor = Cursor::new(normalized.as_deref().unwrap_or(data));
    let workbook =
        calamine::Xlsx::new(cursor).map_err(|e| XbergError::parsing(format!("Failed to parse XLSX: {}", e)))?;
    let mut result = process_xlsx_workbook(workbook, office_metadata, &mut warnings)?;
    result.revisions = extract_xlsx_revisions_from_bytes(data);
    if let Some(comments) = extract_xlsx_comments_from_bytes(data) {
        result.metadata.insert("comments".to_owned(), comments);
    }
    Ok((result, warnings))
}

/// Read `.xlam` bytes: the byte-slice counterpart of [`read_xlam_file`].
fn read_xlam_bytes(
    data: &[u8],
    office_metadata: Option<HashMap<String, String>>,
    mut warnings: Vec<ProcessingWarning>,
) -> Result<ExcelReadResult> {
    let cursor = Cursor::new(data);
    match calamine::Xlsx::new(cursor) {
        Ok(workbook) => {
            let result = process_xlsx_workbook(workbook, office_metadata, &mut warnings)?;
            Ok((result, warnings))
        }
        Err(e) => {
            push_warning(
                &mut warnings,
                "excel",
                format!("Workbook could not be parsed as XLSX and no sheets were extracted ({e})"),
            );
            Ok((
                ExcelWorkbook {
                    sheets: vec![],
                    metadata: office_metadata.unwrap_or_default(),
                    revisions: None,
                },
                warnings,
            ))
        }
    }
}

/// Read `.xls` bytes (no add-in fallback — see [`read_xla_bytes`] for the add-in variant).
fn read_xls_bytes(
    data: &[u8],
    office_metadata: Option<HashMap<String, String>>,
    mut warnings: Vec<ProcessingWarning>,
) -> Result<ExcelReadResult> {
    let cursor = Cursor::new(data);
    let workbook =
        calamine::Xls::new(cursor).map_err(|e| XbergError::parsing(format!("Failed to parse XLS: {}", e)))?;
    let result = process_workbook(workbook, office_metadata, &mut warnings)?;
    Ok((result, warnings))
}

/// Read `.xla` bytes: the byte-slice counterpart of [`read_xla_file`].
fn read_xla_bytes(
    data: &[u8],
    office_metadata: Option<HashMap<String, String>>,
    mut warnings: Vec<ProcessingWarning>,
) -> Result<ExcelReadResult> {
    let cursor = Cursor::new(data);
    match calamine::Xls::new(cursor) {
        Ok(workbook) => {
            let result = process_workbook(workbook, office_metadata, &mut warnings)?;
            Ok((result, warnings))
        }
        Err(e) => {
            push_warning(
                &mut warnings,
                "excel",
                format!("Workbook could not be parsed as XLS and no sheets were extracted ({e})"),
            );
            Ok((
                ExcelWorkbook {
                    sheets: vec![],
                    metadata: office_metadata.unwrap_or_default(),
                    revisions: None,
                },
                warnings,
            ))
        }
    }
}

/// Read `.xlsb` bytes: the byte-slice counterpart of [`read_xlsb_file`].
fn read_xlsb_bytes(
    data: &[u8],
    office_metadata: Option<HashMap<String, String>>,
    mut warnings: Vec<ProcessingWarning>,
) -> Result<ExcelReadResult> {
    let cursor = Cursor::new(data);
    let workbook =
        calamine::Xlsb::new(cursor).map_err(|e| XbergError::parsing(format!("Failed to parse XLSB: {}", e)))?;
    let result = process_workbook(workbook, office_metadata, &mut warnings)?;
    Ok((result, warnings))
}

fn read_ods_bytes(
    data: &[u8],
    office_metadata: Option<HashMap<String, String>>,
    mut warnings: Vec<ProcessingWarning>,
    limits: &SecurityLimits,
) -> Result<ExcelReadResult> {
    let cursor = Cursor::new(data);
    match calamine::Ods::new(cursor) {
        Ok(workbook) => {
            let result = process_workbook(workbook, office_metadata, &mut warnings)?;
            Ok((result, warnings))
        }
        Err(original_error) => {
            let Some(normalized) = normalize_ods_row_whitespace(data, limits)? else {
                return Err(XbergError::parsing(format!("Failed to parse ODS: {original_error}")));
            };
            let workbook = calamine::Ods::new(Cursor::new(normalized))
                .map_err(|error| XbergError::parsing(format!("Failed to parse ODS: {error}")))?;
            let result = process_workbook(workbook, office_metadata, &mut warnings)?;
            Ok((result, warnings))
        }
    }
}

fn read_ods_file(
    file_path: &str,
    office_metadata: Option<HashMap<String, String>>,
    mut warnings: Vec<ProcessingWarning>,
    limits: &SecurityLimits,
) -> Result<ExcelReadResult> {
    let file = std::fs::File::open(file_path)?;
    match calamine::Ods::new(std::io::BufReader::new(file)) {
        Ok(workbook) => {
            let result = process_workbook(workbook, office_metadata, &mut warnings)?;
            Ok((result, warnings))
        }
        Err(original_error) => {
            let bytes = std::fs::read(file_path)?;
            let Some(normalized) = normalize_ods_row_whitespace(&bytes, limits)? else {
                return Err(XbergError::parsing(format!("Failed to parse ODS: {original_error}")));
            };
            let workbook = calamine::Ods::new(Cursor::new(normalized))
                .map_err(|error| XbergError::parsing(format!("Failed to parse ODS: {error}")))?;
            let result = process_workbook(workbook, office_metadata, &mut warnings)?;
            Ok((result, warnings))
        }
    }
}
