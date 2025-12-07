#set document(
  title: "Simple Test Document",
  author: "Test Author",
  date: "2024-12-06"
)

= Introduction

This is a simple Typst document with basic formatting.

== Subsection

Some *bold text* and _italic text_ with `inline code`.

= Features

This document demonstrates:
- Lists
- *Bold* formatting
- _Italic_ text
- Tables
- Links

== Lists

+ First item
+ Second item
+ Third item

== Code

`#let x = 5`

== Tables

#table(
  columns: 2,
  [Header 1], [Header 2],
  [Row 1, Col 1], [Row 1, Col 2],
  [Row 2, Col 1], [Row 2, Col 2],
)

== Links

Visit #link("https://typst.app")[Typst website] for more info.

= Conclusion

This document demonstrates the basic features of Typst.
