use pyo3::prelude::*;
use std::collections::HashMap;

use kreuzberg::types::{EmailAttachment as CoreEmailAttachment, EmailExtractionResult as CoreEmailResult};

#[pyclass(name = "EmailAttachment", module = "kreuzberg._internal_bindings")]
#[derive(Clone)]
pub struct PyEmailAttachment {
    inner: CoreEmailAttachment,
}

#[pymethods]
impl PyEmailAttachment {
    #[getter]
    fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    #[getter]
    fn filename(&self) -> Option<String> {
        self.inner.filename.clone()
    }

    #[getter]
    fn mime_type(&self) -> Option<String> {
        self.inner.mime_type.clone()
    }

    #[getter]
    fn size(&self) -> Option<usize> {
        self.inner.size
    }

    #[getter]
    fn is_image(&self) -> bool {
        self.inner.is_image
    }

    #[getter]
    fn data(&self) -> Option<Vec<u8>> {
        self.inner.data.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "EmailAttachment(name={:?}, size={:?}, mime_type={:?}, is_image={})",
            self.inner.name, self.inner.size, self.inner.mime_type, self.inner.is_image
        )
    }
}

impl From<CoreEmailAttachment> for PyEmailAttachment {
    fn from(inner: CoreEmailAttachment) -> Self {
        Self { inner }
    }
}

#[pyclass(name = "EmailExtractionResult", module = "kreuzberg._internal_bindings")]
#[derive(Clone)]
pub struct PyEmailExtractionResult {
    pub(crate) inner: CoreEmailResult,
}

#[pymethods]
impl PyEmailExtractionResult {
    #[getter]
    fn subject(&self) -> Option<String> {
        self.inner.subject.clone()
    }

    #[getter]
    fn sender_email(&self) -> Option<String> {
        self.inner.from_email.clone()
    }

    #[getter]
    fn to_emails(&self) -> Vec<String> {
        self.inner.to_emails.clone()
    }

    #[getter]
    fn cc_emails(&self) -> Vec<String> {
        self.inner.cc_emails.clone()
    }

    #[getter]
    fn bcc_emails(&self) -> Vec<String> {
        self.inner.bcc_emails.clone()
    }

    #[getter]
    fn date(&self) -> Option<String> {
        self.inner.date.clone()
    }

    #[getter]
    fn message_id(&self) -> Option<String> {
        self.inner.message_id.clone()
    }

    #[getter]
    fn plain_text(&self) -> Option<String> {
        self.inner.plain_text.clone()
    }

    #[getter]
    fn html_content(&self) -> Option<String> {
        self.inner.html_content.clone()
    }

    #[getter]
    fn cleaned_text(&self) -> String {
        self.inner.cleaned_text.clone()
    }

    #[getter]
    fn attachments(&self) -> Vec<PyEmailAttachment> {
        self.inner
            .attachments
            .iter()
            .map(|a| PyEmailAttachment::from(a.clone()))
            .collect()
    }

    #[getter]
    fn metadata(&self) -> HashMap<String, String> {
        self.inner.metadata.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "EmailExtractionResult(subject={:?}, from={:?}, to_count={}, attachments={})",
            self.inner.subject,
            self.inner.from_email,
            self.inner.to_emails.len(),
            self.inner.attachments.len()
        )
    }
}

impl From<CoreEmailResult> for PyEmailExtractionResult {
    fn from(inner: CoreEmailResult) -> Self {
        Self { inner }
    }
}
