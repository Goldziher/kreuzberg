using System.Text;
using System.Text.Json;
using XRay.Content.Core;
using XRay.Content.Internal.Markup;
using XRay.Content.Types;

namespace XRay.Content.Extractors;

/// <summary>
/// LaTeX extractor. Ported from Rust `extractors/latex/`. Builds the InternalDocument from a
/// line-based parser with inline-command stripping and annotation tracking; metadata comes from
/// preamble \title/\author/\date commands.
/// </summary>
public sealed class LatexExtractor : IExtractor
{
    public IEnumerable<string> SupportedMimeTypes => new[] { "application/x-latex", "text/x-tex" };
    public int Priority => 50;

    private static readonly Dictionary<string, byte> HeadingWithChapters = new()
    {
        ["chapter"] = 1, ["chapter*"] = 1, ["section"] = 2, ["section*"] = 2, ["subsection"] = 3, ["subsection*"] = 3,
        ["subsubsection"] = 4, ["subsubsection*"] = 4, ["paragraph"] = 5, ["paragraph*"] = 5,
    };
    private static readonly Dictionary<string, byte> HeadingNoChapters = new()
    {
        ["section"] = 1, ["section*"] = 1, ["subsection"] = 2, ["subsection*"] = 2,
        ["subsubsection"] = 3, ["subsubsection*"] = 3, ["paragraph"] = 4, ["paragraph*"] = 4,
    };

    private static readonly HashSet<string> SkipCommands = new()
    {
        "maketitle", "tableofcontents", "listoffigures", "listoftables", "setcounter", "addtocounter", "newpage",
        "clearpage", "cleardoublepage", "pagestyle", "thispagestyle", "pagenumbering", "setlength", "addtolength",
        "newcommand", "renewcommand", "def", "let", "input", "include", "bibliography", "bibliographystyle",
        "graphicspath", "geometry", "hypersetup", "usepackage", "documentclass", "doublespacing", "singlespacing",
        "onehalfspacing", "VerbatimFootnotes",
    };

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        // Upstream's one-line growth guard: the input's own size is charged against
        // `max_content_size` before parsing, so a document too large to render is refused
        // rather than rendered and then found to be too large.
        SecurityBudget.FromConfig(config).AccountText(content.Length);
        string source = Encoding.UTF8.GetString(content);
        bool inject = true;
        var metadata = ExtractMetadata(source);
        var doc = BuildInternalDocument(source, inject);
        doc.MimeType = mimeType;
        doc.Metadata = metadata;
        return doc;
    }

    // ── metadata ──────────────────────────────────────────────────────────

    private static Metadata ExtractMetadata(string source)
    {
        var meta = new Metadata();
        var additional = new Dictionary<string, string>();
        bool isPlainTex = source.Contains("\\bye") && !source.Contains("\\begin{document}");
        bool inDocument = isPlainTex;
        foreach (var line in MarkupHelpers.Lines(source))
        {
            string trimmed = line.Trim();
            if (isPlainTex && trimmed.Contains("\\bye")) break;
            if (!inDocument && !isPlainTex) ExtractMetadataFromLine(trimmed, meta, additional);
            if (!isPlainTex && trimmed.Contains("\\begin{document}")) { inDocument = true; if (trimmed.Contains("\\end{document}")) break; continue; }
            if (!isPlainTex && trimmed.Contains("\\end{document}")) break;
        }
        foreach (var (k, v) in additional) meta.Additional[k] = JsonSerializer.SerializeToElement(v, Json.Options);
        return meta;
    }

    private static void ExtractMetadataFromLine(string line, Metadata meta, Dictionary<string, string> additional)
    {
        if (line.StartsWith("\\title{"))
        {
            var t = ExtractBraced(line, "title");
            if (t is not null && meta.Title is null) { meta.Title = t; additional["title"] = t; }
        }
        else if (line.StartsWith("\\author{"))
        {
            var a = ExtractBraced(line, "author");
            if (a is not null && meta.CreatedBy is null) { meta.CreatedBy = a; additional["author"] = a; }
        }
        else if (line.StartsWith("\\date{"))
        {
            var d = ExtractBraced(line, "date");
            if (d is not null && meta.CreatedAt is null) { meta.CreatedAt = d; additional["date"] = d; }
        }
    }

    // ── inline command stripping (byte-based) ────────────────────────────────

    private static (string, List<TextAnnotation>) StripInlineCommands(string input)
    {
        byte[] b = Encoding.UTF8.GetBytes(input);
        int len = b.Length;
        var outBuf = new Utf8Buf();
        var anns = new List<TextAnnotation>();
        int pos = 0;

        while (pos < len)
        {
            Cancel.Every(pos, 0xFFF);
            if (b[pos] == (byte)'\\')
            {
                var inlineCmd = TryParseInlineCommand(b, pos);
                if (inlineCmd is not null)
                {
                    var (kind, cmdContent, newPos) = inlineCmd.Value;
                    uint start = outBuf.Len;
                    var (innerText, innerAnns) = StripInlineCommands(cmdContent);
                    outBuf.Append(innerText);
                    uint end = outBuf.Len;
                    foreach (var ia in innerAnns)
                        anns.Add(MarkupHelpers.Annotation(ia.Start + start, ia.End + start, ia.Kind));
                    if (start < end) anns.Add(MarkupHelpers.Annotation(start, end, kind));
                    pos += newPos;
                    continue;
                }
                var special = TryParseSpecialCommand(b, pos);
                if (special is not null)
                {
                    var (replacement, consumed) = special.Value;
                    outBuf.Append(replacement);
                    pos += consumed;
                    continue;
                }
                var unknown = TrySkipUnknownCommand(b, pos);
                if (unknown is not null)
                {
                    var (plain, consumed) = unknown.Value;
                    if (plain.Length != 0)
                    {
                        var (innerText, innerAnns) = StripInlineCommands(plain);
                        uint start = outBuf.Len;
                        outBuf.Append(innerText);
                        foreach (var ia in innerAnns)
                            anns.Add(MarkupHelpers.Annotation(ia.Start + start, ia.End + start, ia.Kind));
                    }
                    pos += consumed;
                    continue;
                }
                outBuf.AppendByte((byte)'\\');
                pos += 1;
            }
            else if (b[pos] == (byte)'$')
            {
                outBuf.AppendByte((byte)'$');
                pos += 1;
                while (pos < len && b[pos] != (byte)'$')
                {
                    int cl = Utf8CharLen(b, pos);
                    outBuf.Append(Enc(b, pos, pos + cl));
                    pos += cl;
                }
                if (pos < len) { outBuf.AppendByte((byte)'$'); pos += 1; }
            }
            else if (b[pos] == (byte)'-' && pos + 2 < len && b[pos + 1] == (byte)'-' && b[pos + 2] == (byte)'-') { outBuf.Append("—"); pos += 3; }
            else if (b[pos] == (byte)'-' && pos + 1 < len && b[pos + 1] == (byte)'-') { outBuf.Append("–"); pos += 2; }
            else if (b[pos] == (byte)'`' && pos + 1 < len && b[pos + 1] == (byte)'`') { outBuf.Append("“"); pos += 2; }
            else if (b[pos] == (byte)'\'' && pos + 1 < len && b[pos + 1] == (byte)'\'') { outBuf.Append("”"); pos += 2; }
            else if (b[pos] == (byte)'`') { outBuf.Append("‘"); pos += 1; }
            else if (b[pos] == (byte)'\'') { outBuf.Append("’"); pos += 1; }
            else { int cl = Utf8CharLen(b, pos); outBuf.Append(Enc(b, pos, pos + cl)); pos += cl; }
        }
        return (outBuf.ToString(), anns);
    }

    private static (AnnotationKind, string, int)? TryParseInlineCommand(byte[] b, int pos)
    {
        string tail = Enc(b, pos, b.Length);
        (string prefix, AnnotationKind kind)[] commands =
        {
            ("\\textbf{", MarkupHelpers.Bold), ("\\emph{", MarkupHelpers.Italic), ("\\textit{", MarkupHelpers.Italic),
            ("\\underline{", MarkupHelpers.Underline), ("\\texttt{", MarkupHelpers.Code),
        };
        foreach (var (prefix, kind) in commands)
        {
            if (tail.StartsWith(prefix, StringComparison.Ordinal))
            {
                var braced = ReadBraced(tail.Substring(prefix.Length));
                if (braced is not null) return (kind, braced.Value.content, prefix.Length + braced.Value.consumed);
            }
        }
        if (tail.StartsWith("\\href{", StringComparison.Ordinal))
        {
            string afterHref = tail.Substring("\\href{".Length);
            var url = ReadBraced(afterHref);
            if (url is not null)
            {
                string afterUrl = afterHref.Substring(url.Value.consumed);
                if (afterUrl.StartsWith('{'))
                {
                    var linkText = ReadBraced(afterUrl.Substring(1));
                    if (linkText is not null)
                    {
                        int total = "\\href{".Length + url.Value.consumed + 1 + linkText.Value.consumed;
                        return (MarkupHelpers.Link(url.Value.content, null), linkText.Value.content, total);
                    }
                }
            }
        }
        if (tail.StartsWith("\\url{", StringComparison.Ordinal))
        {
            var url = ReadBraced(tail.Substring("\\url{".Length));
            if (url is not null)
            {
                int total = "\\url{".Length + url.Value.consumed;
                return (MarkupHelpers.Link(url.Value.content, null), url.Value.content, total);
            }
        }
        if (tail.StartsWith("\\verb", StringComparison.Ordinal))
        {
            string afterVerb = tail.Substring("\\verb".Length);
            if (afterVerb.Length > 0)
            {
                char delim = afterVerb[0];
                if (!char.IsLetter(delim) && delim != '{')
                {
                    int delimLen = Encoding.UTF8.GetByteCount(delim.ToString());
                    string afterDelim = afterVerb.Substring(1);
                    int endPos = afterDelim.IndexOf(delim);
                    if (endPos >= 0)
                    {
                        string cont = afterDelim.Substring(0, endPos);
                        int total = "\\verb".Length + delimLen + Encoding.UTF8.GetByteCount(afterDelim.Substring(0, endPos)) + delimLen;
                        return (MarkupHelpers.Code, cont, total);
                    }
                }
            }
        }
        return null;
    }

    private static (string, int)? TryParseSpecialCommand(byte[] b, int pos)
    {
        string tail = Enc(b, pos, b.Length);
        (string, string)[] braced =
        {
            ("\\textgreater{}", ">"), ("\\textless{}", "<"), ("\\textbackslash{}", "\\"), ("\\ldots{}", "…"),
            ("\\textendash{}", "–"), ("\\textemdash{}", "—"), ("\\textasciitilde{}", "~"),
            ("\\textasciicircum{}", "^"), ("\\textbar{}", "|"),
        };
        foreach (var (prefix, rep) in braced) if (tail.StartsWith(prefix, StringComparison.Ordinal)) return (rep, prefix.Length);
        (string, string)[] simple =
        {
            ("\\ldots", "…"), ("\\dots", "…"), ("\\&", "&"), ("\\#", "#"), ("\\_", "_"), ("\\{", "{"),
            ("\\}", "}"), ("\\%", "%"), ("\\$", "$"), ("\\\\", "\n"), ("\\,", " "), ("\\;", " "), ("\\!", ""),
            ("\\~", "~"), ("\\^{}", "^"),
        };
        foreach (var (prefix, rep) in simple) if (tail.StartsWith(prefix, StringComparison.Ordinal)) return (rep, prefix.Length);
        if (tail.StartsWith("\\ensuremath{", StringComparison.Ordinal))
        {
            var c = ReadBraced(tail.Substring("\\ensuremath{".Length));
            if (c is not null) return (c.Value.content, "\\ensuremath{".Length + c.Value.consumed);
        }
        return null;
    }

    private static (string, int)? TrySkipUnknownCommand(byte[] b, int pos)
    {
        string tail = Enc(b, pos, b.Length);
        if (!tail.StartsWith('\\')) return null;
        string afterBackslash = tail.Substring(1);
        int cmdEnd = 0;
        while (cmdEnd < afterBackslash.Length && char.IsLetter(afterBackslash[cmdEnd])) cmdEnd++;
        if (cmdEnd == 0) return null;
        int totalCmd = 1 + cmdEnd;
        string rest = tail.Substring(totalCmd);
        int consumed = totalCmd;
        if (rest.StartsWith('['))
        {
            int bracketEnd = rest.IndexOf(']');
            if (bracketEnd >= 0) { consumed += bracketEnd + 1; rest = tail.Substring(consumed); }
        }
        if (rest.StartsWith('{'))
        {
            var braced = ReadBraced(rest.Substring(1));
            if (braced is not null) { consumed += 1 + braced.Value.consumed; return (braced.Value.content, consumed); }
        }
        return ("", consumed);
    }

    // Read braced content: `input` starts just after the opening '{'. Returns content and
    // byte count consumed (including the closing '}').
    private static (string content, int consumed)? ReadBraced(string input)
    {
        byte[] b = Encoding.UTF8.GetBytes(input);
        int depth = 1;
        var content = new Utf8Buf();
        int pos = 0;
        while (pos < b.Length)
        {
            int cl = Utf8CharLen(b, pos);
            byte c = b[pos];
            if (cl == 1 && c == (byte)'{') { depth++; content.AppendByte(c); }
            else if (cl == 1 && c == (byte)'}')
            {
                depth--;
                if (depth == 0) return (content.ToString(), pos + cl);
                content.AppendByte(c);
            }
            else content.Append(Enc(b, pos, pos + cl));
            pos += cl;
        }
        return null;
    }

    private static string? ExtractIncludegraphicsPath(string line)
    {
        const string prefix = "\\includegraphics";
        int start = line.IndexOf(prefix, StringComparison.Ordinal);
        if (start < 0) return null;
        string after = line.Substring(start + prefix.Length);
        string rest;
        if (after.StartsWith('['))
        {
            int be = after.IndexOf(']');
            if (be < 0) return null;
            rest = after.Substring(be + 1);
        }
        else rest = after;
        if (!rest.StartsWith('{')) return null;
        string inner = rest.Substring(1);
        int end = inner.IndexOf('}');
        if (end < 0) return null;
        string path = inner.Substring(0, end).Trim();
        return path.Length == 0 ? null : path;
    }

    private static string? ExtractCaption(string content)
    {
        const string prefix = "\\caption{";
        int start = content.IndexOf(prefix, StringComparison.Ordinal);
        if (start < 0) return null;
        var braced = ReadBraced(content.Substring(start + prefix.Length));
        return braced?.content;
    }

    // ── build ───────────────────────────────────────────────────────────────

    private static InternalDocument BuildInternalDocument(string source, bool inject)
    {
        var b = new InternalDocumentBuilder("latex");
        var lines = MarkupHelpers.Lines(source);
        bool isPlainTex = source.Contains("\\bye") && !source.Contains("\\begin{document}");
        bool inDocument = isPlainTex;
        bool hasChapters = source.Contains("\\chapter{") || source.Contains("\\chapter*{");
        var headingMap = hasChapters ? HeadingWithChapters : HeadingNoChapters;

        var metaEntries = new List<(string, string)>();
        foreach (var cmd in new[] { "title", "author", "date" })
        {
            var v = ExtractBraced(source, cmd);
            if (v is not null && v.Length != 0) metaEntries.Add((cmd, v));
        }
        if (metaEntries.Count > 0) b.PushMetadataBlock(metaEntries, null);

        int i = 0;
        while (i < lines.Count)
        {
            string trimmed = lines[i].Trim();
            if (isPlainTex && trimmed.Contains("\\bye")) break;
            if (!isPlainTex && trimmed.Contains("\\begin{document}")) { inDocument = true; i++; continue; }
            if (!isPlainTex && trimmed.Contains("\\end{document}")) break;
            if (!inDocument) { i++; continue; }

            if ((trimmed.Contains("\\begin{") || trimmed.Contains("\\begin {")) && ExtractEnvName(trimmed) is string envName)
            {
                if (HandleEnvironment(b, lines, ref i, trimmed, envName, headingMap, inject)) continue;
            }

            ProcessContentLine(trimmed, lines, ref i, b, headingMap, inject);
            i++;
        }
        return b.Build();
    }

    private static bool HandleEnvironment(InternalDocumentBuilder b, List<string> lines, ref int i, string trimmed,
        string envName, Dictionary<string, byte> headingMap, bool inject)
    {
        switch (envName)
        {
            case "itemize": case "enumerate": case "description":
            {
                bool ordered = envName == "enumerate";
                var (envContent, newI) = CollectEnvironment(lines, i, envName);
                b.PushList(ordered);
                BuildListItems(b, envContent, ordered);
                b.EndList();
                i = newI; return true;
            }
            // `longtable`, `tabularx` and `tabulary` hold the same `&`-separated cells as
            // `tabular`; only their preamble differs, and that is dropped either way.
            case "tabular": case "longtable": case "tabularx": case "tabulary":
            {
                var (envContent, newI) = CollectEnvironment(lines, i, envName);
                var cells = ParseTabularCells(envContent);
                if (cells.Count > 0)
                {
                    var caption = ExtractCaption(envContent);
                    var label = ExtractLabel(envContent);
                    uint idx = b.PushTableFromCells(cells, null, null);
                    if (label is not null) b.SetAnchor(idx, label);
                    if (caption is not null)
                    {
                        uint capIdx = b.PushParagraph(caption, new(), null, null);
                        b.PushRelationship(capIdx, RelationshipTarget.FromIndex(idx), RelationshipKind.Caption);
                    }
                }
                i = newI; return true;
            }
            case "table":
            {
                var (envContent, newI) = CollectEnvironment(lines, i, "table");
                var caption = ExtractCaption(envContent);
                var label = ExtractLabel(envContent);
                const string endTag = "\\end{tabular}";
                if (envContent.Contains("\\begin{tabular}"))
                {
                    int start = envContent.IndexOf("\\begin{tabular}", StringComparison.Ordinal);
                    int end = envContent.IndexOf(endTag, StringComparison.Ordinal);
                    if (start >= 0 && end >= 0)
                    {
                        string tabularContent = envContent.Substring(start, end + endTag.Length - start);
                        var innerLines = MarkupHelpers.Lines(tabularContent);
                        var (innerContent, _) = CollectEnvironment(innerLines, 0, "tabular");
                        var cells = ParseTabularCells(innerContent);
                        if (cells.Count > 0)
                        {
                            uint idx = b.PushTableFromCells(cells, null, null);
                            if (label is not null) b.SetAnchor(idx, label);
                            if (caption is not null)
                            {
                                uint capIdx = b.PushParagraph(caption, new(), null, null);
                                b.PushRelationship(capIdx, RelationshipTarget.FromIndex(idx), RelationshipKind.Caption);
                            }
                        }
                    }
                }
                i = newI; return true;
            }
            case "figure":
            {
                var (envContent, newI) = CollectEnvironment(lines, i, "figure");
                var caption = ExtractCaption(envContent);
                var label = ExtractLabel(envContent);
                var path = ExtractIncludegraphicsPath(envContent);
                if (path is not null)
                {
                    b.PushUri(MarkupHelpers.Image(path, caption));
                    if (inject)
                    {
                        uint idx = b.PushParagraph($"[image: {path}]", new(), null, null);
                        if (label is not null) b.SetAnchor(idx, label);
                        if (caption is not null)
                        {
                            uint capIdx = b.PushParagraph(caption, new(), null, null);
                            b.PushRelationship(capIdx, RelationshipTarget.FromIndex(idx), RelationshipKind.Caption);
                        }
                    }
                }
                i = newI; return true;
            }
            case "equation": case "equation*": case "align": case "align*": case "gather": case "gather*":
            case "multline": case "multline*": case "eqnarray": case "eqnarray*": case "math": case "displaymath":
            case "flalign": case "flalign*": case "cases":
            {
                var (envContent, newI) = CollectEnvironment(lines, i, envName);
                string formula = $"\\begin{{{envName}}}\n{envContent}\\end{{{envName}}}";
                uint idx = b.PushFormula(formula, null, null);
                var lbl = ExtractLabel(envContent);
                if (lbl is not null) b.SetAnchor(idx, lbl);
                i = newI; return true;
            }
            case "lstlisting": case "verbatim": case "minted": case "Verbatim":
            {
                var (envContent, newI) = CollectEnvironment(lines, i, envName);
                string? language = (envName == "lstlisting" || envName == "minted") ? ExtractCodeLanguage(trimmed) : null;
                b.PushCode(envContent.Trim(), language, null, null);
                i = newI; return true;
            }
            case "quote": case "quotation":
            {
                var (envContent, newI) = CollectEnvironment(lines, i, envName);
                b.PushQuoteStart();
                BuildBody(b, MarkupHelpers.Lines(envContent), headingMap, inject);
                b.PushQuoteEnd();
                i = newI; return true;
            }
            case "obeylines":
            {
                var (envContent, newI) = CollectEnvironment(lines, i, envName);
                foreach (var line in MarkupHelpers.Lines(envContent))
                {
                    string lt = line.Trim();
                    if (lt.Length != 0)
                    {
                        var (text, annotations) = StripInlineCommands(lt);
                        if (text.Length != 0) b.PushParagraph(text, annotations, null, null);
                    }
                }
                i = newI; return true;
            }
            case "center":
            {
                var (envContent, newI) = CollectEnvironment(lines, i, "center");
                string ct = envContent.Trim();
                if (ct.StartsWith("\\rule{") || ct.StartsWith("\\rule ")) b.PushParagraph("---", new(), null, null);
                else BuildBody(b, MarkupHelpers.Lines(envContent), headingMap, inject);
                i = newI; return true;
            }
            default:
            {
                var (envContent, newI) = CollectEnvironment(lines, i, envName);
                BuildBody(b, MarkupHelpers.Lines(envContent), headingMap, inject);
                i = newI; return true;
            }
        }
    }

    private static void BuildBody(InternalDocumentBuilder b, List<string> lines, Dictionary<string, byte> headingMap, bool inject)
    {
        int i = 0;
        while (i < lines.Count)
        {
            string trimmed = lines[i].Trim();
            if ((trimmed.Contains("\\begin{") || trimmed.Contains("\\begin {")) && ExtractEnvName(trimmed) is string envName)
            {
                // In body, the Rust code omits table/figure handling; emulate its subset.
                switch (envName)
                {
                    case "itemize": case "enumerate": case "description":
                    {
                        bool ordered = envName == "enumerate";
                        var (envContent, newI) = CollectEnvironment(lines, i, envName);
                        b.PushList(ordered); BuildListItems(b, envContent, ordered); b.EndList();
                        i = newI; continue;
                    }
                    case "tabular": case "longtable": case "tabularx": case "tabulary":
                    {
                        var (envContent, newI) = CollectEnvironment(lines, i, envName);
                        var cells = ParseTabularCells(envContent);
                        if (cells.Count > 0)
                        {
                            var caption = ExtractCaption(envContent);
                            var label = ExtractLabel(envContent);
                            uint idx = b.PushTableFromCells(cells, null, null);
                            if (label is not null) b.SetAnchor(idx, label);
                            if (caption is not null)
                            {
                                uint capIdx = b.PushParagraph(caption, new(), null, null);
                                b.PushRelationship(capIdx, RelationshipTarget.FromIndex(idx), RelationshipKind.Caption);
                            }
                        }
                        i = newI; continue;
                    }
                    case "equation": case "equation*": case "align": case "align*": case "gather": case "gather*":
                    case "multline": case "multline*": case "eqnarray": case "eqnarray*": case "math": case "displaymath":
                    case "flalign": case "flalign*": case "cases":
                    {
                        var (envContent, newI) = CollectEnvironment(lines, i, envName);
                        b.PushFormula($"\\begin{{{envName}}}\n{envContent}\\end{{{envName}}}", null, null);
                        i = newI; continue;
                    }
                    case "lstlisting": case "verbatim": case "minted": case "Verbatim":
                    {
                        var (envContent, newI) = CollectEnvironment(lines, i, envName);
                        string? language = (envName == "lstlisting" || envName == "minted") ? ExtractCodeLanguage(trimmed) : null;
                        b.PushCode(envContent.Trim(), language, null, null);
                        i = newI; continue;
                    }
                    case "quote": case "quotation":
                    {
                        var (envContent, newI) = CollectEnvironment(lines, i, envName);
                        b.PushQuoteStart(); BuildBody(b, MarkupHelpers.Lines(envContent), headingMap, inject); b.PushQuoteEnd();
                        i = newI; continue;
                    }
                    case "center":
                    {
                        var (envContent, newI) = CollectEnvironment(lines, i, "center");
                        string ct = envContent.Trim();
                        if (ct.StartsWith("\\rule{") || ct.StartsWith("\\rule ")) b.PushParagraph("---", new(), null, null);
                        else BuildBody(b, MarkupHelpers.Lines(envContent), headingMap, inject);
                        i = newI; continue;
                    }
                    default:
                    {
                        var (envContent, newI) = CollectEnvironment(lines, i, envName);
                        BuildBody(b, MarkupHelpers.Lines(envContent), headingMap, inject);
                        i = newI; continue;
                    }
                }
            }
            ProcessContentLine(trimmed, lines, ref i, b, headingMap, inject);
            i++;
        }
    }

    private static bool IsSkipCommand(string trimmed)
    {
        if (!trimmed.StartsWith('\\')) return false;
        string after = trimmed.Substring(1);
        int cmdEnd = 0;
        while (cmdEnd < after.Length && char.IsLetter(after[cmdEnd])) cmdEnd++;
        return SkipCommands.Contains(after.Substring(0, cmdEnd));
    }

    private static void ProcessContentLine(string trimmed, List<string> lines, ref int i, InternalDocumentBuilder b,
        Dictionary<string, byte> headingMap, bool inject)
    {
        if (trimmed.Length == 0 || trimmed.StartsWith('%')) return;
        if (IsSkipCommand(trimmed)) return;

        if (trimmed.StartsWith('\\'))
        {
            string afterBackslash = trimmed.Substring(1);
            int cmdEnd = 0;
            while (cmdEnd < afterBackslash.Length && afterBackslash[cmdEnd] != '{' && afterBackslash[cmdEnd] != '[' && !char.IsWhiteSpace(afterBackslash[cmdEnd])) cmdEnd++;
            string cmdName = afterBackslash.Substring(0, cmdEnd);
            if (headingMap.TryGetValue(cmdName, out byte level))
            {
                string rest = afterBackslash.Substring(cmdEnd).TrimStart();
                if (rest.StartsWith('{') || rest.StartsWith('['))
                {
                    var title = ExtractHeadingTitle(trimmed, cmdName);
                    if (title is not null)
                    {
                        var (titleText, titleAnns) = StripInlineCommands(title);
                        uint idx = b.PushHeading(level, titleText, null, null);
                        byte[] tb = Encoding.UTF8.GetBytes(titleText);
                        foreach (var ann in titleAnns)
                        {
                            if (ann.Kind.Which == AnnotationKind.Tag.Link && !string.IsNullOrEmpty(ann.Kind.Url))
                            {
                                string? label = ann.End <= (uint)tb.Length && ann.Start <= ann.End
                                    ? Encoding.UTF8.GetString(tb, (int)ann.Start, (int)(ann.End - ann.Start)) : null;
                                b.PushUri(MarkupHelpers.Hyperlink(ann.Kind.Url!, label));
                            }
                        }
                        var lbl = ExtractLabel(trimmed);
                        if (lbl is not null) b.SetAnchor(idx, lbl);
                    }
                    return;
                }
            }
        }

        if (trimmed.Contains("\\includegraphics") && ExtractIncludegraphicsPath(trimmed) is string path)
        {
            b.PushUri(MarkupHelpers.Image(path, null));
            if (inject) b.PushParagraph($"[image: {path}]", new(), null, null);
            return;
        }

        ExtractRefs(trimmed, b, "\\ref{", RelationshipKind.CrossReference);
        ExtractRefs(trimmed, b, "\\cite{", RelationshipKind.CitationReference);

        if (trimmed.StartsWith("\\["))
        {
            string mathContent = trimmed;
            if (!trimmed.Contains("\\]"))
            {
                i++;
                while (i < lines.Count)
                {
                    mathContent += "\n" + lines[i];
                    if (lines[i].Trim().Contains("\\]")) break;
                    i++;
                }
            }
            string formula = TrimEndStr(TrimStartStr(mathContent, "\\["), "\\]").Trim();
            if (formula.Length != 0) b.PushFormula(formula, null, null);
            return;
        }

        string lineText = trimmed;
        while (true)
        {
            int fnStart = lineText.IndexOf("\\footnote{", StringComparison.Ordinal);
            if (fnStart < 0) break;
            string after = lineText.Substring(fnStart + "\\footnote{".Length);
            var braced = ReadBraced(after);
            if (braced is not null)
            {
                string fnStripped = CleanText(braced.Value.content);
                if (fnStripped.Length != 0)
                {
                    string fnKey = "fn:" + new string(fnStripped.Take(20).ToArray());
                    b.PushFootnoteRef(fnStripped, fnKey, null);
                    b.PushFootnoteDefinition(fnStripped, fnKey, null);
                }
                int end = fnStart + "\\footnote{".Length + braced.Value.consumed;
                lineText = lineText.Substring(0, fnStart) + lineText.Substring(end);
            }
            else break;
        }

        lineText = lineText.Trim();
        if (lineText.Length != 0)
        {
            var (text, annotations) = StripInlineCommands(lineText);
            text = text.Trim();
            if (text.Length != 0)
            {
                byte[] tb = Encoding.UTF8.GetBytes(text);
                foreach (var ann in annotations)
                {
                    if (ann.Kind.Which == AnnotationKind.Tag.Link && !string.IsNullOrEmpty(ann.Kind.Url))
                    {
                        string? label = ann.End <= (uint)tb.Length && ann.Start <= ann.End
                            ? Encoding.UTF8.GetString(tb, (int)ann.Start, (int)(ann.End - ann.Start)) : null;
                        b.PushUri(MarkupHelpers.Hyperlink(ann.Kind.Url!, label));
                    }
                }
                uint idx = b.PushParagraph(text, annotations, null, null);
                var lbl = ExtractLabel(lineText);
                if (lbl is not null) b.SetAnchor(idx, lbl);
            }
        }
    }

    private static string? ExtractLabel(string text)
    {
        const string prefix = "\\label{";
        int start = text.IndexOf(prefix, StringComparison.Ordinal);
        if (start < 0) return null;
        var braced = ReadBraced(text.Substring(start + prefix.Length));
        return braced?.content;
    }

    private static void ExtractRefs(string text, InternalDocumentBuilder b, string prefix, RelationshipKind kind)
    {
        int searchFrom = 0;
        while (true)
        {
            int pos = text.IndexOf(prefix, searchFrom, StringComparison.Ordinal);
            if (pos < 0) break;
            string after = text.Substring(pos + prefix.Length);
            var braced = ReadBraced(after);
            if (braced is not null)
            {
                foreach (var k in braced.Value.content.Split(',').Select(s => s.Trim()))
                {
                    if (k.Length != 0)
                    {
                        uint idx = b.PushParagraph($"[{k}]", new(), null, null);
                        b.PushRelationship(idx, RelationshipTarget.FromKey(k), kind);
                    }
                }
                searchFrom = pos + prefix.Length + braced.Value.consumed;
            }
            else break;
        }
    }

    private static void BuildListItems(InternalDocumentBuilder b, string content, bool ordered)
    {
        var allLines = MarkupHelpers.Lines(content);
        int i = 0;
        while (i < allLines.Count)
        {
            string trimmed = allLines[i].Trim();
            if ((trimmed.Contains("\\begin{itemize}") || trimmed.Contains("\\begin{enumerate}") || trimmed.Contains("\\begin{description}"))
                && ExtractEnvName(trimmed) is string envName)
            {
                bool nestedOrdered = envName == "enumerate";
                var (envContent, newI) = CollectEnvironment(allLines, i, envName);
                b.PushList(nestedOrdered);
                BuildListItems(b, envContent, nestedOrdered);
                b.EndList();
                i = newI; continue;
            }
            if (trimmed.StartsWith("\\item"))
            {
                string after = trimmed.Substring("\\item".Length).Trim();
                var itemParts = new List<string>();
                string firstPart;
                if (after.StartsWith('['))
                {
                    int be = after.IndexOf(']');
                    if (be >= 0)
                    {
                        string label = after.Substring(1, be - 1);
                        string rest = after.Substring(be + 1).Trim();
                        firstPart = rest.Length == 0 ? $"{label}:" : $"{label}: {rest}";
                    }
                    else firstPart = after;
                }
                else firstPart = after;
                if (firstPart.Length != 0) itemParts.Add(firstPart);

                i++;
                while (i < allLines.Count)
                {
                    string next = allLines[i].Trim();
                    if (next.Length == 0 || next.StartsWith("\\item") || next.StartsWith("\\begin{") || next.StartsWith("\\end{") || next.StartsWith("\\setcounter")) break;
                    itemParts.Add(next);
                    i++;
                }
                string text = string.Join(" ", itemParts);
                if (text.Length != 0)
                {
                    var (stripped, annotations) = StripInlineCommands(text);
                    stripped = stripped.Trim();
                    if (stripped.Length != 0) b.PushListItem(stripped, ordered, annotations, null, null);
                }
                continue;
            }
            i++;
        }
    }

    /// <summary>
    /// Whether a line inside a table environment is scaffolding rather than a row
    /// (`latex/utilities.rs::is_table_structural_line`) — a rule, one of longtable's
    /// page-break markers, a caption, a label, or a nested environment's delimiter.
    /// </summary>
    private static bool IsTableStructuralLine(string trimmed) =>
        trimmed.Length == 0
        || trimmed.StartsWith("\\hline", StringComparison.Ordinal)
        || trimmed.StartsWith("\\toprule", StringComparison.Ordinal)
        || trimmed.StartsWith("\\midrule", StringComparison.Ordinal)
        || trimmed.StartsWith("\\bottomrule", StringComparison.Ordinal)
        || trimmed.StartsWith("\\endhead", StringComparison.Ordinal)
        || trimmed.StartsWith("\\endfirsthead", StringComparison.Ordinal)
        || trimmed.StartsWith("\\endfoot", StringComparison.Ordinal)
        || trimmed.StartsWith("\\endlastfoot", StringComparison.Ordinal)
        || trimmed.StartsWith("\\caption", StringComparison.Ordinal)
        || trimmed.StartsWith("\\label", StringComparison.Ordinal)
        || trimmed.StartsWith("\\begin{", StringComparison.Ordinal)
        || trimmed.StartsWith("\\end{", StringComparison.Ordinal);

    private static List<List<string>> ParseTabularCells(string content)
    {
        var rows = new List<List<string>>();
        foreach (var line in MarkupHelpers.Lines(content))
        {
            string trimmed = line.Trim();
            if (IsTableStructuralLine(trimmed)) continue;
            string rowStr = trimmed.Replace("\\\\", "").Replace("\\hline", "");
            var cells = rowStr.Split('&').Select(s => s.Trim()).Where(s => s.Length != 0).ToList();
            if (cells.Count > 0) rows.Add(cells);
        }
        return rows;
    }

    private static string? ExtractCodeLanguage(string beginLine)
    {
        int langPos = beginLine.IndexOf("language=", StringComparison.Ordinal);
        if (langPos >= 0)
        {
            string after = beginLine.Substring(langPos + 9);
            int end = after.IndexOfAny(new[] { ',', ']', '}' });
            if (end < 0) end = after.Length;
            string lang = after.Substring(0, end).Trim();
            if (lang.Length != 0) return lang;
        }
        if (beginLine.Contains("minted"))
        {
            int braceStart = beginLine.LastIndexOf('{');
            if (braceStart >= 0)
            {
                string after = beginLine.Substring(braceStart + 1);
                int be = after.IndexOf('}');
                if (be >= 0)
                {
                    string lang = after.Substring(0, be).Trim();
                    if (lang.Length != 0 && lang != "minted") return lang;
                }
            }
        }
        return null;
    }

    // ── utilities (from latex/utilities.rs) ─────────────────────────────────

    private static string? ExtractBraced(string text, string command)
    {
        string pattern = $"\\{command}{{";
        int start = text.IndexOf(pattern, StringComparison.Ordinal);
        if (start < 0) return null;
        string after = text.Substring(start + pattern.Length);
        int depth = 1;
        var content = new StringBuilder();
        foreach (char ch in after)
        {
            if (ch == '{') { depth++; content.Append(ch); }
            else if (ch == '}') { depth--; if (depth == 0) return CleanText(content.ToString()); content.Append(ch); }
            else content.Append(ch);
        }
        return null;
    }

    private static string? ExtractEnvName(string line)
    {
        int start = line.IndexOf("\\begin{", StringComparison.Ordinal);
        if (start < 0) start = line.IndexOf("\\begin {", StringComparison.Ordinal);
        if (start < 0) return null;
        int bracePos = line.Substring(start).IndexOf('{');
        if (bracePos < 0) return null;
        string after = line.Substring(start + bracePos + 1);
        int end = after.IndexOf('}');
        if (end < 0) return null;
        return after.Substring(0, end);
    }

    private static string CleanText(string text) => text
        .Replace("\\\\", "\n").Replace("\\&", "&").Replace("\\#", "#").Replace("\\_", "_")
        .Replace("\\{", "{").Replace("\\}", "}").Replace("\\%", "%").Trim();

    private static (string, int) CollectEnvironment(List<string> lines, int startIdx, string envName)
    {
        string endMarker = $"\\end{{{envName}}}";
        string beginMarker = $"\\begin{{{envName}}}";
        string startLine = lines[startIdx];
        int beginPos = startLine.IndexOf(beginMarker, StringComparison.Ordinal);
        if (beginPos >= 0)
        {
            string afterBegin = startLine.Substring(beginPos + beginMarker.Length);
            int endPos = afterBegin.IndexOf(endMarker, StringComparison.Ordinal);
            if (endPos >= 0) return (afterBegin.Substring(0, endPos), startIdx + 1);
        }
        string endMarkerSpace = $"\\end {{{envName}}}";
        string beginMarkerSpace = $"\\begin {{{envName}}}";
        var content = new StringBuilder();
        int i = startIdx + 1;
        int depth = 1;
        while (i < lines.Count)
        {
            string line = lines[i];
            string trimmed = line.Trim();
            depth += CountOccurrences(trimmed, beginMarker);
            depth += CountOccurrences(trimmed, beginMarkerSpace);
            depth -= CountOccurrences(trimmed, endMarker);
            depth -= CountOccurrences(trimmed, endMarkerSpace);
            if (depth <= 0) return (content.ToString(), i + 1);
            content.Append(line).Append('\n');
            i++;
        }
        return (content.ToString(), i);
    }

    private static int CountOccurrences(string s, string sub)
    {
        if (sub.Length == 0) return 0;
        int count = 0, idx = 0;
        while ((idx = s.IndexOf(sub, idx, StringComparison.Ordinal)) >= 0) { count++; idx += sub.Length; }
        return count;
    }

    private static string? ExtractHeadingTitle(string line, string command)
    {
        string prefix = $"\\{command}";
        int start = line.IndexOf(prefix, StringComparison.Ordinal);
        if (start < 0) return null;
        string after = line.Substring(start + prefix.Length);
        string rest;
        if (after.StartsWith('['))
        {
            int be = after.IndexOf(']');
            if (be < 0) return null;
            rest = after.Substring(be + 1);
        }
        else rest = after;
        if (!rest.StartsWith('{')) return null;
        string content = rest.Substring(1);
        int depth = 1;
        var result = new StringBuilder();
        foreach (char ch in content)
        {
            if (ch == '{') { depth++; result.Append(ch); }
            else if (ch == '}') { depth--; if (depth == 0) return CleanText(result.ToString()); result.Append(ch); }
            else result.Append(ch);
        }
        return null;
    }

    // ── byte helpers ──────────────────────────────────────────────────────

    private static string Enc(byte[] b, int s, int e) => Encoding.UTF8.GetString(b, s, e - s);
    private static int Utf8CharLen(byte[] b, int i)
    {
        byte c = b[i];
        if (c < 0x80) return 1;
        if ((c & 0xE0) == 0xC0) return 2;
        if ((c & 0xF0) == 0xE0) return 3;
        if ((c & 0xF8) == 0xF0) return 4;
        return 1;
    }
    private static string TrimStartStr(string s, string prefix)
    {
        while (s.StartsWith(prefix, StringComparison.Ordinal)) s = s.Substring(prefix.Length);
        return s;
    }
    private static string TrimEndStr(string s, string suffix)
    {
        while (s.EndsWith(suffix, StringComparison.Ordinal)) s = s.Substring(0, s.Length - suffix.Length);
        return s;
    }
}
