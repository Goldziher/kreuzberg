using System.Text;
using System.Text.Json;
using XRay.Content.Core;
using XRay.Content.Internal.Markup;
using XRay.Content.Types;

namespace XRay.Content.Extractors;

/// <summary>
/// reStructuredText (RST) extractor. Ported from Rust `extractors/rst.rs`.
/// Builds the InternalDocument via a line-based parser and derives field-list metadata.
/// </summary>
public sealed class RstExtractor : IExtractor
{
    public IEnumerable<string> SupportedMimeTypes => new[] { "text/x-rst", "text/prs.fallenstein.rst" };
    public int Priority => 50;

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        // Upstream's one-line growth guard: the input's own size is charged against
        // `max_content_size` before parsing, so a document too large to render is refused
        // rather than rendered and then found to be too large.
        SecurityBudget.FromConfig(config).AccountText(content.Length);
        string text = Encoding.UTF8.GetString(content);
        bool injectPlaceholders = true;

        var metadata = ExtractMetadata(text);

        // Tables are parsed in place inside BuildInternalDocument, which produces table elements
        // positioned where the table actually sits. A second pass raw-pushing the same tables
        // added an unreferenced — and for grid tables less accurate — duplicate of every one of
        // them without contributing anything to the rendered output.
        var doc = BuildInternalDocument(text, injectPlaceholders);
        doc.MimeType = mimeType;
        doc.Metadata = metadata;
        return doc;
    }

    // ── metadata ──────────────────────────────────────────────────────────

    private static Metadata ExtractMetadata(string content)
    {
        var additional = new Dictionary<string, string>();
        ExtractTextFromRst(content, additional);

        var meta = new Metadata();
        if (additional.Remove("title", out var title)) meta.Title = title;
        if (additional.Remove("author", out var author)) meta.Authors = new List<string> { author };
        if (additional.Remove("date", out var date)) meta.CreatedAt = date;

        foreach (var (k, v) in additional)
            meta.Additional[k] = JsonSerializer.SerializeToElement(v, Json.Options);
        return meta;
    }

    private static void AddMetadataField(string key, string value, Dictionary<string, string> meta)
    {
        string kl = key.ToLowerInvariant();
        switch (kl)
        {
            case "author":
            case "authors": meta["author"] = value; break;
            case "date": meta["date"] = value; break;
            case "version":
            case "revision": meta["version"] = value; break;
            case "title": meta["title"] = value; break;
            default: meta[$"field_{kl}"] = value; break;
        }
    }

    // Ported from extract_text_from_rst; only the metadata side effects are consumed.
    private static void ExtractTextFromRst(string content, Dictionary<string, string> meta)
    {
        var lines = MarkupHelpers.Lines(content);
        int i = 0;
        while (i < lines.Count)
        {
            Cancel.ThrowIfRequested();
            string line = lines[i];
            string trimmed = line.Trim();

            if (trimmed.StartsWith(':') && line.Contains(':') && ParseFieldListLine(line) is var (key, value) && key is not null)
            {
                string fullValue = value!;
                while (i + 1 < lines.Count)
                {
                    string next = lines[i + 1];
                    if (next.Length != 0 && (next.StartsWith("   ") || next.StartsWith("\t")))
                    {
                        fullValue += "\n" + next;
                        i++;
                    }
                    else break;
                }
                AddMetadataField(key!, fullValue, meta);
                i++;
                continue;
            }

            if (IsSectionUnderline(line.Trim()) && i + 2 < lines.Count && lines[i + 1].Trim().Length != 0 && IsSectionUnderline(lines[i + 2]))
            {
                char oc = FirstOr(line.Trim(), '=');
                char uc = FirstOr(lines[i + 2].Trim(), '=');
                if (oc == uc) { i += 3; continue; }
            }

            if (i + 1 < lines.Count && IsSectionUnderline(lines[i + 1]) && line.Trim().Length != 0)
            { i += 2; continue; }

            if (trimmed.StartsWith(".. code-block::") || trimmed.StartsWith(".. code::"))
            {
                i++;
                while (i < lines.Count && lines[i].Trim().Length == 0) i++;
                while (i < lines.Count && (lines[i].StartsWith("   ") || lines[i].Length == 0)) i++;
                continue;
            }

            if (trimmed.StartsWith(".. highlight::")) { i++; continue; }

            if (trimmed.EndsWith("::") && !trimmed.StartsWith(".. "))
            {
                i++;
                while (i < lines.Count && (lines[i].StartsWith("    ") || lines[i].Length == 0)) i++;
                continue;
            }

            if (IsListItem(line)) { i++; continue; }

            if (trimmed.StartsWith(".. ") || trimmed == "..")
            {
                string directive = trimmed == ".." ? "" : trimmed.Substring(3);
                if (directive.StartsWith("image::")) { i++; continue; }
                if (StartsWithAny(directive, "note::", "warning::", "important::", "caution::", "hint::", "tip::"))
                {
                    i++;
                    while (i < lines.Count && (lines[i].StartsWith("   ") || lines[i].Length == 0)) i++;
                    continue;
                }
                if (directive.StartsWith("math::"))
                {
                    // Upstream's text path emits the block as `$$…$$` here (it used to write a
                    // literal `math: ` prose marker). Only this pass's metadata side effects are
                    // consumed, so the block is still just skipped; the formula itself is built
                    // on the builder path below, which does carry the `aligned` wrapper.
                    i++;
                    while (i < lines.Count && (lines[i].StartsWith("   ") || lines[i].Length == 0)) i++;
                    continue;
                }
                i++;
                while (i < lines.Count && (lines[i].StartsWith("   ") || lines[i].Length == 0)) i++;
                continue;
            }

            i++;
        }
    }

    /// <summary>
    /// Wrap a <c>.. math::</c> block that uses alignment columns in an <c>aligned</c>
    /// environment. Sphinx renders such a block inside an align environment, so a bare
    /// <c>E &amp;= mc^2 \\ F &amp;= \pi E</c> is only valid LaTeX with the wrapper.
    /// </summary>
    internal static string WrapAlignedMath(string content)
    {
        bool hasAlignment = content.Contains('&') && !content.Contains("\\&");
        return hasAlignment && !content.Contains("\\begin{")
            ? $"\\begin{{aligned}}{content}\\end{{aligned}}"
            : content;
    }

    private static (string? key, string? value) ParseFieldListLine(string line)
    {
        string trimmed = line.Trim();
        if (!trimmed.StartsWith(':')) return (null, null);
        string rest = trimmed.Substring(1);
        int end = rest.IndexOf(':');
        if (end < 0) return (null, null);
        string key = rest.Substring(0, end);
        if (key.Length == 0) return (null, null);
        string value = rest.Substring(end + 1).Trim();
        return (key, value);
    }

    // ── predicates ────────────────────────────────────────────────────────

    private static bool IsSectionUnderline(string line)
    {
        string t = line.Trim();
        if (t.Length < 3) return false;
        char first = t[0];
        return "=-~+^\"`#*".IndexOf(first) >= 0 && t.All(c => c == first);
    }

    private static bool IsListItem(string line)
    {
        string t = line.TrimStart();
        if (t.StartsWith("* ") || t.StartsWith("+ ") || t.StartsWith("- ")
            || t.StartsWith("*\t") || t.StartsWith("+\t") || t.StartsWith("-\t")) return true;
        if (t.StartsWith("#. ") || t.StartsWith("#.\t") || t.StartsWith("(#) ") || t.StartsWith("(#)\t")) return true;
        if (t.StartsWith("("))
        {
            int close = t.IndexOf(')');
            if (close > 1 && close < 6)
            {
                string inner = t.Substring(1, close - 1);
                string after = t.Substring(close + 1);
                if ((after.StartsWith(" ") || after.StartsWith("\t")) && (inner.All(char.IsLetterOrDigit) || inner == "#"))
                    return true;
            }
        }
        int sep = t.IndexOfAny(new[] { ' ', '\t' });
        if (sep > 0 && sep < 6)
        {
            string prefix = t.Substring(0, sep);
            if (prefix.EndsWith('.') || prefix.EndsWith(')'))
            {
                string body = prefix.Substring(0, prefix.Length - 1);
                if (body.Length > 0 && body.All(c => c is >= '0' and <= '9')) return true;
                if (body.Length is > 0 and <= 3 && body.All(c => (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z'))) return true;
            }
        }
        return false;
    }

    private static bool IsMarkupLine(string line)
    {
        string t = line.Trim();
        if (t.Length < 3) return false;
        char first = t[0];
        return t.All(c => c == first) && "=-~+^\"`#*/".IndexOf(first) >= 0;
    }

    // ── tables ────────────────────────────────────────────────────────────

    private static bool IsSimpleTableSeparator(string line)
    {
        string t = line.Trim();
        if (t.Length < 3) return false;
        if (!t.All(c => c == '=' || c == ' ')) return false;
        return t.Contains('=');
    }

    private static List<List<string>> ParseSimpleTableCells(List<string> lines)
    {
        if (lines.Count == 0) return new();
        var ranges = SimpleTableColumnRanges(lines[0]);
        if (ranges.Count == 0) return new();
        var cells = new List<List<string>>();
        foreach (var line in lines)
        {
            if (IsSimpleTableSeparator(line.Trim())) continue;
            byte[] bytes = Encoding.UTF8.GetBytes(line);
            var row = new List<string>();
            foreach (var (start, end) in ranges)
            {
                // The ranges are byte offsets counted against the ASCII ("="/" " only) separator
                // line, but this is an arbitrary data row the document controls and may contain
                // multi-byte UTF-8. A column boundary between the separator's columns is not
                // guaranteed to land on a char boundary in a non-ASCII row, so snap inward rather
                // than cutting a codepoint in half.
                int e = Math.Min(end, bytes.Length);
                int s = Math.Min(start, bytes.Length);
                while (s < e && !IsUtf8CharBoundary(bytes, s)) s++;
                while (e > s && !IsUtf8CharBoundary(bytes, e)) e--;
                row.Add(s >= e ? "" : Encoding.UTF8.GetString(bytes, s, e - s).Trim());
            }
            if (row.Any(c => c.Length != 0)) cells.Add(row);
        }
        return cells;
    }

    /// <summary>Whether <paramref name="offset"/> starts a UTF-8 codepoint in
    /// <paramref name="bytes"/> (i.e. is not a continuation byte).</summary>
    private static bool IsUtf8CharBoundary(byte[] bytes, int offset) =>
        offset >= bytes.Length || (bytes[offset] & 0xC0) != 0x80;

    private static List<(int, int)> SimpleTableColumnRanges(string separator)
    {
        var ranges = new List<(int, int)>();
        byte[] bytes = Encoding.UTF8.GetBytes(separator);
        int i = 0;
        while (i < bytes.Length)
        {
            if (bytes[i] == (byte)'=')
            {
                int start = i;
                while (i < bytes.Length && bytes[i] == (byte)'=') i++;
                ranges.Add((start, i));
            }
            else i++;
        }
        return ranges;
    }

    private static List<List<string>> ParseGridTableCells(List<string> lines)
    {
        var cells = new List<List<string>>();
        foreach (var line in lines)
        {
            string content = line.Trim().Trim('|');
            if (content.Length == 0) continue;
            if (content.All(c => c is '-' or '=' or '+' or '|' or ' ')) continue;
            var row = content.Split('|').Select(s => s.Trim()).Where(s => s.Length != 0).ToList();
            if (row.Count > 0) cells.Add(row);
        }
        return cells;
    }

    private static string CellsToMarkdown(List<List<string>> cells)
    {
        if (cells.Count == 0) return "";
        var md = new StringBuilder();
        md.Append('|');
        foreach (var c in cells[0]) { md.Append(' ').Append(c).Append(" |"); }
        md.Append('\n');
        md.Append('|');
        foreach (var _ in cells[0]) md.Append(" --- |");
        md.Append('\n');
        for (int r = 1; r < cells.Count; r++)
        {
            md.Append('|');
            foreach (var c in cells[r]) { md.Append(' ').Append(c).Append(" |"); }
            md.Append('\n');
        }
        return md.ToString();
    }

    // ── inline markup ─────────────────────────────────────────────────────

    private static (string, List<TextAnnotation>) ParseInlineMarkup(string raw)
    {
        byte[] b = Encoding.UTF8.GetBytes(raw);
        int len = b.Length;
        var outBuf = new Utf8Buf();
        var anns = new List<TextAnnotation>();
        int i = 0;

        string Slice(int s, int e) => Encoding.UTF8.GetString(b, s, e - s);

        while (i < len)
        {
            if (i + 1 < len && b[i] == (byte)'*' && b[i + 1] == (byte)'*')
            {
                int end = FindClosing(b, i + 2, "**");
                if (end >= 0)
                {
                    string inner = Slice(i + 2, end);
                    uint start = outBuf.Len; outBuf.Append(inner); uint e2 = outBuf.Len;
                    if (start < e2) anns.Add(MarkupHelpers.Annotation(start, e2, MarkupHelpers.Bold));
                    i = end + 2; continue;
                }
            }
            if (b[i] == (byte)'*' && (i + 1 >= len || b[i + 1] != (byte)'*'))
            {
                int end = FindClosing(b, i + 1, "*");
                if (end >= 0 && (end + 1 >= len || b[end + 1] != (byte)'*'))
                {
                    string inner = Slice(i + 1, end);
                    uint start = outBuf.Len; outBuf.Append(inner); uint e2 = outBuf.Len;
                    if (start < e2) anns.Add(MarkupHelpers.Annotation(start, e2, MarkupHelpers.Italic));
                    i = end + 1; continue;
                }
            }
            if (i + 1 < len && b[i] == (byte)'`' && b[i + 1] == (byte)'`')
            {
                int end = FindClosing(b, i + 2, "``");
                if (end >= 0)
                {
                    string inner = Slice(i + 2, end);
                    uint start = outBuf.Len; outBuf.Append(inner); uint e2 = outBuf.Len;
                    if (start < e2) anns.Add(MarkupHelpers.Annotation(start, e2, MarkupHelpers.Code));
                    i = end + 2; continue;
                }
            }
            if (b[i] == (byte)'`' && (i + 1 >= len || b[i + 1] != (byte)'`'))
            {
                int end = FindClosingSingleBacktick(b, i + 1);
                if (end >= 0)
                {
                    string inner = Slice(i + 1, end);
                    int afterClose = end + 1;
                    if (afterClose < len && b[afterClose] == (byte)'_')
                    {
                        int angleStart = inner.LastIndexOf('<');
                        int angleEnd = inner.LastIndexOf('>');
                        if (angleStart >= 0 && angleEnd >= 0 && angleEnd > angleStart)
                        {
                            string url = inner.Substring(angleStart + 1, angleEnd - angleStart - 1).Trim();
                            string linkText = inner.Substring(0, angleStart).Trim();
                            uint start = outBuf.Len; outBuf.Append(linkText); uint e2 = outBuf.Len;
                            if (start < e2) anns.Add(MarkupHelpers.Annotation(start, e2, MarkupHelpers.Link(url, null)));
                            i = afterClose + 1; continue;
                        }
                        uint s3 = outBuf.Len; outBuf.Append(inner); uint e3 = outBuf.Len;
                        if (s3 < e3) anns.Add(MarkupHelpers.Annotation(s3, e3, MarkupHelpers.Code));
                        i = afterClose + 1; continue;
                    }
                    uint s4 = outBuf.Len; outBuf.Append(inner); uint e4 = outBuf.Len;
                    if (s4 < e4) anns.Add(MarkupHelpers.Annotation(s4, e4, MarkupHelpers.Code));
                    i = end + 1; continue;
                }
            }
            if (b[i] == (byte)'[')
            {
                int close = IndexOfByte(b, (byte)']', i + 1);
                if (close >= 0)
                {
                    int labelEnd = close;
                    if (labelEnd + 1 < len && b[labelEnd + 1] == (byte)'_')
                    {
                        string label = Slice(i + 1, labelEnd);
                        outBuf.Append("["); outBuf.Append(label); outBuf.Append("]");
                        i = labelEnd + 2; continue;
                    }
                }
            }
            // default: copy one UTF-8 char
            int clen = Utf8CharLen(b, i);
            outBuf.Append(Slice(i, i + clen));
            i += clen;
        }
        return (outBuf.ToString(), anns);
    }

    private static int FindClosing(byte[] b, int from, string marker)
    {
        byte[] m = Encoding.ASCII.GetBytes(marker);
        for (int j = from; j + m.Length <= b.Length; j++)
        {
            bool ok = true;
            for (int k = 0; k < m.Length; k++) if (b[j + k] != m[k]) { ok = false; break; }
            if (ok) return j;
        }
        return -1;
    }

    private static int FindClosingSingleBacktick(byte[] b, int from)
    {
        int j = from;
        while (j < b.Length)
        {
            if (b[j] == (byte)'`')
            {
                if (j + 1 < b.Length && b[j + 1] == (byte)'`') { j += 2; continue; }
                return j;
            }
            j++;
        }
        return -1;
    }

    private static int IndexOfByte(byte[] b, byte target, int from)
    {
        for (int j = from; j < b.Length; j++) if (b[j] == target) return j;
        return -1;
    }

    private static int Utf8CharLen(byte[] b, int i)
    {
        byte c = b[i];
        if (c < 0x80) return 1;
        if ((c & 0xE0) == 0xC0) return 2;
        if ((c & 0xF0) == 0xE0) return 3;
        if ((c & 0xF8) == 0xF0) return 4;
        return 1;
    }

    private static List<string> FindFootnoteReferences(string line)
    {
        var refs = new List<string>();
        byte[] b = Encoding.UTF8.GetBytes(line);
        int i = 0;
        while (i < b.Length)
        {
            if (b[i] == (byte)'[')
            {
                int close = IndexOfByte(b, (byte)']', i + 1);
                if (close >= 0)
                {
                    string label = Encoding.UTF8.GetString(b, i + 1, close - (i + 1));
                    if (close + 1 < b.Length && b[close + 1] == (byte)'_')
                    {
                        if (label.Length > 0 && (label.All(c => c is >= '0' and <= '9') || label.StartsWith('#')))
                            refs.Add(label);
                    }
                }
            }
            i++;
        }
        return refs;
    }

    /// <summary>
    /// Push an image (or figure) URI and, where configured, a placeholder paragraph for it.
    /// Shared by the <c>.. image::</c> and <c>.. figure::</c> handlers so a figure builds on the
    /// same emission logic (`push_image_directive`).
    /// </summary>
    private static void PushImageDirective(InternalDocumentBuilder b, string uri,
                                           Dictionary<string, string> opts, bool injectPlaceholders)
    {
        opts.TryGetValue("alt", out var alt);
        string desc = alt ?? uri;
        if (uri.Length != 0) b.PushUri(MarkupHelpers.Image(uri, alt));
        if (injectPlaceholders)
        {
            uint idx = b.PushParagraph($"[image: {desc}]", new(), null, null);
            if (uri.Length != 0) b.SetAttributes(idx, new Dictionary<string, string> { ["src"] = uri });
        }
    }

    /// <summary>
    /// Parse the row and cell structure of a <c>.. list-table::</c> body. A row is a top-level
    /// bullet (<c>* - cell</c>) and each further cell is a nested bullet (<c>- cell</c>) indented
    /// deeper than the row marker.
    /// </summary>
    private static List<List<string>> ParseListTableRows(List<string> lines, ref int start)
    {
        var rows = new List<List<string>>();
        while (start < lines.Count)
        {
            string line = lines[start];
            if (line.Trim().Length == 0) break;
            int leading = line.Length - line.TrimStart().Length;
            string trimmed = line.TrimStart();
            if (!trimmed.StartsWith("* ", StringComparison.Ordinal)) break;
            string afterStar = trimmed[2..];
            if (!afterStar.StartsWith("- ", StringComparison.Ordinal)) break;

            var row = new List<string> { afterStar[2..].Trim() };
            start++;
            while (start < lines.Count)
            {
                string cellLine = lines[start];
                if (cellLine.Trim().Length == 0) break;
                int cellLeading = cellLine.Length - cellLine.TrimStart().Length;
                string cellTrimmed = cellLine.TrimStart();
                if (cellLeading > leading && cellTrimmed.StartsWith("- ", StringComparison.Ordinal))
                {
                    row.Add(cellTrimmed[2..].Trim());
                    start++;
                }
                else break;
            }
            rows.Add(row);
        }
        return rows;
    }

    /// <summary>
    /// Split one CSV line into fields, honouring double-quoted fields with embedded commas and
    /// <c>""</c>-escaped quotes (RFC 4180-style), as <c>.. csv-table::</c> uses.
    /// </summary>
    private static List<string> ParseCsvLine(string line)
    {
        var fields = new List<string>();
        var current = new StringBuilder();
        bool inQuotes = false;
        for (int idx = 0; idx < line.Length; idx++)
        {
            char ch = line[idx];
            if (inQuotes)
            {
                if (ch == '"')
                {
                    if (idx + 1 < line.Length && line[idx + 1] == '"') { current.Append('"'); idx++; continue; }
                    inQuotes = false;
                    continue;
                }
                current.Append(ch);
                continue;
            }
            if (ch == '"') { inQuotes = true; continue; }
            if (ch == ',') { fields.Add(current.ToString().Trim()); current.Clear(); continue; }
            current.Append(ch);
        }
        fields.Add(current.ToString().Trim());
        return fields;
    }

    private static Dictionary<string, string> ParseImageOptions(List<string> lines, ref int start)
    {
        var opts = new Dictionary<string, string>();
        while (start < lines.Count)
        {
            string line = lines[start];
            if (!line.StartsWith("   ") && !line.StartsWith("\t")) break;
            string trimmed = line.Trim();
            if (trimmed.Length == 0) { start++; break; }
            if (trimmed.StartsWith(':'))
            {
                int colon2 = trimmed.Substring(1).IndexOf(':');
                if (colon2 >= 0)
                {
                    string key = trimmed.Substring(1, colon2);
                    string value = trimmed.Substring(2 + colon2).Trim();
                    opts[key] = value;
                }
            }
            start++;
        }
        return opts;
    }

    // ── build internal document ─────────────────────────────────────────────

    private static InternalDocument BuildInternalDocument(string content, bool injectPlaceholders)
    {
        var b = new InternalDocumentBuilder("rst");
        var lines = MarkupHelpers.Lines(content);
        var headingCharOrder = new List<char>();
        bool hasOverline = false;
        string? highlightLang = null;
        int i = 0;

        while (i < lines.Count)
        {
            Cancel.ThrowIfRequested();
            string line = lines[i];
            string trimmed = line.Trim();

            if (trimmed.StartsWith(':') && trimmed.Length > 1 && ParseFieldListLine(trimmed) is var (fk, fv) && fk is not null)
            {
                string fullValue = fv!;
                while (i + 1 < lines.Count)
                {
                    string next = lines[i + 1];
                    if (next.Length != 0 && (next.StartsWith("   ") || next.StartsWith("\t")))
                    {
                        if (fullValue.Length != 0) fullValue += " ";
                        fullValue += next.Trim();
                        i++;
                    }
                    else break;
                }
                b.PushMetadataBlock(new[] { (fk!, fullValue) }, null);
                i++;
                continue;
            }

            if (IsSectionUnderline(trimmed) && i + 2 < lines.Count && lines[i + 1].Trim().Length != 0 && IsSectionUnderline(lines[i + 2]))
            {
                char oc = FirstOr(trimmed, '=');
                char uc = FirstOr(lines[i + 2].Trim(), '=');
                if (oc == uc)
                {
                    hasOverline = true;
                    b.PushHeading(1, lines[i + 1].Trim(), null, null);
                    i += 3; continue;
                }
            }

            if (i + 1 < lines.Count && trimmed.Length != 0 && IsSectionUnderline(lines[i + 1]))
            {
                char uc = FirstOr(lines[i + 1].Trim(), '=');
                if (!headingCharOrder.Contains(uc)) headingCharOrder.Add(uc);
                int base_ = hasOverline ? 2 : 1;
                int pos = headingCharOrder.IndexOf(uc);
                int level = pos >= 0 ? pos + base_ : base_;
                b.PushHeading((byte)level, trimmed, null, null);
                i += 2; continue;
            }

            if (trimmed.StartsWith(".. code-block::") || trimmed.StartsWith(".. code::"))
            {
                string? language = null;
                if (trimmed.StartsWith(".. code-block::"))
                {
                    string lang = trimmed.Substring(".. code-block::".Length).Trim();
                    language = lang.Length == 0 ? null : lang;
                }
                else
                {
                    string lang = trimmed.Substring(".. code::".Length).Trim();
                    language = lang.Length == 0 ? null : lang;
                }
                i++;
                while (i < lines.Count && lines[i].Trim().Length == 0) i++;
                var code = new StringBuilder();
                while (i < lines.Count && (lines[i].StartsWith("   ") || lines[i].Length == 0))
                {
                    if (code.Length != 0) code.Append('\n');
                    if (lines[i].StartsWith("   ")) code.Append(lines[i].Substring(3));
                    i++;
                }
                b.PushCode(TrimEnd(code.ToString()), language, null, null);
                continue;
            }

            if (StartsWithAny(trimmed, ".. note::", ".. warning::", ".. important::", ".. caution::", ".. hint::", ".. tip::"))
            {
                string kind = trimmed.Substring(".. ".Length);
                kind = TrimEndStr(kind, "::").Trim();
                uint idx = b.PushAdmonition(kind, null, null);
                i++;
                var body = new StringBuilder();
                while (i < lines.Count && (lines[i].StartsWith("   ") || lines[i].Length == 0))
                {
                    if (lines[i].Length != 0)
                    {
                        if (body.Length != 0) body.Append(' ');
                        body.Append(lines[i].Trim());
                    }
                    i++;
                }
                if (body.Length != 0) b.SetText(idx, body.ToString());
                continue;
            }

            if (trimmed.StartsWith(".. image::"))
            {
                string uri = trimmed.Substring(".. image::".Length).Trim();
                i++;
                var opts = ParseImageOptions(lines, ref i);
                PushImageDirective(b, uri, opts, injectPlaceholders);
                continue;
            }

            if (trimmed.StartsWith(".. figure::"))
            {
                string uri = trimmed.Substring(".. figure::".Length).Trim();
                i++;
                var opts = ParseImageOptions(lines, ref i);
                PushImageDirective(b, uri, opts, injectPlaceholders);

                // The figure body — an indented paragraph after the option block — is the
                // figure's caption, and is emitted as a regular paragraph so the text survives.
                var caption = new StringBuilder();
                while (i < lines.Count)
                {
                    Cancel.ThrowIfRequested();
                    if (lines[i].Length == 0)
                    {
                        if (caption.Length != 0) break;
                        i++;
                        continue;
                    }
                    if (!lines[i].StartsWith("   ", StringComparison.Ordinal)
                        && !lines[i].StartsWith("\t", StringComparison.Ordinal)) break;
                    if (caption.Length != 0) caption.Append(' ');
                    caption.Append(lines[i].Trim());
                    i++;
                }
                if (caption.Length != 0)
                {
                    var (stripped, annotations) = ParseInlineMarkup(caption.ToString());
                    b.PushParagraph(stripped, annotations, null, null);
                }
                continue;
            }

            if (trimmed.StartsWith(".. list-table::"))
            {
                i++;
                ParseImageOptions(lines, ref i);
                // `ParseImageOptions` only eats the blank line after the option block when there
                // were options, so any left over are skipped here or the row parser sees a blank
                // first line and stops immediately.
                while (i < lines.Count && lines[i].Trim().Length == 0) i++;
                var cells = ParseListTableRows(lines, ref i);
                if (cells.Count != 0) b.PushTableFromCells(cells, null, null);
                continue;
            }

            if (trimmed.StartsWith(".. csv-table::"))
            {
                i++;
                var tableOpts = ParseImageOptions(lines, ref i);
                while (i < lines.Count && lines[i].Trim().Length == 0) i++;
                var csvCells = new List<List<string>>();
                if (tableOpts.TryGetValue("header", out var headerLine))
                    csvCells.Add(ParseCsvLine(headerLine));
                while (i < lines.Count)
                {
                    Cancel.ThrowIfRequested();
                    string l = lines[i];
                    if (l.Trim().Length == 0
                        || !(l.StartsWith("   ", StringComparison.Ordinal) || l.StartsWith("\t", StringComparison.Ordinal)))
                        break;
                    csvCells.Add(ParseCsvLine(l.Trim()));
                    i++;
                }
                if (csvCells.Count != 0) b.PushTableFromCells(csvCells, null, null);
                continue;
            }

            if (trimmed.StartsWith(".. math::"))
            {
                string inlineMath = trimmed.Substring(".. math::".Length).Trim();
                i++;
                while (i < lines.Count)
                {
                    Cancel.ThrowIfRequested();
                    string l = lines[i].Trim();
                    if ((l.StartsWith(':') && l.EndsWith(':')) || (l.StartsWith(':') && l.Contains(": ")))
                    {
                        if (lines[i].StartsWith("   ") || lines[i].StartsWith("\t")) { i++; continue; }
                    }
                    break;
                }
                string mathContent = inlineMath;
                while (i < lines.Count && (lines[i].StartsWith("   ") || lines[i].Length == 0))
                {
                    if (lines[i].Length == 0)
                    {
                        if (mathContent.Length != 0) { b.PushFormula(WrapAlignedMath(mathContent), null, null); mathContent = ""; }
                    }
                    else
                    {
                        if (mathContent.Length != 0) mathContent += "\n";
                        mathContent += lines[i].Trim();
                    }
                    i++;
                }
                if (mathContent.Length != 0) b.PushFormula(WrapAlignedMath(mathContent), null, null);
                continue;
            }

            if (trimmed.StartsWith(".. ["))
            {
                int close = trimmed.IndexOf(']');
                if (close > 4)
                {
                    string label = trimmed.Substring(4, close - 4);
                    string footnoteText = trimmed.Substring(close + 1).Trim();
                    string fullText = footnoteText;
                    i++;
                    while (i < lines.Count && (lines[i].StartsWith("   ") || lines[i].StartsWith("\t")))
                    {
                        if (fullText.Length != 0) fullText += " ";
                        fullText += lines[i].Trim();
                        i++;
                    }
                    bool isCitation = label.All(c => char.IsLetterOrDigit(c) || c == '_' || c == '-')
                        && !label.All(c => c is >= '0' and <= '9') && !label.StartsWith('#');
                    if (isCitation) b.PushCitation(fullText, label, null);
                    else b.PushFootnoteDefinition(fullText, label, null);
                    continue;
                }
            }

            if (trimmed.StartsWith(".. _"))
            {
                int colonPos = trimmed.Substring(4).IndexOf(": ");
                if (colonPos >= 0)
                {
                    string label = trimmed.Substring(4, colonPos);
                    string url = trimmed.Substring(4 + colonPos + 2).Trim();
                    if (url.Length != 0 && label.Length != 0)
                    {
                        uint labelBytes = (uint)Encoding.UTF8.GetByteCount(label);
                        b.PushParagraph(label, new List<TextAnnotation>
                        {
                            MarkupHelpers.Annotation(0, labelBytes, MarkupHelpers.Link(url, null)),
                        }, null, null);
                    }
                    i++;
                    continue;
                }
            }

            if (trimmed.StartsWith(".. highlight::"))
            {
                string lang = trimmed.Substring(".. highlight::".Length).Trim();
                highlightLang = lang.Length == 0 ? null : lang;
                i++;
                while (i < lines.Count && (lines[i].StartsWith("   ") || lines[i].Length == 0)) i++;
                continue;
            }

            if (trimmed.StartsWith(".. contents::"))
            {
                string title = trimmed.Substring(".. contents::".Length).Trim();
                if (title.Length != 0) b.PushParagraph(title, new(), null, null);
                i++;
                while (i < lines.Count && (lines[i].StartsWith("   ") || lines[i].Length == 0)) i++;
                continue;
            }

            if (trimmed.StartsWith(".. ") || trimmed == "..")
            {
                // A directive this parser does not handle still has a body, and that body is
                // document text. A comment's body is not. The two look alike, so they are told
                // apart by shape: a directive's name is a single word immediately followed by
                // `::`, which `.. some comment text` never is.
                string afterDots = trimmed.StartsWith(".. ") ? trimmed[3..] : "";
                int marker = afterDots.IndexOf("::", StringComparison.Ordinal);
                bool isDirective = trimmed != ".." && marker > 0
                    && !afterDots[..marker].Any(char.IsWhiteSpace);

                i++;
                var bodyText = new StringBuilder();
                while (i < lines.Count)
                {
                    Cancel.ThrowIfRequested();
                    string l = lines[i];
                    if (l.Length == 0)
                    {
                        // A blank line ends the body once there is one; before that it is just
                        // the gap between the directive line and its content.
                        if (bodyText.Length > 0) break;
                        i++;
                        continue;
                    }
                    if (!l.StartsWith("   ") && !l.StartsWith("\t")) break;
                    if (bodyText.Length > 0) bodyText.Append(' ');
                    bodyText.Append(l.Trim());
                    i++;
                }
                if (isDirective && bodyText.Length > 0)
                {
                    var (stripped, annotations) = ParseInlineMarkup(bodyText.ToString());
                    b.PushParagraph(stripped, annotations, null, null);
                }
                continue;
            }

            if (IsSimpleTableSeparator(trimmed))
            {
                var tableLines = new List<string>();
                while (i < lines.Count)
                {
                    Cancel.ThrowIfRequested();
                    string tl = lines[i].Trim();
                    if (tl.Length == 0) break;
                    tableLines.Add(lines[i]);
                    i++;
                }
                var cells = ParseSimpleTableCells(tableLines);
                if (cells.Count > 0) b.PushTableFromCells(cells, null, null);
                continue;
            }

            if (trimmed.StartsWith('+') && trimmed.EndsWith('+') && trimmed.Contains('-'))
            {
                var tableLines = new List<string>();
                while (i < lines.Count && (lines[i].Trim().StartsWith('+') || lines[i].Trim().StartsWith('|')))
                {
                    tableLines.Add(lines[i]);
                    i++;
                }
                var cells = ParseGridTableCells(tableLines);
                if (cells.Count > 0) b.PushTableFromCells(cells, null, null);
                continue;
            }

            if (IsListItem(line))
            {
                bool isOrdered;
                {
                    string t = trimmed.TrimStart();
                    if (t.StartsWith("#. ") || t.StartsWith("#.\t")) isOrdered = true;
                    else
                    {
                        int sp = t.IndexOfAny(new[] { ' ', '\t' });
                        if (sp >= 0) { string prefix = t.Substring(0, sp); isOrdered = prefix.EndsWith('.') || prefix.EndsWith(')'); }
                        else isOrdered = false;
                    }
                }
                b.PushList(isOrdered);
                while (i < lines.Count && IsListItem(lines[i]))
                {
                    string itemTrimmed = lines[i].Trim();
                    string text = StripListPrefix(itemTrimmed);
                    string fullText = text;
                    i++;
                    while (i < lines.Count && lines[i].Trim().Length != 0
                        && (lines[i].StartsWith("   ") || lines[i].StartsWith("\t")) && !IsListItem(lines[i]))
                    {
                        fullText += " " + lines[i].Trim();
                        i++;
                    }
                    var (parsed, itemAnns) = ParseInlineMarkup(fullText);
                    b.PushListItem(parsed, isOrdered, itemAnns, null, null);
                }
                b.EndList();
                continue;
            }

            if (trimmed.EndsWith("::") && !trimmed.StartsWith(".. "))
            {
                string displayText = TrimEndStr(trimmed, "::");
                if (displayText.Length != 0)
                {
                    var (stripped, anns) = ParseInlineMarkup(displayText);
                    b.PushParagraph(stripped, anns, null, null);
                }
                i++;
                while (i < lines.Count && lines[i].Trim().Length == 0) i++;
                int indent = 3;
                {
                    int j = i;
                    while (j < lines.Count)
                    {
                        string l = lines[j];
                        if (l.Trim().Length != 0)
                        {
                            indent = l.Length - l.TrimStart().Length;
                            if (indent == 0) indent = 3;
                            break;
                        }
                        j++;
                    }
                }
                var codeContent = new StringBuilder();
                while (i < lines.Count)
                {
                    Cancel.ThrowIfRequested();
                    string l = lines[i];
                    byte[] lb = Encoding.UTF8.GetBytes(l);
                    bool isIndented = l.StartsWith("\t") || (lb.Length >= indent && AllSpaces(lb, indent));
                    if (!isIndented && l.Length != 0) break;
                    if (codeContent.Length != 0) codeContent.Append('\n');
                    if (l.StartsWith("\t")) codeContent.Append(l.Substring(1));
                    else if (isIndented && l.Length != 0) codeContent.Append(SubstringByBytes(l, indent));
                    i++;
                }
                if (codeContent.Length != 0) b.PushCode(TrimEnd(codeContent.ToString()), highlightLang, null, null);
                continue;
            }

            if (trimmed.Length != 0 && !IsMarkupLine(line))
            {
                string paraText = trimmed;
                while (i + 1 < lines.Count)
                {
                    string next = lines[i + 1];
                    string nt = next.Trim();
                    if (nt.Length == 0) break;
                    if (next.StartsWith(" ") || next.StartsWith("\t")) break;
                    if (IsSectionUnderline(nt)) break;
                    if (IsMarkupLine(next)) break;
                    if (nt.StartsWith(".. ") || nt == "..") break;
                    if (IsListItem(next)) break;
                    if (nt.StartsWith(':') && nt.Length > 1 && ParseFieldListLine(nt).key is not null) break;
                    if (IsSimpleTableSeparator(nt)) break;
                    if (nt.StartsWith('+') && nt.EndsWith('+') && nt.Contains('-')) break;
                    paraText += " " + nt;
                    i++;
                }
                var footnoteRefs = FindFootnoteReferences(paraText);
                var (stripped2, anns2) = ParseInlineMarkup(paraText);
                uint idx2 = b.PushParagraph(stripped2, anns2, null, null);
                foreach (var fref in footnoteRefs) b.PushFootnoteRef($"[{fref}]", fref, null);
                ExtractRstCrossRefs(paraText, idx2, b);
            }

            i++;
        }
        return b.Build();
    }

    private static void ExtractRstCrossRefs(string line, uint sourceIdx, InternalDocumentBuilder b)
    {
        string[] roles = { ":ref:", ":doc:", ":numref:" };
        foreach (var role in roles)
        {
            int searchFrom = 0;
            while (true)
            {
                int pos = line.IndexOf(role, searchFrom, StringComparison.Ordinal);
                if (pos < 0) break;
                string after = line.Substring(pos + role.Length);
                if (after.StartsWith('`'))
                {
                    int close = after.Substring(1).IndexOf('`');
                    if (close >= 0)
                    {
                        string target = after.Substring(1, close);
                        string key;
                        int anglePos = target.IndexOf('<');
                        if (anglePos >= 0)
                        {
                            int end = target.IndexOf('>');
                            if (end < 0) end = target.Length;
                            key = target.Substring(anglePos + 1, end - anglePos - 1);
                        }
                        else key = target;
                        if (key.Length != 0)
                            b.PushRelationship(sourceIdx, RelationshipTarget.FromKey(key), RelationshipKind.CrossReference);
                        searchFrom = pos + role.Length + 1 + close + 1;
                        continue;
                    }
                }
                searchFrom = pos + role.Length;
            }
        }
    }

    // ── small helpers ─────────────────────────────────────────────────────

    private static string StripListPrefix(string itemTrimmed)
    {
        foreach (var p in new[] { "* ", "*\t", "+ ", "+\t", "- ", "-\t", "#. ", "#.\t" })
            if (itemTrimmed.StartsWith(p)) return itemTrimmed.Substring(p.Length);
        int sp = itemTrimmed.IndexOfAny(new[] { ' ', '\t' });
        if (sp >= 0) return itemTrimmed.Substring(sp + 1);
        return itemTrimmed;
    }

    private static char FirstOr(string s, char fallback) => s.Length > 0 ? s[0] : fallback;
    private static bool StartsWithAny(string s, params string[] prefixes) => prefixes.Any(p => s.StartsWith(p, StringComparison.Ordinal));
    private static string TrimEndStr(string s, string suffix) => s.EndsWith(suffix, StringComparison.Ordinal) ? s.Substring(0, s.Length - suffix.Length) : s;
    private static string TrimEnd(string s) => s.TrimEnd();
    private static bool AllSpaces(byte[] b, int n) { for (int k = 0; k < n; k++) if (b[k] != (byte)' ') return false; return true; }
    private static string SubstringByBytes(string s, int startByte)
    {
        byte[] b = Encoding.UTF8.GetBytes(s);
        return Encoding.UTF8.GetString(b, startByte, b.Length - startByte);
    }
}
