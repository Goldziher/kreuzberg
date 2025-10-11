use html_to_markdown_rs::{
    CodeBlockStyle, ConversionOptions, HeadingStyle, HighlightStyle, InlineImage,
    InlineImageConfig as RustInlineImageConfig, InlineImageFormat, InlineImageWarning, ListIndentType, NewlineStyle,
    PreprocessingOptions, PreprocessingPreset, Result as HtmlResult, WhitespaceMode, convert as convert_html,
    convert_with_inline_images,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes, PyDict, PyList};

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

fn build_conversion_options(dict: Option<Bound<'_, PyDict>>) -> PyResult<ConversionOptions> {
    let mut options = ConversionOptions::default();

    let Some(dict) = dict else {
        return Ok(options);
    };

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
    if let Some(value) = dict.get_item("debug")? {
        options.debug = value.extract::<bool>()?;
    }
    if let Some(value) = dict.get_item("strip_tags")? {
        options.strip_tags = extract_string_list(&value)?;
    }
    if let Some(value) = dict.get_item("encoding")? {
        options.encoding = value.extract::<String>()?;
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

    Ok(options)
}

fn inline_image_format_to_str(format: &InlineImageFormat) -> String {
    match format {
        InlineImageFormat::Png => "png".to_string(),
        InlineImageFormat::Jpeg => "jpeg".to_string(),
        InlineImageFormat::Gif => "gif".to_string(),
        InlineImageFormat::Bmp => "bmp".to_string(),
        InlineImageFormat::Webp => "webp".to_string(),
        InlineImageFormat::Svg => "svg".to_string(),
        InlineImageFormat::Other(custom) => {
            let trimmed = custom.trim();
            if trimmed.is_empty() {
                return "bin".to_string();
            }

            let lower = trimmed.to_ascii_lowercase();
            if lower.starts_with("svg") {
                return "svg".to_string();
            }

            let mut candidate = lower.as_str();

            if let Some(idx) = candidate.find(['+', ';']) {
                candidate = &candidate[..idx];
            }

            if let Some(idx) = candidate.rfind('.') {
                candidate = &candidate[idx + 1..];
            }

            candidate = candidate.trim_start_matches("x-");

            if candidate.is_empty() {
                "bin".to_string()
            } else {
                candidate.to_string()
            }
        }
    }
}

fn inline_image_to_py(py: Python<'_>, image: InlineImage) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("data", PyBytes::new(py, &image.data))?;
    dict.set_item("format", inline_image_format_to_str(&image.format))?;

    match image.filename {
        Some(filename) => dict.set_item("filename", filename)?,
        None => dict.set_item("filename", py.None())?,
    }

    match image.description {
        Some(description) => dict.set_item("description", description)?,
        None => dict.set_item("description", py.None())?,
    }

    if let Some((width, height)) = image.dimensions {
        dict.set_item("dimensions", (width, height))?;
    } else {
        dict.set_item("dimensions", py.None())?;
    }

    let attributes = PyDict::new(py);
    for (key, value) in image.attributes {
        attributes.set_item(key, value)?;
    }
    dict.set_item("attributes", attributes)?;

    Ok(dict.into())
}

fn convert_html_inner(html: &str, options: ConversionOptions) -> HtmlResult<String> {
    convert_html(html, Some(options))
}

fn convert_with_images_inner(
    html: &str,
    options: ConversionOptions,
    max_image_size: u64,
) -> HtmlResult<(String, Vec<InlineImage>, Vec<InlineImageWarning>)> {
    let mut config = RustInlineImageConfig::new(max_image_size);
    config.filename_prefix = Some("inline-image".to_string());
    let extraction = convert_with_inline_images(html, Some(options), config)?;
    Ok((extraction.markdown, extraction.inline_images, extraction.warnings))
}

#[pyfunction]
pub fn convert_html_to_markdown(html: &str, options: Option<Bound<'_, PyDict>>) -> PyResult<String> {
    let conversion_options = build_conversion_options(options)?;
    convert_html_inner(html, conversion_options)
        .map_err(|err| PyValueError::new_err(format!("Failed to convert HTML to Markdown: {err}")))
}

#[pyfunction]
#[pyo3(signature = (html, options=None, extract_images=false, max_image_size=10 * 1024 * 1024))]
pub fn process_html(
    py: Python<'_>,
    html: &str,
    options: Option<Bound<'_, PyDict>>,
    extract_images: bool,
    max_image_size: usize,
) -> PyResult<(String, Py<PyList>, Vec<String>)> {
    let conversion_options = build_conversion_options(options)?;

    if extract_images {
        let (markdown, images, warnings) = convert_with_images_inner(html, conversion_options, max_image_size as u64)
            .map_err(|err| {
            PyValueError::new_err(format!("Failed to convert HTML to Markdown with images: {err}"))
        })?;

        let py_images = PyList::empty(py);
        for image in images {
            py_images.append(inline_image_to_py(py, image)?)?;
        }

        let warning_messages = warnings.into_iter().map(|warning| warning.message).collect();

        Ok((markdown, py_images.into(), warning_messages))
    } else {
        let markdown = convert_html_inner(html, conversion_options)
            .map_err(|err| PyValueError::new_err(format!("Failed to convert HTML to Markdown: {err}")))?;
        Ok((markdown, PyList::empty(py).into(), Vec::new()))
    }
}
