use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

impl Default for TesseractConfigDTO {
    fn default() -> Self {
        Self {
            language: "eng".to_string(),
            psm: 3,
            output_format: "markdown".to_string(),
            enable_table_detection: true,
            table_min_confidence: 0.0,
            table_column_threshold: 50,
            table_row_threshold_ratio: 0.5,
            use_cache: true,
            classify_use_pre_adapted_templates: true,
            language_model_ngram_on: false,
            tessedit_dont_blkrej_good_wds: true,
            tessedit_dont_rowrej_good_wds: true,
            tessedit_enable_dict_correction: true,
            tessedit_char_whitelist: String::new(),
            tessedit_use_primary_params_model: true,
            textord_space_size_is_variable: true,
            thresholding_method: false,
        }
    }
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

#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psm_mode_creation_valid() {
        let modes = [
            (0, PSMMode::OsdOnly),
            (1, PSMMode::AutoOsd),
            (2, PSMMode::AutoOnly),
            (3, PSMMode::Auto),
            (4, PSMMode::SingleColumn),
            (5, PSMMode::SingleBlockVertical),
            (6, PSMMode::SingleBlock),
            (7, PSMMode::SingleLine),
            (8, PSMMode::SingleWord),
            (9, PSMMode::CircleWord),
            (10, PSMMode::SingleChar),
        ];

        for (value, expected) in modes {
            let mode = PSMMode::new(value).unwrap();
            assert_eq!(mode, expected);
        }
    }

    #[test]
    fn test_psm_mode_creation_invalid() {
        let invalid_values = [11, 12, 255, 100];

        for value in invalid_values {
            let result = PSMMode::new(value);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Invalid PSM mode"));
        }
    }

    #[test]
    fn test_psm_mode_repr() {
        let mode = PSMMode::Auto;
        let repr = mode.__repr__();
        assert!(repr.contains("PSMMode"));
        assert!(repr.contains("Auto"));
    }

    #[test]
    fn test_tesseract_config_default() {
        let config = TesseractConfigDTO {
            language: "eng".to_string(),
            psm: 3,
            output_format: "markdown".to_string(),
            enable_table_detection: true,
            table_min_confidence: 0.0,
            table_column_threshold: 50,
            table_row_threshold_ratio: 0.5,
            use_cache: true,
            classify_use_pre_adapted_templates: true,
            language_model_ngram_on: false,
            tessedit_dont_blkrej_good_wds: true,
            tessedit_dont_rowrej_good_wds: true,
            tessedit_enable_dict_correction: true,
            tessedit_char_whitelist: String::new(),
            tessedit_use_primary_params_model: true,
            textord_space_size_is_variable: true,
            thresholding_method: false,
        };

        assert_eq!(config.language, "eng");
        assert_eq!(config.psm, 3);
        assert_eq!(config.output_format, "markdown");
        assert!(config.enable_table_detection);
        assert_eq!(config.table_min_confidence, 0.0);
        assert_eq!(config.table_column_threshold, 50);
        assert_eq!(config.table_row_threshold_ratio, 0.5);
        assert!(config.use_cache);
    }

    #[test]
    fn test_tesseract_config_valid_output_formats() {
        let valid_formats = ["text", "markdown", "hocr", "tsv"];

        for format in valid_formats {
            let config = TesseractConfigDTO {
                language: "eng".to_string(),
                psm: 3,
                output_format: format.to_string(),
                enable_table_detection: false,
                table_min_confidence: 0.0,
                table_column_threshold: 50,
                table_row_threshold_ratio: 0.5,
                use_cache: true,
                classify_use_pre_adapted_templates: true,
                language_model_ngram_on: false,
                tessedit_dont_blkrej_good_wds: true,
                tessedit_dont_rowrej_good_wds: true,
                tessedit_enable_dict_correction: true,
                tessedit_char_whitelist: String::new(),
                tessedit_use_primary_params_model: true,
                textord_space_size_is_variable: true,
                thresholding_method: false,
            };

            assert_eq!(config.output_format, format);
        }
    }

    #[test]
    fn test_tesseract_config_repr() {
        let config = TesseractConfigDTO {
            language: "eng".to_string(),
            psm: 3,
            output_format: "text".to_string(),
            enable_table_detection: false,
            table_min_confidence: 0.0,
            table_column_threshold: 50,
            table_row_threshold_ratio: 0.5,
            use_cache: true,
            classify_use_pre_adapted_templates: true,
            language_model_ngram_on: false,
            tessedit_dont_blkrej_good_wds: true,
            tessedit_dont_rowrej_good_wds: true,
            tessedit_enable_dict_correction: true,
            tessedit_char_whitelist: String::new(),
            tessedit_use_primary_params_model: true,
            textord_space_size_is_variable: true,
            thresholding_method: false,
        };

        let repr = config.__repr__();
        assert!(repr.contains("TesseractConfigDTO"));
        assert!(repr.contains("eng"));
        assert!(repr.contains("psm=3"));
        assert!(repr.contains("text"));
    }

    #[test]
    fn test_extraction_result_dto_creation() {
        let mut metadata = HashMap::new();
        metadata.insert("key".to_string(), "value".to_string());

        let table = TableDTO::new(vec![vec!["A".to_string(), "B".to_string()]], "| A | B |".to_string(), 0);

        let result = ExtractionResultDTO::new(
            "Test content".to_string(),
            "text/plain".to_string(),
            Some(metadata.clone()),
            Some(vec![table.clone()]),
        );

        assert_eq!(result.content, "Test content");
        assert_eq!(result.mime_type, "text/plain");
        assert_eq!(result.metadata.get("key").unwrap(), "value");
        assert_eq!(result.tables.len(), 1);
    }

    #[test]
    fn test_extraction_result_dto_defaults() {
        let result = ExtractionResultDTO::new("Test".to_string(), "text/plain".to_string(), None, None);

        assert_eq!(result.content, "Test");
        assert_eq!(result.mime_type, "text/plain");
        assert!(result.metadata.is_empty());
        assert!(result.tables.is_empty());
    }

    #[test]
    fn test_extraction_result_dto_repr() {
        let result = ExtractionResultDTO::new("Test content".to_string(), "text/plain".to_string(), None, None);

        let repr = result.__repr__();
        assert!(repr.contains("ExtractionResultDTO"));
        assert!(repr.contains("text/plain"));
        assert!(repr.contains("content_length=12"));
        assert!(repr.contains("tables=0"));
    }

    #[test]
    fn test_table_dto_creation() {
        let cells = vec![
            vec!["Header1".to_string(), "Header2".to_string()],
            vec!["Value1".to_string(), "Value2".to_string()],
        ];

        let markdown = "| Header1 | Header2 |\n| ------- | ------- |\n| Value1  | Value2  |".to_string();

        let table = TableDTO::new(cells.clone(), markdown.clone(), 1);

        assert_eq!(table.cells.len(), 2);
        assert_eq!(table.cells[0].len(), 2);
        assert_eq!(table.markdown, markdown);
        assert_eq!(table.page_number, 1);
    }

    #[test]
    fn test_table_dto_repr() {
        let cells = vec![
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
            vec!["1".to_string(), "2".to_string(), "3".to_string()],
        ];

        let table = TableDTO::new(cells, "markdown".to_string(), 2);

        let repr = table.__repr__();
        assert!(repr.contains("TableDTO"));
        assert!(repr.contains("rows=2"));
        assert!(repr.contains("cols=3"));
        assert!(repr.contains("page=2"));
    }

    #[test]
    fn test_table_dto_empty() {
        let table = TableDTO::new(vec![], String::new(), 0);

        assert_eq!(table.cells.len(), 0);
        assert_eq!(table.markdown, "");
        assert_eq!(table.page_number, 0);

        let repr = table.__repr__();
        assert!(repr.contains("rows=0"));
        assert!(repr.contains("cols=0"));
    }

    #[test]
    fn test_batch_item_result_success() {
        let result = ExtractionResultDTO::new("content".to_string(), "text/plain".to_string(), None, None);

        let batch_result = BatchItemResult::new("/path/to/file.png".to_string(), true, Some(result), None);

        assert_eq!(batch_result.file_path, "/path/to/file.png");
        assert!(batch_result.success);
        assert!(batch_result.result.is_some());
        assert!(batch_result.error.is_none());
    }

    #[test]
    fn test_batch_item_result_failure() {
        let batch_result = BatchItemResult::new(
            "/path/to/file.png".to_string(),
            false,
            None,
            Some("File not found".to_string()),
        );

        assert_eq!(batch_result.file_path, "/path/to/file.png");
        assert!(!batch_result.success);
        assert!(batch_result.result.is_none());
        assert_eq!(batch_result.error.as_ref().unwrap(), "File not found");
    }

    #[test]
    fn test_tesseract_config_custom_settings() {
        let config = TesseractConfigDTO {
            language: "fra".to_string(),
            psm: 6,
            output_format: "hocr".to_string(),
            enable_table_detection: true,
            table_min_confidence: 90.0,
            table_column_threshold: 100,
            table_row_threshold_ratio: 0.8,
            use_cache: false,
            classify_use_pre_adapted_templates: false,
            language_model_ngram_on: true,
            tessedit_dont_blkrej_good_wds: false,
            tessedit_dont_rowrej_good_wds: false,
            tessedit_enable_dict_correction: false,
            tessedit_char_whitelist: "0123456789".to_string(),
            tessedit_use_primary_params_model: false,
            textord_space_size_is_variable: false,
            thresholding_method: true,
        };

        assert_eq!(config.language, "fra");
        assert_eq!(config.psm, 6);
        assert_eq!(config.output_format, "hocr");
        assert!(config.enable_table_detection);
        assert_eq!(config.table_min_confidence, 90.0);
        assert_eq!(config.table_column_threshold, 100);
        assert_eq!(config.table_row_threshold_ratio, 0.8);
        assert!(!config.use_cache);
        assert!(!config.classify_use_pre_adapted_templates);
        assert!(config.language_model_ngram_on);
        assert_eq!(config.tessedit_char_whitelist, "0123456789");
        assert!(config.thresholding_method);
    }

    #[test]
    fn test_extraction_result_clone() {
        let result1 = ExtractionResultDTO::new("Test".to_string(), "text/plain".to_string(), None, None);

        let result2 = result1.clone();

        assert_eq!(result1.content, result2.content);
        assert_eq!(result1.mime_type, result2.mime_type);
    }

    #[test]
    fn test_table_dto_clone() {
        let table1 = TableDTO::new(vec![vec!["A".to_string()]], "| A |".to_string(), 0);

        let table2 = table1.clone();

        assert_eq!(table1.cells, table2.cells);
        assert_eq!(table1.markdown, table2.markdown);
        assert_eq!(table1.page_number, table2.page_number);
    }

    #[test]
    fn test_tesseract_config_default_trait() {
        let config = TesseractConfigDTO::default();

        assert_eq!(config.language, "eng");
        assert_eq!(config.psm, 3);
        assert_eq!(config.output_format, "markdown");
        assert!(config.enable_table_detection);
        assert_eq!(config.table_min_confidence, 0.0);
        assert_eq!(config.table_column_threshold, 50);
        assert_eq!(config.table_row_threshold_ratio, 0.5);
        assert!(config.use_cache);
        assert!(config.classify_use_pre_adapted_templates);
        assert!(!config.language_model_ngram_on);
        assert!(config.tessedit_dont_blkrej_good_wds);
        assert!(config.tessedit_dont_rowrej_good_wds);
        assert!(config.tessedit_enable_dict_correction);
        assert!(config.tessedit_char_whitelist.is_empty());
        assert!(config.tessedit_use_primary_params_model);
        assert!(config.textord_space_size_is_variable);
        assert!(!config.thresholding_method);
    }
}
