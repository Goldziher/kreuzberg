//! Test-only fixtures: page layouts with gaps and documents from the shared `test_documents/` corpus.

use std::path::PathBuf;

use xberg::types::PageBoundary;

/// Page boundaries with gaps between pages, so some offsets belong to no page.
pub fn gapped_pages(text: &str, page_bytes: usize) -> Vec<PageBoundary> {
    let mut boundaries = Vec::new();
    let mut start = 0;
    let mut page = 1;
    while start < text.len() {
        let mut end = (start + page_bytes).min(text.len());
        while !text.is_char_boundary(end) {
            end += 1;
        }
        boundaries.push(PageBoundary {
            byte_start: start,
            byte_end: end,
            page_number: page,
        });
        let mut next = (end + page_bytes / 7).min(text.len());
        while !text.is_char_boundary(next) {
            next += 1;
        }
        start = next;
        page += 1;
    }
    boundaries
}

fn test_documents_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test_documents")
}

/// Text-like documents from the shared corpus. Empty when the submodule is not checked out.
pub fn real_documents() -> Vec<(String, String)> {
    const DIRS: &[&str] = &["markdown", "text", "csv", "code", "data_formats"];
    const EXTENSIONS: &[&str] = &[
        "md",
        "markdown",
        "mdx",
        "txt",
        "csv",
        "tsv",
        "rs",
        "py",
        "js",
        "sh",
        "toml",
        "commonmark",
    ];
    let root = test_documents_dir();
    let mut documents = Vec::new();
    for dir in DIRS {
        let Ok(entries) = std::fs::read_dir(root.join(dir)) else {
            continue;
        };
        let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        paths.sort();
        for path in paths {
            let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !EXTENSIONS.contains(&extension) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            if text.trim().is_empty() {
                continue;
            }
            documents.push((format!("{dir}/{}", path.file_name().unwrap().to_string_lossy()), text));
        }
    }
    documents
}

/// A JSON object with one object-valued member per line, for section-chunker tests.
pub fn json_object(size: usize, seed: u64) -> String {
    let mut rng = crate::corpus::Rng::new(seed);
    let mut out = String::from("{\n");
    let mut n = 0;
    while out.len() < size {
        n += 1;
        out.push_str(&format!(
            "  \"item_{n}\": {{\"label\": \"{}, {{x}}\", \"count\": {}, \"tags\": [\"{}\"]}},\n",
            crate::corpus::words(&mut rng, 3),
            rng.below(1000),
            rng.pick_str(crate::corpus::WORDS)
        ));
    }
    out.push_str("  \"end\": true\n}\n");
    out
}
