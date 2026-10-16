using System.Text;
using XRay.Content.Types;

namespace XRay.Content.Rendering;

/// <summary>Kind of container on the nesting stack.</summary>
public enum NestingKindTag { List, BlockQuote, Group }

public struct NestingKind
{
    public NestingKindTag Tag;
    public bool Ordered;
    public uint ItemCount;

    public static NestingKind ListKind(bool ordered, uint itemCount) =>
        new() { Tag = NestingKindTag.List, Ordered = ordered, ItemCount = itemCount };
    public static NestingKind BlockQuote => new() { Tag = NestingKindTag.BlockQuote };
    public static NestingKind Group => new() { Tag = NestingKindTag.Group };
}

/// <summary>Tracks nesting depth during a linear pass over elements.</summary>
public sealed class RenderState
{
    private readonly List<(ushort Depth, NestingKind Kind)> _stack = new();

    public void PushContainer(NestingKind kind, ushort depth) => _stack.Add((depth, kind));

    public void PopContainer(NestingKindTag tag)
    {
        for (int i = _stack.Count - 1; i >= 0; i--)
        {
            if (_stack[i].Kind.Tag == tag)
            {
                _stack.RemoveAt(i);
                return;
            }
        }
    }

    public void PopToDepth(ushort depth)
    {
        while (_stack.Count > 0 && _stack[^1].Depth >= depth)
            _stack.RemoveAt(_stack.Count - 1);
    }

    public int ListDepth() => _stack.Count(e => e.Kind.Tag == NestingKindTag.List);

    public int BlockquoteDepth() => _stack.Count(e => e.Kind.Tag == NestingKindTag.BlockQuote);

    public uint NextListNumber()
    {
        for (int i = _stack.Count - 1; i >= 0; i--)
        {
            var kind = _stack[i].Kind;
            if (kind.Tag == NestingKindTag.List && kind.Ordered)
            {
                kind.ItemCount += 1;
                _stack[i] = (_stack[i].Depth, kind);
                return kind.ItemCount;
            }
            if (kind.Tag == NestingKindTag.List && !kind.Ordered)
                break;
        }
        return 1;
    }
}

/// <summary>Shared rendering helpers ported from Rust `rendering/common.rs`.</summary>
public static class RenderCommon
{
    // ------------------------------------------------------------------
    // Document-level PDF annotations (upstream issue #63): one section appended to the rendering.
    // ------------------------------------------------------------------

    /// <summary>The label an annotation type is rendered under.</summary>
    internal static string AnnotationTypeLabel(PdfAnnotationType type) => type switch
    {
        PdfAnnotationType.Text => "Text",
        PdfAnnotationType.Highlight => "Highlight",
        PdfAnnotationType.Link => "Link",
        PdfAnnotationType.Stamp => "Stamp",
        PdfAnnotationType.Underline => "Underline",
        PdfAnnotationType.StrikeOut => "StrikeOut",
        PdfAnnotationType.Squiggly => "Squiggly",
        PdfAnnotationType.Ink => "Ink",
        PdfAnnotationType.Square => "Square",
        PdfAnnotationType.Circle => "Circle",
        PdfAnnotationType.Polygon => "Polygon",
        PdfAnnotationType.PolyLine => "PolyLine",
        PdfAnnotationType.Line => "Line",
        PdfAnnotationType.Caret => "Caret",
        PdfAnnotationType.FileAttachment => "FileAttachment",
        PdfAnnotationType.Sound => "Sound",
        PdfAnnotationType.Movie => "Movie",
        _ => "Other",
    };

    /// <summary>
    /// The best text for a rendered annotation: the text a markup annotation covers wins over the
    /// free-form comment or URL, since that is what the annotation is about.
    /// </summary>
    internal static string? AnnotationDisplayText(PdfAnnotation a) =>
        !string.IsNullOrEmpty(a.MarkedText) ? a.MarkedText
        : !string.IsNullOrEmpty(a.Content) ? a.Content
        : null;

    /// <summary>Minimal HTML text escaping: <c>&amp;</c>, <c>&lt;</c>, <c>&gt;</c>.</summary>
    internal static string EscapeHtmlText(string input) =>
        input.IndexOfAny(new[] { '&', '<', '>' }) < 0
            ? input
            : input.Replace("&", "&amp;").Replace("<", "&lt;").Replace(">", "&gt;");

    /// <summary>One line per annotation: <c>{prefix}{type}{suffix} (page N) by author: text</c>.</summary>
    private static string AnnotationLines(
        IReadOnlyList<PdfAnnotation> annotations, string open, string close, bool escapeHtml)
    {
        var sb = new System.Text.StringBuilder();
        foreach (var a in annotations)
        {
            sb.Append(open).Append(AnnotationTypeLabel(a.AnnotationType)).Append(close);
            sb.Append(" (page ").Append(a.PageNumber).Append(')');
            if (!string.IsNullOrEmpty(a.Author))
                sb.Append(" by ").Append(escapeHtml ? EscapeHtmlText(a.Author) : a.Author);
            if (AnnotationDisplayText(a) is { } text)
                sb.Append(": ").Append(escapeHtml ? EscapeHtmlText(text) : text);
            sb.Append(escapeHtml ? "</li>\n" : "\n");
        }
        return sb.ToString();
    }

    internal static string RenderAnnotationsPlain(IReadOnlyList<PdfAnnotation> annotations) =>
        annotations.Count == 0 ? "" : "Annotations:\n" + AnnotationLines(annotations, "", "", false);

    internal static string RenderAnnotationsMarkdown(IReadOnlyList<PdfAnnotation> annotations) =>
        annotations.Count == 0 ? "" : "## Annotations\n\n" + AnnotationLines(annotations, "- **", "**", false);

    internal static string RenderAnnotationsDjot(IReadOnlyList<PdfAnnotation> annotations) =>
        annotations.Count == 0 ? "" : "## Annotations\n\n" + AnnotationLines(annotations, "- _", "_", false) + "\n";

    internal static string RenderAnnotationsHtml(IReadOnlyList<PdfAnnotation> annotations) =>
        annotations.Count == 0
            ? ""
            : "<h2>Annotations</h2>\n<ul>\n" + AnnotationLines(annotations, "<li><strong>", "</strong>", true) + "</ul>\n";

    public static bool IsBodyElement(InternalElement elem) => elem.Layer == ContentLayer.Body;

    public static bool IsContainerEnd(InternalElement elem) => elem.Kind.IsContainerEnd;

    public static bool HandleContainerEnd(ElementKind kind, RenderState state)
    {
        switch (kind.Tag)
        {
            case ElementKindTag.ListEnd:
                state.PopContainer(NestingKindTag.List);
                return true;
            case ElementKindTag.QuoteEnd:
                state.PopContainer(NestingKindTag.BlockQuote);
                return true;
            case ElementKindTag.GroupEnd:
                state.PopContainer(NestingKindTag.Group);
                return true;
            default:
                return false;
        }
    }

    public static string? GetLanguage(InternalElement elem) =>
        elem.Attributes is not null && elem.Attributes.TryGetValue("language", out var v) ? v : null;

    public static string GetAdmonitionKind(InternalElement elem) =>
        elem.Attributes is not null && elem.Attributes.TryGetValue("kind", out var v) ? v : "note";

    public static string? GetAdmonitionTitle(InternalElement elem) =>
        elem.Attributes is not null && elem.Attributes.TryGetValue("title", out var v) ? v : null;

    public static List<(string Key, string Value)> ParseMetadataEntries(string text)
    {
        var result = new List<(string, string)>();
        foreach (var line in SplitLines(text))
        {
            int idx = line.IndexOf(':');
            if (idx < 0) continue;
            string key = line.Substring(0, idx).Trim();
            string value = line.Substring(idx + 1).Trim();
            if (key.Length > 0) result.Add((key, value));
        }
        return result;
    }

    /// <summary>Render a table (Table.cells) as a GFM pipe table. Matches Rust `render_table_markdown`.</summary>
    public static string RenderTableMarkdown(IReadOnlyList<List<string>> cells)
    {
        if (cells.Count == 0) return "";
        int numCols = cells.Max(r => r.Count);
        if (numCols == 0) return "";

        var sb = new StringBuilder();
        var header = cells[0];
        sb.Append('|');
        for (int col = 0; col < numCols; col++)
        {
            sb.Append(' ');
            PushEscapedPipe(sb, col < header.Count ? header[col] : "");
            sb.Append(" |");
        }
        sb.Append('\n');
        sb.Append('|');
        for (int i = 0; i < numCols; i++) sb.Append(" --- |");
        sb.Append('\n');

        for (int r = 1; r < cells.Count; r++)
        {
            var row = cells[r];
            sb.Append('|');
            for (int col = 0; col < numCols; col++)
            {
                sb.Append(' ');
                PushEscapedPipe(sb, col < row.Count ? row[col] : "");
                sb.Append(" |");
            }
            sb.Append('\n');
        }
        return sb.ToString();
    }

    private static void PushEscapedPipe(StringBuilder sb, string content)
    {
        if (!content.Contains('|'))
        {
            sb.Append(content);
            return;
        }
        foreach (var ch in content)
        {
            if (ch == '|') sb.Append("\\|");
            else sb.Append(ch);
        }
    }

    public static string RenderTablePlain(IReadOnlyList<List<string>> cells)
    {
        if (cells.Count == 0) return "";
        var sb = new StringBuilder();
        foreach (var row in cells)
        {
            sb.Append(string.Join(" ", row));
            sb.Append('\n');
        }
        return sb.ToString();
    }

    public static string RenderTableDjot(IReadOnlyList<List<string>> cells) => RenderTableMarkdown(cells);

    public static string NormalizeInlineText(string text)
    {
        var sb = new StringBuilder(text.Length);
        bool prevSpace = false;
        foreach (var ch in text)
        {
            if (ch == '\n' || ch == ' ')
            {
                if (!prevSpace) sb.Append(' ');
                prevSpace = true;
            }
            else if (ch < ' ' && ch != '\t')
            {
                // strip control characters
            }
            else
            {
                prevSpace = false;
                sb.Append(ch);
            }
        }
        return sb.ToString();
    }

    public static void EnsureTrailingNewline(StringBuilder sb)
    {
        if (sb.Length == 0 || sb[^1] != '\n') sb.Append('\n');
    }

    public static string FinalizeOutput(string outStr)
    {
        int trimmedLen = TrimEndLen(outStr);
        if (trimmedLen == 0) return "";
        return outStr.Substring(0, trimmedLen) + "\n";
    }

    public static string ApplyBlockquotePrefix(string text, int depth)
    {
        if (depth == 0) return text;
        string prefix = string.Concat(Enumerable.Repeat("> ", depth));
        var sb = new StringBuilder(text.Length + prefix.Length);
        foreach (var line in SplitLines(text))
        {
            sb.Append(prefix);
            sb.Append(line);
            sb.Append('\n');
        }
        return sb.ToString();
    }

    public static void PushWithBq(StringBuilder sb, string text, int bqDepth)
    {
        if (bqDepth > 0) sb.Append(ApplyBlockquotePrefix(text, bqDepth));
        else sb.Append(text);
    }

    /// <summary>Render text with byte-range annotations. Mirrors Rust `render_annotated_text_with_plain`.</summary>
    public static string RenderAnnotatedTextWithPlain(
        string text, IReadOnlyList<TextAnnotation> annotations,
        Func<string, AnnotationKind, string> emit, Func<string, string> plain)
    {
        if (annotations.Count == 0) return plain(text);

        var bytes = Encoding.UTF8.GetBytes(text);
        var sorted = annotations.OrderBy(a => a.Start).ThenBy(a => a.End).ToList();
        uint len = (uint)bytes.Length;
        uint pos = 0;
        var sb = new StringBuilder(text.Length + 64);

        foreach (var ann in sorted)
        {
            // TextAnnotation offsets are byte offsets that may originate from any extractor, not
            // just the ones that derive them from the exact same buffer and are provably
            // boundary-safe. Nothing here guarantees that in general, so clamp to the nearest char
            // boundary before slicing rather than trusting the offset outright and cutting a
            // codepoint in half.
            uint start = CeilCharBoundary(bytes, Math.Min(ann.Start, len));
            uint end = FloorCharBoundary(bytes, Math.Min(ann.End, len));
            if (start < pos || start >= end) continue;
            if (start > pos)
                sb.Append(plain(Encoding.UTF8.GetString(bytes, (int)pos, (int)(start - pos))));
            string span = Encoding.UTF8.GetString(bytes, (int)start, (int)(end - start));
            sb.Append(emit(span, ann.Kind));
            pos = end;
        }

        if (pos < bytes.Length)
            sb.Append(plain(Encoding.UTF8.GetString(bytes, (int)pos, (int)(len - pos))));

        return sb.ToString();
    }

    public static string RenderAnnotatedText(string text, IReadOnlyList<TextAnnotation> annotations,
        Func<string, AnnotationKind, string> emit) =>
        RenderAnnotatedTextWithPlain(text, annotations, emit, s => s);

    /// <summary>Whether <paramref name="b"/> starts a UTF-8 codepoint (i.e. is not a continuation
    /// byte).</summary>
    private static bool IsCharBoundary(byte b) => (b & 0xC0) != 0x80;

    /// <summary>The nearest char boundary at or after <paramref name="offset"/>. Rust's
    /// <c>str::ceil_char_boundary</c>.</summary>
    private static uint CeilCharBoundary(byte[] bytes, uint offset)
    {
        while (offset < bytes.Length && !IsCharBoundary(bytes[offset])) offset++;
        return offset;
    }

    /// <summary>The nearest char boundary at or before <paramref name="offset"/>. Rust's
    /// <c>str::floor_char_boundary</c>.</summary>
    private static uint FloorCharBoundary(byte[] bytes, uint offset)
    {
        if (offset > bytes.Length) return (uint)bytes.Length;
        while (offset > 0 && offset < bytes.Length && !IsCharBoundary(bytes[offset])) offset--;
        return offset;
    }

    // Rust `str::lines()`: split on '\n', dropping a trailing '\r' on each line, and no trailing empty line.
    internal static IEnumerable<string> SplitLines(string text)
    {
        int start = 0;
        for (int i = 0; i < text.Length; i++)
        {
            if (text[i] == '\n')
            {
                int end = i;
                if (end > start && text[end - 1] == '\r') end--;
                yield return text.Substring(start, end - start);
                start = i + 1;
            }
        }
        if (start < text.Length)
        {
            int end = text.Length;
            if (end > start && text[end - 1] == '\r') end--;
            yield return text.Substring(start, end - start);
        }
    }

    internal static int TrimEndLen(string s)
    {
        int len = s.Length;
        while (len > 0 && char.IsWhiteSpace(s[len - 1])) len--;
        return len;
    }
}

/// <summary>Collected footnote data.</summary>
public sealed class FootnoteEntry
{
    public string Text = "";
    public uint Number;
}

/// <summary>Pre-scans elements and relationships to build sequential footnote numbering.</summary>
public sealed class FootnoteCollector
{
    private readonly Dictionary<uint, uint> _refNumbers = new();
    private readonly List<FootnoteEntry> _definitions = new();

    public FootnoteCollector(InternalDocument doc)
    {
        var defByAnchor = new Dictionary<string, (uint Idx, string Text)>();
        for (int i = 0; i < doc.Elements.Count; i++)
        {
            var elem = doc.Elements[i];
            if (elem.Kind.Tag == ElementKindTag.FootnoteDefinition && elem.Anchor is not null)
                defByAnchor[elem.Anchor] = ((uint)i, elem.Text);
        }

        var refToDefAnchor = new Dictionary<uint, string>();
        foreach (var rel in doc.Relationships)
        {
            if (rel.Kind == RelationshipKind.FootnoteReference)
            {
                if (rel.Target.Key is not null)
                    refToDefAnchor[rel.Source] = rel.Target.Key;
                else if (rel.Target.Index is uint idx && idx < doc.Elements.Count)
                {
                    var tgt = doc.Elements[(int)idx];
                    if (tgt.Anchor is not null) refToDefAnchor[rel.Source] = tgt.Anchor;
                }
            }
        }

        for (int i = 0; i < doc.Elements.Count; i++)
        {
            var elem = doc.Elements[i];
            if (elem.Kind.Tag == ElementKindTag.FootnoteRef)
            {
                uint idx = (uint)i;
                if (!refToDefAnchor.ContainsKey(idx))
                {
                    if (elem.Anchor is not null) refToDefAnchor[idx] = elem.Anchor;
                    else if (elem.Text.Length > 0) refToDefAnchor[idx] = elem.Text;
                }
            }
        }

        var anchorToNumber = new Dictionary<string, uint>();
        uint nextNumber = 1;
        for (int i = 0; i < doc.Elements.Count; i++)
        {
            var elem = doc.Elements[i];
            if (elem.Kind.Tag == ElementKindTag.FootnoteRef)
            {
                uint idx = (uint)i;
                if (refToDefAnchor.TryGetValue(idx, out var anchor))
                {
                    if (!anchorToNumber.TryGetValue(anchor, out var number))
                    {
                        number = nextNumber++;
                        anchorToNumber[anchor] = number;
                        string text = defByAnchor.TryGetValue(anchor, out var d) ? d.Text : "";
                        _definitions.Add(new FootnoteEntry { Text = text, Number = number });
                    }
                    _refNumbers[idx] = number;
                }
            }
        }

        // A definition that no reference points at is still authored content. `_definitions` is
        // populated only from inside the FootnoteRef loop above, so an unreferenced definition
        // never reaches any renderer. Append the orphans after the referenced ones, in document
        // order, continuing the same numbering. (Rust GH#68.)
        foreach (var elem in doc.Elements)
        {
            if (elem.Kind.Tag != ElementKindTag.FootnoteDefinition) continue;
            if (elem.Anchor is not { } anchor) continue;
            if (anchorToNumber.ContainsKey(anchor)) continue;
            uint number = nextNumber++;
            anchorToNumber[anchor] = number;
            _definitions.Add(new FootnoteEntry { Text = elem.Text, Number = number });
        }
    }

    public uint? RefNumber(uint elemIndex) => _refNumbers.TryGetValue(elemIndex, out var n) ? n : null;

    public IReadOnlyList<FootnoteEntry> Definitions => _definitions;
}
