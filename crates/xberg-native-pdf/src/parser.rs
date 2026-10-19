//! PDF object parser.
//!
//! This module provides parsing of PDF objects by combining tokens from the lexer
//! into complete objects (arrays, dictionaries, indirect references, etc.).
//!
//! # Architecture
//!
//! The parser uses a recursive descent approach:
//! 1. Read token from lexer
//! 2. Based on token type, decide how to parse
//! 3. For composite types (arrays, dicts), recursively parse contents
//!
//! # Error Handling
//!
//! All parsing functions return `IResult` from nom. Parse errors contain
//! descriptive messages about what went wrong and where.

use crate::error::{Error, Result};
use crate::lexer::{Token, token, token_lenient};
use crate::object::{Object, ObjectRef};
use nom::IResult;
use std::collections::HashMap;

/// Maximum object nesting depth for `parse_object` <-> `parse_array` /
/// `parse_dictionary` mutual recursion.
///
/// Without this bound, an object body such as `[[[[[...` or
/// `<</A<</A<</A...` with tens of thousands of opening brackets recurses
/// `parse_object` -> `parse_array`/`parse_dictionary` -> `parse_object` with
/// no limit, which overflows the call stack. A stack overflow is a
/// SIGSEGV/abort, not a catchable panic, so it takes down the whole host
/// process — no `catch_unwind` at any extractor boundary can contain it.
///
/// NOTE: `ParserOptions::max_nesting` and `ParserOptions::max_recursion_depth`
/// (see `parser_config.rs`) look like the natural home for this bound (both
/// default to 100, same as this constant), but neither `parse_object`,
/// `parse_array`, nor `parse_dictionary` takes a `ParserOptions` parameter.
/// Threading one through would change `parse_object`'s public signature,
/// which is called from `objstm.rs`, `xref.rs`, and `document.rs` outside
/// this module — out of scope for this fix. Those two config fields remain
/// dead; do not assume changing them affects this guard.
const MAX_OBJECT_NESTING_DEPTH: usize = 100;

/// Decode escape sequences in PDF literal strings.
///
/// PDF literal strings (enclosed in parentheses) support escape sequences
/// per ISO 32000-1:2008, Section 7.3.4.2:
///
/// - `\n` → Line Feed (0x0A)
/// - `\r` → Carriage Return (0x0D)
/// - `\t` → Horizontal Tab (0x09)
/// - `\b` → Backspace (0x08)
/// - `\f` → Form Feed (0x0C)
/// - `\(` → Left Parenthesis
/// - `\)` → Right Parenthesis
/// - `\\` → Backslash
/// - `\ddd` → Character with octal code (1-3 digits)
/// - `\<newline>` → Line continuation (ignored)
///
/// # Examples
///
/// ```ignore
/// # // `parser` is a crate-internal module (see `src/lib.rs`), so this
/// # // example is illustrative only and does not compile as a doctest.
/// # use xberg_native_pdf::parser::decode_literal_string_escapes;
/// let input = b"Section \\247 71.01";
/// let decoded = decode_literal_string_escapes(input);
/// assert_eq!(decoded, b"Section \xa7 71.01"); // \247 = § (section sign)
/// ```
pub fn decode_literal_string_escapes(raw: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(raw.len());
    let mut i = 0;

    while i < raw.len() {
        if raw[i] == b'\\' && i + 1 < raw.len() {
            match raw[i + 1] {
                b'n' => {
                    result.push(b'\n');
                    i += 2;
                }
                b'r' => {
                    result.push(b'\r');
                    i += 2;
                }
                b't' => {
                    result.push(b'\t');
                    i += 2;
                }
                b'b' => {
                    result.push(8);
                    i += 2;
                }
                b'f' => {
                    result.push(12);
                    i += 2;
                }
                b'(' => {
                    result.push(b'(');
                    i += 2;
                }
                b')' => {
                    result.push(b')');
                    i += 2;
                }
                b'\\' => {
                    result.push(b'\\');
                    i += 2;
                }
                // Line continuation: \<newline> is ignored ~keep
                b'\n' => {
                    i += 2;
                }
                b'\r' => {
                    i += 2;
                    if i < raw.len() && raw[i] == b'\n' {
                        i += 1;
                    }
                }
                // Octal escape: \ddd (1-3 octal digits) ~keep
                c if c.is_ascii_digit() && c < b'8' => {
                    let start = i + 1;
                    let mut octal_value = 0u32;
                    let mut octal_len = 0;

                    for j in 0..3 {
                        if start + j < raw.len() {
                            let digit = raw[start + j];
                            if (b'0'..b'8').contains(&digit) {
                                octal_value = octal_value * 8 + (digit - b'0') as u32;
                                octal_len += 1;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if octal_len > 0 {
                        // Octal value must fit in a byte ~keep
                        result.push((octal_value & 0xFF) as u8);
                        i += 1 + octal_len;
                    } else {
                        result.push(b'\\');
                        i += 1;
                    }
                }
                // Unknown escape: keep backslash literal (PDF spec allows this) ~keep
                _ => {
                    result.push(b'\\');
                    i += 1;
                }
            }
        } else {
            result.push(raw[i]);
            i += 1;
        }
    }

    result
}

/// Parse a PDF object from input bytes.
///
/// This is the main entry point for parsing PDF objects. It handles all
/// PDF object types:
/// - Primitives: null, boolean, integer, real, string, name
/// - Composites: array, dictionary
/// - References: indirect object references (10 0 R)
///
/// # Example
///
/// ```ignore
/// // `parser` is a crate-internal module (see `src/lib.rs`), so this
/// // example is illustrative only and does not compile as a doctest.
/// use xberg_native_pdf::parser::parse_object;
///
/// let input = b"[ 1 2 /Name ]";
/// let (remaining, obj) = parse_object(input).unwrap();
/// ```
///
/// # Errors
///
/// Returns `Err` if:
/// - Input is not a valid PDF object
/// - Nested structures are malformed (unclosed arrays/dicts)
/// - Hex strings contain invalid hex digits
pub fn parse_object(input: &[u8]) -> IResult<&[u8], Object> {
    parse_object_at_depth(input, 0)
}

/// Depth-tracking implementation behind [`parse_object`].
///
/// `depth` counts container nesting: it is 0 for a top-level object and
/// increases by 1 each time `parse_array`/`parse_dictionary` recurse back
/// into this function for an element/value. See [`MAX_OBJECT_NESTING_DEPTH`]
/// for why this exists.
fn parse_object_at_depth(input: &[u8], depth: usize) -> IResult<&[u8], Object> {
    let (input, tok) = token(input)?;

    match tok {
        Token::Null => Ok((input, Object::Null)),
        Token::True => Ok((input, Object::Boolean(true))),
        Token::False => Ok((input, Object::Boolean(false))),

        Token::Integer(i) => {
            // Could be a plain integer OR the start of an indirect reference (obj_num gen R)
            // Try to parse as reference first ~keep

            if let Ok((input2, Token::Integer(generation))) = token(input)
                && let Ok((input3, Token::R)) = token(input2)
            {
                return Ok((input3, Object::Reference(ObjectRef::new(i as u32, generation as u16))));
            }

            Ok((input, Object::Integer(i)))
        }

        Token::Real(r) => Ok((input, Object::Real(r))),

        Token::LiteralString(bytes) => {
            // Decode escape sequences per ISO 32000-1:2008, Section 7.3.4.2 ~keep
            let decoded = decode_literal_string_escapes(bytes);
            Ok((input, Object::String(decoded)))
        }

        Token::HexString(hex_bytes) => match decode_hex(hex_bytes) {
            Ok(decoded) => Ok((input, Object::String(decoded))),
            Err(_) => Err(nom::Err::Failure(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Fail,
            ))),
        },

        Token::Name(name) => Ok((input, Object::Name(name))),

        Token::ArrayStart => parse_array(input, depth),

        Token::DictStart => {
            let (remaining, dict_obj) = parse_dictionary(input, depth)?;

            if let Ok((stream_input, Token::StreamStart)) = token(remaining) {
                let dict = match dict_obj {
                    Object::Dictionary(d) => d,
                    _ => {
                        // parse_dictionary guarantees Dictionary return type
                        // This should never happen, but handle gracefully if it does ~keep
                        return Err(nom::Err::Error(nom::error::Error::new(
                            input,
                            nom::error::ErrorKind::Tag,
                        )));
                    }
                };

                let (final_input, stream_data) = parse_stream_data(stream_input, &dict)?;

                return Ok((
                    final_input,
                    Object::Stream {
                        dict,
                        data: bytes::Bytes::from(stream_data),
                    },
                ));
            }

            Ok((remaining, dict_obj))
        }

        _ => Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        ))),
    }
}

/// Parse stream data after the `stream` keyword.
///
/// Stream data starts after a newline following `stream` and ends with `endstream`.
/// The Length entry in the dictionary tells us how many bytes to read.
///
/// PDF Spec: ISO 32000-1:2008, Section 7.3.8.1 - Stream Objects
/// The keyword stream must be followed by either a CRLF or LF sequence, but not CR alone.
fn parse_stream_data<'a>(input: &'a [u8], dict: &HashMap<String, Object>) -> IResult<&'a [u8], Vec<u8>> {
    // SPEC COMPLIANCE: PDF Spec ISO 32000-1:2008, Section 7.3.8.1 states that
    // the 'stream' keyword must be followed by either CRLF or LF, but NOT CR alone.
    //
    // We accept CR alone in lenient mode for compatibility with malformed PDFs,
    // but log a warning. In strict mode, this should be an error (requires ParserOptions). ~keep

    let input = if input.starts_with(b"\r\n") {
        &input[2..]
    } else if input.starts_with(b"\n") {
        &input[1..]
    } else if input.starts_with(b"\r") {
        let msg = "SPEC VIOLATION: Stream keyword followed by CR alone (should be CRLF or LF). \
            Accepting in lenient mode for compatibility. \
            PDF Spec: ISO 32000-1:2008, Section 7.3.8.1";
        tracing::warn!("{}", msg);
        // push into the process-wide structured sink
        // so callers can retrieve via `flatten_warnings()` instead of
        // parsing stderr from `tracing::warn`. ~keep
        crate::extractors::warnings::push_global_warning(crate::extractors::warnings::Warning {
            category: crate::extractors::warnings::WarningCategory::SpecViolation,
            page: None,
            message: msg.to_string(),
            spec_section: Some("7.3.8.1"),
        });
        &input[1..]
    } else {
        let msg = "SPEC VIOLATION: No newline after stream keyword (should be CRLF or LF). \
            PDF Spec: ISO 32000-1:2008, Section 7.3.8.1";
        tracing::warn!("{}", msg);
        crate::extractors::warnings::push_global_warning(crate::extractors::warnings::Warning {
            category: crate::extractors::warnings::WarningCategory::SpecViolation,
            page: None,
            message: msg.to_string(),
            spec_section: Some("7.3.8.1"),
        });
        input
    };

    if let Some(length_obj) = dict.get("Length")
        && let Some(length) = length_obj.as_integer()
    {
        let length = length as usize;
        if length <= input.len() {
            let stream_data = input[..length].to_vec();
            let remaining = &input[length..];

            let ws_result: nom::IResult<&[u8], &[u8]> =
                nom::bytes::complete::take_while(|c: u8| c.is_ascii_whitespace())(remaining);
            if let Ok((remaining, _)) = ws_result
                && let Ok((remaining, _)) = token(remaining)
            {
                return Ok((remaining, stream_data));
            }
        }

        // /Length was wrong — fall back to scanning for 'endstream' ~keep
        let msg = format!("Stream /Length {} is incorrect, falling back to endstream scan", length);
        tracing::warn!("{}", msg);
        crate::extractors::warnings::push_global_warning(crate::extractors::warnings::Warning {
            category: crate::extractors::warnings::WarningCategory::SpecViolation,
            page: None,
            message: msg,
            spec_section: Some("7.3.8.2"),
        });
        if let Some(pos) = find_endstream(input) {
            // Strip trailing CR/LF per PDF spec Section 7.3.8.1 ~keep
            let mut end = pos;
            if end > 0 && input[end - 1] == b'\n' {
                end -= 1;
            }
            if end > 0 && input[end - 1] == b'\r' {
                end -= 1;
            }
            let stream_data = input[..end].to_vec();
            let remaining = &input[pos..];
            let (remaining, _) = token(remaining)?;
            return Ok((remaining, stream_data));
        }
    }

    // SPEC DEVIATION (LOW PRIORITY): PDF Spec ISO 32000-1:2008, Section 7.3.8.1
    // requires stream dictionaries to have a /Length entry. This fallback scans for
    // 'endstream' keyword when /Length is missing or invalid.
    //
    // Rationale: Many malformed PDFs in the wild lack correct /Length values.
    // This heuristic makes parsing more robust at the cost of spec compliance.
    //
    // Proper implementation: Only use this fallback in lenient mode (ParserOptions).
    // In strict mode, should fail with error when /Length is missing/invalid.
    //
    // If no Length or invalid Length, scan for 'endstream' keyword
    // This is less reliable but acts as fallback ~keep
    if let Some(pos) = find_endstream(input) {
        // Strip trailing CR/LF per PDF spec Section 7.3.8.1 ~keep
        let mut end = pos;
        if end > 0 && input[end - 1] == b'\n' {
            end -= 1;
        }
        if end > 0 && input[end - 1] == b'\r' {
            end -= 1;
        }
        let stream_data = input[..end].to_vec();
        let remaining = &input[pos..];

        let (remaining, _) = token(remaining)?;

        return Ok((remaining, stream_data));
    }

    Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::Eof,
    )))
}

/// Find the position of 'endstream' keyword in input.
fn find_endstream(input: &[u8]) -> Option<usize> {
    let keyword = b"endstream";
    input.windows(keyword.len()).position(|window| window == keyword)
}

/// Parse a PDF array: `[ obj1 obj2 ... objN ]`
///
/// Arrays can contain any PDF objects, including nested arrays and dictionaries.
/// Empty arrays are valid: `[]`
///
/// # Example
///
/// ```ignore
/// // `parser` is a crate-internal module (see `src/lib.rs`), so this
/// // example is illustrative only and does not compile as a doctest.
/// use xberg_native_pdf::parser::parse_object;
///
/// let input = b"[ 1 2 /Name (string) [ 3 4 ] ]";
/// let (_, obj) = parse_object(input).unwrap();
/// assert!(obj.as_array().is_some());
/// ```
///
/// # Errors
///
/// Returns `Err` if:
/// - Array is not properly closed with `]`
/// - Array contains malformed objects
/// - Nesting depth exceeds [`MAX_OBJECT_NESTING_DEPTH`]
fn parse_array(input: &[u8], depth: usize) -> IResult<&[u8], Object> {
    if depth >= MAX_OBJECT_NESTING_DEPTH {
        return Err(nom::Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::TooLarge,
        )));
    }

    let mut objects = Vec::new();
    let mut remaining = input;

    loop {
        let token_result = token(remaining);

        match token_result {
            Ok((inp, tok)) => {
                if tok == Token::ArrayEnd {
                    return Ok((inp, Object::Array(objects)));
                }

                // Otherwise, we need to parse this as an object
                // Put the token back by re-parsing from remaining ~keep
                match parse_object_at_depth(remaining, depth + 1) {
                    Ok((inp, obj)) => {
                        objects.push(obj);
                        remaining = inp;
                    }
                    Err(e) => {
                        if remaining.is_empty() {
                            // Unclosed array, but return what we have ~keep
                            return Ok((remaining, Object::Array(objects)));
                        }
                        return Err(e);
                    }
                }
            }
            Err(nom::Err::Incomplete(_)) | Err(nom::Err::Error(_)) if remaining.is_empty() => {
                // Hit EOF before closing array - return what we have ~keep
                return Ok((remaining, Object::Array(objects)));
            }
            Err(e) => return Err(e),
        }
    }
}

/// Parse a PDF dictionary: `<< /Key1 value1 /Key2 value2 ... >>`
///
/// Dictionary keys must be names (starting with /). Values can be any PDF object.
/// Empty dictionaries are valid: `<< >>`
///
/// # Example
///
/// ```ignore
/// // `parser` is a crate-internal module (see `src/lib.rs`), so this
/// // example is illustrative only and does not compile as a doctest.
/// use xberg_native_pdf::parser::parse_object;
///
/// let input = b"<< /Type /Page /Count 3 >>";
/// let (_, obj) = parse_object(input).unwrap();
/// assert!(obj.as_dict().is_some());
/// ```
///
/// # Errors
///
/// Returns `Err` if:
/// - Dictionary is not properly closed with `>>`
/// - A key is not a name
/// - Values are malformed
/// - Nesting depth exceeds [`MAX_OBJECT_NESTING_DEPTH`]
fn parse_dictionary(input: &[u8], depth: usize) -> IResult<&[u8], Object> {
    if depth >= MAX_OBJECT_NESTING_DEPTH {
        return Err(nom::Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::TooLarge,
        )));
    }

    let mut dict = HashMap::new();
    let mut remaining = input;

    loop {
        let token_result = token(remaining);

        match token_result {
            Ok((inp, tok)) => {
                if tok == Token::DictEnd {
                    return Ok((inp, Object::Dictionary(dict)));
                }

                match tok {
                    Token::Name(key) => {
                        // Parse the value; fall back to lenient token for bare words
                        // (e.g. `OBJR` without a `/` prefix in malformed PDFs) ~keep
                        let value_result = parse_object_at_depth(inp, depth + 1).or_else(|_| {
                            token_lenient(inp).and_then(|(inp2, tok)| {
                                if let Token::Name(name) = tok {
                                    Ok((inp2, Object::Name(name)))
                                } else {
                                    Err(nom::Err::Error(nom::error::Error::new(inp, nom::error::ErrorKind::Alt)))
                                }
                            })
                        });
                        match value_result {
                            Ok((inp, value)) => {
                                dict.insert(key, value);
                                remaining = inp;
                            }
                            Err(e) => {
                                if inp.is_empty() {
                                    // Incomplete dictionary, return what we have ~keep
                                    return Ok((inp, Object::Dictionary(dict)));
                                }
                                return Err(e);
                            }
                        }
                    }
                    _ => {
                        // Invalid dictionary - key must be a name
                        // But if we hit EOF, return what we have ~keep
                        if remaining.is_empty() {
                            return Ok((remaining, Object::Dictionary(dict)));
                        }
                        return Err(nom::Err::Error(nom::error::Error::new(
                            remaining,
                            nom::error::ErrorKind::Tag,
                        )));
                    }
                }
            }
            Err(nom::Err::Incomplete(_)) | Err(nom::Err::Error(_)) if remaining.is_empty() => {
                // Hit EOF before closing dictionary - return what we have ~keep
                return Ok((remaining, Object::Dictionary(dict)));
            }
            Err(e) => return Err(e),
        }
    }
}

/// Decode a hex string to bytes.
///
/// PDF hex strings contain pairs of hexadecimal digits representing bytes.
/// Whitespace is ignored. If there's an odd number of hex digits, the last
/// digit is padded with 0.
///
/// # Example
///
/// ```ignore
/// // `parser` is a crate-internal module (see `src/lib.rs`), so this
/// // example is illustrative only and does not compile as a doctest.
/// use xberg_native_pdf::parser::decode_hex;
///
/// let decoded = decode_hex(b"48656C6C6F").unwrap();
/// assert_eq!(decoded, b"Hello");
/// ```
///
/// # Errors
///
/// Returns `Err` if:
/// - Input contains non-hex, non-whitespace characters
/// - Hex digit parsing fails
pub fn decode_hex(hex_bytes: &[u8]) -> Result<Vec<u8>> {
    let hex_str: Vec<u8> = hex_bytes
        .iter()
        .filter(|&&c| !c.is_ascii_whitespace())
        .copied()
        .collect();

    if hex_str.is_empty() {
        return Ok(Vec::new());
    }

    // Per PDF spec ISO 32000-1:2008 §7.3.4.3: non-hex characters are treated as 0 ~keep
    fn hex_val(b: u8) -> u8 {
        match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => 0,
        }
    }

    let mut result = Vec::with_capacity(hex_str.len() / 2 + 1);

    for chunk in hex_str.chunks(2) {
        match chunk.len() {
            2 => {
                let byte = (hex_val(chunk[0]) << 4) | hex_val(chunk[1]);
                result.push(byte);
            }
            1 => {
                // Odd number of hex digits - pad last digit with 0 ~keep
                let byte = hex_val(chunk[0]) << 4;
                result.push(byte);
            }
            _ => {
                // chunks(2) guarantees max 2 elements, this should never execute ~keep
                return Err(Error::ParseError {
                    offset: 0,
                    reason: "Invalid hex string chunk size".to_string(),
                });
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests;
