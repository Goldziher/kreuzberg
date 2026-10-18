//! `--pdf-*` overrides.

#[cfg(feature = "pdf-surface")]
use anyhow::{Result, bail};
#[cfg(feature = "pdf-surface")]
use xberg::{ExtractionConfig, PdfBackend};

use super::ExtractionOverrides;

impl ExtractionOverrides {
    /// Reject an unparsable or unsupported `--pdf-backend` value.
    #[cfg(feature = "pdf-surface")]
    pub(super) fn validate_pdf_backend(&self) -> Result<()> {
        if let Some(ref backend) = self.pdf_backend {
            match backend.parse::<PdfBackend>() {
                Ok(PdfBackend::Native) => {}
                #[cfg(feature = "pdf-pdfium-surface")]
                Ok(PdfBackend::Pdfium) => {}
                #[cfg(not(feature = "pdf-pdfium-surface"))]
                Ok(PdfBackend::Pdfium) => {
                    bail!(
                        "--pdf-backend pdfium requires the pdf-pdfium-surface feature, which this \
                         binary was not built with. Rebuild with --features pdf-pdfium-surface. \
                         Note that the pdfium engine also loads the pdfium shared library at run \
                         time: install it on the system library search path, or point \
                         PDFIUM_DYNAMIC_LIB_PATH at a directory containing it."
                    );
                }
                Err(_) => {
                    bail!("Invalid PDF backend '{}'. Valid values: native, pdfium.", backend);
                }
            }
        }
        Ok(())
    }

    #[cfg(feature = "pdf-surface")]
    pub(super) fn apply_pdf(&self, config: &mut ExtractionConfig) {
        let has_pdf_flag = self.pdf_extract_images.is_some()
            || self.pdf_extract_tables.is_some()
            || self.pdf_extract_metadata.is_some()
            || !self.pdf_password.is_empty()
            || self.pdf_backend.is_some();
        #[cfg(feature = "ocr-surface")]
        let has_pdf_flag = has_pdf_flag || self.pdf_ocr_inline_images.is_some();
        if has_pdf_flag {
            let pdf_opts = config.pdf_options.get_or_insert_with(Default::default);
            if let Some(extract_img) = self.pdf_extract_images {
                pdf_opts.extract_images = extract_img;
            }
            if let Some(extract_tables) = self.pdf_extract_tables {
                pdf_opts.extract_tables = extract_tables;
            }
            #[cfg(feature = "ocr-surface")]
            if let Some(ocr_img) = self.pdf_ocr_inline_images {
                pdf_opts.ocr_inline_images = ocr_img;
            }
            if let Some(extract_meta) = self.pdf_extract_metadata {
                pdf_opts.extract_metadata = extract_meta;
            }
            if !self.pdf_password.is_empty() {
                pdf_opts.passwords = Some(self.pdf_password.clone());
            }
            // `validate()` runs before `apply()` (see main.rs) and already rejected any
            // value that fails to parse, so `unwrap_or_default()` here mirrors the
            // established --layout-strategy / --layout-table-model pattern: it is
            // unreachable in practice, never a silent behavior change.
            if let Some(ref backend) = self.pdf_backend {
                pdf_opts.backend = backend.parse().unwrap_or_default();
            }
        }
    }
}
