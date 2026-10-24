fn gfm_row_cells(line: &str) -> Option<Vec<String>> {
    let inner = line.trim().strip_prefix('|')?.strip_suffix('|')?;
    Some(inner.split('|').map(|cell| cell.trim().to_owned()).collect())
}

fn is_gfm_separator_row(cells: &[String]) -> bool {
    cells
        .iter()
        .all(|cell| !cell.is_empty() && cell.trim_matches(':').chars().all(|c| c == '-'))
}

/// ~keep Remove GFM table blocks from OCR text when `tables` already carries the same cells, so the
/// structured table renders once instead of also surviving as a collapsed pipe paragraph.
/// Blocks that match no returned table stay in the text untouched.
pub(crate) fn strip_embedded_gfm_tables<'a>(
    text: &str,
    tables: impl Iterator<Item = &'a crate::types::Table> + Clone,
) -> String {
    if tables.clone().next().is_none() {
        return text.to_owned();
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut kept: Vec<&str> = Vec::with_capacity(lines.len());
    let mut index = 0;
    while index < lines.len() {
        let mut end = index;
        let mut rows: Vec<Vec<String>> = Vec::new();
        while end < lines.len() {
            let Some(cells) = gfm_row_cells(lines[end]) else { break };
            if !is_gfm_separator_row(&cells) {
                rows.push(cells);
            }
            end += 1;
        }
        if !rows.is_empty() && tables.clone().any(|table| table.cells == rows) {
            index = end;
        } else {
            kept.push(lines[index]);
            index += 1;
        }
    }
    kept.join("\n")
}
