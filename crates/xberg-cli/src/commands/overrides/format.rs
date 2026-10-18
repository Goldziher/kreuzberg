//! Flags that only matter for one input or output format, flattened into `ExtractionOverrides`.

/// Per-format CLI flags (PDF backend, MSG codepage, styled HTML, CSV/TSV).
#[derive(Debug, Default, clap::Args)]
pub struct FormatOverrides {
    /// PDF extraction backend to use: "native" (default) or "pdfium".
    ///
    /// "pdfium" requires the CLI to be built with the `pdf-pdfium-surface`
    /// feature, and requires the pdfium shared library to be loadable at run
    /// time (system library search path, or `PDFIUM_DYNAMIC_LIB_PATH`).
    /// Selecting it on a build without that feature is rejected with an error
    /// rather than silently falling back to native.
    ///
    /// The pdfium engine is deliberately narrower than native: page count,
    /// per-page plain text, and Info-dictionary metadata only -- no tables,
    /// layout detection, annotations, form fields, embedded files, or OCR
    /// fallback. It is not a drop-in replacement for the native backend.
    #[cfg(feature = "pdf-surface")]
    #[arg(long, value_name = "BACKEND")]
    pub pdf_backend: Option<String>,

    /// Windows codepage fallback for MSG files without codepage metadata.
    /// Common values: 1250 (Central European), 1251 (Cyrillic), 1252 (Western).
    #[arg(long)]
    pub msg_codepage: Option<u32>,

    /// Built-in colour theme for styled HTML output (default, github, dark, light, unstyled).
    /// Implies --content-format html and enables the styled HTML renderer.
    #[cfg(feature = "html")]
    #[arg(long, value_name = "THEME")]
    pub html_theme: Option<String>,

    /// Inline CSS string appended after the theme stylesheet in styled HTML output.
    #[cfg(feature = "html")]
    #[arg(long, value_name = "CSS")]
    pub html_css: Option<String>,

    /// Path to a CSS file loaded once and appended after the theme stylesheet in styled HTML output.
    #[cfg(feature = "html")]
    #[arg(long, value_name = "PATH")]
    pub html_css_file: Option<std::path::PathBuf>,

    /// CSS class prefix used on every emitted class name (default: "kb-").
    #[cfg(feature = "html")]
    #[arg(long, value_name = "PREFIX")]
    pub html_class_prefix: Option<String>,

    /// Suppress the embedded `<style>` block in styled HTML output.
    #[cfg(feature = "html")]
    #[arg(long)]
    pub html_no_embed_css: bool,

    /// CSV/TSV field delimiter (single ASCII character, e.g. ";", "|", "\t").
    /// When unset, the delimiter is auto-detected from the file.
    #[arg(long, value_name = "CHAR")]
    pub csv_delimiter: Option<String>,

    /// Line prefix marking a CSV/TSV comment line to skip entirely (e.g. "#").
    /// Can be specified multiple times. Default: no comment filtering.
    #[arg(long, value_name = "PREFIX")]
    pub csv_comment_prefix: Vec<String>,
}
