#[cfg(feature = "pdf-surface")]
use super::super::*;
#[cfg(feature = "pdf-surface")]
use super::default_overrides;
#[cfg(feature = "pdf-surface")]
use xberg::ExtractionConfig;

// -- --pdf-backend (#700) ------------------------------------------------------

/// Before this change, `apply_pdf`'s `has_pdf_flag` disjunction never checked
/// `pdf_backend`, so a bare `--pdf-backend native` with no other PDF flag left
/// `config.pdf_options` at `None` -- the flag was applied to nothing. This does not
/// need `xberg::PdfBackend` to compile, so it exercises today's actual bug directly:
/// this assertion fails against unfixed code (`pdf_options` stays `None`).
#[cfg(feature = "pdf-surface")]
#[test]
fn test_pdf_backend_flag_alone_populates_pdf_options() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        pdf_backend: Some("native".to_string()),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    assert!(
        config.pdf_options.is_some(),
        "a bare --pdf-backend flag must populate pdf_options even with no other PDF flag set"
    );
}

/// New surface: `xberg::PdfBackend` does not exist before this change, so this test
/// cannot even compile against unfixed code -- it is new-surface-only, not a
/// fails-today regression test.
#[cfg(feature = "pdf-surface")]
#[test]
fn test_pdf_backend_pdfium_applied() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        pdf_backend: Some("pdfium".to_string()),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let pdf = config.pdf_options.expect("pdf_options must be populated");
    assert_eq!(pdf.backend, xberg::PdfBackend::Pdfium);
}

/// New surface, same reason as above -- `xberg::PdfBackend` does not exist today.
#[cfg(feature = "pdf-surface")]
#[test]
fn test_pdf_backend_default_applied_is_native() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        pdf_backend: Some("native".to_string()),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let pdf = config.pdf_options.expect("pdf_options must be populated");
    assert_eq!(pdf.backend, xberg::PdfBackend::Native);
}

#[cfg(feature = "pdf-surface")]
#[test]
fn test_pdf_backend_rejects_unknown_value() {
    let overrides = ExtractionOverrides {
        pdf_backend: Some("xyz".to_string()),
        ..default_overrides()
    };
    let error = overrides.validate().expect_err("unknown backend must fail");
    assert!(
        error.to_string().contains("native"),
        "error should mention 'native', got: {error}"
    );
}

/// Fails today: the old validator's message is "Invalid PDF backend '<x>'. Only
/// 'native' is currently supported." for *any* value other than "native",
/// including "pdfium" -- it does not name a rebuild feature, so
/// `contains("pdf-pdfium-surface")` is false against unfixed code. It becomes true
/// only once the validator gains the feature-gated actionable-error branch this
/// change adds.
#[cfg(all(feature = "pdf-surface", not(feature = "pdf-pdfium-surface")))]
#[test]
fn test_pdf_backend_pdfium_rejected_without_feature_names_rebuild_hint() {
    let overrides = ExtractionOverrides {
        pdf_backend: Some("pdfium".to_string()),
        ..default_overrides()
    };
    let error = overrides
        .validate()
        .expect_err("pdfium must be rejected without the feature");
    assert!(
        error.to_string().contains("pdf-pdfium-surface"),
        "error should name the feature to rebuild with, got: {error}"
    );
}

#[cfg(feature = "pdf-pdfium-surface")]
#[test]
fn test_pdf_backend_pdfium_accepted_with_feature() {
    let overrides = ExtractionOverrides {
        pdf_backend: Some("pdfium".to_string()),
        ..default_overrides()
    };
    assert!(overrides.validate().is_ok());
}
