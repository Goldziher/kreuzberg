---
title: "Comprehensive Markdown Test Document"
author: "Dr. Jane Smith"
date: "2024-01-15"
keywords:
  - markdown
  - testing
  - pandoc
  - metadata
description: "A comprehensive test document to validate markdown extraction capabilities"
abstract: "This document tests various markdown features including tables, lists, code blocks, and formatting"
subject: "Markdown Testing"
tags: ["test", "validation", "quality"]
version: 1.2.3
custom_field: "custom_value"
nested:
  organization: "Test Corp"
  department: "Engineering"
  contact:
    email: "test@example.com"
    phone: "+1-555-0100"
authors:
  - name: "Jane Smith"
    affiliation: "University A"
  - name: "John Doe"
    affiliation: "Company B"
---

# Main Heading

This is a comprehensive test document with **bold**, *italic*, ***bold italic***, ~~strikethrough~~, and `inline code`.

## Second Level Heading

### Third Level Heading

#### Fourth Level Heading

##### Fifth Level Heading

###### Sixth Level Heading

## Links and Images

Check out [Google](https://google.com) and [Rust](https://rust-lang.org).

![Alt text for image](https://example.com/image.png)

Reference-style links: [Link text][reference]

[reference]: https://example.com "Optional Title"

## Lists

### Unordered Lists

- Item 1
- Item 2
  - Nested item 2.1
  - Nested item 2.2
    - Double nested 2.2.1
- Item 3

### Ordered Lists

1. First item
2. Second item
   1. Nested 2.1
   2. Nested 2.2
3. Third item

### Mixed Lists

1. Ordered item
   - Unordered nested
   - Another unordered
2. Another ordered
   1. Nested ordered
   2. More nested

## Code Blocks

### Fenced Code Block with Language

```rust
fn main() {
    println!("Hello, world!");
    let x = 42;
    let result = compute(x);
}

fn compute(n: i32) -> i32 {
    n * 2
}
```

### JavaScript Example

```javascript
function greet(name) {
    console.log(`Hello, ${name}!`);
}

greet("World");
```

### Plain Code Block

```
This is a plain code block
without syntax highlighting
```

## Tables

### Simple Table

| Name    | Age | City        |
|---------|-----|-------------|
| Alice   | 30  | New York    |
| Bob     | 25  | London      |
| Charlie | 35  | Tokyo       |

### Complex Table with Alignment

| Left Aligned | Center Aligned | Right Aligned |
|:-------------|:--------------:|--------------:|
| Left         | Center         | Right         |
| A            | B              | C             |
| 123          | 456            | 789           |

### Table with Code and Formatting

| Feature         | Status      | Description                  |
|-----------------|-------------|------------------------------|
| **Bold Text**   | ✓           | `Supported`                  |
| *Italic*        | ✓           | Works fine                   |
| ~~Strikethrough~~ | Partial   | May vary                     |

## Blockquotes

> This is a blockquote.
> It can span multiple lines.
>
> > This is a nested blockquote.
> > Even deeper nesting is possible.

> **Note:** Blockquotes can contain other markdown elements.
>
> - Lists
> - **Bold text**
> - `Code`

## Horizontal Rules

---

***

___

## Task Lists

- [x] Completed task
- [ ] Incomplete task
- [x] Another completed task
  - [ ] Nested incomplete
  - [x] Nested complete

## Inline HTML

<div class="custom-class">
This is HTML content within markdown.
</div>

<span style="color: red;">Red text</span>

## Footnotes

Here is a footnote reference[^1].

Another footnote[^note].

[^1]: This is the first footnote.

[^note]: This is a named footnote with more details.

## Definition Lists

Term 1
:   Definition 1

Term 2
:   Definition 2a
:   Definition 2b

## Emphasis Variations

*single asterisks*

_single underscores_

**double asterisks**

__double underscores__

## Escape Characters

\*Not italic\*

\# Not a heading

\[Not a link\]

## Special Characters

& copyright © trademark ™ registered ®

Em dash — en dash –

Quotes: "double" and 'single'

## Math (if supported)

Inline math: $E = mc^2$

Display math:

$$
\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}
$$

## Line Blocks

| The right words
| in the right order
| can create magic.

## Multiple Paragraphs

This is the first paragraph. It has multiple sentences. Each sentence adds information.

This is the second paragraph. There's a blank line between paragraphs.

This is the third paragraph with some **bold** and *italic* text mixed in.

## Combinations

> **Quote with formatting**
>
> 1. Ordered list in quote
> 2. Second item
>    - Nested unordered
>    - Another nested
>
> ```python
> def code_in_quote():
>     return "yes"
> ```

## Empty Elements

Empty paragraph:

Empty blockquote:

>

## Unicode Support

### Emojis

😀 😃 😄 😁 🎉 🚀 ⭐ 💯

### International Characters

- Japanese: 日本語のテキスト
- Russian: Русский текст
- Arabic: النص العربي
- Chinese: 中文文本
- Hebrew: טקסט עברית
- Greek: Ελληνικό κείμενο

### Special Symbols

→ ← ↑ ↓ ↔ ⇒ ⇐ ⇑ ⇓ ⇔
∀ ∃ ∈ ∋ ⊂ ⊃ ⊆ ⊇
± × ÷ √ ∞ ≈ ≠ ≤ ≥

## Edge Cases

### Consecutive Formatting

***Bold and italic*** then **just bold** then *just italic* then regular.

### Empty Links

[Empty]()

[](#anchor)

### Nested Emphasis

*italic with **bold** inside*

**bold with *italic* inside**

### URLs in Text

Visit https://example.com or http://test.org directly.

Email: test@example.com

## Final Section

This document comprehensively tests markdown features for validation against Pandoc output.
