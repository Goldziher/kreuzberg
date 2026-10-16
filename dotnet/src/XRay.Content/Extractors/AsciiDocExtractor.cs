// Ported from crates/xberg/src/extractors/asciidoc.rs (AsciiDocExtractor + AsciiDocParser).

using System.Text;
using System.Text.Json;
using XRay.Content.Core;
using XRay.Content.Internal.MathMarkup;
using XRay.Content.Types;

namespace XRay.Content.Extractors;

/// <summary>
/// AsciiDoc extractor. Parses the structural subset of AsciiDoc that carries document
/// meaning, rather than flattening the source into undifferentiated paragraphs.
/// </summary>
/// <remarks>
/// Covered: the document header (<c>= Title</c>, the implicit author line, <c>:name: value</c>
/// attributes, which are also substituted into body text as <c>{name}</c>); sections
/// <c>==</c> through <c>======</c>; unordered and ordered lists with nesting; the delimited
/// blocks <c>----</c> listing, <c>....</c> literal, <c>____</c> quote and <c>====</c> example;
/// admonitions in both the inline <c>NOTE:</c> and the <c>[NOTE]</c> block form; <c>|===</c>
/// tables; and the constrained inline spans plus link macros.
/// <para>
/// Malformed input degrades rather than failing: an unterminated block or table is closed at
/// end of input and reported as a warning.
/// </para>
/// </remarks>
public sealed class AsciiDocExtractor : IExtractor
{
    // text/x-asciidoc is an alias of text/asciidoc. The registry resolves by exact string with
    // no alias awareness, so an unclaimed alias would advertise as supported and then fail.
    public IEnumerable<string> SupportedMimeTypes => new[] { "text/asciidoc", "text/x-asciidoc" };

    public int Priority => 50;

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        // Upstream's one-line growth guard: the input's own size is charged against
        // `max_content_size` before parsing, so a document too large to render is refused
        // rather than rendered and then found to be too large.
        SecurityBudget.FromConfig(config).AccountText(content.Length);
        string source = TextTransform.NormalizeLineEndings(Encoding.UTF8.GetString(content));

        var parser = new AsciiDocParser(source);
        var doc = parser.Parse();
        doc.MimeType = mimeType;

        string body = string.Join("\n", doc.Elements.Select(e => e.Text));
        doc.Metadata.Format = FormatMetadata.Text(new TextMetadata
            {
                LineCount = (uint)CountLines(source),
                WordCount = (uint)body.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries).Length,
                // Rust counts `body.chars()` — Unicode scalar values, so a non-BMP character
                // counts once where `string.Length` counts its two UTF-16 code units. These
                // documents are full of them: `𝜎` and `𝜏` come out of the maths.
                CharacterCount = (uint)body.EnumerateRunes().Count(),
                Headers = parser.Headers.Count > 0 ? parser.Headers : null,
                Links = parser.Links.Count > 0 ? parser.Links : null,
                CodeBlocks = parser.CodeBlocks.Count > 0 ? parser.CodeBlocks : null,
            });
        return doc;
    }

    private static int CountLines(string text)
    {
        if (text.Length == 0) return 0;
        int count = 0;
        for (int i = 0; i < text.Length; i++) if (text[i] == '\n') count++;
        return text[^1] == '\n' ? count : count + 1;
    }
}

/// <summary>Line-oriented AsciiDoc parser: one short handler per block form.</summary>
internal sealed class AsciiDocParser
{
    /// <summary>Copies of a delimiter character a line needs before it opens a block.</summary>
    private const int MinDelimiterRun = 4;

    /// <summary>Deepest section level AsciiDoc defines (<c>====== </c> is level 6).</summary>
    private const int MaxSectionLevel = 6;

    private static readonly string[] AdmonitionLabels = { "NOTE", "TIP", "IMPORTANT", "WARNING", "CAUTION" };

    /// <summary>Characters a table cell specifier may contain (<c>2+|</c>, <c>.3+|</c>, <c>a|</c>).</summary>
    private const string CellSpecChars = "0123456789+*.<>";

    private readonly string[] _lines;
    private int _index;
    private readonly InternalDocumentBuilder _builder = new("asciidoc");

    /// <summary>Open list nesting: one entry per level, true where that level is ordered.</summary>
    private readonly List<bool> _listStack = new();

    private readonly Dictionary<string, string> _attributes = new(StringComparer.Ordinal);
    private List<string> _pendingAttrs = new();
    private string? _pendingTitle;
    private string? _title;
    private List<string>? _authors;
    private readonly List<ProcessingWarning> _warnings = new();

    public List<string> Headers { get; } = new();
    public List<MarkdownLink> Links { get; } = new();
    public List<MarkdownCodeBlock> CodeBlocks { get; } = new();

    public AsciiDocParser(string text) => _lines = text.Split('\n');

    public InternalDocument Parse()
    {
        ParseDocumentHeader();
        while (_index < _lines.Length) ParseBlock();
        CloseLists();

        var metadata = new Metadata { Title = _title, Authors = _authors };
        foreach (var (key, value) in _attributes)
            metadata.Additional[$"asciidoc_{key}"] = JsonSerializer.SerializeToElement(value, Json.Options);
        _builder.SetMetadata(metadata);

        var doc = _builder.Build();
        foreach (var warning in _warnings) doc.ProcessingWarnings.Add(warning);
        return doc;
    }

    // ── header ───────────────────────────────────────────────────────────────

    /// <summary>Consume the document header: the title, the implicit author line, attributes.</summary>
    private void ParseDocumentHeader()
    {
        SkipBlankAndCommentLines();
        if (_index >= _lines.Length) return;
        string first = _lines[_index];
        if (!first.StartsWith("= ", StringComparison.Ordinal)) return;

        var (text, annotations) = ParseInline(first[2..].Trim());
        uint idx = _builder.PushTitle(text, null, null);
        if (annotations.Count > 0) _builder.SetAnnotations(idx, annotations);
        Headers.Add(text);
        _title = text;
        _index++;

        // The line straight after the title is the author line, unless it is something else the
        // header can hold. AsciiDoc marks it no other way.
        if (_index < _lines.Length)
        {
            string trimmed = _lines[_index].Trim();
            bool isAuthor = trimmed.Length > 0
                && !trimmed.StartsWith(':') && !trimmed.StartsWith("//") && !trimmed.StartsWith('=');
            if (isAuthor)
            {
                _authors = trimmed.Split(';').Select(a => a.Trim()).ToList();
                _index++;
            }
        }
        ConsumeAttributeEntries();
    }

    private void ConsumeAttributeEntries()
    {
        while (_index < _lines.Length)
        {
            Cancel.ThrowIfRequested();
            string trimmed = _lines[_index].Trim();
            if (trimmed.Length == 0 || trimmed.StartsWith("//", StringComparison.Ordinal)) { _index++; continue; }
            if (ParseAttributeEntry(trimmed) is not { } entry) return;
            _attributes[entry.Name] = entry.Value;
            _index++;
        }
    }

    private void SkipBlankAndCommentLines()
    {
        while (_index < _lines.Length)
        {
            Cancel.ThrowIfRequested();
            string trimmed = _lines[_index].Trim();
            if (trimmed.Length == 0 || (trimmed.StartsWith("//", StringComparison.Ordinal) && !IsDelimiter(trimmed, '/')))
                _index++;
            else return;
        }
    }

    // ── block dispatch ───────────────────────────────────────────────────────

    private void ParseBlock()
    {
        if (_index >= _lines.Length) return;
        string trimmed = _lines[_index].Trim();

        if (trimmed.Length == 0) { _index++; return; }
        if (IsDelimiter(trimmed, '/')) { SkipCommentBlock(); return; }
        if (trimmed.StartsWith("//", StringComparison.Ordinal)) { _index++; return; }
        if (ParseAttributeEntry(trimmed) is { } entry)
        {
            _attributes[entry.Name] = entry.Value;
            _index++;
            return;
        }
        if (trimmed.StartsWith('[') && trimmed.EndsWith(']'))
        {
            _pendingAttrs = ParseBlockAttributes(trimmed);
            _index++;
            return;
        }
        if (ParseBlockTitle(trimmed) is { } blockTitle)
        {
            _pendingTitle = SubstituteAttributes(blockTitle);
            _index++;
            return;
        }
        if (trimmed.StartsWith("|===", StringComparison.Ordinal)) { ParseTable(); return; }
        if (SectionLevel(trimmed) is { } level) { ParseSectionHeading(level, trimmed); return; }
        if (IsDelimiter(trimmed, '+') && PendingMathNotation() is { } blockNotation)
        {
            ParseMathBlock(trimmed, blockNotation);
            return;
        }
        if (IsDelimiter(trimmed, '-') || IsDelimiter(trimmed, '.')) { ParseVerbatimBlock(trimmed); return; }
        if (IsDelimiter(trimmed, '_')) { ParseQuoteBlock(trimmed); return; }
        if (IsDelimiter(trimmed, '=')) { ParseExampleBlock(trimmed); return; }
        if (SplitInlineAdmonition(trimmed) is { } admonition)
        {
            PushAdmonitionWithBody(admonition.Label.ToLowerInvariant(), admonition.Body);
            _index++;
            return;
        }
        if (ListMarker(trimmed) is { } marker) { ParseListItem(marker.Depth, marker.Ordered, trimmed); return; }
        ParseParagraph();
    }

    private void ParseSectionHeading(int level, string trimmed)
    {
        CloseLists();
        var (text, annotations) = ParseInline(trimmed[level..].Trim());
        uint idx = _builder.PushHeading((byte)level, text, null, null);
        if (annotations.Count > 0) _builder.SetAnnotations(idx, annotations);
        Headers.Add(text);
        _index++;
        ClearPending();
    }

    // ── math ─────────────────────────────────────────────────────────────────

    /// <summary>Which notation a math macro or block carries.</summary>
    private enum MathNotation { Latex, AsciiMath }

    /// <summary>
    /// The math notation a pending <c>[latexmath]</c>, <c>[asciimath]</c> or <c>[stem]</c> block
    /// attribute selects. <c>stem</c> follows the document's <c>:stem:</c> attribute, which
    /// AsciiDoc defines as AsciiMath unless the document names <c>latexmath</c>.
    /// </summary>
    private MathNotation? PendingMathNotation()
    {
        if (_pendingAttrs.Count == 0) return null;
        string first = _pendingAttrs[0].Split(',')[0].Trim().ToLowerInvariant();
        return first switch
        {
            "latexmath" => MathNotation.Latex,
            "asciimath" => MathNotation.AsciiMath,
            "stem" => StemNotation(),
            _ => null,
        };
    }

    /// <summary>What <c>stem</c> means in this document.</summary>
    private MathNotation StemNotation()
    {
        if (_attributes.TryGetValue("stem", out string? value))
        {
            string v = value.Trim().ToLowerInvariant();
            if (v == "latexmath" || v == "latex") return MathNotation.Latex;
        }
        return MathNotation.AsciiMath;
    }

    /// <summary>
    /// Parse a <c>++++</c> passthrough block that a math attribute introduced. The body is one
    /// display equation, so it becomes a formula element.
    /// </summary>
    private void ParseMathBlock(string delimiter, MathNotation notation)
    {
        CloseLists();
        _index++;
        var (body, terminated) = CollectUntilDelimiter(delimiter);
        if (!terminated) Warn("unterminated delimited block closed at end of input");
        string? latex = MathToLatex(body.Trim(), notation);
        if (!string.IsNullOrEmpty(latex)) _builder.PushFormula(latex!, null, null);
        ClearPending();
    }

    /// <summary>
    /// Convert math in <paramref name="notation"/> to LaTeX. LaTeX passes through with its
    /// delimiters removed, since the formula element holds bare LaTeX; AsciiMath goes through
    /// the shared converter.
    /// </summary>
    private static string? MathToLatex(string source, MathNotation notation)
    {
        if (source.Length == 0) return null;
        if (notation == MathNotation.Latex)
        {
            string bare = MathMl.StripMathDelimiters(source).Trim();
            return bare.Length == 0 ? null : bare;
        }
        return AsciiMath.ConvertToLatex(source);
    }

    /// <summary>
    /// Parse an inline math macro: <c>latexmath:[…]</c>, <c>asciimath:[…]</c> or
    /// <c>stem:[…]</c>, whose notation the document's <c>:stem:</c> attribute selects.
    /// </summary>
    /// <remarks>
    /// Returns the consumed length, the macro's content and its notation. The content may hold
    /// nested brackets, so the scan tracks depth.
    /// </remarks>
    private (int Consumed, string Source, MathNotation Notation)? ParseMathMacro(string text, int pos)
    {
        string? name = null;
        MathNotation notation = MathNotation.Latex;
        foreach (var (candidate, kind) in new[]
                 {
                     ("latexmath", MathNotation.Latex),
                     ("asciimath", MathNotation.AsciiMath),
                     ("stem", StemNotation()),
                 })
        {
            string prefix = candidate + ":[";
            if (string.CompareOrdinal(text, pos, prefix, 0, prefix.Length) != 0) continue;
            name = candidate;
            notation = kind;
            break;
        }
        if (name is null) return null;

        int open = pos + name.Length + 2;
        int depth = 1;
        for (int i = open; i < text.Length; i++)
        {
            if (text[i] == '[') depth++;
            else if (text[i] == ']')
            {
                depth--;
                if (depth == 0) return (i + 1 - pos, text[open..i], notation);
            }
        }
        return null;
    }

    /// <summary>A <c>----</c> listing block or a <c>....</c> literal block: verbatim code.</summary>
    private void ParseVerbatimBlock(string delimiter)
    {
        CloseLists();
        string? language = PendingSourceLanguage();
        string? title = _pendingTitle;
        _pendingTitle = null;
        _index++;
        var (body, terminated) = CollectUntilDelimiter(delimiter);
        if (!terminated) Warn("unterminated delimited block closed at end of input");

        uint idx = _builder.PushCode(body, language, null, null);
        if (title is not null) _builder.SetAttributes(idx, new Dictionary<string, string> { ["title"] = title });
        CodeBlocks.Add(new MarkdownCodeBlock { Language = language ?? "", Code = body });
        _pendingAttrs.Clear();
    }

    private void ParseQuoteBlock(string delimiter)
    {
        CloseLists();
        _index++;
        var (body, terminated) = CollectUntilDelimiter(delimiter);
        if (!terminated) Warn("unterminated quote block closed at end of input");

        _builder.PushQuoteStart();
        foreach (string paragraph in body.Split("\n\n"))
        {
            string trimmed = paragraph.Trim();
            if (trimmed.Length == 0) continue;
            var (text, annotations) = ParseInline(trimmed.Replace('\n', ' '));
            _builder.PushParagraph(text, annotations, null, null);
        }
        _builder.PushQuoteEnd();
        ClearPending();
    }

    /// <summary>
    /// An <c>====</c> example block. With a preceding <c>[NOTE]</c>-style attribute it is an
    /// admonition; otherwise its body is ordinary prose.
    /// </summary>
    private void ParseExampleBlock(string delimiter)
    {
        CloseLists();
        string? admonition = PendingAdmonitionKind();
        _index++;
        var (body, terminated) = CollectUntilDelimiter(delimiter);
        if (!terminated) Warn("unterminated example block closed at end of input");

        string joined = string.Join(" ", body.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries));
        if (admonition is not null)
        {
            PushAdmonitionWithBody(admonition, joined);
        }
        else if (joined.Length > 0)
        {
            var (text, annotations) = ParseInline(joined);
            _builder.PushParagraph(text, annotations, null, null);
        }
        ClearPending();
    }

    private void PushAdmonitionWithBody(string kind, string body)
    {
        CloseLists();
        uint idx = _builder.PushAdmonition(kind, null, null);
        var (text, annotations) = ParseInline(body.Trim());
        if (text.Length > 0) _builder.SetText(idx, text);
        if (annotations.Count > 0) _builder.SetAnnotations(idx, annotations);
        ClearPending();
    }

    /// <summary>A <c>|===</c> delimited table.</summary>
    private void ParseTable()
    {
        CloseLists();
        int? expectedColumns = PendingColumnCount();
        string? title = _pendingTitle;
        _pendingTitle = null;
        _index++;

        var cells = new List<string>();
        int firstRowWidth = 0;
        bool terminated = false;
        while (_index < _lines.Length)
        {
            Cancel.ThrowIfRequested();
            string trimmed = _lines[_index].Trim();
            _index++;
            if (trimmed.StartsWith("|===", StringComparison.Ordinal)) { terminated = true; break; }
            if (trimmed.Length == 0) continue;
            if (trimmed.StartsWith('|'))
            {
                var row = SplitTableRow(trimmed);
                if (firstRowWidth == 0) firstRowWidth = row.Count;
                foreach (string cell in row) cells.Add(ParseInline(cell).Text);
            }
            else if (cells.Count > 0)
            {
                // A cell's text may run over several lines; it belongs to the cell above it.
                cells[^1] = cells[^1] + " " + trimmed;
            }
        }
        if (!terminated) Warn("unterminated table block closed at end of input");

        int columns = Math.Max(expectedColumns ?? firstRowWidth, 1);
        var grid = new List<List<string>>();
        for (int i = 0; i < cells.Count; i += columns)
            grid.Add(cells.GetRange(i, Math.Min(columns, cells.Count - i)));
        if (grid.Count == 0) { _pendingAttrs.Clear(); return; }

        uint idx = _builder.PushTableFromCells(grid, null, null);
        if (title is not null) _builder.SetAttributes(idx, new Dictionary<string, string> { ["title"] = title });
        _pendingAttrs.Clear();
    }

    private void ParseListItem(int depth, bool ordered, string trimmed)
    {
        OpenLists(depth, ordered);
        int space = trimmed.IndexOfAny(new[] { ' ', '\t' });
        var text = new StringBuilder(space < 0 ? "" : trimmed[(space + 1)..].Trim());
        _index++;
        // An item's text continues onto following lines until something else starts.
        while (_index < _lines.Length)
        {
            Cancel.ThrowIfRequested();
            string next = _lines[_index].Trim();
            if (next.Length == 0 || ListMarker(next) is not null || SectionLevel(next) is not null) break;
            text.Append(' ').Append(next);
            _index++;
        }
        var (itemText, annotations) = ParseInline(text.ToString());
        _builder.PushListItem(itemText, ordered, annotations, null, null);
        ClearPending();
    }

    private void ParseParagraph()
    {
        CloseLists();
        var text = new StringBuilder();
        while (_index < _lines.Length)
        {
            Cancel.ThrowIfRequested();
            string trimmed = _lines[_index].Trim();
            if (trimmed.Length == 0 || StartsNewBlock(trimmed)) break;
            if (text.Length > 0) text.Append(' ');
            text.Append(trimmed);
            _index++;
        }
        if (text.Length == 0) { _index++; return; }

        var (paragraphText, annotations) = ParseInline(text.ToString());
        _builder.PushParagraph(paragraphText, annotations, null, null);
        ClearPending();
    }

    /// <summary>Whether a line opens a block that must not be absorbed into a paragraph.</summary>
    private bool StartsNewBlock(string trimmed) =>
        SectionLevel(trimmed) is not null
        || ListMarker(trimmed) is not null
        || trimmed.StartsWith("|===", StringComparison.Ordinal)
        || trimmed.StartsWith("//", StringComparison.Ordinal)
        || ParseAttributeEntry(trimmed) is not null
        || ParseBlockTitle(trimmed) is not null
        || (trimmed.StartsWith('[') && trimmed.EndsWith(']'))
        || IsDelimiter(trimmed, '-')
        || IsDelimiter(trimmed, '.')
        || IsDelimiter(trimmed, '=')
        || IsDelimiter(trimmed, '_')
        || SplitInlineAdmonition(trimmed) is not null;

    /// <summary>
    /// Collect raw lines until one equals <paramref name="delimiter"/>, reporting whether the
    /// closing delimiter was actually found.
    /// </summary>
    private (string Body, bool Terminated) CollectUntilDelimiter(string delimiter)
    {
        var body = new List<string>();
        while (_index < _lines.Length)
        {
            Cancel.ThrowIfRequested();
            string raw = _lines[_index];
            _index++;
            if (raw.Trim() == delimiter) return (string.Join("\n", body), true);
            body.Add(raw);
        }
        return (string.Join("\n", body), false);
    }

    private void SkipCommentBlock()
    {
        _index++;
        while (_index < _lines.Length)
        {
            Cancel.ThrowIfRequested();
            string raw = _lines[_index];
            _index++;
            if (IsDelimiter(raw.Trim(), '/')) return;
        }
        Warn("unterminated comment block closed at end of input");
    }

    // ── list nesting ─────────────────────────────────────────────────────────

    private void OpenLists(int depth, bool ordered)
    {
        while (_listStack.Count > depth) { _builder.EndList(); _listStack.RemoveAt(_listStack.Count - 1); }
        // A level that switches between numbered and bulleted is a different list, not a
        // continuation of the one already open at that depth.
        if (_listStack.Count == depth && _listStack.Count > 0 && _listStack[^1] != ordered)
        {
            _builder.EndList();
            _listStack.RemoveAt(_listStack.Count - 1);
        }
        while (_listStack.Count < depth) { _builder.PushList(ordered); _listStack.Add(ordered); }
    }

    private void CloseLists()
    {
        while (_listStack.Count > 0) { _builder.EndList(); _listStack.RemoveAt(_listStack.Count - 1); }
    }

    // ── pending block attributes ─────────────────────────────────────────────

    private void ClearPending()
    {
        _pendingAttrs.Clear();
        _pendingTitle = null;
    }

    private string? PendingSourceLanguage()
    {
        if (_pendingAttrs.Count == 0) return null;
        if (!string.Equals(_pendingAttrs[0], "source", StringComparison.OrdinalIgnoreCase)) return null;
        return _pendingAttrs.Count > 1 && _pendingAttrs[1].Length > 0 ? _pendingAttrs[1] : null;
    }

    private string? PendingAdmonitionKind()
    {
        if (_pendingAttrs.Count == 0) return null;
        string first = _pendingAttrs[0];
        return AdmonitionLabels
            .FirstOrDefault(label => string.Equals(first, label, StringComparison.OrdinalIgnoreCase))
            ?.ToLowerInvariant();
    }

    private int? PendingColumnCount()
    {
        foreach (string attr in _pendingAttrs)
        {
            if (!attr.StartsWith("cols=", StringComparison.Ordinal)) continue;
            int count = CountColumns(attr[5..].Trim('"'));
            if (count > 0) return count;
        }
        return null;
    }

    private void Warn(string message) =>
        _warnings.Add(new ProcessingWarning { Source = "asciidoc", Message = message });

    // ── inline ───────────────────────────────────────────────────────────────

    /// <summary>
    /// Replace <c>{name}</c> references with defined document attributes.
    /// </summary>
    /// <remarks>An undefined reference is left verbatim: AsciiDoc treats it as literal text, and
    /// dropping it would silently lose content.</remarks>
    private string SubstituteAttributes(string text)
    {
        if (!text.Contains('{') || _attributes.Count == 0) return text;
        var outBuf = new StringBuilder(text.Length);
        ReadOnlySpan<char> rest = text;
        while (true)
        {
            int open = rest.IndexOf('{');
            if (open < 0) break;
            outBuf.Append(rest[..open]);
            var after = rest[(open + 1)..];
            int close = after.IndexOf('}');
            if (close < 0) { outBuf.Append('{'); rest = after; break; }
            string name = after[..close].ToString();
            if (_attributes.TryGetValue(name, out string? value)) outBuf.Append(value);
            else outBuf.Append('{').Append(name).Append('}');
            rest = after[(close + 1)..];
        }
        outBuf.Append(rest);
        return outBuf.ToString();
    }

    /// <summary>Strip AsciiDoc inline markup, producing plain text plus annotations.</summary>
    private (string Text, List<TextAnnotation> Annotations) ParseInline(string raw)
    {
        string substituted = SubstituteAttributes(raw);
        var outBuf = new StringBuilder(substituted.Length);
        var annotations = new List<TextAnnotation>();
        int pos = 0;
        // A constrained span may only open at the start of the text or after whitespace or an
        // opening bracket, so `midword*not*bold` stays literal.
        bool atBoundary = true;

        while (pos < substituted.Length)
        {
            // An inline math macro stays in the sentence, as inline math does for markdown,
            // but reaches the text as LaTeX between `$` delimiters rather than as the raw macro.
            if (ParseMathMacro(substituted, pos) is { } math)
            {
                string? latex = MathToLatex(math.Source, math.Notation);
                if (latex is not null) outBuf.Append('$').Append(latex).Append('$');
                pos += math.Consumed;
                atBoundary = false;
                continue;
            }
            if (ParseLinkMacro(substituted, pos) is { } link)
            {
                uint start = (uint)Utf8Length(outBuf);
                outBuf.Append(link.Display);
                annotations.Add(new TextAnnotation
                {
                    Start = start,
                    End = (uint)Utf8Length(outBuf),
                    Kind = new AnnotationKind { Which = AnnotationKind.Tag.Link, Url = link.Url },
                });
                Links.Add(new MarkdownLink { Text = link.Display, Url = link.Url });
                pos += link.Consumed;
                atBoundary = false;
                continue;
            }
            if (atBoundary && ParseConstrainedSpan(substituted, pos) is { } span)
            {
                uint start = (uint)Utf8Length(outBuf);
                outBuf.Append(span.Inner);
                annotations.Add(new TextAnnotation
                {
                    Start = start,
                    End = (uint)Utf8Length(outBuf),
                    Kind = span.Kind,
                });
                pos += span.Consumed;
                atBoundary = false;
                continue;
            }
            char ch = substituted[pos];
            outBuf.Append(ch);
            atBoundary = char.IsWhiteSpace(ch) || ch == '(' || ch == '[';
            pos++;
        }

        return (outBuf.ToString(), annotations);
    }

    private static int Utf8Length(StringBuilder sb) => Encoding.UTF8.GetByteCount(sb.ToString());

    // ── line-shape helpers ───────────────────────────────────────────────────

    /// <summary>True when a line is a run of at least four copies of one delimiter character.</summary>
    private static bool IsDelimiter(string line, char ch) =>
        line.Length >= MinDelimiterRun && line.All(c => c == ch);

    /// <summary>The section level of a <c>== Heading</c> line: its count of leading equals signs.</summary>
    private static int? SectionLevel(string line)
    {
        int equals = 0;
        while (equals < line.Length && line[equals] == '=') equals++;
        if (equals == 0 || equals > MaxSectionLevel) return null;
        string rest = line[equals..];
        return rest.StartsWith(' ') && rest.Trim().Length > 0 ? equals : null;
    }

    private static (string Name, string Value)? ParseAttributeEntry(string line)
    {
        if (!line.StartsWith(':')) return null;
        string rest = line[1..];
        int colon = rest.IndexOf(':');
        if (colon < 0) return null;
        string name = rest[..colon];
        if (name.Length == 0 || name.Any(char.IsWhiteSpace)) return null;
        return (name.Trim('!'), rest[(colon + 1)..].Trim());
    }

    /// <summary>Split a <c>[source,rust]</c> or <c>[cols="2*",options="header"]</c> line.</summary>
    private static List<string> ParseBlockAttributes(string line)
    {
        string inner = line.TrimStart('[').TrimEnd(']');
        var parts = new List<string>();
        var current = new StringBuilder();
        bool inQuotes = false;
        foreach (char ch in inner)
        {
            if (ch == '"') inQuotes = !inQuotes;
            else if (ch == ',' && !inQuotes) { parts.Add(current.ToString().Trim()); current.Clear(); }
            else current.Append(ch);
        }
        parts.Add(current.ToString().Trim());
        return parts;
    }

    /// <summary>A <c>.Block title</c> line. <c>. item</c> is an ordered list, and <c>..</c> is not a title.</summary>
    private static string? ParseBlockTitle(string line)
    {
        if (!line.StartsWith('.') || line.Length < 2) return null;
        char first = line[1];
        return char.IsWhiteSpace(first) || first == '.' ? null : line[1..];
    }

    private static (string Label, string Body)? SplitInlineAdmonition(string line)
    {
        foreach (string label in AdmonitionLabels)
        {
            if (!line.StartsWith(label, StringComparison.Ordinal)) continue;
            string rest = line[label.Length..];
            if (!rest.StartsWith(": ", StringComparison.Ordinal)) continue;
            return (label, rest[2..].Trim());
        }
        return null;
    }

    /// <summary>Nesting depth (one-based) and orderedness of a list marker.</summary>
    private static (int Depth, bool Ordered)? ListMarker(string line)
    {
        if (line.Length == 0) return null;
        char first = line[0];
        bool ordered;
        switch (first)
        {
            case '*': ordered = false; break;
            case '.': ordered = true; break;
            // A dash marker has no repeated form: "--" is a block delimiter, not depth two.
            case '-': return line.StartsWith("- ", StringComparison.Ordinal) ? (1, false) : null;
            default: return null;
        }
        int depth = 0;
        while (depth < line.Length && line[depth] == first) depth++;
        string rest = line[depth..];
        return depth > 0 && rest.StartsWith(' ') && rest.Trim().Length > 0 ? (depth, ordered) : null;
    }

    /// <summary>Count declared columns from a <c>cols</c> spec such as <c>"&gt;,&lt;,^"</c> or <c>"3*"</c>.</summary>
    private static int CountColumns(string spec)
    {
        int total = 0;
        foreach (string item in spec.Split(','))
        {
            string trimmed = item.Trim();
            int star = trimmed.IndexOf('*');
            if (star >= 0 && int.TryParse(trimmed[..star].Trim(), out int repeat)) total += repeat;
            else total += 1;
        }
        return total;
    }

    /// <summary>Split a <c>|a |b |c</c> row into cells, discarding specifiers such as <c>2+</c>.</summary>
    private static List<string> SplitTableRow(string line)
    {
        var cells = new List<string>();
        var fragments = line.Split('|');
        for (int i = 1; i < fragments.Length; i++)
        {
            string trimmed = fragments[i].Trim();
            bool isSpec = trimmed.Length > 0 && trimmed.Length <= 4
                && trimmed.All(c => CellSpecChars.Contains(c) || c == 'a');
            if (isSpec && trimmed.Any(c => c is '+' or '*')) continue;
            cells.Add(trimmed);
        }
        return cells;
    }

    /// <summary>Parse <c>link:target[text]</c>, <c>https://url[text]</c> or a bare URL.</summary>
    private static (int Consumed, string Display, string Url)? ParseLinkMacro(string text, int pos)
    {
        string[] schemes = { "link:", "https://", "http://", "mailto:" };
        string? scheme = schemes.FirstOrDefault(s => string.CompareOrdinal(text, pos, s, 0, s.Length) == 0);
        if (scheme is null) return null;

        int afterScheme = pos + scheme.Length;
        int targetEnd = afterScheme;
        while (targetEnd < text.Length && !char.IsWhiteSpace(text[targetEnd]) && text[targetEnd] != '[') targetEnd++;
        string target = text[afterScheme..targetEnd];
        if (target.Length == 0) return null;
        string url = scheme == "link:" ? target : scheme + target;

        if (targetEnd < text.Length && text[targetEnd] == '[')
        {
            int close = text.IndexOf(']', targetEnd + 1);
            if (close >= 0)
            {
                string display = text[(targetEnd + 1)..close].Trim();
                if (display.Length == 0) display = url;
                return (close + 1 - pos, display, url);
            }
        }
        // `link:` without a bracketed target is not a link macro.
        if (scheme == "link:") return null;

        // A bare URL displays itself; sentence punctuation at its end is not part of it.
        url = url.TrimEnd('.', ',', ';', ':', ')');
        if (url.Length <= scheme.Length) return null;
        return (url.Length, url, url);
    }

    /// <summary>Parse a constrained <c>*strong*</c>, <c>_emphasis_</c> or <c>`mono`</c> span.</summary>
    private static (int Consumed, string Inner, AnnotationKind Kind)? ParseConstrainedSpan(string text, int pos)
    {
        if (pos >= text.Length) return null;
        char marker = text[pos];
        AnnotationKind kind;
        switch (marker)
        {
            case '*': kind = AnnotationKind.Bold; break;
            case '_': kind = AnnotationKind.Italic; break;
            case '`': kind = new AnnotationKind { Which = AnnotationKind.Tag.Code }; break;
            default: return null;
        }
        int afterMarker = pos + 1;
        if (afterMarker >= text.Length || char.IsWhiteSpace(text[afterMarker])) return null;
        int close = text.IndexOf(marker, afterMarker);
        if (close < 0) return null;
        string inner = text[afterMarker..close];
        if (inner.Length == 0 || char.IsWhiteSpace(inner[^1])) return null;
        return (close + 1 - pos, inner, kind);
    }
}
