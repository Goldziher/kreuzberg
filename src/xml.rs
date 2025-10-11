use pyo3::prelude::*;
use quick_xml::Reader;
use quick_xml::events::Event;
use std::collections::HashSet;

#[pyclass]
pub struct XmlExtractionResult {
    #[pyo3(get)]
    pub content: String,
    #[pyo3(get)]
    pub element_count: usize,
    #[pyo3(get)]
    pub unique_elements: Vec<String>,
}

#[pymethods]
impl XmlExtractionResult {
    fn __repr__(&self) -> String {
        format!(
            "XmlExtractionResult(content_len={}, element_count={}, unique_elements={})",
            self.content.len(),
            self.element_count,
            self.unique_elements.len()
        )
    }
}

#[pyfunction]
pub fn parse_xml(py: Python<'_>, xml_bytes: &[u8], preserve_whitespace: bool) -> PyResult<XmlExtractionResult> {
    py.detach(|| parse_xml_internal(xml_bytes, preserve_whitespace))
}

fn parse_xml_internal(xml_bytes: &[u8], preserve_whitespace: bool) -> PyResult<XmlExtractionResult> {
    let mut reader = Reader::from_reader(xml_bytes);
    reader.config_mut().trim_text(!preserve_whitespace);
    reader.config_mut().check_end_names = false;

    let mut content = String::new();
    let mut element_count = 0usize;
    let mut unique_elements_set = HashSet::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if let Ok(name) = std::str::from_utf8(e.name().as_ref()) {
                    element_count += 1;
                    unique_elements_set.insert(name.to_string());
                }
            }
            Ok(Event::Text(e)) => match e.unescape() {
                Ok(text) => {
                    let text_str = text.as_ref();
                    if preserve_whitespace {
                        content.push_str(text_str);
                        content.push(' ');
                    } else {
                        let trimmed = text_str.trim();
                        if !trimmed.is_empty() {
                            content.push_str(trimmed);
                            content.push(' ');
                        }
                    }
                }
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Failed to unescape XML text: {}",
                        e
                    )));
                }
            },
            Ok(Event::CData(e)) => match std::str::from_utf8(&e) {
                Ok(text) => {
                    content.push_str(text);
                    content.push(' ');
                }
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Invalid UTF-8 in CDATA: {}",
                        e
                    )));
                }
            },
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "XML parsing error at position {}: {}",
                    reader.buffer_position(),
                    e
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    let content = content.trim_end().to_string();
    let mut unique_elements: Vec<String> = unique_elements_set.into_iter().collect();
    unique_elements.sort();

    Ok(XmlExtractionResult {
        content,
        element_count,
        unique_elements,
    })
}

pub fn register_xml_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_xml, m)?)?;
    m.add_class::<XmlExtractionResult>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_xml() {
        let xml = b"<root><item>Hello</item><item>World</item></root>";
        let result = parse_xml_internal(xml, false).unwrap();
        assert_eq!(result.content, "Hello World");
        assert_eq!(result.element_count, 3);
        assert!(result.unique_elements.contains(&"root".to_string()));
        assert!(result.unique_elements.contains(&"item".to_string()));
        assert_eq!(result.unique_elements.len(), 2);
    }

    #[test]
    fn test_xml_with_cdata() {
        let xml = b"<root><![CDATA[Special <characters> & data]]></root>";
        let result = parse_xml_internal(xml, false).unwrap();
        assert!(result.content.contains("Special <characters> & data"));
        assert_eq!(result.element_count, 1);
    }

    #[test]
    fn test_malformed_xml_lenient() {
        let xml = b"<root><item>Unclosed<item2>Content</root>";
        let result = parse_xml_internal(xml, false).unwrap();
        assert!(!result.content.is_empty());
        assert!(result.content.contains("Content"));
    }

    #[test]
    fn test_empty_xml() {
        let xml = b"<root></root>";
        let result = parse_xml_internal(xml, false).unwrap();
        assert_eq!(result.content, "");
        assert_eq!(result.element_count, 1);
        assert_eq!(result.unique_elements.len(), 1);
    }

    #[test]
    fn test_whitespace_handling() {
        let xml = b"<root>  <item>  Text  </item>  </root>";
        let result = parse_xml_internal(xml, false).unwrap();
        assert_eq!(result.content, "Text");
    }

    #[test]
    fn test_preserve_whitespace() {
        let xml = b"<root>  Text with   spaces  </root>";
        let result_trimmed = parse_xml_internal(xml, false).unwrap();
        let result_preserved = parse_xml_internal(xml, true).unwrap();
        assert_eq!(result_trimmed.content.trim(), "Text with   spaces");
        assert!(result_preserved.content.len() >= result_trimmed.content.len());
    }

    #[test]
    fn test_element_counting() {
        let xml = b"<root><a/><b/><c/><b/><d/></root>";
        let result = parse_xml_internal(xml, false).unwrap();
        assert_eq!(result.element_count, 6);
        assert_eq!(result.unique_elements.len(), 5);
        assert!(result.unique_elements.contains(&"b".to_string()));
    }
}
