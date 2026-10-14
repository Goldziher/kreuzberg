//! Formula derivation: OCR side-channel / element / recorded formula merging and LaTeX delimiter handling.

use crate::types::internal::InternalDocument;

/// Combine the OCR side channel (`doc.formulas`, with geometry) with element-derived and
/// recorded formulas into the final list.
///
/// The OCR pipeline fills `doc.formulas` directly (with geometry). Markup extractors
/// emit `ElementKind::Formula` elements instead. Element-derived formulas are appended
/// so every source reaches the public list; this must run before document-structure
/// derivation moves formula text out of the elements.
///
/// Some OCR paths represent one formula twice: the layout image path pushes a
/// side-channel `Formula` and a matching geometry-less element for the same region. A
/// geometry-less element whose normalized LaTeX already appears in the side channel is
/// a second representation, not a second formula, so it is skipped. An element that
/// carries its own page or bbox is always its own formula: a mixed native+scanned
/// document can hold the same equation on two different pages. Duplicate formulas
/// WITHIN the element stream are all kept. The FFI round-trip
/// (`InternalDocument::from(ExtractedDocument)`) restores `doc.formulas` with an empty
/// element list, so re-derivation cannot duplicate either.
pub(super) fn derive_formulas(doc: &mut InternalDocument) -> Vec<crate::types::Formula> {
    let mut formulas = std::mem::take(&mut doc.formulas);
    let side_channel_latex: std::collections::HashSet<String> = formulas
        .iter()
        .map(|f| normalized_latex(strip_math_delimiters(&f.latex)))
        .collect();
    let side_channel_paged: std::collections::HashSet<(String, u32)> = formulas
        .iter()
        .filter_map(|f| Some((normalized_latex(strip_math_delimiters(&f.latex)), f.page?)))
        .collect();
    formulas.extend(
        doc.elements
            .iter()
            .filter(|e| matches!(e.kind, crate::types::internal::ElementKind::Formula))
            .filter_map(|e| {
                let latex = strip_math_delimiters(&e.text);
                if latex.is_empty() {
                    return None;
                }
                // ~keep: A geometry-less element that repeats a side-channel formula is
                // the same formula's second representation. A paged element is
                // one only when the side channel holds the same latex on the
                // SAME page: the OCR pipeline emits both an element and a
                // side-channel entry per detected region.
                let norm = normalized_latex(latex);
                let duplicate = match e.page {
                    None if e.bbox.is_none() => side_channel_latex.contains(&norm),
                    Some(page) => side_channel_paged.contains(&(norm, page)),
                    _ => false,
                };
                if duplicate {
                    return None;
                }
                Some(crate::types::Formula {
                    latex: latex.to_string(),
                    bbox: e.bbox,
                    page: e.page,
                })
            }),
    );

    // ~keep: A formula that stays inside its text is its own formula, never a second
    // representation of an element, so it joins the list without the dedup
    // above.
    formulas.extend(std::mem::take(&mut doc.recorded_formulas));
    formulas
}

/// Remove one pair of TeX math delimiters (`$$..$$`, `\[..\]`, or `$..$`)
/// from formula text.
///
/// `Formula.latex` holds bare LaTeX; extractors that store delimited math in
/// the element text stay renderable while the projection stays delimiter-free.
/// Text that holds more than one delimited formula (`$x$ and $y$`) is left
/// untouched: stripping the outer pair would splice unrelated math together.
pub(crate) fn strip_math_delimiters(text: &str) -> &str {
    let t = text.trim();
    for (open, close) in [("$$", "$$"), ("\\[", "\\]"), ("$", "$")] {
        if t.len() > open.len() + close.len()
            && let Some(inner) = t.strip_prefix(open).and_then(|s| s.strip_suffix(close))
            && !contains_unescaped(inner, open)
            && !contains_unescaped(inner, close)
        {
            return inner.trim();
        }
    }
    t
}

/// True when `needle` occurs in `text` outside a backslash escape. `\$` is
/// LaTeX for a literal dollar sign and does not end a math span.
fn contains_unescaped(text: &str, needle: &str) -> bool {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(pos) = text[from..].find(needle) {
        let at = from + pos;
        let escaped = at > 0 && bytes[at - 1] == b'\\';
        if !escaped {
            return true;
        }
        from = at + 1;
    }
    false
}

/// Whitespace-free form of a LaTeX string, for duplicate detection between
/// the OCR side channel and formula elements.
fn normalized_latex(latex: &str) -> String {
    latex.chars().filter(|c| !c.is_whitespace()).collect()
}
