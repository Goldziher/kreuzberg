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

pub fn register_chunking_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTextSplitter>()?;
    m.add_class::<PyMarkdownSplitter>()?;
    Ok(())
}
