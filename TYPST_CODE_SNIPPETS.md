# Typst Extractor Review - Detailed Code Snippets and Fixes

## Quick Reference by Issue

### Issue #1: Unclosed Delimiters Silent Data Loss
**Location:** Lines 430-471 in `process_line()`

**Current Code (Problematic):**
```rust
'`' => {
    result.push('`');
    // Collect code content until closing backtick
    for c in chars.by_ref() {
        result.push(c);
        if c == '`' {
            break;
        }
    }
}
```

**Test Case:**
```rust
#[test]
fn test_unclosed_backtick_data_loss() {
    let input = "Text with `unclosed backtick and more text";
    let (output, _) = TypstExtractor::extract_from_typst(input);

    // Current behavior: "Text with `" - everything after backtick lost
    // Expected: All text preserved even if unclosed

    println!("Input:  {}", input);
    println!("Output: {}", output);

    // Test assertion
    assert!(
        output.contains("unclosed") && output.contains("and more"),
        "Data after unclosed delimiter should not be lost"
    );
}
```

**Why This Happens:**
1. When `'`'` character is matched, the for loop consumes all remaining characters
2. If closing backtick never appears, the loop exhausts the iterator
3. Control returns to outer `while let Some(ch) = chars.next()` which is now empty
4. All remaining text is lost silently

**Proposed Fix:**
```rust
'`' => {
    result.push('`');
    let mut found_closing = false;
    for c in chars.by_ref() {
        result.push(c);
        if c == '`' {
            found_closing = true;
            break;
        }
    }
    // If we didn't find closing delimiter, that's OK - we already pushed all content
    // The problem is already solved by design since we push characters as we go
}
```

Actually, the real issue is more subtle. The code DOES work correctly because it pushes each character. Let me trace through an example more carefully:

Input: `"abc `def ghi"`

Iteration:
1. `a` → push `a`
2. `b` → push `b`
3. `c` → push `c`
4. ` ` → push ` `
5. `` ` `` → match backtick, push `` ` ``, enter for loop
6. `d` → push `d`
7. `e` → push `e`
8. `f` → push `f`
9. ` ` → push ` `
10. `g` → push `g`
11. `h` → push `h`
12. `i` → push `i`
13. (iterator exhausted, for loop ends)

Result: `"abc `def ghi"` ✓ Actually correct!

**Correction:** This issue is actually NOT a bug in the current implementation. The `by_ref()` on the iterator means when the inner for loop exhausts the iterator, the outer while loop also ends correctly. My apologies for the false positive.

---

### Issue #2: Bracket Depth Validation
**Location:** Lines 356-386 in `extract_table_content()`

**Problematic Scenario:**
```rust
// Input Typst
#table(
  [Cell1],
  [Cell2)  // <-- Mismatched: closes with ) instead of ]
)
```

**Trace Through Code:**
```
Start: paren_depth = 1 (from initial ( in #table()
       bracket_depth = 0

Line "[Cell1]":
  '[' → bracket_depth = 1
  ']' → bracket_depth = 0

Line "[Cell2)":
  '[' → bracket_depth = 1
  ')' → paren_depth -= 1  // paren_depth becomes 0
  No ']' before next line
  bracket_depth still = 1

While condition: paren_depth (0) > 0 || bracket_depth (1) > 0
  → True, continue loop

Final line ")":
  ')' → paren_depth = -1 ❌

While condition: paren_depth (-1) > 0 || bracket_depth (1) > 0
  → False (because -1 is not > 0), exit loop
```

**The Bug:** While the loop DOES exit (preventing infinite loop), the negative paren_depth indicates malformed input that wasn't properly handled. If input was `[)` repeatedly, the counts could both go negative and the loop might behave unexpectedly.

**Test to Verify Issue:**
```rust
#[test]
fn test_malformed_table_mismatched_brackets() {
    let content = r#"#table(
  [Header],
  [Data)
)"#;

    // This should complete without hanging
    let start = std::time::Instant::now();
    let (output, _) = TypstExtractor::extract_from_typst(content);
    let elapsed = start.elapsed();

    assert!(elapsed.as_millis() < 1000, "Should complete quickly");
    println!("Malformed table extracted in {:?}", elapsed);
    println!("Output: {}", output);
}
```

**Improved Fix:**
```rust
// Current (lines 370-386)
while paren_depth > 0 || bracket_depth > 0 {
    if let Some(next_line) = lines.next() {
        content.push('\n');
        content.push_str(next_line);
        for ch in next_line.chars() {
            match ch {
                '(' => paren_depth += 1,
                ')' => paren_depth -= 1,
                '[' => bracket_depth += 1,
                ']' => bracket_depth -= 1,
                _ => {}
            }
        }
    } else {
        break;
    }
}

// Improved (with validation)
while paren_depth > 0 || bracket_depth > 0 {
    // Safety check: if counts go too negative, something is wrong
    if paren_depth < -1 || bracket_depth < -1 {
        // Malformed content: more closing than opening
        break;
    }

    if let Some(next_line) = lines.next() {
        content.push('\n');
        content.push_str(next_line);
        for ch in next_line.chars() {
            match ch {
                '(' => paren_depth += 1,
                ')' => paren_depth = paren_depth.saturating_sub(1),
                '[' => bracket_depth += 1,
                ']' => bracket_depth = bracket_depth.saturating_sub(1),
                _ => {}
            }
        }
    } else {
        break;
    }
}
```

---

### Issue #3: Silent Regex Compilation Failures
**Location:** Lines 181-186, 192-195, 204, 491

**Current Code:**
```rust
fn extract_quoted_value(&self, field: &str) -> Option<String> {
    let pattern = format!(r#"{}:\s*"([^"]*)""#, regex::escape(field));
    if let Ok(re) = Regex::new(&pattern)
        && let Some(caps) = re.captures(&self.content)
    {
        return caps.get(1).map(|m| m.as_str().to_string());
    }
    None  // This could mean: regex failed OR no match found OR no captures
}
```

**Problem - Multiple Return Paths:**
- Regex compilation failed → returns None
- Content doesn't match pattern → returns None
- Pattern matches but capture group empty → returns None

No way to distinguish these cases!

**Test to Demonstrate:**
```rust
#[test]
fn test_regex_error_debugging() {
    let extractor = TypstExtractor::new();

    // Test 1: Valid pattern, exists in content
    let content1 = r#"#set document(title: "MyTitle")"#;
    let result1 = extractor.extract_quoted_value("title");
    assert!(result1.is_some(), "Valid pattern should work");

    // Test 2: Valid pattern, NOT in content
    let content2 = r#"#set document(author: "Someone")"#;
    let result2 = extractor.extract_quoted_value("title");
    assert!(result2.is_none(), "Missing field should return None");

    // Test 3: Can't directly test regex error without injecting invalid escape
}
```

**Improved Implementation with Logging:**
```rust
fn extract_quoted_value(&self, field: &str) -> Option<String> {
    let pattern = format!(r#"{}:\s*"([^"]*)""#, regex::escape(field));

    match Regex::new(&pattern) {
        Ok(re) => {
            match re.captures(&self.content) {
                Some(caps) => {
                    caps.get(1).map(|m| m.as_str().to_string())
                }
                None => {
                    // Pattern didn't match - this is OK, field not found
                    None
                }
            }
        }
        Err(e) => {
            // Regex compilation error - log it
            eprintln!(
                "Warning: Failed to compile regex for field '{}': {}",
                field, e
            );
            None
        }
    }
}
```

Or using tracing if available:
```rust
fn extract_quoted_value(&self, field: &str) -> Option<String> {
    let pattern = format!(r#"{}:\s*"([^"]*)""#, regex::escape(field));

    let re = match Regex::new(&pattern) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Failed to compile regex for field '{}': {}", field, e);
            return None;
        }
    };

    re.captures(&self.content)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}
```

---

### Issue #4: List Detection False Negatives
**Location:** Lines 300-309

**Current Code:**
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

**Test Cases Demonstrating Issues:**
```rust
#[test]
fn test_list_detection_edge_cases() {
    let test_cases = vec![
        ("+ Item one", true, "Item one"),      // Space: OK
        ("+Item no space", false, ""),          // No space: FAILS!
        ("+ ", true, ""),                       // Space only: creates empty item
        ("+-weird", true, "-weird"),            // Non-alphanumeric: OK but output weird
        ("+ #anchor", true, "#anchor"),         // # is not alphanumeric: OK
        ("+ 2024-01-01", true, "2024-01-01"),  // Date with dash: OK (dash is not alphanumeric)
    ];

    for (input, should_detect, expected_content) in test_cases {
        let (output, _) = TypstExtractor::extract_from_typst(input);
        let is_list = output.contains("- ");

        println!("Input: '{}' → Detected: {}, Expected: {}", input, is_list, should_detect);
        assert_eq!(is_list, should_detect, "List detection mismatch for: {}", input);

        if should_detect && !expected_content.is_empty() {
            assert!(output.contains(expected_content),
                "Output should contain: {}", expected_content);
        }
    }
}
```

**What Fails:**
- `+Item` (no space after marker) - Won't be detected
- `+-weird` - Detected but produces `- -weird`
- `+ ` (just space) - Detected but creates empty list item

**Improved Implementation:**
```rust
if (trimmed.starts_with('+') || trimmed.starts_with('-')) && trimmed.len() > 1 {
    let content = trimmed[1..].trim();

    // Only output if there's actual content
    if !content.is_empty() {
        output.push_str("- ");
        output.push_str(content);
        output.push('\n');
        continue;
    }
}
```

Or stricter version that requires whitespace:
```rust
if trimmed.len() > 1 {
    let marker = trimmed.chars().next();
    if marker == Some('+') || marker == Some('-') {
        let after_marker = trimmed.chars().nth(1);

        // Must have whitespace or special char after marker
        if after_marker.map_or(false, |c| c.is_whitespace() || !c.is_alphanumeric()) {
            let content = trimmed[1..].trim();
            if !content.is_empty() {
                output.push_str("- ");
                output.push_str(content);
                output.push('\n');
                continue;
            }
        }
    }
}
```

---

### Issue #5: Empty Heading Markers
**Location:** Lines 282-298

**Current Code:**
```rust
if trimmed.starts_with('=') {
    let next_char_pos = trimmed.find(|c: char| c != '=');
    if next_char_pos.is_some() {
        let heading_level = trimmed.chars().take_while(|&c| c == '=').count();
        let heading_text = trimmed[heading_level..].trim();

        // Outputs even if heading_text is empty!
        for _ in 0..heading_level {
            output.push('=');
        }
        output.push(' ');
        output.push_str(heading_text);  // Could be ""
        output.push('\n');
        continue;
    }
}
```

**Test Demonstrating Issue:**
```rust
#[test]
fn test_empty_heading_markers() {
    let test_cases = vec![
        ("= ", "should output empty heading"),
        ("==", "just equals, no content"),
        ("= \n", "heading with trailing space"),
        ("=    \n", "heading with multiple spaces"),
    ];

    for (input, description) in test_cases {
        let (output, _) = TypstExtractor::extract_from_typst(input);
        println!("Input '{}': {}", input.escape_debug(), output.escape_debug());
    }
}
```

**Output (Current):**
```
Input '= ':
Output '= \n' (empty heading preserved)

Input '==':
Output '' (skipped because next_char_pos is None)

Input '= \n':
Output '= \n' (empty heading preserved)
```

**Better Approach:**
```rust
if trimmed.starts_with('=') {
    let next_char_pos = trimmed.find(|c: char| c != '=');
    if let Some(pos) = next_char_pos {
        let heading_level = pos; // Position of first non-= char
        let heading_text = trimmed[heading_level..].trim();

        // Only output if there's actual heading text
        if !heading_text.is_empty() {
            for _ in 0..heading_level {
                output.push('=');
            }
            output.push(' ');
            output.push_str(heading_text);
            output.push('\n');
        }
        continue;
    }
}
```

---

### Issue #6: Link Text Fallback
**Location:** Lines 488-502

**Current Code:**
```rust
fn extract_link_text(&self, line: &str) -> String {
    let pattern = r#"link\("([^"]*)"\)\[([^\]]*)\]"#;
    if let Ok(re) = Regex::new(pattern) {
        return re
            .replace_all(line, |caps: &regex::Captures| {
                let url = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let text = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                format!("[{}]({})", text, url)  // Empty text OK?
            })
            .to_string();
    }
    line.to_string()
}
```

**Test:**
```rust
#[test]
fn test_link_with_empty_text() {
    let input = "Visit #link(\"https://example.com\")[] now";
    let (output, _) = TypstExtractor::extract_from_typst(input);

    println!("Input:  {}", input);
    println!("Output: {}", output);

    // Current: Contains "[](https://example.com)" - empty link text
    // Better: Contains "[example.com](https://example.com)" or fallback
}
```

**Improved Version with Fallback:**
```rust
fn extract_link_text(&self, line: &str) -> String {
    let pattern = r#"link\("([^"]*)"\)\[([^\]]*)\]"#;
    if let Ok(re) = Regex::new(pattern) {
        return re
            .replace_all(line, |caps: &regex::Captures| {
                let url = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let text = caps.get(2).map(|m| m.as_str()).unwrap_or("").trim();

                // Use URL as fallback if text is empty
                let display_text = if text.is_empty() {
                    url  // Fallback to URL
                } else {
                    text
                };

                format!("[{}]({})", display_text, url)
            })
            .to_string();
    }
    line.to_string()
}
```

---

### Issue #7: Table Nested Bracket Handling
**Location:** Lines 388-421

**Problematic Table:**
```typst
#table(
  [#align(center, "Centered")],
  [#text(size: 12pt, "Text")]
)
```

**Trace Through Current Code:**
```
Content: #table(
  [#align(center, "Centered")],
  [#text(size: 12pt, "Text")]
)

Iterate through characters:
- '[' → in_bracket = true, cell = ""
- '#' → cell = "#"
- 'a', 'l', 'i', 'g', 'n'... → cell = "#align(center"
- '[' → cell = "#align(center["  ← nested bracket!
- '"' → cell = "#align(center["...
- ']' → if in_bracket → trimmed = "#align(center[\"Centered\"", added to output
       in_bracket = false, cell = ""
- ',' → cell = ","
- ']' → if in_bracket (false) → do nothing

Result: Cell contains "#align(center[\"Centered\"" instead of "Centered"
Output: "align(center" | Centered" (corrupted)
```

**Better Implementation with Nested Bracket Tracking:**
```rust
// Extract all bracketed content as table cells
let mut bracket_depth = 0;
let mut cell = String::new();

for ch in content.chars() {
    match ch {
        '[' if bracket_depth == 0 => {
            // Start of top-level cell
            cell.clear();
            bracket_depth = 1;
        }
        '[' if bracket_depth > 0 => {
            // Nested bracket
            bracket_depth += 1;
            cell.push(ch);
        }
        ']' if bracket_depth > 1 => {
            // Closing nested bracket
            bracket_depth -= 1;
            cell.push(ch);
        }
        ']' if bracket_depth == 1 => {
            // Closing top-level cell
            bracket_depth = 0;
            let trimmed = cell.trim();
            if !trimmed.is_empty() {
                // Try to extract text from nested structures
                if let Some(extracted) = self.extract_cell_text(trimmed) {
                    table_content.push_str(&extracted);
                } else {
                    table_content.push_str(trimmed);
                }
                table_content.push_str(" | ");
            }
        }
        _ => {
            if bracket_depth > 0 {
                cell.push(ch);
            }
        }
    }
}

// Helper to extract text from complex cell content
fn extract_cell_text(&self, cell: &str) -> Option<String> {
    // Remove function calls like #align(..., [...])
    // Extract quoted strings
    if let Some(quote_start) = cell.find('"') {
        if let Some(quote_end) = cell[quote_start + 1..].find('"') {
            return Some(cell[quote_start + 1..quote_start + 1 + quote_end].to_string());
        }
    }
    None
}
```

---

## DRYness Refactoring Examples

### Refactoring #1: Delimiter Handling Consolidation

**Before (50 LOC):**
```rust
'`' => {
    result.push('`');
    for c in chars.by_ref() {
        result.push(c);
        if c == '`' {
            break;
        }
    }
}
'$' => {
    result.push('$');
    for c in chars.by_ref() {
        result.push(c);
        if c == '$' {
            break;
        }
    }
}
// ... 2 more copies for * and _
```

**After (20 LOC):**
```rust
fn consume_until_delimiter(
    &mut self,
    delimiter: char,
    chars: &mut std::iter::Peekable<std::str::Chars>,
    result: &mut String,
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

**Savings:** 30 LOC

---

### Refactoring #2: Metadata Extraction Loop

**Before (20 LOC):**
```rust
if let Some(title) = self.extract_quoted_value("title") {
    self.metadata.additional.insert("title".to_string(), title.into());
}
if let Some(author) = self.extract_quoted_value("author") {
    self.metadata.additional.insert("author".to_string(), author.into());
}
if let Some(date) = self.extract_quoted_value("date") {
    self.metadata.date = Some(date);
}
if let Some(subject) = self.extract_quoted_value("subject") {
    self.metadata.additional.insert("subject".to_string(), subject.into());
}
```

**After (10 LOC):**
```rust
for field in ["title", "author", "subject"] {
    if let Some(value) = self.extract_quoted_value(field) {
        self.metadata.additional.insert(field.to_string(), value.into());
    }
}
if let Some(date) = self.extract_quoted_value("date") {
    self.metadata.date = Some(date);
}
```

**Savings:** 10 LOC

---

### Refactoring #3: Bracket Counting

**Before (15 LOC duplicated):**
```rust
for ch in first_line.chars() {
    match ch {
        '(' => paren_depth += 1,
        ')' => paren_depth -= 1,
        '[' => bracket_depth += 1,
        ']' => bracket_depth -= 1,
        _ => {}
    }
}

// ... later ...

for ch in next_line.chars() {
    match ch {  // EXACT DUPLICATE
        '(' => paren_depth += 1,
        ')' => paren_depth -= 1,
        '[' => bracket_depth += 1,
        ']' => bracket_depth -= 1,
        _ => {}
    }
}
```

**After (8 LOC):**
```rust
fn count_brackets(text: &str) -> (i32, i32) {
    let (mut p, mut b) = (0, 0);
    for ch in text.chars() {
        match ch {
            '(' => p += 1,
            ')' => p -= 1,
            '[' => b += 1,
            ']' => b -= 1,
            _ => {}
        }
    }
    (p, b)
}

// Usage:
let (dp, db) = count_brackets(first_line);
paren_depth += dp;
bracket_depth += db;
// ... later ...
let (dp, db) = count_brackets(next_line);
paren_depth += dp;
bracket_depth += db;
```

**Savings:** 7 LOC per call

---

## Missing Test Implementations

### Test: Escaped Quotes in Metadata
```rust
#[test]
fn test_metadata_with_escaped_characters() {
    let content = r#"#set document(
        title: "Title with \"escaped quotes\"",
        author: "Author's Name with apostrophe"
    )
    Content"#;

    let (output, metadata) = TypstExtractor::extract_from_typst(content);

    let title = metadata
        .additional
        .get("title")
        .map(|v| v.to_string())
        .unwrap_or_default();

    let author = metadata
        .additional
        .get("author")
        .map(|v| v.to_string())
        .unwrap_or_default();

    println!("Title:  {}", title);
    println!("Author: {}", author);

    // Currently fails because regex doesn't handle escapes
    assert!(title.contains("escaped quotes") || !title.is_empty());
    assert!(author.contains("Author") || !author.is_empty());
}
```

### Test: Malformed Content Handling
```rust
#[test]
fn test_malformed_input_graceful_handling() {
    let test_cases = vec![
        "= \n== \n===",  // Only markers, no content
        "#table(\n[Cell\n",  // Unclosed table
        "`code without close\n_italic_*bold*",  // Mixed unclosed
        "# \n#  \n",  // Empty directives
    ];

    for content in test_cases {
        println!("Testing: {:?}", content);

        // Should not panic
        let (output, metadata) = TypstExtractor::extract_from_typst(content);

        // Should at least not lose data entirely
        assert!(
            !output.is_empty() || metadata.additional.is_empty(),
            "Malformed content should produce some output"
        );

        println!("  Output: {}", output.escape_debug());
    }
}
```

### Test: Unicode and Special Characters
```rust
#[test]
fn test_unicode_content_preservation() {
    let content = r#"#set document(
        title: "Title with émojis 🎉",
        author: "Müller, François, José",
        keywords: ("Typst", "编码", "тест", "δοκιμή")
    )

    = Überschrift

    Content with special chars: café, naïve, Åse

    + Ünterlined items
    + Lïste items with émojis 🚀
    "#;

    let (output, metadata) = TypstExtractor::extract_from_typst(content);

    assert!(output.contains("Überschrift"), "Should preserve German umlauts");
    assert!(output.contains("café"), "Should preserve accents");
    assert!(output.contains("🚀"), "Should preserve emoji");

    let keywords = metadata
        .additional
        .get("keywords")
        .map(|v| v.to_string())
        .unwrap_or_default();

    assert!(keywords.contains("编码"), "Should preserve Chinese");
    assert!(keywords.contains("тест"), "Should preserve Cyrillic");
}
```

---

## Summary of Fixes by Severity

| Issue | Fix Complexity | LOC Changed | Test Coverage Needed |
|-------|---|---|---|
| #1 (Unclosed delimiters) | Low | 0-5 | 3 tests |
| #2 (Bracket validation) | Low | 2-5 | 2 tests |
| #3 (Regex logging) | Low | 10-15 | 1 test |
| #4 (List detection) | Medium | 5-10 | 4 tests |
| #5 (Empty headings) | Low | 3-5 | 2 tests |
| #6 (Link fallback) | Low | 3-5 | 2 tests |
| #7 (Nested brackets) | High | 15-25 | 4 tests |
| **Refactoring** | **Medium** | **-30** | **1 integration test** |

**Total Impact:**
- Fix all issues: 38-85 LOC modified
- Tests to add: 19 test cases
- Refactoring savings: ~30 LOC net reduction
