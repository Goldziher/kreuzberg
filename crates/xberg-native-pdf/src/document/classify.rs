//! Cheap text-vs-OCR page and document classification (the `classify_page` preflight).
//!
//! Split out of `open.rs`; the `impl` is the same inherent `impl PdfDocument`. ~keep

use super::*;
use crate::content::{Operator, TextElement};
use crate::extractors::auto::{ImageCodecClass, PageSignals, ProducerPrior, ReasonCode};

impl PdfDocument {
    /// Axis-aligned intersection area of a [`Rect`](crate::geometry::Rect)
    /// with the page box `(x0, y0, x1, y1)`.
    fn rect_isect_area(r: &crate::geometry::Rect, x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
        let (rx1, ry1) = (r.x + r.width, r.y + r.height);
        let ix = (rx1.min(x1) - r.x.max(x0)).max(0.0);
        let iy = (ry1.min(y1) - r.y.max(y0)).max(0.0);
        ix * iy
    }

    #[allow(clippy::type_complexity)]
    fn accumulate_text_signals(
        &self,
        page: usize,
        (px0, py0, px1, py1): (f32, f32, f32, f32),
        page_area: f32,
    ) -> (usize, f32, f32, f32, f32, String) {
        let spans = self.extract_spans(page).unwrap_or_default();
        let mut text = String::new();
        let mut glyphs = 0usize;
        let mut text_area = 0.0f32;
        for s in &spans {
            if s.artifact_type.is_some() {
                continue;
            }
            let n = s.text.chars().count();
            if n == 0 {
                continue;
            }
            glyphs += n;
            text.push_str(&s.text);
            text.push(' ');
            text_area += Self::rect_isect_area(&s.bbox, px0, py0, px1, py1);
        }
        let text_area_ratio = (text_area / page_area).clamp(0.0, 1.0);
        let chars: Vec<char> = text.chars().collect();
        let total = chars.len().max(1);
        let bad = chars
            .iter()
            .filter(|&&c| c == '\u{FFFD}' || c.is_control() || ('\u{E000}'..='\u{F8FF}').contains(&c))
            .count();
        let garbled_ratio = bad as f32 / total as f32;
        let word_text: String = self
            .extract_words(page)
            .unwrap_or_default()
            .into_iter()
            .map(|w| w.text)
            .collect::<Vec<_>>()
            .join(" ");
        let words: Vec<&str> = word_text.split_whitespace().collect();
        let (fragmented_word_ratio, consecutive_repeat_ratio) = if words.is_empty() {
            (0.0, 0.0)
        } else {
            let frag = words.iter().filter(|w| w.chars().count() <= 2).count() as f32 / words.len() as f32;
            let rep = words.windows(2).filter(|w| w[0] == w[1]).count() as f32 / words.len() as f32;
            let frag = if crate::extractors::auto::is_cjk_dominant_text(&word_text) {
                0.0
            } else {
                frag
            };
            (frag, rep)
        };
        (
            glyphs,
            text_area_ratio,
            garbled_ratio,
            fragmented_word_ratio,
            consecutive_repeat_ratio,
            word_text,
        )
    }

    fn classify_page_images(
        &self,
        page: usize,
        (px0, py0, px1, py1): (f32, f32, f32, f32),
        page_area: f32,
    ) -> (f32, ImageCodecClass, usize) {
        // Uses the cheap Phase 1 handle enumeration (`page_image_handles`) instead of
        // `extract_images`, which fully decodes pixel data; `handle.bbox`/`filter_chain`
        // give this loop everything it needs without that cost (GH#1732 is the only
        // caller needing decoded pixels). Keeps `extract_images`'s 8x8 px size floor:
        // a 1x1 background-fill image stretched full-bleed would otherwise mis-route a
        // born-digital slide to OCR. ~keep
        let images = self.page_image_handles(page).unwrap_or_default();
        let size_floor = ImageExtractFilter::default();
        let mut img_area = 0.0f32;
        let mut codec = ImageCodecClass::None;
        for im in &images {
            if i64::from(im.width) < size_floor.min_width || i64::from(im.height) < size_floor.min_height {
                continue;
            }
            img_area += Self::rect_isect_area(&im.bbox, px0, py0, px1, py1);
            let has_ccitt = im
                .filter_chain
                .iter()
                .any(|f| matches!(f, crate::extractors::images::PdfFilter::CCITTFaxDecode));
            let has_dct = im
                .filter_chain
                .iter()
                .any(|f| matches!(f, crate::extractors::images::PdfFilter::DCTDecode));
            let c = if has_ccitt {
                ImageCodecClass::Ccitt
            } else if has_dct {
                ImageCodecClass::Dct
            } else {
                ImageCodecClass::Other
            };
            codec = match (codec, c) {
                (ImageCodecClass::None, x) => x,
                (_, ImageCodecClass::Ccitt) => ImageCodecClass::Ccitt,
                (cur, _) => cur,
            };
        }
        let image_area_ratio = (img_area / page_area).clamp(0.0, 1.0);
        (image_area_ratio, codec, images.len())
    }

    fn compute_invisible_text_ratio(&self, page: usize) -> f32 {
        let mut invisible = 0usize;
        let mut glyph_bytes = 0usize;
        if let Ok(data) = self.get_page_content_data(page)
            && let Ok(ops) = crate::content::parse_content_stream(&data)
        {
            let mut rm: u8 = 0;
            let mut stack: Vec<u8> = Vec::new();
            for op in &ops {
                match op {
                    Operator::SaveState => stack.push(rm),
                    Operator::RestoreState => {
                        if let Some(p) = stack.pop() {
                            rm = p;
                        }
                    }
                    Operator::Tr { render } => rm = *render,
                    Operator::Tj { text } => {
                        glyph_bytes += text.len();
                        if rm == 3 {
                            invisible += text.len();
                        }
                    }
                    Operator::TJ { array } => {
                        let g: usize = array
                            .iter()
                            .map(|e| match e {
                                TextElement::String(b) => b.len(),
                                TextElement::Offset(_) => 0,
                            })
                            .sum();
                        glyph_bytes += g;
                        if rm == 3 {
                            invisible += g;
                        }
                    }
                    _ => {}
                }
            }
        }
        if glyph_bytes == 0 {
            0.0
        } else {
            invisible as f32 / glyph_bytes as f32
        }
    }

    fn classify_producer_prior(&self) -> ProducerPrior {
        let p = format!(
            "{} {}",
            self.document_producer().unwrap_or_default(),
            self.document_creator().unwrap_or_default()
        )
        .to_lowercase();
        const SCAN: &[&str] = &[
            "scan",
            "abbyy",
            "tesseract",
            "scansnap",
            "finereader",
            "ocr",
            "lens",
            "camscanner",
            "kofax",
        ];
        const AUTH: &[&str] = &[
            "word",
            "libreoffice",
            "latex",
            "pdftex",
            "chromium",
            "skia",
            "quartz",
            "wkhtmltopdf",
            // All three names are kept: "pdf_oxide" (upstream), "xberg-pdf-oxide" (the
            // fork), "xberg-native-pdf" (since vendoring) all still appear in real
            // files; dropping the older two would misclassify real documents as
            // scanned. ~keep
            "pdf_oxide",
            "xberg-pdf-oxide",
            "xberg-native-pdf",
            "reportlab",
            "prince",
            "weasyprint",
            "powerpoint",
            "excel",
            "indesign",
        ];
        if SCAN.iter().any(|k| p.contains(k)) {
            ProducerPrior::Scanner
        } else if AUTH.iter().any(|k| p.contains(k)) {
            ProducerPrior::Authoring
        } else {
            ProducerPrior::Unknown
        }
    }

    /// Gather per-page classification signals from xberg-native-pdf
    /// **internals** (00-common-foundation §9 — never the flattened
    /// output string). Returns the signals plus the enriched T0.5
    /// quality-gate verdict (research §3a) computed from the *same*
    /// single span extraction (no double work). Pure inspection.
    fn gather_page_signals(&self, page: usize) -> Result<(PageSignals, Option<ReasonCode>)> {
        let (llx, lly, urx, ury) = self.get_page_media_box(page)?;
        let rot = self.get_page_rotation(page).unwrap_or(0);
        let (mut pw, mut ph) = ((urx - llx).abs(), (ury - lly).abs());
        if rot % 180 != 0 {
            std::mem::swap(&mut pw, &mut ph);
        }
        let page_area = (pw * ph).max(1.0);
        let bbox = (llx.min(urx), lly.min(ury), llx.max(urx), lly.max(ury));

        let (glyphs, text_area_ratio, garbled_ratio, fragmented_word_ratio, consecutive_repeat_ratio, word_text) =
            self.accumulate_text_signals(page, bbox, page_area);
        let (image_area_ratio, codec, images_len) = self.classify_page_images(page, bbox, page_area);
        let invisible_text_ratio = self.compute_invisible_text_ratio(page);
        let path_count = self.extract_paths(page).map(|p| p.len()).unwrap_or(0);
        let vector_path_density = {
            let denom = (path_count + glyphs + images_len).max(1) as f32;
            (path_count as f32 / denom).clamp(0.0, 1.0)
        };
        let has_reliable_structure = self.mark_info().map(|m| m.is_structure_reliable()).unwrap_or(false);
        let producer_prior = self.classify_producer_prior();
        let page_is_empty = glyphs == 0 && image_area_ratio < 0.01 && path_count == 0;

        let signals = PageSignals {
            text_glyph_count: glyphs,
            text_area_ratio,
            image_area_ratio,
            codec,
            invisible_text_ratio,
            garbled_ratio,
            fragmented_word_ratio,
            consecutive_repeat_ratio,
            vector_path_density,
            has_reliable_structure,
            producer_prior,
            page_is_empty,
        };
        let gate = crate::extractors::auto::text_quality_gate(&word_text);
        Ok((signals, gate))
    }

    /// Cheap per-page text-vs-OCR classification (the `classify_page`
    /// preflight — no OCR, no rasterisation). Returns kind +
    /// confidence + typed [`ReasonCode`](crate::extractors::auto::ReasonCode)
    /// + the raw signals (explainable).
    ///
    /// Fails closed on an encrypted-unauthenticated document
    /// (`Error::EncryptedPdf`, case L) — consistent with every other
    /// `extract_*`; the graceful warn+fallback applies to *extraction*
    /// (`extract_page_auto`), not this preflight.
    pub fn classify_page(&self, page: usize) -> Result<crate::extractors::auto::PageClassification> {
        use crate::extractors::auto::{AutoExtractOptions, PageClassification, PageKind, classify_from_signals};
        if !self.is_authenticated() {
            return Err(Error::EncryptedPdf);
        }
        let (signals, gate) = self.gather_page_signals(page)?;
        let opts = AutoExtractOptions::balanced();
        let (mut kind, mut confidence, mut reason) = classify_from_signals(&signals, &opts);
        if matches!(kind, PageKind::TextLayer)
            && let Some(r) = gate
        {
            kind = PageKind::Scanned;
            confidence = confidence.min(0.80);
            reason = r;
        }
        Ok(PageClassification {
            page,
            kind,
            confidence,
            reason,
            signals,
        })
    }

    /// Cheap whole-document classification: per-page kinds (the
    /// decision is **per-page**, never one forced doc mode — case Q),
    /// the 0-based `pages_needing_ocr` list, and an aggregate summary.
    ///
    /// Fails closed on an encrypted-unauthenticated document
    /// (`Error::EncryptedPdf`) — a security op must never be silently
    /// degraded to a benign `Empty`. Any *non-security*
    /// per-page failure degrades to `Empty` (graceful — only security
    /// ops fail closed).
    pub fn classify_document(&self) -> Result<crate::extractors::auto::DocumentClassification> {
        use crate::extractors::auto::{DocumentClassification, PageKind, summarise};
        let n = self.page_count()?;
        let mut kinds = Vec::with_capacity(n);
        let mut need = Vec::new();
        for p in 0..n {
            let k = match self.classify_page(p) {
                Ok(c) => c.kind,
                Err(e @ Error::EncryptedPdf) => return Err(e),
                Err(_) => PageKind::Empty,
            };
            if matches!(k, PageKind::Scanned | PageKind::ImageText | PageKind::Mixed) {
                need.push(p);
            }
            kinds.push(k);
        }
        let summary = summarise(&kinds);
        Ok(DocumentClassification {
            pages: kinds,
            pages_needing_ocr: need,
            summary,
        })
    }

    /// One-shot convenience for the 90% case: equivalent to
    /// `AutoExtractor::new().extract_text(self, page)`. **Strictly
    /// additive** — the existing [`extract_text`](Self::extract_text) is
    /// byte-identical/unchanged; this is a *new* opt-in entry point that
    /// auto-routes text-vs-OCR with graceful native fallback.
    pub fn extract_text_auto(&self, page: usize) -> Result<String> {
        crate::extractors::auto::AutoExtractor::new().extract_text(self, page)
    }
}
