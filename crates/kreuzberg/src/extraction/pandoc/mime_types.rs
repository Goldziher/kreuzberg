use crate::error::{KreuzbergError, Result};
use std::collections::HashMap;

/// MIME type to Pandoc format mapping
pub fn get_pandoc_format_from_mime(mime_type: &str) -> Result<String> {
    let mappings = get_mime_to_pandoc_mapping();

    // Direct lookup
    if let Some(format) = mappings.get(mime_type) {
        return Ok(format.to_string());
    }

    // Special case for markdown
    if mime_type == "text/markdown" {
        return Ok("markdown".to_string());
    }

    // Prefix matching fallback
    for (key, value) in &mappings {
        if mime_type.starts_with(key) {
            return Ok(value.to_string());
        }
    }

    Err(KreuzbergError::Validation(format!(
        "Unsupported MIME type: {}",
        mime_type
    )))
}

/// MIME type to file extension mapping
pub fn get_extension_from_mime(mime_type: &str) -> Result<String> {
    let mappings = get_mime_to_extension_mapping();

    mappings
        .get(mime_type)
        .map(|s| s.to_string())
        .ok_or_else(|| KreuzbergError::Validation(format!("No file extension mapping for MIME type: {}", mime_type)))
}

fn get_mime_to_pandoc_mapping() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("application/csl+json", "csljson"),
        ("application/docbook+xml", "docbook"),
        ("application/epub+zip", "epub"),
        ("application/rtf", "rtf"),
        ("application/vnd.oasis.opendocument.text", "odt"),
        (
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "docx",
        ),
        ("application/x-biblatex", "biblatex"),
        ("application/x-bibtex", "bibtex"),
        ("application/x-endnote+xml", "endnotexml"),
        ("application/x-fictionbook+xml", "fb2"),
        ("application/x-ipynb+json", "ipynb"),
        ("application/x-jats+xml", "jats"),
        ("application/x-latex", "latex"),
        ("application/x-opml+xml", "opml"),
        ("application/x-research-info-systems", "ris"),
        ("application/x-typst", "typst"),
        ("text/csv", "csv"),
        ("text/tab-separated-values", "tsv"),
        ("text/troff", "man"),
        ("text/x-commonmark", "commonmark"),
        ("text/x-dokuwiki", "dokuwiki"),
        ("text/x-gfm", "gfm"),
        ("text/x-markdown", "markdown"),
        ("text/x-markdown-extra", "markdown_phpextra"),
        ("text/x-mdoc", "mdoc"),
        ("text/x-multimarkdown", "markdown_mmd"),
        ("text/x-org", "org"),
        ("text/x-pod", "pod"),
        ("text/x-rst", "rst"),
    ])
}

fn get_mime_to_extension_mapping() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("application/csl+json", "json"),
        ("application/docbook+xml", "xml"),
        ("application/epub+zip", "epub"),
        ("application/rtf", "rtf"),
        ("application/vnd.oasis.opendocument.text", "odt"),
        (
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "docx",
        ),
        ("application/x-biblatex", "bib"),
        ("application/x-bibtex", "bib"),
        ("application/x-endnote+xml", "xml"),
        ("application/x-fictionbook+xml", "fb2"),
        ("application/x-ipynb+json", "ipynb"),
        ("application/x-jats+xml", "xml"),
        ("application/x-latex", "tex"),
        ("application/x-opml+xml", "opml"),
        ("application/x-research-info-systems", "ris"),
        ("application/x-typst", "typst"),
        ("text/csv", "csv"),
        ("text/tab-separated-values", "tsv"),
        ("text/troff", "1"),
        ("text/x-commonmark", "md"),
        ("text/x-dokuwiki", "wiki"),
        ("text/x-gfm", "md"),
        ("text/x-markdown", "md"),
        ("text/x-markdown-extra", "md"),
        ("text/x-mdoc", "md"),
        ("text/x-multimarkdown", "md"),
        ("text/x-org", "org"),
        ("text/x-pod", "pod"),
        ("text/x-rst", "rst"),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_pandoc_format_from_mime_docx() {
        let format =
            get_pandoc_format_from_mime("application/vnd.openxmlformats-officedocument.wordprocessingml.document")
                .unwrap();
        assert_eq!(format, "docx");
    }

    #[test]
    fn test_get_pandoc_format_from_mime_markdown() {
        let format = get_pandoc_format_from_mime("text/markdown").unwrap();
        assert_eq!(format, "markdown");
    }

    #[test]
    fn test_get_pandoc_format_from_mime_gfm() {
        let format = get_pandoc_format_from_mime("text/x-gfm").unwrap();
        assert_eq!(format, "gfm");
    }

    #[test]
    fn test_get_pandoc_format_from_mime_rst() {
        let format = get_pandoc_format_from_mime("text/x-rst").unwrap();
        assert_eq!(format, "rst");
    }

    #[test]
    fn test_get_pandoc_format_from_mime_latex() {
        let format = get_pandoc_format_from_mime("application/x-latex").unwrap();
        assert_eq!(format, "latex");
    }

    #[test]
    fn test_get_pandoc_format_from_mime_invalid() {
        let result = get_pandoc_format_from_mime("application/invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_extension_from_mime_docx() {
        let ext =
            get_extension_from_mime("application/vnd.openxmlformats-officedocument.wordprocessingml.document").unwrap();
        assert_eq!(ext, "docx");
    }

    #[test]
    fn test_get_extension_from_mime_md() {
        let ext = get_extension_from_mime("text/x-markdown").unwrap();
        assert_eq!(ext, "md");
    }

    #[test]
    fn test_get_extension_from_mime_tex() {
        let ext = get_extension_from_mime("application/x-latex").unwrap();
        assert_eq!(ext, "tex");
    }

    #[test]
    fn test_get_extension_from_mime_invalid() {
        let result = get_extension_from_mime("application/invalid");
        assert!(result.is_err());
    }
}
