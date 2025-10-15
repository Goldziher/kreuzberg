use pyo3::prelude::*;
use std::collections::HashMap;

// PyO3 wrapper types for OCR

#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PSMMode {
    OsdOnly = 0,
    AutoOsd = 1,
    AutoOnly = 2,
    Auto = 3,
    SingleColumn = 4,
    SingleBlockVertical = 5,
    SingleBlock = 6,
    SingleLine = 7,
    SingleWord = 8,
    CircleWord = 9,
    SingleChar = 10,
}

#[pymethods]
impl PSMMode {
    #[new]
    fn new(value: u8) -> PyResult<Self> {
        match value {
            0 => Ok(PSMMode::OsdOnly),
            1 => Ok(PSMMode::AutoOsd),
            2 => Ok(PSMMode::AutoOnly),
            3 => Ok(PSMMode::Auto),
            4 => Ok(PSMMode::SingleColumn),
            5 => Ok(PSMMode::SingleBlockVertical),
            6 => Ok(PSMMode::SingleBlock),
            7 => Ok(PSMMode::SingleLine),
            8 => Ok(PSMMode::SingleWord),
            9 => Ok(PSMMode::CircleWord),
            10 => Ok(PSMMode::SingleChar),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid PSM mode value: {}",
                value
            ))),
        }
    }

    fn __repr__(&self) -> String {
        format!("PSMMode.{:?}", self)
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct TesseractConfigDTO {
    #[pyo3(get, set)]
    pub language: String,

    #[pyo3(get, set)]
    pub psm: u8,

    #[pyo3(get, set)]
    pub output_format: String,

    #[pyo3(get, set)]
    pub enable_table_detection: bool,

    #[pyo3(get, set)]
    pub table_min_confidence: f64,

    #[pyo3(get, set)]
    pub table_column_threshold: u32,

    #[pyo3(get, set)]
    pub table_row_threshold_ratio: f64,

    #[pyo3(get, set)]
    pub use_cache: bool,

    #[pyo3(get, set)]
    pub classify_use_pre_adapted_templates: bool,

    #[pyo3(get, set)]
    pub language_model_ngram_on: bool,

    #[pyo3(get, set)]
    pub tessedit_dont_blkrej_good_wds: bool,

    #[pyo3(get, set)]
    pub tessedit_dont_rowrej_good_wds: bool,

    #[pyo3(get, set)]
    pub tessedit_enable_dict_correction: bool,

    #[pyo3(get, set)]
    pub tessedit_char_whitelist: String,

    #[pyo3(get, set)]
    pub tessedit_use_primary_params_model: bool,

    #[pyo3(get, set)]
    pub textord_space_size_is_variable: bool,

    #[pyo3(get, set)]
    pub thresholding_method: bool,
}

#[pymethods]
impl TesseractConfigDTO {
    #[new]
    #[pyo3(signature = (
        language = "eng".to_string(),
        psm = 3,
        output_format = "markdown".to_string(),
        enable_table_detection = true,
        table_min_confidence = 0.0,
        table_column_threshold = 50,
        table_row_threshold_ratio = 0.5,
        use_cache = true,
        classify_use_pre_adapted_templates = true,
        language_model_ngram_on = false,
        tessedit_dont_blkrej_good_wds = true,
        tessedit_dont_rowrej_good_wds = true,
        tessedit_enable_dict_correction = true,
        tessedit_char_whitelist = String::new(),
        tessedit_use_primary_params_model = true,
        textord_space_size_is_variable = true,
        thresholding_method = false,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        language: String,
        psm: u8,
        output_format: String,
        enable_table_detection: bool,
        table_min_confidence: f64,
        table_column_threshold: u32,
        table_row_threshold_ratio: f64,
        use_cache: bool,
        classify_use_pre_adapted_templates: bool,
        language_model_ngram_on: bool,
        tessedit_dont_blkrej_good_wds: bool,
        tessedit_dont_rowrej_good_wds: bool,
        tessedit_enable_dict_correction: bool,
        tessedit_char_whitelist: String,
        tessedit_use_primary_params_model: bool,
        textord_space_size_is_variable: bool,
        thresholding_method: bool,
    ) -> PyResult<Self> {
        match output_format.as_str() {
            "text" | "markdown" | "hocr" | "tsv" => {}
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid output_format: '{}'. Must be one of: text, markdown, hocr, tsv",
                    output_format
                )));
            }
        }

        Ok(Self {
            language,
            psm,
            output_format,
            enable_table_detection,
            table_min_confidence,
            table_column_threshold,
            table_row_threshold_ratio,
            use_cache,
            classify_use_pre_adapted_templates,
            language_model_ngram_on,
            tessedit_dont_blkrej_good_wds,
            tessedit_dont_rowrej_good_wds,
            tessedit_enable_dict_correction,
            tessedit_char_whitelist,
            tessedit_use_primary_params_model,
            textord_space_size_is_variable,
            thresholding_method,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "TesseractConfigDTO(language='{}', psm={}, output_format='{}')",
            self.language, self.psm, self.output_format
        )
    }
}

impl From<&TesseractConfigDTO> for kreuzberg::ocr::TesseractConfig {
    fn from(dto: &TesseractConfigDTO) -> Self {
        kreuzberg::ocr::TesseractConfig {
            language: dto.language.clone(),
            psm: dto.psm,
            output_format: dto.output_format.clone(),
            enable_table_detection: dto.enable_table_detection,
            table_min_confidence: dto.table_min_confidence,
            table_column_threshold: dto.table_column_threshold,
            table_row_threshold_ratio: dto.table_row_threshold_ratio,
            use_cache: dto.use_cache,
            classify_use_pre_adapted_templates: dto.classify_use_pre_adapted_templates,
            language_model_ngram_on: dto.language_model_ngram_on,
            tessedit_dont_blkrej_good_wds: dto.tessedit_dont_blkrej_good_wds,
            tessedit_dont_rowrej_good_wds: dto.tessedit_dont_rowrej_good_wds,
            tessedit_enable_dict_correction: dto.tessedit_enable_dict_correction,
            tessedit_char_whitelist: dto.tessedit_char_whitelist.clone(),
            tessedit_use_primary_params_model: dto.tessedit_use_primary_params_model,
            textord_space_size_is_variable: dto.textord_space_size_is_variable,
            thresholding_method: dto.thresholding_method,
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct ExtractionResultDTO {
    #[pyo3(get, set)]
    pub content: String,

    #[pyo3(get, set)]
    pub mime_type: String,

    #[pyo3(get, set)]
    pub metadata: HashMap<String, String>,

    #[pyo3(get, set)]
    pub tables: Vec<TableDTO>,
}

#[pymethods]
impl ExtractionResultDTO {
    #[new]
    fn new(
        content: String,
        mime_type: String,
        metadata: Option<HashMap<String, String>>,
        tables: Option<Vec<TableDTO>>,
    ) -> Self {
        Self {
            content,
            mime_type,
            metadata: metadata.unwrap_or_default(),
            tables: tables.unwrap_or_default(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ExtractionResultDTO(mime_type='{}', content_length={}, tables={})",
            self.mime_type,
            self.content.len(),
            self.tables.len()
        )
    }
}

impl From<kreuzberg::types::OcrExtractionResult> for ExtractionResultDTO {
    fn from(result: kreuzberg::types::OcrExtractionResult) -> Self {
        Self {
            content: result.content,
            mime_type: result.mime_type,
            metadata: result.metadata.into_iter().map(|(k, v)| (k, v.to_string())).collect(),
            tables: result.tables.into_iter().map(TableDTO::from).collect(),
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct TableDTO {
    #[pyo3(get, set)]
    pub cells: Vec<Vec<String>>,

    #[pyo3(get, set)]
    pub markdown: String,

    #[pyo3(get, set)]
    pub page_number: i32,
}

#[pymethods]
impl TableDTO {
    #[new]
    fn new(cells: Vec<Vec<String>>, markdown: String, page_number: i32) -> Self {
        Self {
            cells,
            markdown,
            page_number,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "TableDTO(rows={}, cols={}, page={})",
            self.cells.len(),
            self.cells.first().map(|r| r.len()).unwrap_or(0),
            self.page_number
        )
    }
}

impl From<kreuzberg::types::OcrTable> for TableDTO {
    fn from(table: kreuzberg::types::OcrTable) -> Self {
        Self {
            cells: table.cells,
            markdown: table.markdown,
            page_number: table.page_number as i32,
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct BatchItemResult {
    #[pyo3(get, set)]
    pub file_path: String,

    #[pyo3(get, set)]
    pub success: bool,

    #[pyo3(get, set)]
    pub result: Option<ExtractionResultDTO>,

    #[pyo3(get, set)]
    pub error: Option<String>,
}

#[pymethods]
impl BatchItemResult {
    #[new]
    fn new(file_path: String, success: bool, result: Option<ExtractionResultDTO>, error: Option<String>) -> Self {
        Self {
            file_path,
            success,
            result,
            error,
        }
    }
}

impl From<kreuzberg::ocr::BatchItemResult> for BatchItemResult {
    fn from(result: kreuzberg::ocr::BatchItemResult) -> Self {
        Self {
            file_path: result.file_path,
            success: result.success,
            result: result.result.map(|r| ExtractionResultDTO::from(r)),
            error: result.error,
        }
    }
}

#[pyclass]
pub struct OCRProcessor {
    processor: kreuzberg::ocr::OcrProcessor,
}

#[pymethods]
impl OCRProcessor {
    #[new]
    #[pyo3(signature = (cache_dir = None))]
    pub fn new(cache_dir: Option<std::path::PathBuf>) -> PyResult<Self> {
        let processor = kreuzberg::ocr::OcrProcessor::new(cache_dir)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { processor })
    }

    pub fn process_image(&self, image_bytes: &[u8], config: &TesseractConfigDTO) -> PyResult<ExtractionResultDTO> {
        let rust_config: kreuzberg::ocr::TesseractConfig = config.into();
        let result = self
            .processor
            .process_image(image_bytes, &rust_config)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(ExtractionResultDTO::from(result))
    }

    pub fn clear_cache(&self) -> PyResult<()> {
        self.processor
            .clear_cache()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    pub fn get_cache_stats(&self) -> PyResult<crate::bindings::cache::OCRCacheStats> {
        self.processor
            .get_cache_stats()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
            .map(crate::bindings::cache::OCRCacheStats::from)
    }

    pub fn process_file(&self, file_path: &str, config: &TesseractConfigDTO) -> PyResult<ExtractionResultDTO> {
        let rust_config: kreuzberg::ocr::TesseractConfig = config.into();
        let result = self
            .processor
            .process_file(file_path, &rust_config)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(ExtractionResultDTO::from(result))
    }

    pub fn process_files_batch(
        &self,
        file_paths: Vec<String>,
        config: &TesseractConfigDTO,
    ) -> PyResult<Vec<BatchItemResult>> {
        let rust_config: kreuzberg::ocr::TesseractConfig = config.into();
        let results = self.processor.process_files_batch(file_paths, &rust_config);
        Ok(results.into_iter().map(BatchItemResult::from).collect())
    }
}

#[pyfunction]
pub fn validate_language_code(language: &str) -> PyResult<()> {
    kreuzberg::ocr::validate_language_code(language).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

#[pyfunction]
pub fn validate_tesseract_version(min_version: u32) -> PyResult<()> {
    kreuzberg::ocr::validate_tesseract_version(min_version)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

pub fn register_ocr_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(validate_language_code, m)?)?;
    m.add_function(wrap_pyfunction!(validate_tesseract_version, m)?)?;
    m.add_class::<PSMMode>()?;
    m.add_class::<TesseractConfigDTO>()?;
    m.add_class::<ExtractionResultDTO>()?;
    m.add_class::<TableDTO>()?;
    m.add_class::<BatchItemResult>()?;
    m.add_class::<OCRProcessor>()?;
    Ok(())
}
