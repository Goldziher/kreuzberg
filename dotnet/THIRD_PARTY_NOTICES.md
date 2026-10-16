# Third-Party Notices — .NET port

X-Ray.Content is licensed under [MIT](../LICENSE). The C# files listed below are
**derivative works of Apache-2.0 licensed Rust crates**, translated to C#. They
remain subject to the Apache License 2.0, a copy of which is in
[`third_party/LICENSE-Apache-2.0.txt`](third_party/LICENSE-Apache-2.0.txt).

This is what Apache-2.0 §2 permits and §4 requires: the license text is
included, each derived file carries a notice naming its source and stating that
it was modified, and the upstream attribution is retained. None of the three
crates ships a `NOTICE` file, so §4(d) does not apply.

> **Note for packagers.** Every other Rust crate the .NET port derives from is
> MIT or dual `MIT OR Apache-2.0` (where MIT can be taken), so the assembly was
> previously MIT throughout. These three are Apache-2.0 *only*, so plain MIT does
> not describe the whole assembly. The package therefore declares
> `<PackageLicenseExpression>MIT AND Apache-2.0</PackageLicenseExpression>` in
> `src/XRay.Content/XRay.Content.csproj`, which is what a consumer of
> `XRay.Content` on NuGet now sees. Both halves are satisfied by shipping the
> repository's [MIT `LICENSE`](../LICENSE) and the Apache-2.0 text in
> [`third_party/`](third_party/LICENSE-Apache-2.0.txt); removing the last
> Apache-2.0-derived file is what would let the expression narrow back to MIT.

## typst-syntax 0.15.1 — Apache-2.0

- Copyright: The Typst Project Developers
- Source: <https://github.com/typst/typst>
- Derived files: `src/XRay.Content/Internal/Math/TypstKind.cs`,
  `src/XRay.Content/Internal/Math/TypstLexer.cs`,
  `src/XRay.Content/Internal/Math/TypstNode.cs`,
  `src/XRay.Content/Internal/Math/TypstParser.cs`,
  and the `default_math_class` overrides in
  `src/XRay.Content/Internal/Math/TypstMathClass.cs` (from `typst-utils` 0.15.1, also
  Apache-2.0 and also by The Typst Project Developers)
- Modifications: the math-mode slice of the crate's lexer, syntax tree, and
  parser translated to C#. Markup mode, spans, the newline modes, incremental
  reparsing, memoization and diagnostics are omitted; only what `parse_math`
  reaches is ported. Code mode, which math enters at a `#`, is reduced to the
  shapes a `#` takes inside math — literals, names, field accesses, calls with
  named and spread arguments, bracketed groups, let bindings, set and show
  rules, and closures — with other keywords consumed rather than modelled.
  Validated at 486 of 487 trees identical to the crate's own parser over every
  `$…$` span in the corpus, the last being a documentation placeholder that
  renders the same either way.

## mathemascii 0.4.0 — Apache-2.0

- Copyright: Nadir Fejzic
- Source: <https://github.com/nfejzic/mathemascii>
- Derived files: `src/XRay.Content/Internal/Math/AsciiMathLexer.cs`,
  `src/XRay.Content/Internal/Math/AsciiMath.cs`,
  `src/XRay.Content/Internal/Math/AsciiMathSymbols.cs`
- Modifications: the scanner, lexer, parser, and AST translated to C#. The
  crate's panics on multi-byte input and on `cancel` are raised as an exception
  rather than aborting, so the caller can drop the equation instead of the
  process.

## alemat 0.8.0 — Apache-2.0

- Copyright: Nadir Fejzic
- Source: <https://github.com/nfejzic/alemat>
- Derived files: `src/XRay.Content/Internal/Math/AsciiMath.cs` (the MathML element
  tree and its writer), `src/XRay.Content/Internal/Math/AsciiMathSymbols.cs` (symbol
  values resolved through the crate's `Ident`/`Operator` dictionaries)
- Modifications: only the element kinds `mathemascii` builds are ported, with
  the builder and writer behaviour of `BufMathMlWriter` reproduced; the crate's
  type-state builders, renderer trait, and unused elements are omitted.

## Dependencies of the optional OCR pass — not derivative works

The sections above cover *translated* code. The OCR pass (see the deviation
section of `../CLAUDE.md`) instead takes a dependency on code it does not derive
from, so nothing here is a derivative work — but it changes what the package
carries, which the packagers' note above is about:

| Package | License | Brings |
|---|---|---|
| `PaddleOCR` 26.8.4668 | Apache-2.0 | the recognizer; SkiaSharp (MIT) transitively |
| `PaddleOCR.Pdf` 26.8.4668 | Apache-2.0 | page rasterisation; `PDFtoImage` (MIT) and PDFium (Apache-2.0) transitively |

Two consequences worth stating plainly. **Native binaries** now reach a consumer
who restores the package — SkiaSharp's and PDFium's — which is the trade-off the
`../CLAUDE.md` deviation section records; the pass is off by default and neither is
loaded until it is enabled. And these are two more Apache-2.0 components in a
package declaring `MIT`, so the mismatch the note above describes now covers
dependencies as well as derived files. That declaration is still left as it is,
for the same reason.

Model weights are **not** redistributed: nothing in this repository or its
package downloads or ships a checkpoint. `OcrOptions.ModelDirectory` names where
one already is, so whoever stages it accepts its own license.

## X-Ray.Psd — not a derivative work

`PsdExtractor` reads Photoshop files through the `X-Ray.Psd` package (MIT; ported from the Patchy
image editor, © Seth A. Robinson and contributors). It is a dependency, not translated code, and
it carries no dependencies or native binaries of its own, so the package's licence expression is
unchanged by it. The test-only `tests/XRay.Content.Tests/SyntheticPsd.cs` is a trimmed adaptation
of Patchy's MIT-licensed test writer `PsdBuilder.cs`; it is not part of the package.
