// Ported from crates/xberg/src/extractors/epub/content.rs
// EPUB body-document reading, XHTML sanitisation and plain-text extraction.
// The budget-gated text walk lives in EpubExtractor, which charges each spine document before
// handing it here; these helpers stay budget-free so both callers share one implementation.

using System.IO.Compression;
using System.Text;
using System.Xml.Linq;
using XRay.Content.Internal.MathMarkup;
using XRay.Content.Types;

namespace XRay.Content.Internal.Epub;

/// <summary>A resolved, sanitised XHTML spine document. Mirrors Rust `EpubSpineDocument`.</summary>
internal sealed class EpubSpineDocument
{
    public string FilePath { get; set; } = "";
    public string Xhtml { get; set; } = "";
}

internal static class EpubContent
{
    // Block-level elements that produce newlines before/after their content.
    private static readonly HashSet<string> BlockElements = new(StringComparer.Ordinal)
    {
        "address", "article", "aside", "blockquote", "caption", "dd", "details", "dialog", "div", "dl",
        "dt", "fieldset", "figcaption", "figure", "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6",
        "header", "hgroup", "hr", "legend", "li", "main", "nav", "ol", "p", "pre", "section", "summary",
        "table", "tbody", "td", "tfoot", "th", "thead", "title", "tr", "ul",
    };

    // Elements whose entire subtree is skipped.
    //
    // `math` is deliberately absent: its subtree is converted to LaTeX (see RenderMathElement)
    // rather than dropped.
    private static readonly HashSet<string> SkipElements = new(StringComparer.Ordinal)
    {
        "head", "script", "style", "svg", "video", "audio", "source", "track", "object", "embed", "iframe",
    };

    /// <summary>Read all body documents in spine order, downgrading per-item I/O failures to warnings.</summary>
    public static (List<EpubSpineDocument> Documents, List<ProcessingWarning> Warnings) ReadBodyDocuments(
        ZipArchive archive, EpubPackageDocument package)
    {
        var documents = new List<EpubSpineDocument>();
        var warnings = new List<ProcessingWarning>();

        foreach (var spineItem in package.SpineItems)
        {
            Cancel.ThrowIfRequested();
            if (!package.Manifest.TryGetValue(spineItem.Idref, out var sourceItem))
            {
                warnings.Add(new ProcessingWarning
                {
                    Source = "epub",
                    Message = $"Spine item '{spineItem.Idref}' references a missing manifest entry",
                });
                continue;
            }

            var (renderItem, resolveErr) = ResolveRenderableManifestItem(package, spineItem.Idref);
            if (renderItem is null)
            {
                warnings.Add(new ProcessingWarning
                {
                    Source = "epub",
                    Message = $"Skipping spine item '{spineItem.Idref}' (href '{sourceItem.RawHref}'): {resolveErr}",
                });
                continue;
            }

            var (resolvedPath, pathErr) = renderItem.ResolvedPath();
            if (resolvedPath is null)
            {
                // One spine item with an href that escapes the package root used to fail the whole
                // book. It is skipped with a warning now, like a missing manifest entry or a
                // missing archive member.
                warnings.Add(new ProcessingWarning
                {
                    Source = "epub",
                    Message =
                        $"Unsafe manifest href for spine item '{spineItem.Idref}' (href '{renderItem.RawHref}'): {pathErr}",
                });
                continue;
            }
            string filePath = resolvedPath;

            // The package names its navigation documents: properties="nav" in EPUB 3,
            // <guide><reference type="toc"> in EPUB 2. Only those items are checked against the
            // content heuristic, because a navigation document often also carries body prose that
            // must be kept — and running the heuristic on every item dropped bibliographies,
            // endnotes, licence pages and chapter overviews.
            bool navigationCandidate =
                sourceItem.IsNav() || renderItem.IsNav()
                || (sourceItem.Path is not null && package.IsGuideTocCandidatePath(sourceItem.Path))
                || (renderItem.Path is not null && package.IsGuideTocCandidatePath(renderItem.Path));

            string rawXhtml;
            try
            {
                rawXhtml = EpubContainer.ReadFileFromZip(archive, filePath);
            }
            catch (EpubParseException err)
            {
                warnings.Add(new ProcessingWarning
                {
                    Source = "epub",
                    Message =
                        $"Failed to read body spine item '{filePath}' (idref '{spineItem.Idref}') from EPUB archive: {err.Message}",
                });
                continue;
            }

            string normalizedXhtml = NormalizeXhtml(rawXhtml);
            string renderXhtml = StripEmbeddedMediaElements(
                StripSpecializedNavigationSections(StripDocumentHead(normalizedXhtml)));

            if (navigationCandidate && LooksLikeNavigationDocument(renderXhtml))
                continue;

            // A page with no text but with image markup is kept, so covers, plates and
            // fixed-layout pages reach the image pipeline instead of being dropped.
            if (ExtractTextFromXhtml(renderXhtml).Length == 0 && !HasImageMarkup(renderXhtml))
                continue;

            documents.Add(new EpubSpineDocument { FilePath = filePath, Xhtml = renderXhtml });
        }

        return (documents, warnings);
    }

    /// <summary>Follow the manifest fallback chain to the first renderable body document. Mirrors Rust.</summary>
    private static (ManifestItem? Item, string Error) ResolveRenderableManifestItem(
        EpubPackageDocument package, string startIdref)
    {
        string currentId = startIdref;
        var visited = new HashSet<string>(StringComparer.Ordinal);

        while (true)
        {
            if (!visited.Add(currentId))
                return (null, $"manifest fallback cycle detected at '{currentId}'");

            if (!package.Manifest.TryGetValue(currentId, out var item))
                return (null, $"missing manifest entry '{currentId}'");

            if (item.IsRenderableBodyDocument())
                return (item, "");

            if (item.Fallback is null)
            {
                // EPUB 3 allows an SVG content document in the spine. It is rendered through the
                // SVG text walk when no XHTML fallback exists.
                if (item.IsSvg()) return (item, "");

                string mediaType = item.MediaType ?? "unknown";
                return (null, $"no renderable XHTML/DTBook fallback found for media type '{mediaType}'");
            }

            currentId = item.Fallback;
        }
    }

    /// <summary>Element names that make a page worth keeping even with no text of its own.</summary>
    private static readonly string[] ImageMarkupTags = { "<img", "<svg", "<image", "<picture" };

    /// <summary>
    /// Whether the page carries image markup, and so is worth keeping even when it has no text —
    /// a cover, a plate, or a fixed-layout page.
    /// </summary>
    public static bool HasImageMarkup(string xhtml)
    {
        string lower = xhtml.ToLowerInvariant();
        return ImageMarkupTags.Any(tag => lower.Contains(tag, StringComparison.Ordinal));
    }

    // -----------------------------------------------------------------------
    // XHTML sanitisation
    // -----------------------------------------------------------------------

    /// <summary>Strip XML declaration + DOCTYPE prelude. Mirrors `normalize_xhtml`/`strip_xml_prelude`.</summary>
    public static string NormalizeXhtml(string xml)
    {
        string rest = xml.TrimStart();

        while (true)
        {
            if (rest.StartsWith("<?xml", StringComparison.Ordinal))
            {
                int end = rest.IndexOf("?>", StringComparison.Ordinal);
                if (end >= 0)
                {
                    rest = rest.Substring(end + 2).TrimStart();
                    continue;
                }
            }

            if (rest.StartsWith("<!DOCTYPE", StringComparison.Ordinal))
            {
                int end = FindDoctypeEnd(rest.Substring("<!DOCTYPE".Length));
                if (end >= 0)
                {
                    // `end` is relative to the slice after "<!DOCTYPE"; +len("<!DOCTYPE") gives the '>' index,
                    // matching Rust `tail[end + 1..]` on the "<!DOCTYPE"-stripped tail.
                    int absAfter = "<!DOCTYPE".Length + end + 1;
                    rest = rest.Substring(absAfter).TrimStart();
                    continue;
                }
            }

            break;
        }

        return rest;
    }

    private static int FindDoctypeEnd(string tail)
    {
        int bracketDepth = 0;
        for (int idx = 0; idx < tail.Length; idx++)
        {
            char ch = tail[idx];
            if (ch == '[') bracketDepth++;
            else if (ch == ']') bracketDepth = bracketDepth > 0 ? bracketDepth - 1 : 0;
            else if (ch == '>' && bracketDepth == 0) return idx;
        }
        return -1;
    }

    /// <summary>EPUB structural semantics namespace (`epub:` prefix).</summary>
    public const string EpubNamespace = "http://www.idpf.org/2007/ops";

    /// <summary>XHTML namespace, always a supported switch-case namespace.</summary>
    public const string XhtmlNamespace = "http://www.w3.org/1999/xhtml";

    /// <summary>MathML namespace, supported only by the markup renderers.</summary>
    public const string MathmlNamespace = "http://www.w3.org/1998/Math/MathML";

    /// <summary>
    /// Resolve deprecated EPUB 3 <c>epub:switch</c> elements to the branch this renderer draws.
    /// Mirrors `resolve_epub_switch_elements`: the first <c>epub:case</c> whose
    /// <c>required-namespace</c> is one the renderer handles wins, otherwise <c>epub:default</c>,
    /// and every other branch is cut out of the markup by byte range. Markup that does not parse
    /// as XML is returned untouched.
    /// </summary>
    public static string ResolveEpubSwitchElements(string xhtml, IReadOnlyList<string> supportedNamespaces)
    {
        List<XmlElementSpan>? elements = ScanElementSpans(xhtml);
        if (elements is null)
            return xhtml;

        var removedRanges = new List<(int Start, int End)>();
        for (int i = 0; i < elements.Count; i++)
        {
            if (!IsEpubElement(elements[i], "switch"))
                continue;

            int selected = -1;
            for (int c = i + 1; c < elements.Count; c++)
            {
                var child = elements[c];
                if (child.Parent != i) continue;
                if (!IsEpubElement(child, "case")) continue;
                if (child.RequiredNamespace is not { } required) continue;
                foreach (string supported in supportedNamespaces)
                {
                    if (required.Trim() == supported) { selected = c; break; }
                }
                if (selected >= 0) break;
            }

            if (selected < 0)
            {
                for (int c = i + 1; c < elements.Count; c++)
                {
                    if (elements[c].Parent == i && IsEpubElement(elements[c], "default")) { selected = c; break; }
                }
            }

            for (int c = i + 1; c < elements.Count; c++)
            {
                var child = elements[c];
                if (child.Parent != i) continue;
                if (!IsEpubElement(child, "case") && !IsEpubElement(child, "default")) continue;
                if (c == selected) continue;
                removedRanges.Add((child.Start, child.End));
            }
        }

        if (removedRanges.Count == 0)
            return xhtml;

        removedRanges.Sort((left, right) =>
            left.Start != right.Start ? left.Start.CompareTo(right.Start) : right.End.CompareTo(left.End));

        var outerRanges = new List<(int Start, int End)>(removedRanges.Count);
        foreach (var range in removedRanges)
        {
            if (outerRanges.Count == 0 || range.Start >= outerRanges[^1].End)
                outerRanges.Add(range);
        }

        var resolved = new StringBuilder(xhtml);
        for (int i = outerRanges.Count - 1; i >= 0; i--)
            resolved.Remove(outerRanges[i].Start, outerRanges[i].End - outerRanges[i].Start);
        return resolved.ToString();
    }

    private static bool IsEpubElement(XmlElementSpan element, string localName) =>
        element.NamespaceUri == EpubNamespace
        && string.Equals(element.LocalName, localName, StringComparison.OrdinalIgnoreCase);

    /// <summary>One element of the parsed document, with the source range its markup occupies.</summary>
    private readonly record struct XmlElementSpan(
        string NamespaceUri, string LocalName, string? RequiredNamespace, int Parent, int Start, int End);

    /// <summary>
    /// Walk the document in order, recording every element's namespace, parent and source range.
    /// Returns null when the markup is not well-formed XML, which is the signal to leave it alone.
    /// </summary>
    private static List<XmlElementSpan>? ScanElementSpans(string xhtml)
    {
        var lineStarts = new List<int> { 0 };
        for (int i = 0; i < xhtml.Length; i++)
        {
            if (xhtml[i] == '\n') lineStarts.Add(i + 1);
        }

        int Offset(int line, int column)
        {
            if (line < 1 || line > lineStarts.Count) return -1;
            int offset = lineStarts[line - 1] + column - 1;
            return offset >= 0 && offset <= xhtml.Length ? offset : -1;
        }

        var settings = new System.Xml.XmlReaderSettings
        {
            DtdProcessing = System.Xml.DtdProcessing.Ignore,
            IgnoreComments = false,
            IgnoreWhitespace = false,
            CheckCharacters = false,
        };

        var elements = new List<XmlElementSpan>();
        var openParents = new Stack<int>();
        var starts = new Dictionary<int, (string Ns, string Local, string? Required, int Parent, int Start)>();

        try
        {
            using var reader = System.Xml.XmlReader.Create(new StringReader(xhtml), settings);
            var lineInfo = (System.Xml.IXmlLineInfo)reader;
            while (reader.Read())
            {
                if (reader.NodeType == System.Xml.XmlNodeType.Element)
                {
                    // The reader reports the position of the element *name*, one past the `<`.
                    int start = Offset(lineInfo.LineNumber, lineInfo.LinePosition) - 1;
                    if (start < 0 || start >= xhtml.Length || xhtml[start] != '<') return null;

                    int index = elements.Count;
                    string ns = reader.NamespaceURI;
                    string local = reader.LocalName;
                    string? required = reader.GetAttribute("required-namespace");
                    int parent = openParents.Count > 0 ? openParents.Peek() : -1;
                    elements.Add(default);
                    starts[index] = (ns, local, required, parent, start);

                    if (reader.IsEmptyElement)
                    {
                        int end = FindTagEnd(xhtml, start);
                        if (end < 0) return null;
                        elements[index] = new XmlElementSpan(ns, local, required, parent, start, end);
                        starts.Remove(index);
                    }
                    else
                    {
                        openParents.Push(index);
                    }
                }
                else if (reader.NodeType == System.Xml.XmlNodeType.EndElement)
                {
                    if (openParents.Count == 0) return null;
                    int index = openParents.Pop();
                    // The reader reports the name inside `</name>`, two past the `<`.
                    int tagStart = Offset(lineInfo.LineNumber, lineInfo.LinePosition) - 2;
                    if (tagStart < 0) return null;
                    int end = FindTagEnd(xhtml, tagStart);
                    if (end < 0) return null;
                    var record = starts[index];
                    elements[index] = new XmlElementSpan(
                        record.Ns, record.Local, record.Required, record.Parent, record.Start, end);
                    starts.Remove(index);
                }
            }
        }
        catch (System.Xml.XmlException)
        {
            return null;
        }

        return starts.Count == 0 && openParents.Count == 0 ? elements : null;
    }

    /// <summary>Index one past the `&gt;` closing the tag that starts at <paramref name="from"/>.</summary>
    private static int FindTagEnd(string s, int from)
    {
        char quote = '\0';
        for (int i = from; i < s.Length; i++)
        {
            char ch = s[i];
            if (quote != '\0')
            {
                if (ch == quote) quote = '\0';
            }
            else if (ch == '"' || ch == '\'')
            {
                quote = ch;
            }
            else if (ch == '>')
            {
                return i + 1;
            }
        }
        return -1;
    }

    /// <summary>Remove the entire &lt;head&gt; element. Mirrors `strip_document_head`.</summary>
    public static string StripDocumentHead(string xhtml) => StripElements(xhtml, "head", _ => true);

    /// <summary>
    /// Remove &lt;audio&gt; and &lt;video&gt; subtrees (`strip_embedded_media_elements`). They are
    /// delivery controls rather than book text, and the HTML conversion would otherwise emit
    /// their source URLs and the serialized fallback markup alongside the prose.
    /// </summary>
    public static string StripEmbeddedMediaElements(string xhtml) =>
        StripElements(StripElements(xhtml, "audio", _ => true), "video", _ => true);

    /// <summary>Remove specialized navigation &lt;nav&gt; sections (toc/landmarks/page-list).</summary>
    public static string StripSpecializedNavigationSections(string xhtml) =>
        StripElements(xhtml, "nav", IsSpecializedNav);

    private static bool IsSpecializedNav(string attrs)
    {
        string? type = EpubHtmlStructure.ExtractAttr(attrs, "type");
        if (type is null) return false;
        foreach (var value in type.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries))
        {
            string v = value.ToLowerInvariant();
            if (v is "toc" or "landmarks" or "page-list") return true;
        }
        return false;
    }

    /// <summary>
    /// Remove the outermost elements named <paramref name="tag"/> whose attribute string satisfies
    /// <paramref name="attrsPredicate"/>. String-surgical (byte-range) removal so the walker sees
    /// the remaining markup verbatim — mirrors the roxmltree range-removal in `strip_xml_elements`.
    /// </summary>
    private static string StripElements(string xhtml, string tag, Func<string, bool> attrsPredicate)
    {
        var sb = new StringBuilder(xhtml.Length);
        int i = 0, n = xhtml.Length;
        while (i < n)
        {
            if (xhtml[i] == '<' && MatchesTagName(xhtml, i + 1, tag, out int afterName))
            {
                int gt = xhtml.IndexOf('>', i);
                if (gt < 0)
                {
                    sb.Append(xhtml, i, n - i);
                    break;
                }
                string inner = xhtml.Substring(i + 1, gt - i - 1);
                bool selfClose = inner.TrimEnd().EndsWith('/');
                string attrs = afterName <= gt ? xhtml.Substring(afterName, gt - afterName) : "";

                if (attrsPredicate(attrs))
                {
                    if (selfClose)
                    {
                        i = gt + 1;
                        continue;
                    }
                    i = FindMatchingClose(xhtml, gt + 1, tag);
                    continue;
                }

                sb.Append(xhtml, i, gt + 1 - i);
                i = gt + 1;
                continue;
            }

            sb.Append(xhtml[i]);
            i++;
        }
        return sb.ToString();
    }

    private static bool MatchesTagName(string s, int pos, string tag, out int afterName)
    {
        afterName = pos + tag.Length;
        if (afterName > s.Length) return false;
        for (int k = 0; k < tag.Length; k++)
        {
            if (char.ToLowerInvariant(s[pos + k]) != tag[k]) return false;
        }
        if (afterName == s.Length) return true;
        char next = s[afterName];
        return next == '>' || next == '/' || char.IsWhiteSpace(next);
    }

    private static bool MatchesCloseTag(string s, int i, string tag)
    {
        if (i + 2 + tag.Length > s.Length) return false;
        if (s[i] != '<' || s[i + 1] != '/') return false;
        for (int k = 0; k < tag.Length; k++)
        {
            if (char.ToLowerInvariant(s[i + 2 + k]) != tag[k]) return false;
        }
        int after = i + 2 + tag.Length;
        if (after >= s.Length) return true;
        char next = s[after];
        return next == '>' || char.IsWhiteSpace(next);
    }

    /// <summary>Find the index just past the matching close tag, accounting for same-name nesting.</summary>
    private static int FindMatchingClose(string s, int from, string tag)
    {
        int depth = 1;
        int i = from, n = s.Length;
        while (i < n)
        {
            if (s[i] == '<')
            {
                if (MatchesCloseTag(s, i, tag))
                {
                    int gt = s.IndexOf('>', i);
                    if (gt < 0) return n;
                    depth--;
                    i = gt + 1;
                    if (depth == 0) return i;
                    continue;
                }
                if (MatchesTagName(s, i + 1, tag, out _))
                {
                    int gt = s.IndexOf('>', i);
                    if (gt < 0) return n;
                    string inner = s.Substring(i + 1, gt - i - 1);
                    if (!inner.TrimEnd().EndsWith('/')) depth++;
                    i = gt + 1;
                    continue;
                }
            }
            i++;
        }
        return n;
    }

    // -----------------------------------------------------------------------
    // Navigation-document heuristic
    // -----------------------------------------------------------------------

    public static bool LooksLikeNavigationDocument(string xhtml)
    {
        XDocument doc;
        try
        {
            doc = Cancel.ParseXml(xhtml, LoadOptions.PreserveWhitespace);
        }
        catch (Exception) when (Cancel.NotCancelling)
        {
            return false;
        }

        int linkCount = 0, listItemCount = 0, paragraphCount = 0;
        bool headingMentionsContents = false;

        foreach (var node in doc.Descendants())
        {
            string tag = node.Name.LocalName.ToLowerInvariant();
            switch (tag)
            {
                case "nav":
                    if (HasTypeToken(node, "toc")) return true;
                    break;
                case "a": linkCount++; break;
                case "li": listItemCount++; break;
                case "p": paragraphCount++; break;
                case "title":
                case "h1":
                case "h2":
                    var text = DirectText(node)?.Trim().ToLowerInvariant();
                    if (text is "contents" or "table of contents")
                        headingMentionsContents = true;
                    break;
            }
        }

        return (linkCount >= 2 && listItemCount >= 2 && paragraphCount <= 1)
            || (headingMentionsContents && linkCount >= 2);
    }

    private static bool HasTypeToken(XElement node, string token)
    {
        foreach (var attr in node.Attributes())
        {
            if (!attr.Name.LocalName.Equals("type", StringComparison.OrdinalIgnoreCase)) continue;
            foreach (var value in attr.Value.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries))
            {
                if (value.Equals(token, StringComparison.OrdinalIgnoreCase)) return true;
            }
        }
        return false;
    }

    private static string? DirectText(XElement node)
    {
        // Mirror roxmltree `Node::text()`: the text of the FIRST child node, and only when
        // that first child is itself a text node. When an element (e.g. `<a id="CONTENTS"/>`)
        // precedes the text, roxmltree returns None — so a `<h2><a/>CONTENTS</h2>` heading must
        // NOT be treated as a "Contents" nav marker. (XCData derives from XText, so CDATA counts.)
        return node.FirstNode is XText t ? t.Value : null;
    }

    // -----------------------------------------------------------------------
    // Plain-text extraction (used for emptiness checks + fallback chapters)
    // -----------------------------------------------------------------------

    /// <summary>Extract plain text via XML tree traversal, falling back to tag stripping. Mirrors Rust.</summary>
    public static string ExtractTextFromXhtml(string xhtml)
    {
        string sanitized = NormalizeXhtml(xhtml);
        try
        {
            var doc = Cancel.ParseXml(sanitized, LoadOptions.PreserveWhitespace);
            var output = new StringBuilder(xhtml.Length / 2);
            VisitNode(doc, output);
            string result = CollapseBlankLines(output.ToString()).Trim();
            if (result.Length > 0) return result;
        }
        catch (Exception) when (Cancel.NotCancelling)
        {
            // fall through to tag stripping
        }

        return StripHtmlTags(NormalizeXhtml(xhtml));
    }

    private static void VisitNode(XNode node, StringBuilder output)
    {
        switch (node)
        {
            case XText t:
            {
                string normalised = NormaliseInlineWhitespace(t.Value);
                if (normalised.Length > 0)
                {
                    string fragment = (output.Length == 0 || output[^1] == '\n')
                        ? normalised.TrimStart()
                        : normalised;
                    if (fragment.Length > 0) output.Append(fragment);
                }
                break;
            }
            case XElement e:
            {
                string tag = e.Name.LocalName.ToLowerInvariant();
                if (tag == "math") { RenderMathElement(e, output); return; }
                if (SkipElements.Contains(tag)) return;
                if (tag == "br") { output.Append('\n'); return; }
                if (tag == "hr")
                {
                    if (output.Length > 0 && output[^1] != '\n') output.Append('\n');
                    return;
                }
                bool isBlock = BlockElements.Contains(tag);
                if (isBlock && output.Length > 0 && output[^1] != '\n') output.Append('\n');
                foreach (var child in e.Nodes()) VisitNode(child, output);
                if (isBlock && output.Length > 0 && output[^1] != '\n') output.Append('\n');
                break;
            }
            case XDocument d:
                foreach (var child in d.Nodes()) VisitNode(child, output);
                break;
        }
    }

    /// <summary>
    /// Convert a <c>&lt;math&gt;</c> element to LaTeX and append it as its own <c>$$...$$</c>
    /// block, isolated by blank lines so it survives the <c>\n\n</c> paragraph split callers use
    /// downstream. Never leaks raw MathML tag text: an element that converts to nothing is dropped.
    /// </summary>
    private static void RenderMathElement(XElement node, StringBuilder output)
    {
        string trimmed = MathMl.ConvertMathmlNodeToLatex(node).Trim();
        if (trimmed.Length == 0) return;
        if (output.Length > 0 && output[^1] != '\n') output.Append('\n');
        output.Append("\n$$").Append(trimmed).Append("$$\n\n");
    }

    /// <summary>Collapse inline whitespace runs to a single space. Mirrors `normalise_inline_whitespace`.</summary>
    private static string NormaliseInlineWhitespace(string text)
    {
        var result = new StringBuilder(text.Length);
        bool prevWasWs = false;
        foreach (char ch in text)
        {
            if (ch is '\n' or '\r' or '\t' or ' ')
            {
                if (!prevWasWs) result.Append(' ');
                prevWasWs = true;
            }
            else
            {
                result.Append(ch);
                prevWasWs = false;
            }
        }
        return result.ToString();
    }

    /// <summary>Collapse 3+ consecutive newlines into exactly two. Mirrors `collapse_blank_lines`.</summary>
    private static string CollapseBlankLines(string text)
    {
        var result = new StringBuilder(text.Length);
        int consecutive = 0;
        foreach (char ch in text)
        {
            if (ch == '\n')
            {
                consecutive++;
                if (consecutive <= 2) result.Append('\n');
            }
            else
            {
                consecutive = 0;
                result.Append(ch);
            }
        }
        return result.ToString();
    }

    /// <summary>Fallback tag stripper. Mirrors `strip_html_tags`.</summary>
    private static string StripHtmlTags(string html)
    {
        var text = new StringBuilder();
        bool inTag = false;
        bool inScriptStyle = false;
        var tagName = new StringBuilder();

        foreach (char ch in html)
        {
            if (ch == '<')
            {
                inTag = true;
                tagName.Clear();
                continue;
            }
            if (ch == '>')
            {
                inTag = false;
                string tagLower = tagName.ToString().ToLowerInvariant();
                if (tagLower.Contains("script") || tagLower.Contains("style"))
                    inScriptStyle = !tagName.ToString().StartsWith('/');
                continue;
            }
            if (inTag)
            {
                tagName.Append(ch);
                continue;
            }
            if (inScriptStyle) continue;

            if (ch is '\n' or '\r' or '\t' or ' ')
            {
                if (text.Length > 0 && text[^1] != ' ') text.Append(' ');
            }
            else
            {
                text.Append(ch);
            }
        }

        var result = new StringBuilder();
        bool prevSpace = false;
        foreach (char ch in text.ToString())
        {
            if (ch == ' ')
            {
                if (!prevSpace) result.Append(ch);
                prevSpace = true;
            }
            else
            {
                result.Append(ch);
                prevSpace = false;
            }
        }
        return result.ToString().Trim();
    }
}
