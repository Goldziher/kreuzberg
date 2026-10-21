#![cfg(feature = "pdf")]

use xberg::pdf::PdfError;

#[test]
fn should_convert_lopdf_045_io_errors_without_changing_the_message() {
    let source = lopdf_compat::Error::IO(std::io::Error::other("cannot read the PDF"));

    let converted = PdfError::from(source);

    assert_eq!(converted.to_string(), "I/O error: cannot read the PDF");
}

#[test]
fn should_convert_lopdf_045_parse_errors_without_changing_the_message() {
    let converted = PdfError::from(lopdf_compat::Error::NoOutline);

    assert_eq!(
        converted.to_string(),
        "Invalid PDF: PDF document does not have an outline"
    );
}

#[test]
fn should_convert_lopdf_046_io_errors_without_changing_the_message() {
    let source = lopdf::Error::IO(std::io::Error::other("cannot read the PDF"));

    let converted = PdfError::from(source);

    assert_eq!(converted.to_string(), "I/O error: cannot read the PDF");
}

#[test]
fn should_convert_lopdf_046_parse_errors_without_changing_the_message() {
    let converted = PdfError::from(lopdf::Error::NoOutline);

    assert_eq!(
        converted.to_string(),
        "Invalid PDF: PDF document does not have an outline"
    );
}
