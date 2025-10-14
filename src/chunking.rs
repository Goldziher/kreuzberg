use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

#[pyclass(module = "kreuzberg._internal_bindings", name = "TextSplitter", frozen)]
pub struct PyTextSplitter {
    max_characters: usize,
    overlap: usize,
    trim: bool,
}

#[pymethods]
impl PyTextSplitter {
    #[new]
    #[pyo3(signature = (max_characters, overlap=0, trim=true))]
    fn new(max_characters: usize, overlap: usize, trim: bool) -> PyResult<Self> {
        // Validate config early to match old behavior
        let config = kreuzberg::chunking::ChunkingConfig {
            max_characters,
            overlap,
            trim,
            chunker_type: kreuzberg::chunking::ChunkerType::Text,
        };

        // Test the config by attempting to chunk empty string
        kreuzberg::chunking::chunk_text("", &config).map_err(|e| PyValueError::new_err(e.to_string()))?;

        Ok(Self {
            max_characters,
            overlap,
            trim,
        })
    }

    fn chunks(&self, text: &str) -> PyResult<Vec<String>> {
        let config = kreuzberg::chunking::ChunkingConfig {
            max_characters: self.max_characters,
            overlap: self.overlap,
            trim: self.trim,
            chunker_type: kreuzberg::chunking::ChunkerType::Text,
        };

        let result =
            kreuzberg::chunking::chunk_text(text, &config).map_err(|e| PyValueError::new_err(e.to_string()))?;

        Ok(result.chunks)
    }
}

#[pyclass(module = "kreuzberg._internal_bindings", name = "MarkdownSplitter", frozen)]
pub struct PyMarkdownSplitter {
    max_characters: usize,
    overlap: usize,
    trim: bool,
}

#[pymethods]
impl PyMarkdownSplitter {
    #[new]
    #[pyo3(signature = (max_characters, overlap=0, trim=true))]
    fn new(max_characters: usize, overlap: usize, trim: bool) -> PyResult<Self> {
        // Validate config early to match old behavior
        let config = kreuzberg::chunking::ChunkingConfig {
            max_characters,
            overlap,
            trim,
            chunker_type: kreuzberg::chunking::ChunkerType::Markdown,
        };

        // Test the config by attempting to chunk empty string
        kreuzberg::chunking::chunk_text("", &config).map_err(|e| PyValueError::new_err(e.to_string()))?;

        Ok(Self {
            max_characters,
            overlap,
            trim,
        })
    }

    fn chunks(&self, text: &str) -> PyResult<Vec<String>> {
        let config = kreuzberg::chunking::ChunkingConfig {
            max_characters: self.max_characters,
            overlap: self.overlap,
            trim: self.trim,
            chunker_type: kreuzberg::chunking::ChunkerType::Markdown,
        };

        let result =
            kreuzberg::chunking::chunk_text(text, &config).map_err(|e| PyValueError::new_err(e.to_string()))?;

        Ok(result.chunks)
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
        let chunks = splitter.chunks("0123456789ABCDEFGHIJ").unwrap();
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
        let chunks = splitter.chunks(markdown).unwrap();
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
            message.contains("overlap") || message.contains("must be") || message.contains("Invalid"),
            "unexpected error message: {message}"
        );
    }
}
