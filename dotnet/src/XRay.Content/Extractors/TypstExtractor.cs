using System.Text;
using System.Text.RegularExpressions;
using XRay.Content.Core;
using XRay.Content.Internal.Markup;
using XRay.Content.Internal.MathMarkup;
using XRay.Content.Types;

namespace XRay.Content.Extractors;

/// <summary>
/// Typst document extractor. Ported from Rust `extractors/typst.rs`. The InternalDocument is
/// built by a line-based parser; metadata (title/author/date/subject/keywords) is derived by
/// regex over `#set document(...)`.
/// </summary>
public sealed partial class TypstExtractor : IExtractor
{
    public IEnumerable<string> SupportedMimeTypes => new[] { "application/x-typst", "text/x-typst" };
    public int Priority => 50;

    // The hash is optional: inside `#figure(...)` the call is written bare, as `image("…")`.
    [GeneratedRegex(@"#?image\(""([^""]*)""")] private static partial Regex ImageRe();
    [GeneratedRegex(@"columns:\s*(\d+)")] private static partial Regex ColumnsRe();
    [GeneratedRegex(@"#cite\(<([^>]+)>\)")] private static partial Regex CiteRe();
    [GeneratedRegex(@"(^|[\s(])@([A-Za-z][A-Za-z0-9_:.-]*)")] private static partial Regex AtCiteRe();
    [GeneratedRegex(@"^#link\(""([^""]*)""\)\[([^\]]*)\]")] private static partial Regex LinkRe();
    [GeneratedRegex(@"keywords:\s*(?:""([^""]*)""|(\([^)]*\)))")] private static partial Regex KeywordsRe();
    [GeneratedRegex(@"""([^""]*)""")] private static partial Regex QuotedItemRe();

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        string text = Encoding.UTF8.GetString(content);
        var metadata = ExtractMetadata(text);
        var doc = BuildInternalDocument(text);
        doc.MimeType = mimeType;
        doc.Metadata = metadata;
        return doc;
    }

    private static Metadata ExtractMetadata(string content)
    {
        var meta = new Metadata();
        string? title = ExtractQuoted(content, "title"); if (title is not null) meta.Title = title;
        string? author = ExtractQuoted(content, "author"); if (author is not null) meta.Authors = new List<string> { author };
        string? date = ExtractQuoted(content, "date"); if (date is not null) meta.CreatedAt = date;
        string? subject = ExtractQuoted(content, "subject"); if (subject is not null) meta.Subject = subject;
        var keywords = ExtractKeywords(content); if (keywords is not null) meta.Keywords = keywords;
        return meta;
    }

    private static string? ExtractQuoted(string content, string field)
    {
        var re = new Regex($@"{Regex.Escape(field)}:\s*""([^""]*)""");
        var m = re.Match(content);
        return m.Success ? m.Groups[1].Value : null;
    }

    private static List<string>? ExtractKeywords(string content)
    {
        var m = KeywordsRe().Match(content);
        if (!m.Success) return null;
        if (m.Groups[1].Success)
        {
            var kws = m.Groups[1].Value.Split(',').Select(s => s.Trim()).Where(s => s.Length != 0).ToList();
            if (kws.Count > 0) return kws;
        }
        if (m.Groups[2].Success)
        {
            var kws = new List<string>();
            foreach (Match im in QuotedItemRe().Matches(m.Groups[2].Value)) kws.Add(im.Groups[1].Value);
            if (kws.Count > 0) return kws;
        }
        return null;
    }

    private static InternalDocument BuildInternalDocument(string content)
    {
        var builder = new InternalDocumentBuilder("typst");
        bool inCodeBlock = false;
        var codeText = new StringBuilder();
        string? codeLang = null;
        bool inSetDocument = false;
        int parenDepth = 0;
        var paragraphBuf = new StringBuilder();
        bool inTable = false;
        var tableBuf = new StringBuilder();
        int tableParen = 0, tableBracket = 0;
        int footnoteCounter = 0;
        bool? activeList = null;
        bool inDisplayMath = false;
        var mathBuf = new StringBuilder();

        var lines = MarkupHelpers.Lines(content);
        int lineIdx = 0;

        while (lineIdx < lines.Count)
        {
            Cancel.ThrowIfRequested();
            string trimmed = lines[lineIdx].Trim();
            lineIdx++;

            if (inTable)
            {
                tableBuf.Append('\n').Append(trimmed);
                foreach (char ch in trimmed) CountBrackets(ch, ref tableParen, ref tableBracket);
                if (tableParen <= 0 && tableBracket <= 0)
                {
                    inTable = false;
                    EmitTable(tableBuf.ToString(), builder);
                    tableBuf.Clear();
                }
                continue;
            }

            if (inSetDocument)
            {
                foreach (char ch in trimmed) { if (ch == '(') parenDepth++; else if (ch == ')') parenDepth--; }
                if (parenDepth <= 0) { inSetDocument = false; parenDepth = 0; }
                continue;
            }

            // A `$` that opens a block runs until the next one, however many lines later, so the
            // body has to be accumulated rather than matched on a single line.
            if (inDisplayMath)
            {
                int closeIdx = trimmed.IndexOf('$');
                if (closeIdx >= 0)
                {
                    string beforeClose = trimmed.Substring(0, closeIdx).Trim();
                    if (beforeClose.Length != 0)
                    {
                        if (mathBuf.Length != 0) mathBuf.Append('\n');
                        mathBuf.Append(beforeClose);
                    }
                    inDisplayMath = false;
                    string mathText = mathBuf.ToString().Trim();
                    mathBuf.Clear();
                    if (mathText.Length != 0) builder.PushFormula(TypstMath.ConvertToLatex(mathText), null, null);
                }
                else
                {
                    if (mathBuf.Length != 0) mathBuf.Append('\n');
                    mathBuf.Append(trimmed);
                }
                continue;
            }

            if (trimmed.StartsWith("```"))
            {
                if (inCodeBlock)
                {
                    if (trimmed == "```")
                    {
                        inCodeBlock = false;
                        string t = codeText.ToString().TrimEnd();
                        if (t.Length != 0) builder.PushCode(t, codeLang, null, null);
                        codeText.Clear(); codeLang = null;
                        continue;
                    }
                }
                else
                {
                    FlushParagraph(paragraphBuf, builder);
                    inCodeBlock = true;
                    codeText.Clear();
                    string lang = trimmed.Substring(3).Trim();
                    codeLang = lang.Length == 0 ? null : lang;
                    continue;
                }
            }

            if (inCodeBlock)
            {
                codeText.Append(lines[lineIdx - 1]).Append('\n');
                continue;
            }

            if (trimmed.StartsWith("#set document("))
            {
                FlushParagraph(paragraphBuf, builder);
                parenDepth = 0;
                foreach (char ch in trimmed) { if (ch == '(') parenDepth++; else if (ch == ')') parenDepth--; }
                if (parenDepth > 0) inSetDocument = true;
                continue;
            }

            if (trimmed.StartsWith("#set ") || trimmed.StartsWith("#let ") || trimmed.StartsWith("#import ")
                || trimmed.StartsWith("#include ") || trimmed.StartsWith("#pagebreak") || trimmed.StartsWith("#colbreak")
                || trimmed.StartsWith("#v(") || trimmed.StartsWith("#h(") || trimmed.StartsWith("#bibliography("))
                continue;

            if (trimmed.StartsWith("/ "))
            {
                FlushParagraph(paragraphBuf, builder);
                string rest = trimmed.Substring(2).Trim();
                int colonIdx = rest.IndexOf(':');
                if (colonIdx >= 0)
                {
                    string term = rest.Substring(0, colonIdx).Trim();
                    string definition = rest.Substring(colonIdx + 1).Trim();
                    if (term.Length != 0)
                    {
                        builder.PushDefinitionTerm(term, null);
                        builder.PushDefinitionDescription(definition, null);
                    }
                }
                continue;
            }

            if ((trimmed.StartsWith('+') || trimmed.StartsWith('-')) && trimmed.Length > 1
                && !char.IsLetterOrDigit(trimmed[1]))
            {
                FlushParagraph(paragraphBuf, builder);
                bool ordered = trimmed.StartsWith('+');
                if (!(activeList is bool prev && prev == ordered))
                {
                    if (activeList is not null) builder.EndList();
                    builder.PushList(ordered);
                    activeList = ordered;
                }
                builder.PushListItem(trimmed.Substring(1).Trim(), ordered, new(), null, null);
                continue;
            }

            if (activeList is not null) { builder.EndList(); activeList = null; }

            if (trimmed.StartsWith('='))
            {
                int level = trimmed.TakeWhile(c => c == '=').Count();
                string headingText = trimmed.Substring(level).Trim();
                if (headingText.Length != 0)
                {
                    FlushParagraph(paragraphBuf, builder);
                    // The marker run is the level and nothing else; it must not survive into the
                    // heading's text, or every renderer repeats it after its own marker.
                    builder.PushHeading((byte)level, headingText, null, null);
                }
                continue;
            }

            if (trimmed == "$")
            {
                FlushParagraph(paragraphBuf, builder);
                inDisplayMath = true;
                mathBuf.Clear();
                continue;
            }

            if (trimmed.StartsWith('$') && trimmed.Length > 1)
            {
                FlushParagraph(paragraphBuf, builder);
                string rest = trimmed.Substring(1);
                int close = rest.IndexOf('$');
                if (close >= 0)
                {
                    // The line closes its own math. Whatever follows the closing `$` is ordinary
                    // text — a trailing comma inside a table call, a code span, prose. Treating
                    // such a line as the start of a block swallowed all of it up to the next `$`
                    // somewhere further down the document.
                    string math = rest[..close].Trim();
                    if (math.Length != 0) builder.PushFormula(TypstMath.ConvertToLatex(math), null, null);
                    string after = rest[(close + 1)..].Trim();
                    if (after.Length != 0) paragraphBuf.Append(after).Append(' ');
                }
                else
                {
                    inDisplayMath = true;
                    mathBuf.Clear();
                    string openingContent = rest.Trim();
                    if (openingContent.Length != 0) mathBuf.Append(openingContent);
                }
                continue;
            }

            if (trimmed.Length == 0) { FlushParagraph(paragraphBuf, builder); continue; }

            if (trimmed.StartsWith("#table("))
            {
                FlushParagraph(paragraphBuf, builder);
                tableBuf.Clear();
                tableBuf.Append(trimmed);
                tableParen = 0; tableBracket = 0;
                foreach (char ch in trimmed) CountBrackets(ch, ref tableParen, ref tableBracket);
                if (tableParen > 0 || tableBracket > 0) inTable = true;
                else { EmitTable(tableBuf.ToString(), builder); tableBuf.Clear(); }
                continue;
            }

            if (trimmed.StartsWith("#quote["))
            {
                FlushParagraph(paragraphBuf, builder);
                string? quoted = ExtractBracketContent(trimmed, "#quote[");
                if (quoted is not null)
                {
                    builder.PushQuoteStart();
                    builder.PushParagraph(quoted, new(), null, null);
                    builder.PushQuoteEnd();
                }
                continue;
            }

            // A figure's arguments run until its parentheses balance, which is usually several
            // lines below. Consuming only the first line leaves the rest to be read as prose.
            if (trimmed.StartsWith("#figure("))
            {
                FlushParagraph(paragraphBuf, builder);
                var figureBuf = new StringBuilder(trimmed);
                int figureParen = 0;
                foreach (char ch in trimmed) { if (ch == '(') figureParen++; else if (ch == ')') figureParen--; }
                while (figureParen > 0 && lineIdx < lines.Count)
                {
                    string nextLine = lines[lineIdx].Trim();
                    lineIdx++;
                    figureBuf.Append('\n').Append(nextLine);
                    foreach (char ch in nextLine) { if (ch == '(') figureParen++; else if (ch == ')') figureParen--; }
                }

                string figureSource = figureBuf.ToString();
                var fm = ImageRe().Match(figureSource);
                string? imagePath = fm.Success ? fm.Groups[1].Value : null;
                string? caption = ExtractFigureCaption(figureSource);

                if (imagePath is not null)
                {
                    builder.PushUri(MarkupHelpers.Image(imagePath, caption));
                    builder.PushParagraph($"[Image: {imagePath}]", new(), null, null);
                }
                if (caption is not null) builder.PushParagraph(caption, new(), null, null);
                continue;
            }

            if (trimmed.StartsWith("#footnote["))
            {
                FlushParagraph(paragraphBuf, builder);
                string? t = ExtractBracketContent(trimmed, "#footnote[");
                if (t is not null)
                {
                    footnoteCounter++;
                    builder.PushFootnoteDefinition(t, $"fn-{footnoteCounter}", null);
                }
                continue;
            }

            if (trimmed.StartsWith("#image("))
            {
                FlushParagraph(paragraphBuf, builder);
                var m = ImageRe().Match(trimmed);
                string? path = m.Success ? m.Groups[1].Value : null;
                if (path is not null) builder.PushUri(MarkupHelpers.Image(path, null));
                string descText = path is not null ? $"[Image: {path}]" : "[Image]";
                builder.PushParagraph(descText, new(), null, null);
                continue;
            }

            if (paragraphBuf.Length != 0) paragraphBuf.Append(' ');
            paragraphBuf.Append(trimmed);
        }

        // An unterminated block still carries its content; drop the delimiter, not the formula.
        if (inDisplayMath)
        {
            string trailingMath = mathBuf.ToString().Trim();
            if (trailingMath.Length != 0) builder.PushFormula(TypstMath.ConvertToLatex(trailingMath), null, null);
        }

        if (activeList is not null) builder.EndList();
        FlushParagraph(paragraphBuf, builder);
        return builder.Build();
    }

    /// <summary>The bracketed argument of a figure's <c>caption:</c>, if it has one.</summary>
    private static string? ExtractFigureCaption(string figureSource)
    {
        int idx = figureSource.IndexOf("caption:", StringComparison.Ordinal);
        if (idx < 0) return null;
        string after = figureSource.Substring(idx + "caption:".Length);
        int start = after.IndexOf('[');
        if (start < 0) return null;

        int depth = 0;
        for (int i = start; i < after.Length; i++)
        {
            char ch = after[i];
            if (ch == '[') depth++;
            else if (ch == ']')
            {
                depth--;
                if (depth == 0) return after.Substring(start + 1, i - start - 1).Trim();
            }
        }
        return null;
    }

    private static void CountBrackets(char ch, ref int paren, ref int bracket)
    {
        switch (ch) { case '(': paren++; break; case ')': paren--; break; case '[': bracket++; break; case ']': bracket--; break; }
    }

    private static void FlushParagraph(StringBuilder buf, InternalDocumentBuilder builder)
    {
        if (buf.Length == 0) return;
        var (resolved, citationKeys) = ExtractCitations(buf.ToString().Trim());
        var (text, annotations) = ParseInlineAnnotations(resolved);
        byte[] tb = Encoding.UTF8.GetBytes(text);
        foreach (var ann in annotations)
        {
            if (ann.Kind.Which != AnnotationKind.Tag.Link) continue;
            string url = ann.Kind.Url ?? "";
            if (url.Length == 0) continue;
            string? label = null;
            if (ann.End <= (uint)tb.Length && ann.Start <= ann.End)
                label = Encoding.UTF8.GetString(tb, (int)ann.Start, (int)(ann.End - ann.Start));
            builder.PushUri(MarkupHelpers.Hyperlink(url, label));
        }
        builder.PushParagraph(text, annotations, null, null);
        // Each citation follows the paragraph that made it, so the prose keeps its marker and the
        // reference is still addressable on its own.
        foreach (string key in citationKeys) builder.PushCitation(key, key, null);
        buf.Clear();
    }

    /// <summary>
    /// Replace Typst's citation forms with a readable <c>[key]</c> marker and return the keys.
    /// </summary>
    /// <remarks>
    /// A reference is written <c>#cite(&lt;key&gt;)</c> or, far more often, <c>@key</c>. Leaving
    /// the raw form in the text puts Typst source into the prose; dropping it loses the
    /// reference. The <c>@</c> form must be preceded by the start of the text, whitespace or an
    /// opening parenthesis, so an email address is not read as a citation.
    /// </remarks>
    private static (string Text, List<string> Keys) ExtractCitations(string raw)
    {
        var keys = new List<string>();
        string afterCite = CiteRe().Replace(raw, m =>
        {
            keys.Add(m.Groups[1].Value);
            return $"[{m.Groups[1].Value}]";
        });
        string afterAt = AtCiteRe().Replace(afterCite, m =>
        {
            keys.Add(m.Groups[2].Value);
            return $"{m.Groups[1].Value}[{m.Groups[2].Value}]";
        });
        return (afterAt, keys);
    }

    private static void EmitTable(string tableStr, InternalDocumentBuilder builder)
    {
        int numCols = 0;
        var cm = ColumnsRe().Match(tableStr);
        if (cm.Success && int.TryParse(cm.Groups[1].Value, out var nc)) numCols = nc;

        var cells = new List<string>();
        bool inBracket = false;
        var cell = new StringBuilder();
        foreach (char ch in tableStr)
        {
            if (ch == '[') { inBracket = true; cell.Clear(); }
            else if (ch == ']' && inBracket) { cells.Add(cell.ToString().Trim()); inBracket = false; cell.Clear(); }
            else if (inBracket) cell.Append(ch);
        }
        if (cells.Count == 0) return;

        int effCols = numCols > 0 ? numCols : cells.Count;
        var rows = new List<List<string>>();
        for (int i = 0; i < cells.Count; i += effCols)
            rows.Add(cells.GetRange(i, Math.Min(effCols, cells.Count - i)));
        builder.PushTableFromCells(rows, null, null);
    }

    // ── inline annotations (byte-based) ─────────────────────────────────────

    private static (string, List<TextAnnotation>) ParseInlineAnnotations(string raw)
    {
        byte[] b = Encoding.UTF8.GetBytes(raw);
        int len = b.Length;
        var outBuf = new Utf8Buf();
        var anns = new List<TextAnnotation>();
        int i = 0;

        while (i < len)
        {
            if (b[i] == (byte)'#' && SliceStartsWith(b, i, "#link(\""))
            {
                var link = ParseLinkAt(b, i);
                if (link is not null)
                {
                    uint start = outBuf.Len;
                    outBuf.Append(link.Value.display);
                    uint end = outBuf.Len;
                    anns.Add(MarkupHelpers.Annotation(start, end, MarkupHelpers.Link(link.Value.url, null)));
                    i += link.Value.consumed;
                    continue;
                }
            }

            int clen = Utf8CharLen(b, i);
            char c = (char)0;
            if (clen == 1) c = (char)b[i];

            if (c == '*') HandleMarker(b, ref i, len, outBuf, anns, (byte)'*', MarkupHelpers.Bold);
            else if (c == '_') HandleMarker(b, ref i, len, outBuf, anns, (byte)'_', MarkupHelpers.Italic);
            else if (c == '`') HandleMarker(b, ref i, len, outBuf, anns, (byte)'`', MarkupHelpers.Code);
            else { outBuf.Append(Enc(b, i, i + clen)); i += clen; }
        }
        return (outBuf.ToString(), anns);
    }

    private static void HandleMarker(byte[] b, ref int i, int len, Utf8Buf outBuf, List<TextAnnotation> anns, byte marker, AnnotationKind kind)
    {
        int close = FindClosingMarkerByte(b, i + 1, marker);
        if (close >= 0)
        {
            uint start = outBuf.Len;
            outBuf.Append(Enc(b, i + 1, close));
            uint end = outBuf.Len;
            if (end > start) anns.Add(MarkupHelpers.Annotation(start, end, kind));
            i = close + 1;
        }
        else { outBuf.AppendByte(marker); i += 1; }
    }

    private static int FindClosingMarkerByte(byte[] b, int start, byte marker)
    {
        for (int j = start; j < b.Length; j++) if (b[j] == marker) return j;
        return -1;
    }

    private static (string url, string display, int consumed)? ParseLinkAt(byte[] b, int at)
    {
        string tail = Encoding.UTF8.GetString(b, at, b.Length - at);
        var m = LinkRe().Match(tail);
        if (!m.Success || m.Index != 0) return null;
        string url = m.Groups[1].Value;
        string display = m.Groups[2].Value;
        int consumed = Encoding.UTF8.GetByteCount(m.Value);
        return (url, display, consumed);
    }

    private static string? ExtractBracketContent(string s, string prefix)
    {
        if (!s.StartsWith(prefix, StringComparison.Ordinal)) return null;
        string after = s.Substring(prefix.Length);
        int end = after.IndexOf(']');
        if (end < 0) return null;
        return after.Substring(0, end);
    }

    private static string Enc(byte[] b, int s, int e) => Encoding.UTF8.GetString(b, s, e - s);
    private static bool SliceStartsWith(byte[] b, int at, string s)
    {
        byte[] m = Encoding.ASCII.GetBytes(s);
        if (at + m.Length > b.Length) return false;
        for (int k = 0; k < m.Length; k++) if (b[at + k] != m[k]) return false;
        return true;
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
}
