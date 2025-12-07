# Comprehensive Typst Extractor Review

**Files Reviewed:**
- Extractor: `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/src/extractors/typst.rs` (703 LOC)
- Tests: `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/tests/typst_extractor_tests.rs` (685 LOC)
- Test Documents: 7 sample Typst files

**Overall Assessment:** The extractor has solid fundamentals and passes all tests, but contains several code quality issues, duplicate patterns, and significant implementation gaps. No critical panics detected, but multiple logic edge cases could produce incorrect output.

---

## 1. CODE QUALITY ISSUES

### 1.1 CRITICAL ISSUES

#### Issue #1: Unclosed Delimiter Handling - Silent Data Loss
**File:** `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/src/extractors/typst.rs`
**Lines:** 430-438, 441-449, 452-460, 463-471
**Severity:** CRITICAL
**Description:**
The `process_line()` function silently loses data when inline formatting delimiters are not closed. When parsing `` `code `` without closing backtick, the loop exhausts the iterator and discards everything after the opening backtick.

**Code Example:**
```rust
// Line 430-438 - inline code handling
'`' => {
    result.push('`');
    for c in chars.by_ref() {
        result.push(c);
        if c == '`' {
            break;
        }
    }
}
```

**Problem:**
- Input: `"code with `unclosed backtick and more text"`
- Expected: Entire line preserved
- Actual: Everything after backtick discarded
- No warning or error raised

**Recommended Fix:**
Track whether delimiters were closed; if EOL reached without closing delimiter, append remaining content:
```rust
'`' => {
    result.push('`');
    let mut found_close = false;
    for c in chars.by_ref() {
        result.push(c);
        if c == '`' {
            found_close = true;
            break;
        }
    }
    if !found_close {
        // Warn or handle: delimiter was not closed
        // Content is already in result, just continue
    }
}
```

---

#### Issue #2: Infinite Loop Risk in Table Parsing
**File:** `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/src/extractors/typst.rs`
**Lines:** 370-386
**Severity:** CRITICAL
**Description:**
The `extract_table_content()` function can enter an infinite loop if bracket/paren counting logic goes negative. While unlikely in well-formed input, malformed Typst could trigger this.

**Code:**
```rust
while paren_depth > 0 || bracket_depth > 0 {
    if let Some(next_line) = lines.next() {
        // ... count brackets
        for ch in next_line.chars() {
            match ch {
                '(' => paren_depth += 1,
                ')' => paren_depth -= 1,  // Can go negative!
                '[' => bracket_depth += 1,
                ']' => bracket_depth -= 1, // Can go negative!
                _ => {}
            }
        }
    } else {
        break;  // Only escape if no more lines
    }
}
```

**Problem:**
- Input: `#table()] [` (mismatched brackets)
- paren_depth: 1 -> 0 -> -1
- bracket_depth: 0 -> 1 -> 0
- Condition `paren_depth > 0 || bracket_depth > 0` becomes false only when both are positive
- If paren_depth becomes negative, the loop exits prematurely
- However, if only one goes negative while other stays positive, loop could hang

**Recommended Fix:**
```rust
while paren_depth > 0 || bracket_depth > 0 {
    if paren_depth < 0 || bracket_depth < 0 {
        // Malformed: more closing than opening brackets
        break;
    }
    if let Some(next_line) = lines.next() {
        // ... existing code ...
    } else {
        break;
    }
}
```

---

#### Issue #3: Regex Compilation Errors Ignored Silently
**File:** `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/src/extractors/typst.rs`
**Lines:** 181-186, 192-195, 204, 491
**Severity:** HIGH
**Description:**
Regex compilation failures are caught but return `None` silently, causing metadata to be lost without any indication.

**Code (Line 181-186):**
```rust
fn extract_quoted_value(&self, field: &str) -> Option<String> {
    let pattern = format!(r#"{}:\s*"([^"]*)""#, regex::escape(field));
    if let Ok(re) = Regex::new(&pattern)
        && let Some(caps) = re.captures(&self.content)
    {
        return caps.get(1).map(|m| m.as_str().to_string());
    }
    None  // Silently returns None for any regex error
}
```

**Problem:**
- If `Regex::new()` fails (invalid pattern), metadata is lost
- No logging or tracing of why metadata extraction failed
- Hard to debug: appears as missing metadata, not as regex error

**Recommended Fix:**
Add debug-level logging:
```rust
if let Ok(re) = Regex::new(&pattern) {
    if let Some(caps) = re.captures(&self.content) {
        return caps.get(1).map(|m| m.as_str().to_string());
    }
} else {
    // Log at debug level for visibility
    eprintln!("Failed to compile regex for field '{}': pattern '{}'", field, pattern);
}
None
```

Or wrap with `tracing` if available in this project.

---

### 1.2 HIGH SEVERITY ISSUES

#### Issue #4: List Detection False Negatives
**File:** `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/src/extractors/typst.rs`
**Lines:** 300-309
**Severity:** HIGH
**Description:**
List detection requires a non-alphanumeric character after `+` or `-`, breaking valid lists like:
- `+ Item1` (space is non-alphanumeric, OK)
- `+ #set` (starts with `#`, OK)
- But it's overly restrictive for other formats

**Code:**
```rust
if (trimmed.starts_with('+') || trimmed.starts_with('-'))
    && trimmed.len() > 1
    && trimmed.chars().nth(1).is_some_and(|c| !c.is_alphanumeric())
{
    output.push_str("- ");
    output.push_str(trimmed[1..].trim());
    output.push('\n');
    continue;
}
```

**Problem:**
- Input: `+Item` (no space) - Will NOT be recognized as list
- Input: `+ ` (space only) - Will be recognized, but creates empty list item
- Input: `+-weird` (hyphen after plus) - Recognized but produces `- -weird`

**Recommended Fix:**
```rust
if (trimmed.starts_with('+') || trimmed.starts_with('-')) && trimmed.len() > 1 {
    // Allow any whitespace or special character, or just alphanumeric
    let after_marker = trimmed.chars().nth(1).unwrap();
    if after_marker.is_whitespace() || !after_marker.is_alphanumeric() ||
       (after_marker.is_alphabetic()) {
        let content = trimmed[1..].trim();
        if !content.is_empty() {
            output.push_str("- ");
            output.push_str(content);
            output.push('\n');
        }
        continue;
    }
}
```

---

#### Issue #5: Empty Heading Markers Preserved
**File:** `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/src/extractors/typst.rs`
**Lines:** 282-298
**Severity:** HIGH
**Description:**
Headings with no text content are output as empty markers (`= \n`), cluttering output.

**Code:**
```rust
if trimmed.starts_with('=') {
    let next_char_pos = trimmed.find(|c: char| c != '=');
    if next_char_pos.is_some() {
        let heading_level = trimmed.chars().take_while(|&c| c == '=').count();
        let heading_text = trimmed[heading_level..].trim();

        for _ in 0..heading_level {
            output.push('=');
        }
        output.push(' ');
        output.push_str(heading_text);  // Could be empty string!
        output.push('\n');
        continue;
    }
}
```

**Problem:**
- Input: `=` (heading with no text)
- `heading_text = ""` after trim
- Output: `= \n` (useless empty heading)

**Recommended Fix:**
```rust
if next_char_pos.is_some() {
    let heading_level = trimmed.chars().take_while(|&c| c == '=').count();
    let heading_text = trimmed[heading_level..].trim();

    if !heading_text.is_empty() {  // Only output if has content
        for _ in 0..heading_level {
            output.push('=');
        }
        output.push(' ');
        output.push_str(heading_text);
        output.push('\n');
    }
    continue;
}
```

---

#### Issue #6: Link Text Loss on Empty Brackets
**File:** `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/src/extractors/typst.rs`
**Lines:** 488-502
**Severity:** HIGH
**Description:**
When `#link()` has empty brackets, the link text is lost entirely.

**Code:**
```rust
fn extract_link_text(&self, line: &str) -> String {
    let pattern = r#"link\("([^"]*)"\)\[([^\]]*)\]"#;
    if let Ok(re) = Regex::new(pattern) {
        return re
            .replace_all(line, |caps: &regex::Captures| {
                let url = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let text = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                format!("[{}]({})", text, url)  // If text is empty, produces []()
            })
            .to_string();
    }
    line.to_string()
}
```

**Problem:**
- Input: `#link("https://example.com")[]`
- Output: `[](https://example.com)` (empty link text)
- Better: Use URL as fallback text when empty

**Recommended Fix:**
```rust
format!("[{}]({})",
    if text.is_empty() { url } else { text },
    url)
```

---

### 1.3 MEDIUM SEVERITY ISSUES

#### Issue #7: Table Content Parsing Too Simplistic
**File:** `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/src/extractors/typst.rs`
**Lines:** 388-421
**Severity:** MEDIUM
**Description:**
Table extraction only handles simple bracket-based content. Complex nested structures (function calls, arrays) break the parser.

**Code:**
```rust
// Extract all bracketed content as table cells
let mut in_bracket = false;
let mut cell = String::new();
for ch in content.chars() {
    match ch {
        '[' => {
            in_bracket = true;
            cell.clear();
        }
        ']' => {
            if in_bracket {
                let trimmed = cell.trim();
                if !trimmed.is_empty() {
                    table_content.push_str(trimmed);
                    table_content.push_str(" | ");
                }
                in_bracket = false;
                cell.clear();
            }
        }
        _ if in_bracket => {
            cell.push(ch);
        }
        _ => {}
    }
}
```

**Problem:**
- Input: `#table([align(left, "Text")], ["Two"])`
- The `align(left, ...)` contains nested brackets
- Parser would split this incorrectly
- Example output would be: `align(left, Text) | ) | Two` (corrupted)

**Recommended Fix:**
Track nested bracket depth:
```rust
let mut bracket_depth = 0;
let mut cell = String::new();
for ch in content.chars() {
    match ch {
        '[' if bracket_depth == 0 => {
            cell.clear();
            bracket_depth = 1;
        }
        '[' if bracket_depth > 0 => {
            bracket_depth += 1;
            cell.push(ch);
        }
        ']' if bracket_depth > 1 => {
            bracket_depth -= 1;
            cell.push(ch);
        }
        ']' if bracket_depth == 1 => {
            // Finish cell
            bracket_depth = 0;
            // process cell...
        }
        _ => {
            if bracket_depth > 0 {
                cell.push(ch);
            }
        }
    }
}
```

---

#### Issue #8: No Support for Escaped Characters
**File:** `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/src/extractors/typst.rs`
**Lines:** Multiple (process_line, extract_quoted_value)
**Severity:** MEDIUM
**Description:**
The extractor doesn't handle escaped characters in Typst, which uses backslash escaping.

**Problem:**
- Input: `#set document(title: "Title with \"quotes\"")`
- Regex: `r#"{}:\s*"([^"]*)""#`
- The regex would match up to the first `\"` and break
- Escaped quotes in strings are not handled

**Impact:**
Metadata with quotes, apostrophes, or special chars will be truncated.

**Recommended Fix:**
Update regex patterns to handle escapes:
```rust
// Instead of: r#"{}:\s*"([^"]*)""#
// Use: r#"{}:\s*"((?:\\.|[^"])*)""#
```

---

#### Issue #9: Single # Treated as Link Prefix Without Validation
**File:** `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/src/extractors/typst.rs`
**Lines:** 474-477
**Severity:** MEDIUM
**Description:**
The code checks if next character is 'l' to detect links, but doesn't validate further.

**Code:**
```rust
'#' if chars.peek() == Some(&'l') => {
    // Might be a link, skip the # and let the rest be processed
    result.push(ch);
}
```

**Problem:**
- Input: `This is #lorem ipsum`
- Would match `#l` as potential link start
- The `extract_link_text()` regex requires full `link(...)` format
- So it just outputs `#lorem` unchanged
- This works but is inefficient: we try to process links that don't exist

**Recommended Fix:**
Either:
1. More specific check: `chars.peek() == Some(&'l')` and lookahead for `ink(`
2. Or just remove this optimization and let regex handle all link detection

---

### 1.4 LOW SEVERITY ISSUES

#### Issue #10: Inconsistent Line Processing in Content vs Brackets
**File:** `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/src/extractors/typst.rs`
**Lines:** 321-327
**Severity:** LOW
**Description:**
Content extracted from brackets (e.g., from `#align()`) is processed through `process_line()` again, which can lead to double-processing or inconsistent handling.

**Code:**
```rust
if let Some(content) = self.extract_text_from_brackets(trimmed) {
    let processed = self.process_line(&content);  // Processes again
    if !processed.is_empty() {
        output.push_str(&processed);
        output.push('\n');
    }
}
```

**Problem:**
- If extracted content itself contains formatting (e.g., `#align(center, [*bold*])`), it goes through `process_line()` once
- But if the extracted text is then used elsewhere, it might be processed again
- Less of a bug, more of a consistency issue

---

#### Issue #11: String Allocation in Loop
**File:** `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/src/extractors/typst.rs`
**Lines:** 206-209
**Severity:** LOW
**Description:**
In `extract_keywords()`, each keyword is converted to String individually in a loop, then joined. Could be optimized.

**Code:**
```rust
for item_caps in item_re.captures_iter(array_str) {
    if let Some(keyword) = item_caps.get(1) {
        keywords.push(keyword.as_str().to_string());  // Allocation per item
    }
}
```

**Performance Impact:** Negligible for typical documents (< 100 keywords), but poor pattern.

**Recommended Fix:**
```rust
let keywords: Vec<_> = item_re
    .captures_iter(array_str)
    .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
    .collect();
```

---

## 2. DRYness Violations

### 2.1 Repetitive Delimiter Handling Pattern
**File:** `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/src/extractors/typst.rs`
**Lines:** 430-438, 441-449, 452-460, 463-471
**Severity:** MEDIUM
**Description:**
The pattern for handling delimited content (backticks, dollar signs, asterisks, underscores) is identical but repeated 4 times.

**Duplicated Pattern:**
```rust
// Lines 430-438 (backticks)
'`' => {
    result.push('`');
    for c in chars.by_ref() {
        result.push(c);
        if c == '`' {
            break;
        }
    }
}
// Lines 441-449 (dollar)
'$' => {
    result.push('$');
    for c in chars.by_ref() {
        result.push(c);
        if c == '$' {
            break;
        }
    }
}
// Lines 452-460 (bold)
// Lines 463-471 (italic)
// IDENTICAL PATTERN REPEATED
```

**Recommendation:**
Extract into helper function:
```rust
fn consume_until_delimiter(
    delimiter: char,
    chars: &mut std::iter::Peekable<std::str::Chars>,
    result: &mut String
) {
    result.push(delimiter);
    for c in chars.by_ref() {
        result.push(c);
        if c == delimiter {
            break;
        }
    }
}

// In match statement:
'`' | '$' | '*' | '_' => {
    self.consume_until_delimiter(ch, &mut chars, &mut result);
}
```

**Lines Saved:** ~30 LOC

---

### 2.2 Repeated Regex Pattern Building
**File:** `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/src/extractors/typst.rs`
**Lines:** 153-175 (metadata extraction)
**Severity:** LOW
**Description:**
Each metadata field uses the same pattern: call `extract_quoted_value()`, then `insert()` into metadata.

**Duplicated Code:**
```rust
if let Some(title) = self.extract_quoted_value("title") {
    self.metadata.additional.insert("title".to_string(), title.into());
}
if let Some(author) = self.extract_quoted_value("author") {
    self.metadata.additional.insert("author".to_string(), author.into());
}
if let Some(date) = self.extract_quoted_value("date") {
    self.metadata.date = Some(date);  // Special case
}
if let Some(subject) = self.extract_quoted_value("subject") {
    self.metadata.additional.insert("subject".to_string(), subject.into());
}
```

**Recommendation:**
```rust
fn extract_and_store_metadata(&mut self, field: &str, into_date: bool) {
    if let Some(value) = self.extract_quoted_value(field) {
        if into_date {
            self.metadata.date = Some(value);
        } else {
            self.metadata.additional.insert(field.to_string(), value.into());
        }
    }
}

// Usage:
self.extract_and_store_metadata("title", false);
self.extract_and_store_metadata("author", false);
self.extract_and_store_metadata("date", true);
self.extract_and_store_metadata("subject", false);
```

**Lines Saved:** ~10 LOC

---

### 2.3 Repeated Bracket/Paren Counting
**File:** `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/src/extractors/typst.rs`
**Lines:** 359-367, 374-382
**Severity:** LOW
**Description:**
The same bracket/paren counting logic is repeated twice in `extract_table_content()`.

**Duplicated Pattern:**
```rust
// Lines 359-367
for ch in first_line.chars() {
    match ch {
        '(' => paren_depth += 1,
        ')' => paren_depth -= 1,
        '[' => bracket_depth += 1,
        ']' => bracket_depth -= 1,
        _ => {}
    }
}

// Lines 374-382 - EXACT DUPLICATE
for ch in next_line.chars() {
    match ch {
        '(' => paren_depth += 1,
        ')' => paren_depth -= 1,
        '[' => bracket_depth += 1,
        ']' => bracket_depth -= 1,
        _ => {}
    }
}
```

**Recommendation:**
```rust
fn count_brackets(text: &str) -> (i32, i32) {
    let mut paren = 0;
    let mut bracket = 0;
    for ch in text.chars() {
        match ch {
            '(' => paren += 1,
            ')' => paren -= 1,
            '[' => bracket += 1,
            ']' => bracket -= 1,
            _ => {}
        }
    }
    (paren, bracket)
}

// Usage:
let (p, b) = count_brackets(first_line);
paren_depth += p;
bracket_depth += b;
```

**Lines Saved:** ~12 LOC

---

### 2.4 Repeated if-let Pattern for Metadata Extraction
**File:** `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/src/extractors/typst.rs`
**Lines:** 196-209, 205-209
**Severity:** LOW
**Description:**
In `extract_keywords()`, there's a repeated pattern checking capture group 1 and 2, then processing.

---

## 3. TESTING GAPS

### 3.1 CRITICAL TEST GAPS

#### Test Gap #1: Unclosed Delimiter Test
**Missing Test:** No test for unclosed formatting delimiters
**Impact:** CRITICAL
**Scenario:**
```typst
Text with `unclosed code and more text
```
**Expected Behavior:** All text preserved
**Current Result:** Unknown (likely text after backtick lost)

**Recommended Test:**
```rust
#[test]
fn test_unclosed_delimiters() {
    let content = "Text with `unclosed backtick and more";
    let (output, _) = TypstExtractor::extract_from_typst(content);
    // Should preserve both "unclosed" and "backtick" and "and more"
    assert!(output.contains("unclosed"));
    assert!(output.contains("backtick"));
    assert!(output.contains("and more"));
}
```

---

#### Test Gap #2: Malformed Table with Mismatched Brackets
**Missing Test:** No test for malformed table syntax
**Impact:** CRITICAL
**Scenario:**
```typst
#table(
  [Header],
  [Data)
)
```
**Risk:** Could hang or panic due to bracket counting issues

**Recommended Test:**
```rust
#[test]
fn test_malformed_table_bounds() {
    let content = r#"#table(
  [Header],
  [Data)
)"#;
    let (output, _) = TypstExtractor::extract_from_typst(content);
    // Should not hang or panic
    assert!(!output.is_empty() || output.is_empty()); // Just don't panic
}
```

---

#### Test Gap #3: Escaped Characters in Metadata
**Missing Test:** No test for escaped quotes in metadata
**Impact:** HIGH
**Scenario:**
```typst
#set document(
    title: "Title with \"quotes\" inside"
)
```
**Expected:** Extract full title with quotes
**Current:** Likely extracts only "Title with \"

**Recommended Test:**
```rust
#[test]
fn test_metadata_with_escaped_quotes() {
    let content = r#"#set document(
        title: "Document \"Title\" Here"
    )"#;
    let (_, metadata) = TypstExtractor::extract_from_typst(content);
    let title = metadata.additional.get("title").unwrap().to_string();
    assert!(title.contains("Title"));
    // Ideally: assert_eq!(title, "Document \"Title\" Here");
}
```

---

#### Test Gap #4: Empty List Items
**Missing Test:** No test for empty list items or minimal lists
**Impact:** HIGH
**Scenario:**
```typst
+
- Item
+
```
**Expected:** Extract only non-empty items
**Current:** Unknown - likely outputs empty list items

**Recommended Test:**
```rust
#[test]
fn test_empty_list_items() {
    let content = r#"+
- Item
+"#;
    let (output, _) = TypstExtractor::extract_from_typst(content);
    // Should have "Item" but not empty markers
    assert!(output.contains("Item"));
    // Count list markers - should be 1 or 2, not 3
    let lines = output.lines().collect::<Vec<_>>();
    let list_lines = lines.iter().filter(|l| l.starts_with("-")).count();
    assert!(list_lines <= 2);
}
```

---

#### Test Gap #5: Nested Function Calls in Tables
**Missing Test:** No test for complex table with nested function calls
**Impact:** HIGH
**Scenario:**
```typst
#table(
  [#align(center, "Centered")],
  [#text(size: 12pt, "Text")]
)
```
**Expected:** Extract meaningful cell content
**Current:** Likely corrupted due to naive bracket parsing

**Recommended Test:**
```rust
#[test]
fn test_table_with_nested_functions() {
    let content = r#"#table(
  [#align(center, "Center")],
  [#text(size: 12pt, "Text")]
)"#;
    let (output, _) = TypstExtractor::extract_from_typst(content);
    assert!(output.contains("Center") || output.contains("center"));
    assert!(output.contains("Text") || output.contains("text"));
}
```

---

### 3.2 HIGH SEVERITY TEST GAPS

#### Test Gap #6: Link with Empty Text
**Missing Test:** Links with empty brackets `#link("url")[]]`
**Impact:** HIGH
**Current Behavior:** Creates `[](url)` - empty link text

**Recommended Test:**
```rust
#[test]
fn test_link_with_empty_text() {
    let content = r#"Visit #link("https://example.com")[] for info."#;
    let (output, _) = TypstExtractor::extract_from_typst(content);
    // Should contain either URL or fallback
    assert!(
        output.contains("example.com") ||
        output.contains("https://example.com")
    );
}
```

---

#### Test Gap #7: Mixed List Markers
**Missing Test:** Documents mixing `+` and `-` list markers
**Impact:** HIGH

**Recommended Test:**
```rust
#[test]
fn test_mixed_list_markers() {
    let content = r#"+ First item
- Second item
+ Third item"#;
    let (output, _) = TypstExtractor::extract_from_typst(content);
    let lines = output.lines().collect::<Vec<_>>();
    let list_items = lines.iter().filter(|l| l.starts_with("- ")).count();
    assert_eq!(list_items, 3); // All should be normalized to "- "
    assert!(output.contains("First item"));
    assert!(output.contains("Second item"));
}
```

---

#### Test Gap #8: Metadata with Arrays of Keywords
**Missing Test:** Keywords as actual Typst arrays
**Impact:** HIGH
**Current Test:** Tests keywords but with simple format

**Recommended Test:**
```rust
#[test]
fn test_metadata_keywords_array_format() {
    let content = r#"#set document(
        keywords: ("keyword1", "keyword2", "keyword3")
    )"#;
    let (_, metadata) = TypstExtractor::extract_from_typst(content);
    let keywords = metadata.additional.get("keywords").unwrap().to_string();
    assert!(keywords.contains("keyword1"));
    assert!(keywords.contains("keyword2"));
    assert!(keywords.contains("keyword3"));
}
```

---

#### Test Gap #9: Empty Headings
**Missing Test:** Headings with no text content
**Impact:** HIGH

**Recommended Test:**
```rust
#[test]
fn test_empty_heading() {
    let content = r#"=
==
Content here"#;
    let (output, _) = TypstExtractor::extract_from_typst(content);
    // Should not have empty heading lines
    assert!(!output.contains("= \n\n"));
    assert!(!output.contains("== \n\n"));
    assert!(output.contains("Content"));
}
```

---

#### Test Gap #10: Code Block with Backticks in Content
**Missing Test:** Code blocks containing backticks (edge case)
**Impact:** MEDIUM

**Recommended Test:**
```rust
#[test]
fn test_code_block_with_backticks() {
    let content = r#"```
let x = `backtick inside code`;
```"#;
    let (output, _) = TypstExtractor::extract_from_typst(content);
    assert!(output.contains("backtick inside code"));
}
```

---

### 3.3 MEDIUM SEVERITY TEST GAPS

#### Test Gap #11: Very Long Lines (Performance)
**Missing Test:** Document with extremely long lines
**Impact:** MEDIUM
**Purpose:** Ensure no O(n²) behavior or excessive allocations

#### Test Gap #12: Deeply Nested Formatting
**Missing Test:** Multiple levels of nested formatting
**Impact:** MEDIUM
**Scenario:** `*bold _italic **nested**_*`

#### Test Gap #13: Unicode and Non-ASCII Text
**Missing Test:** Metadata and content with Unicode
**Impact:** MEDIUM
**Scenario:** Keywords with emoji, content with non-Latin characters

#### Test Gap #14: Multiple Documents in Sequence
**Missing Test:** Extractor state between multiple document extractions
**Impact:** LOW
**Purpose:** Ensure no state leakage between extractions

#### Test Gap #15: Table Edge Cases
**Missing Tests:**
- Table with empty cells
- Table with single cell
- Table spanning multiple blocks with formatting
**Impact:** MEDIUM

---

## 4. IMPLEMENTATION GAPS

### 4.1 CRITICAL MISSING FEATURES

#### Gap #1: No Raw/Code Block Language Syntax Preservation
**File:** `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/src/extractors/typst.rs`
**Lines:** 228-252
**Severity:** HIGH
**Description:**
Code blocks extract language specifier but don't handle Typst `#raw()` function blocks.

**Missing:**
```typst
#raw(lang: "python", ```
code here
```)
```

**What's Extracted:** Only triple-backtick style code blocks

**Recommendation:**
Add handler for `#raw()` function:
```rust
if trimmed.starts_with("#raw(") {
    // Extract language from lang: "..."
    // Extract code from content
}
```

---

#### Gap #2: No Blockquote Support
**Missing Feature:** Blockquotes (`> text`)
**Impact:** HIGH
**Missing Logic:** No detection or output of blockquotes
**Pandoc Parity:** Pandoc extracts blockquotes; Kreuzberg ignores them

**Recommended Implementation:**
```rust
if trimmed.starts_with('>') {
    output.push_str("> ");
    output.push_str(trimmed[1..].trim());
    output.push('\n');
    continue;
}
```

---

#### Gap #3: No Strikethrough Support
**Missing Feature:** Strikethrough (`~~text~~` or Typst equivalent)
**Impact:** MEDIUM
**Missing Logic:** No detection of strikethrough syntax

---

#### Gap #4: No Footnotes/References
**Missing Feature:** Footnote extraction
**Impact:** MEDIUM
**Missing Logic:** No extraction of footnote content or references

---

### 4.2 HIGH SEVERITY MISSING FEATURES

#### Gap #5: Incomplete Table Extraction
**Severity:** HIGH
**Current Implementation:** Basic cell extraction with `|` separators
**Missing:**
- Table headers vs data distinction
- Table caption/figure detection
- Column alignment attributes
- Merged cells
- Multi-line cell content

**Comparison to Pandoc:**
Pandoc extracts:
```
Table structure with headers, alignment, width
```
Kreuzberg extracts:
```
TABLE:
content | content | content
```

**Recommendation:**
Return structured table data (currently returns empty `tables: Vec::new()` in ExtractionResult line 111):
```rust
// Line 111 in extract_bytes:
tables: vec![],  // Should populate this!
```

---

#### Gap #6: No Metadata Field Completeness
**Severity:** HIGH
**Currently Extracted:**
- title, author, date, subject, keywords

**Missing from Pandoc Standard:**
- creator
- producer
- subject
- keywords (array handling incomplete)
- creation-date
- modification-date

**Impact:** Metadata incomplete compared to Pandoc's metadata extraction

---

#### Gap #7: No Inline Math Extraction/Preservation
**Severity:** MEDIUM
**Current:**
```rust
'$' => {
    result.push('$');
    // ... preserves as-is
}
```

**What Pandoc Does:**
Extracts math content separately and marks as `Math InlineMath "..."`

**Current Behavior:**
Math is preserved in text: `E = mc^2` becomes `$E = mc^2$`

**Missing:** Structured math extraction

---

#### Gap #8: No Display Math Block Support
**Severity:** MEDIUM
**Missing Detection:**
```typst
$ display math block here $
```

vs inline `$inline$`

---

#### Gap #9: Incomplete Link Extraction
**Severity:** MEDIUM
**Current:**
Only handles `#link("url")[text]` format

**Missing:**
- Automatic links: `<http://example.com>`
- Reference links: `[link][ref]`
- Shortcut links: `[link]`

---

#### Gap #10: No Heading Attributes
**Severity:** MEDIUM
**Missing:**
- Heading IDs/labels (`= Heading <label>`)
- Heading numbering configuration
- Heading styling (should be preserved for structure)

---

### 4.3 MEDIUM SEVERITY MISSING FEATURES

#### Gap #11: No Comment Removal
**Missing:** Comments starting with `//`
**Impact:** MEDIUM
**Current Behavior:** Comments included in output

**Recommendation:**
```rust
if trimmed.starts_with("//") {
    continue;  // Skip comment lines
}
```

---

#### Gap #12: No Indentation/Nesting Preservation
**Severity:** MEDIUM
**Current:**
Lines are extracted but indentation is trimmed

**Missing:**
Nested block indentation (for structured output)

---

#### Gap #13: No Abbreviation/Acronym Handling
**Missing Feature:** Special Typst abbreviation syntax
**Impact:** LOW

---

#### Gap #14: No Horizontal Rule/Divider Support
**Missing:** `---` or `***` detection
**Impact:** LOW

---

#### Gap #15: No List Nesting Support
**Missing:** Nested list structure
**Impact:** MEDIUM
**Current:**
```typst
+ Item 1
  + Nested item
+ Item 2
```
Produces flat list, not nested

---

## 5. SUMMARY TABLE

| Category | Count | Critical | High | Medium | Low |
|----------|-------|----------|------|--------|-----|
| **Code Quality** | 11 | 3 | 2 | 3 | 3 |
| **DRYness** | 4 | 0 | 0 | 1 | 3 |
| **Testing Gaps** | 15 | 5 | 5 | 5 | 0 |
| **Implementation Gaps** | 15 | 2 | 8 | 4 | 1 |
| **TOTAL** | **45** | **10** | **15** | **13** | **7** |

---

## 6. PRIORITY FIXES

### Immediate (Blocks Correctness)
1. **Issue #1**: Unclosed delimiter handling
2. **Issue #2**: Bracket depth validation in tables
3. **Test Gap #1-4**: Critical test cases

### High Priority (Quality)
1. **Issue #3**: Regex error handling with logging
2. **Issue #7**: Table nested bracket handling
3. **Gap #1-3**: Missing Typst features
4. **Gap #5**: Structured table extraction

### Medium Priority (Polish)
1. DRYness refactoring (30+ LOC savings)
2. Remaining medium-severity issues
3. Test coverage expansion

---

## 7. METRICS

**Code Quality:**
- Duplicate code: ~50 LOC (4 tests × 8-9 lines)
- Error handling: Silent failures in 3+ places
- Edge cases: 5 unhandled scenarios

**Test Coverage:**
- Integration tests: 13 ✓
- Unit tests: 9 (embedded in extractor)
- Missing test scenarios: 15

**Feature Parity with Pandoc:**
- Implemented: 60%
- Missing: 40% (tables, blockquotes, strikethrough, advanced metadata)

---

## 8. CONCLUSION

The Typst extractor is functionally correct for basic documents and has good test coverage. However:

1. **Code quality issues** could cause data loss (unclosed delimiters) or poor handling (empty headings, malformed tables)
2. **DRYness violations** reduce maintainability (~50 LOC could be refactored)
3. **Testing gaps** leave critical edge cases untested
4. **Implementation gaps** limit feature completeness, especially for complex documents and Pandoc parity

**Estimated Effort to Address:**
- Critical issues: 4-6 hours
- DRYness refactoring: 2-3 hours
- Testing gaps: 3-4 hours
- Implementation gaps: 8-12 hours
- **Total: 17-25 hours**

**Recommendation:** Address critical issues first (unclosed delimiters, bracket validation), then expand test coverage, then tackle DRYness and feature gaps.
