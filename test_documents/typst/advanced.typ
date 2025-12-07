#set document(
  title: "Advanced Typst Features",
  author: "Advanced Author",
  keywords: ("typst", "markup", "document"),
  date: "2024-12-06"
)

#set heading(numbering: "1.")

= Mathematical Notation

The equation $x = (-b plus.minus sqrt(b^2 - 4a c)) / (2a)$ is well-known.

Display math:
$ x^2 + y^2 = r^2 $

= Formatting Showcase

Different types of *bold*, _italic_, and `code` can be combined:
- *_bold and italic_*
- `#set heading(numbering: "1.")`

= Structured Content

== Code Blocks

```python
def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n-1) + fibonacci(n-2)
```

== Nested Headings

=== Level 3 heading

This is under a level 3 heading.

==== Level 4 heading

And this is level 4.

= Tables and Data

#table(
  columns: (1fr, 1fr, 1fr),
  [Name], [Age], [City],
  [Alice], [30], [New York],
  [Bob], [25], [San Francisco],
  [Carol], [35], [Boston],
)

= Multiple Paragraphs

First paragraph here.

Second paragraph with some _emphasized_ text.

Third paragraph with a #link("https://example.com")[link to example].

= Conclusion

This demonstrates various Typst features for testing purposes.
