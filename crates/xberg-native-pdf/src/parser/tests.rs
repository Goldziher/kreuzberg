use super::*;

#[test]
fn test_parse_null() {
    let input = b"null";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::Null);
}

#[test]
fn test_parse_boolean_true() {
    let input = b"true";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::Boolean(true));
}

#[test]
fn test_parse_boolean_false() {
    let input = b"false";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::Boolean(false));
}

#[test]
fn test_parse_integer() {
    let input = b"42";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::Integer(42));
}

#[test]
fn test_parse_negative_integer() {
    let input = b"-123";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::Integer(-123));
}

#[test]
#[allow(clippy::approx_constant)]
fn test_parse_real() {
    let input = b"3.14";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::Real(3.14));
}

#[test]
fn test_parse_name() {
    let input = b"/Type";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::Name("Type".to_string()));
}

#[test]
fn test_parse_literal_string() {
    let input = b"(Hello World)";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(b"Hello World".to_vec()));
}

#[test]
fn test_parse_empty_literal_string() {
    let input = b"()";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(b"".to_vec()));
}

#[test]
fn test_escape_sequence_newline() {
    let input = b"(Line1\\nLine2)";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(b"Line1\nLine2".to_vec()));
}

#[test]
fn test_escape_sequence_carriage_return() {
    let input = b"(Line1\\rLine2)";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(b"Line1\rLine2".to_vec()));
}

#[test]
fn test_escape_sequence_tab() {
    let input = b"(Col1\\tCol2)";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(b"Col1\tCol2".to_vec()));
}

#[test]
fn test_escape_sequence_backspace() {
    let input = b"(Text\\bmore)";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(b"Text\x08more".to_vec()));
}

#[test]
fn test_escape_sequence_form_feed() {
    let input = b"(Page1\\fPage2)";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(b"Page1\x0CPage2".to_vec()));
}

#[test]
fn test_escape_sequence_parentheses() {
    let input = b"(Open \\( Close \\))";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(b"Open ( Close )".to_vec()));
}

#[test]
fn test_escape_sequence_backslash() {
    let input = b"(Path\\\\to\\\\file)";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(b"Path\\to\\file".to_vec()));
}

#[test]
fn test_escape_sequence_octal_three_digits() {
    // \247 = octal 247 = decimal 167 = 0xA7 = § (section sign) ~keep
    let input = b"(Section \\247)";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(b"Section \xa7".to_vec()));
}

#[test]
fn test_escape_sequence_octal_two_digits() {
    // \53 = octal 53 = decimal 43 = 0x2B = '+' ~keep
    let input = b"(Plus \\53)";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(b"Plus +".to_vec()));
}

#[test]
fn test_escape_sequence_octal_one_digit() {
    // \7 = octal 7 = decimal 7 = bell character ~keep
    let input = b"(Bell \\7)";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(b"Bell \x07".to_vec()));
}

#[test]
fn test_escape_sequence_octal_stops_at_non_octal() {
    // \128 = \12 (octal 12 = 10) + '8' (literal) ~keep
    let input = b"(Value \\128)";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    // \12 = octal 12 = decimal 10 = newline ~keep
    assert_eq!(obj, Object::String(b"Value \n8".to_vec()));
}

#[test]
fn test_escape_sequence_real_pdf_case() {
    // This is the actual case from XYUJKKMUXDLLC6JTCXEWHK5ZMNSTPHF6.pdf
    // \247 = § (section sign), \261 = ± (plus-minus) ~keep
    let input = b"(\\247 71.01\\26115 Temporary certificate.)";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(b"\xa7 71.01\xb115 Temporary certificate.".to_vec()));
}

#[test]
fn test_escape_sequence_line_continuation() {
    let input = b"(This is a long \\\nstring)";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(b"This is a long string".to_vec()));
}

#[test]
fn test_escape_sequence_mixed() {
    let input = b"(Tab:\\tNewline:\\nOctal:\\53)";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(b"Tab:\tNewline:\nOctal:+".to_vec()));
}

#[test]
fn test_decode_literal_string_escapes_directly() {
    assert_eq!(decode_literal_string_escapes(b"Hello"), b"Hello");
    assert_eq!(decode_literal_string_escapes(b"\\n"), b"\n");
    assert_eq!(decode_literal_string_escapes(b"\\247"), b"\xa7");
    assert_eq!(decode_literal_string_escapes(b"\\(\\)"), b"()");
    assert_eq!(decode_literal_string_escapes(b"\\\\"), b"\\");
}

#[test]
fn test_parse_hex_string() {
    let input = b"<48656C6C6F>";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(b"Hello".to_vec()));
}

#[test]
fn test_parse_hex_string_with_whitespace() {
    let input = b"<48 65 6C 6C 6F>";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(b"Hello".to_vec()));
}

#[test]
fn test_parse_empty_hex_string() {
    let input = b"<>";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(b"".to_vec()));
}

#[test]
fn test_parse_hex_string_odd_length() {
    let input = b"<ABC>";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::String(vec![0xAB, 0xC0]));
}

#[test]
fn test_decode_hex() {
    let result = decode_hex(b"48656C6C6F").unwrap();
    assert_eq!(result, b"Hello");
}

#[test]
fn test_decode_hex_with_whitespace() {
    let result = decode_hex(b"48 65 6C 6C 6F").unwrap();
    assert_eq!(result, b"Hello");
}

#[test]
fn test_decode_hex_empty() {
    let result = decode_hex(b"").unwrap();
    assert_eq!(result, b"");
}

#[test]
fn test_decode_hex_odd_length() {
    let result = decode_hex(b"ABC").unwrap();
    assert_eq!(result, vec![0xAB, 0xC0]);
}

#[test]
fn test_parse_indirect_reference() {
    let input = b"10 0 R";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::Reference(ObjectRef::new(10, 0)));
}

#[test]
fn test_parse_indirect_reference_with_generation() {
    let input = b"42 5 R";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::Reference(ObjectRef::new(42, 5)));
}

#[test]
fn test_parse_integer_not_reference() {
    let input = b"10";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::Integer(10));
}

#[test]
fn test_parse_empty_array() {
    let input = b"[]";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::Array(vec![]));
}

#[test]
fn test_parse_array_with_integers() {
    let input = b"[ 1 2 3 ]";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(
        obj,
        Object::Array(vec![Object::Integer(1), Object::Integer(2), Object::Integer(3),])
    );
}

#[test]
fn test_parse_array_mixed_types() {
    let input = b"[ 1 /Name (string) true ]";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(
        obj,
        Object::Array(vec![
            Object::Integer(1),
            Object::Name("Name".to_string()),
            Object::String(b"string".to_vec()),
            Object::Boolean(true),
        ])
    );
}

#[test]
fn test_parse_nested_arrays() {
    let input = b"[ 1 [ 2 3 ] 4 ]";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(
        obj,
        Object::Array(vec![
            Object::Integer(1),
            Object::Array(vec![Object::Integer(2), Object::Integer(3)]),
            Object::Integer(4),
        ])
    );
}

#[test]
fn test_parse_array_with_references() {
    let input = b"[ 10 0 R 20 0 R ]";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(
        obj,
        Object::Array(vec![
            Object::Reference(ObjectRef::new(10, 0)),
            Object::Reference(ObjectRef::new(20, 0)),
        ])
    );
}

#[test]
fn test_parse_empty_dictionary() {
    let input = b"<<>>";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::Dictionary(HashMap::new()));
}

#[test]
fn test_parse_dictionary_single_entry() {
    let input = b"<< /Type /Page >>";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);

    let dict = obj.as_dict().unwrap();
    assert_eq!(dict.len(), 1);
    assert_eq!(dict.get("Type").unwrap().as_name(), Some("Page"));
}

#[test]
fn test_parse_dictionary_multiple_entries() {
    let input = b"<< /Type /Page /Count 3 /Title (My Page) >>";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);

    let dict = obj.as_dict().unwrap();
    assert_eq!(dict.len(), 3);
    assert_eq!(dict.get("Type").unwrap().as_name(), Some("Page"));
    assert_eq!(dict.get("Count").unwrap().as_integer(), Some(3));
    assert_eq!(dict.get("Title").unwrap().as_string(), Some(&b"My Page"[..]));
}

#[test]
fn test_parse_dictionary_with_array() {
    let input = b"<< /MediaBox [ 0 0 612 792 ] >>";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);

    let dict = obj.as_dict().unwrap();
    assert_eq!(dict.len(), 1);
    let media_box = dict.get("MediaBox").unwrap().as_array().unwrap();
    assert_eq!(media_box.len(), 4);
}

#[test]
fn test_parse_nested_dictionaries() {
    let input = b"<< /Outer << /Inner /Value >> >>";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);

    let dict = obj.as_dict().unwrap();
    let inner = dict.get("Outer").unwrap().as_dict().unwrap();
    assert_eq!(inner.get("Inner").unwrap().as_name(), Some("Value"));
}

#[test]
fn test_parse_dictionary_with_reference() {
    let input = b"<< /Pages 2 0 R >>";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);

    let dict = obj.as_dict().unwrap();
    assert_eq!(dict.get("Pages").unwrap().as_reference(), Some(ObjectRef::new(2, 0)));
}

#[test]
fn test_parse_complex_nested_structure() {
    let input = b"<< /Type /Catalog /Pages [ 1 0 R 2 0 R ] /Metadata << /Author (John) >> >>";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);

    let dict = obj.as_dict().unwrap();
    assert_eq!(dict.get("Type").unwrap().as_name(), Some("Catalog"));

    let pages = dict.get("Pages").unwrap().as_array().unwrap();
    assert_eq!(pages.len(), 2);

    let metadata = dict.get("Metadata").unwrap().as_dict().unwrap();
    assert_eq!(metadata.get("Author").unwrap().as_string(), Some(&b"John"[..]));
}

#[test]
fn test_parse_unclosed_array() {
    // Lenient parsing: unclosed arrays return what they have ~keep
    let input = b"[ 1 2 3";
    let result = parse_object(input);
    assert!(result.is_ok());
    let (_, obj) = result.unwrap();
    let arr = obj.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0].as_integer(), Some(1));
    assert_eq!(arr[1].as_integer(), Some(2));
    assert_eq!(arr[2].as_integer(), Some(3));
}

#[test]
fn test_parse_unclosed_dictionary() {
    // Lenient parsing: unclosed dictionaries return what they have ~keep
    let input = b"<< /Type /Page";
    let result = parse_object(input);
    assert!(result.is_ok());
    let (_, obj) = result.unwrap();
    let dict = obj.as_dict().unwrap();
    assert_eq!(dict.get("Type").and_then(|o| o.as_name()), Some("Page"));
}

#[test]
fn test_parse_dictionary_missing_value() {
    let input = b"<< /Type >>";
    let result = parse_object(input);
    assert!(result.is_err());
}

#[test]
fn test_parse_dictionary_non_name_key() {
    let input = b"<< 123 /Value >>";
    let result = parse_object(input);
    assert!(result.is_err());
}

#[test]
fn test_parse_with_leading_whitespace() {
    let input = b"  \n\t  42";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    assert_eq!(obj, Object::Integer(42));
}

#[test]
fn test_parse_array_with_extra_whitespace() {
    let input = b"[  1   2    3  ]";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    let arr = obj.as_array().unwrap();
    assert_eq!(arr.len(), 3);
}

#[test]
fn test_parse_dictionary_with_extra_whitespace() {
    let input = b"<<  /Type   /Page  >>";
    let (remaining, obj) = parse_object(input).unwrap();
    assert_eq!(remaining, &b""[..]);
    let dict = obj.as_dict().unwrap();
    assert_eq!(dict.get("Type").unwrap().as_name(), Some("Page"));
}
