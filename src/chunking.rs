use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use text_splitter::{Characters, ChunkCapacity, ChunkConfig, ChunkConfigError, MarkdownSplitter, TextSplitter};

#[derive(Debug)]
struct PyChunkConfigError(ChunkConfigError);

impl From<ChunkConfigError> for PyChunkConfigError {
    fn from(err: ChunkConfigError) -> Self {
        Self(err)
    }
}

impl From<PyChunkConfigError> for PyErr {
    fn from(err: PyChunkConfigError) -> Self {
        PyValueError::new_err(err.0.to_string())
    }
}

fn build_config(capacity: usize, overlap: usize, trim: bool) -> Result<ChunkConfig<Characters>, PyChunkConfigError> {
    let config = ChunkConfig::new(ChunkCapacity::new(capacity))
        .with_overlap(overlap)?
        .with_trim(trim);
    Ok(config)
}

#[pyclass(module = "kreuzberg._internal_bindings", name = "TextSplitter", frozen)]
pub struct PyTextSplitter {
    splitter: TextSplitter<Characters>,
}

#[pymethods]
impl PyTextSplitter {
    #[new]
    #[pyo3(signature = (max_characters, overlap=0, trim=true))]
    fn new(max_characters: usize, overlap: usize, trim: bool) -> PyResult<Self> {
        let config = build_config(max_characters, overlap, trim)?;
        Ok(Self {
            splitter: TextSplitter::new(config),
        })
    }

    fn chunks(&self, text: &str) -> Vec<String> {
        self.splitter.chunks(text).map(|chunk| chunk.to_string()).collect()
    }
}

#[pyclass(module = "kreuzberg._internal_bindings", name = "MarkdownSplitter", frozen)]
pub struct PyMarkdownSplitter {
    splitter: MarkdownSplitter<Characters>,
}

#[pymethods]
impl PyMarkdownSplitter {
    #[new]
    #[pyo3(signature = (max_characters, overlap=0, trim=true))]
    fn new(max_characters: usize, overlap: usize, trim: bool) -> PyResult<Self> {
        let config = build_config(max_characters, overlap, trim)?;
        Ok(Self {
            splitter: MarkdownSplitter::new(config),
        })
    }

    fn chunks(&self, text: &str) -> Vec<String> {
        self.splitter.chunks(text).map(|chunk| chunk.to_string()).collect()
    }
}

pub fn register_chunking(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTextSplitter>()?;
    m.add_class::<PyMarkdownSplitter>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::Python;

    fn ensure_python() {
        Python::initialize();
    }

    #[test]
    fn text_splitter_produces_overlapping_chunks() {
        ensure_python();
        let splitter = PyTextSplitter::new(12, 2, true).expect("text splitter instantiation failed");
        let chunks = splitter.chunks("0123456789ABCDEFGHIJ");
        assert!(chunks.len() >= 2, "expected multiple chunks, got {:?}", chunks);
        assert!(chunks.iter().all(|chunk| chunk.len() <= 12));
        let overlap = &chunks[0][chunks[0].len().saturating_sub(2)..];
        assert!(chunks[1].starts_with(overlap), "expected overlap between chunks");
    }

    #[test]
    fn markdown_splitter_keeps_markdown_structure() {
        ensure_python();
        let markdown = "# Title\n\nParagraph one with **bold** text.\n\n## Section\nMore text here.";
        let splitter = PyMarkdownSplitter::new(32, 4, true).expect("markdown splitter instantiation failed");
        let chunks = splitter.chunks(markdown);
        assert!(chunks.len() >= 2, "expected multiple markdown chunks");
        assert!(chunks[0].contains("# Title"));
        assert!(chunks.iter().all(|chunk| !chunk.is_empty()));
        assert!(chunks.iter().all(|chunk| chunk.len() <= 32));
    }

    #[test]
    fn invalid_overlap_raises_value_error() {
        ensure_python();
        let err = PyTextSplitter::new(8, 16, true)
            .err()
            .expect("expected overlap validation error");
        let message = err.to_string();
        assert!(
            message.contains("overlap") || message.contains("must be"),
            "unexpected error message: {message}"
        );
    }
}
