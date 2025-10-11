use html_to_markdown_rs::{
    CodeBlockStyle, ConversionOptions, HeadingStyle, HighlightStyle, ListIndentType, NewlineStyle, ParsingOptions,
    PreprocessingOptions, PreprocessingPreset, WhitespaceMode, convert as convert_html,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};

fn extract_string_list(value: &Bound<'_, PyAny>) -> PyResult<Vec<String>> {
    if value.is_none() {
        return Ok(Vec::new());
    }
    let items: Vec<String> = value.extract()?;
    Ok(items.into_iter().filter(|item| !item.is_empty()).collect())
}

fn extract_char(value: &Bound<'_, PyAny>) -> PyResult<char> {
    let text: String = value.extract()?;
    Ok(text.chars().next().unwrap_or('*'))
}

fn parse_heading_style(value: &Bound<'_, PyAny>) -> PyResult<HeadingStyle> {
    match value.extract::<&str>()?.to_lowercase().as_str() {
        "underlined" => Ok(HeadingStyle::Underlined),
        "atx" => Ok(HeadingStyle::Atx),
        "atx_closed" => Ok(HeadingStyle::AtxClosed),
        unexpected => Err(PyValueError::new_err(format!(
            "Unsupported heading_style '{unexpected}'"
        ))),
    }
}

fn parse_list_indent_type(value: &Bound<'_, PyAny>) -> PyResult<ListIndentType> {
    match value.extract::<&str>()?.to_lowercase().as_str() {
        "spaces" => Ok(ListIndentType::Spaces),
        "tabs" => Ok(ListIndentType::Tabs),
        unexpected => Err(PyValueError::new_err(format!(
            "Unsupported list_indent_type '{unexpected}'"
        ))),
    }
}

fn parse_whitespace_mode(value: &Bound<'_, PyAny>) -> PyResult<WhitespaceMode> {
    match value.extract::<&str>()?.to_lowercase().as_str() {
        "normalized" => Ok(WhitespaceMode::Normalized),
        "strict" => Ok(WhitespaceMode::Strict),
        unexpected => Err(PyValueError::new_err(format!(
            "Unsupported whitespace_mode '{unexpected}'"
        ))),
    }
}

fn parse_newline_style(value: &Bound<'_, PyAny>) -> PyResult<NewlineStyle> {
    match value.extract::<&str>()?.to_lowercase().as_str() {
        "spaces" => Ok(NewlineStyle::Spaces),
        "backslash" => Ok(NewlineStyle::Backslash),
        unexpected => Err(PyValueError::new_err(format!(
            "Unsupported newline_style '{unexpected}'"
        ))),
    }
}

fn parse_code_block_style(value: &Bound<'_, PyAny>) -> PyResult<CodeBlockStyle> {
    match value.extract::<&str>()?.to_lowercase().as_str() {
        "indented" => Ok(CodeBlockStyle::Indented),
        "backticks" => Ok(CodeBlockStyle::Backticks),
        "tildes" => Ok(CodeBlockStyle::Tildes),
        unexpected => Err(PyValueError::new_err(format!(
            "Unsupported code_block_style '{unexpected}'"
        ))),
    }
}

fn parse_highlight_style(value: &Bound<'_, PyAny>) -> PyResult<HighlightStyle> {
    match value.extract::<&str>()?.to_lowercase().as_str() {
        "double-equal" => Ok(HighlightStyle::DoubleEqual),
        "html" => Ok(HighlightStyle::Html),
        "bold" => Ok(HighlightStyle::Bold),
        "none" => Ok(HighlightStyle::None),
        unexpected => Err(PyValueError::new_err(format!(
            "Unsupported highlight_style '{unexpected}'"
        ))),
    }
}

fn parse_preprocessing_preset(value: &Bound<'_, PyAny>) -> PyResult<PreprocessingPreset> {
    match value.extract::<&str>()?.to_lowercase().as_str() {
        "minimal" => Ok(PreprocessingPreset::Minimal),
        "standard" => Ok(PreprocessingPreset::Standard),
        "aggressive" => Ok(PreprocessingPreset::Aggressive),
        unexpected => Err(PyValueError::new_err(format!(
            "Unsupported preprocessing preset '{unexpected}'"
        ))),
    }
}

fn build_conversion_options(dict: &Bound<'_, PyDict>) -> PyResult<ConversionOptions> {
    let mut options = ConversionOptions::default();

    if let Some(value) = dict.get_item("heading_style")? {
        options.heading_style = parse_heading_style(&value)?;
    }
    if let Some(value) = dict.get_item("list_indent_type")? {
        options.list_indent_type = parse_list_indent_type(&value)?;
    }
    if let Some(value) = dict.get_item("list_indent_width")? {
        options.list_indent_width = value.extract::<usize>()?;
    }
    if let Some(value) = dict.get_item("bullets")? {
        options.bullets = value.extract::<String>()?;
    }
    if let Some(value) = dict.get_item("strong_em_symbol")? {
        options.strong_em_symbol = extract_char(&value)?;
    }
    if let Some(value) = dict.get_item("escape_asterisks")? {
        options.escape_asterisks = value.extract::<bool>()?;
    }
    if let Some(value) = dict.get_item("escape_underscores")? {
        options.escape_underscores = value.extract::<bool>()?;
    }
    if let Some(value) = dict.get_item("escape_misc")? {
        options.escape_misc = value.extract::<bool>()?;
    }
    if let Some(value) = dict.get_item("escape_ascii")? {
        options.escape_ascii = value.extract::<bool>()?;
    }
    if let Some(value) = dict.get_item("code_language")? {
        options.code_language = value.extract::<String>()?;
    }
    if let Some(value) = dict.get_item("autolinks")? {
        options.autolinks = value.extract::<bool>()?;
    }
    if let Some(value) = dict.get_item("default_title")? {
        options.default_title = value.extract::<bool>()?;
    }
    if let Some(value) = dict.get_item("br_in_tables")? {
        options.br_in_tables = value.extract::<bool>()?;
    }
    if let Some(value) = dict.get_item("highlight_style")? {
        options.highlight_style = parse_highlight_style(&value)?;
    }
    if let Some(value) = dict.get_item("extract_metadata")? {
        options.extract_metadata = value.extract::<bool>()?;
    }
    if let Some(value) = dict.get_item("whitespace_mode")? {
        options.whitespace_mode = parse_whitespace_mode(&value)?;
    }
    if let Some(value) = dict.get_item("strip_newlines")? {
        options.strip_newlines = value.extract::<bool>()?;
    }
    if let Some(value) = dict.get_item("wrap")? {
        options.wrap = value.extract::<bool>()?;
    }
    if let Some(value) = dict.get_item("wrap_width")? {
        options.wrap_width = value.extract::<usize>()?;
    }
    if let Some(value) = dict.get_item("convert_as_inline")? {
        options.convert_as_inline = value.extract::<bool>()?;
    }
    if let Some(value) = dict.get_item("sub_symbol")? {
        options.sub_symbol = value.extract::<String>()?;
    }
    if let Some(value) = dict.get_item("sup_symbol")? {
        options.sup_symbol = value.extract::<String>()?;
    }
    if let Some(value) = dict.get_item("newline_style")? {
        options.newline_style = parse_newline_style(&value)?;
    }
    if let Some(value) = dict.get_item("code_block_style")? {
        options.code_block_style = parse_code_block_style(&value)?;
    }
    if let Some(value) = dict.get_item("keep_inline_images_in")? {
        options.keep_inline_images_in = extract_string_list(&value)?;
    }
    if let Some(value) = dict.get_item("hocr_extract_tables")? {
        options.hocr_extract_tables = value.extract::<bool>()?;
    }
    if let Some(value) = dict.get_item("hocr_table_column_threshold")? {
        options.hocr_table_column_threshold = value.extract::<u32>()?;
    }
    if let Some(value) = dict.get_item("hocr_table_row_threshold_ratio")? {
        options.hocr_table_row_threshold_ratio = value.extract::<f64>()?;
    }
    if let Some(value) = dict.get_item("debug")? {
        options.debug = value.extract::<bool>()?;
    }
    if let Some(value) = dict.get_item("strip_tags")? {
        options.strip_tags = extract_string_list(&value)?;
    }
    if let Some(value) = dict.get_item("preprocessing")? {
        let prep_dict = value.downcast::<PyDict>()?;
        let mut preprocessing = PreprocessingOptions::default();
        if let Some(enabled) = prep_dict.get_item("enabled")? {
            preprocessing.enabled = enabled.extract::<bool>()?;
        }
        if let Some(preset) = prep_dict.get_item("preset")? {
            preprocessing.preset = parse_preprocessing_preset(&preset)?;
        }
        if let Some(remove_navigation) = prep_dict.get_item("remove_navigation")? {
            preprocessing.remove_navigation = remove_navigation.extract::<bool>()?;
        }
        if let Some(remove_forms) = prep_dict.get_item("remove_forms")? {
            preprocessing.remove_forms = remove_forms.extract::<bool>()?;
        }
        options.preprocessing = preprocessing;
    }

    if let Some(value) = dict.get_item("parsing")? {
        let parsing_dict = value.downcast::<PyDict>()?;
        let mut parsing = ParsingOptions::default();
        if let Some(encoding) = parsing_dict.get_item("encoding")? {
            parsing.encoding = encoding.extract::<String>()?;
        }
        if let Some(parser) = parsing_dict.get_item("parser")? {
            parsing.parser = parser.extract::<Option<String>>()?;
        }
        options.parsing = parsing;
    }

    Ok(options)
}

#[pyfunction]
pub fn convert_html_to_markdown(html: &str, options: Option<Bound<'_, PyDict>>) -> PyResult<String> {
    let conversion_options = if let Some(dict) = options {
        build_conversion_options(&dict)?
    } else {
        ConversionOptions::default()
    };

    convert_html(html, Some(conversion_options))
        .map_err(|err| PyValueError::new_err(format!("Failed to convert HTML to Markdown: {err}")))
}
