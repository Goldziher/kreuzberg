using System.Text;

namespace XRay.Content.Internal.Html;

/// <summary>
/// Port of the `html-to-markdown-rs` crate's Tier-2 markdown conversion pipeline
/// (converter/main.rs + block/inline handlers), restricted to xberg's default options:
/// ATX headings, `*` emphasis, bullets "-*+", 2-space list indent, no escaping,
/// backtick code fences, inline links, Angle url style, padded tables,
/// extract_metadata=true (YAML frontmatter), Standard preprocessing preset.
/// The output is stored as `InternalDocument.PreRenderedContent` (after the xberg-side
/// `normalize_html_markdown` pass) so the pipeline returns it verbatim for markdown output.
/// </summary>
internal static class HtmlToMarkdown
{
    // ── public entry ─────────────────────────────────────────────────────────
    public static string Convert(string html) => Convert(html, null);

    /// <summary>
    /// Convert to markdown, and report the document structure the conversion saw.
    /// </summary>
    /// <remarks>
    /// The structure is collected during the same walk rather than derived from a second one, so
    /// each node carries the markdown its block rendered to.
    /// </remarks>
    public static string ConvertWithStructure(string html, out HtmlStructureCollector structure)
        => ConvertWithStructure(html, plainText: false, out structure);

    /// <summary>
    /// Convert and report the structure, optionally returning plain text instead of markdown.
    /// </summary>
    /// <remarks>
    /// `converter/main.rs` runs the markdown walk either way — that walk is what fills the
    /// structure collector — and only swaps the returned string for
    /// <see cref="HtmlPlainText"/>'s when the caller asked for plain output.
    /// </remarks>
    public static string ConvertWithStructure(string html, bool plainText, out HtmlStructureCollector structure)
    {
        structure = new HtmlStructureCollector();
        return Convert(html, structure, plainText);
    }

    /// <summary>
    /// The normalization upstream applies to the input before any of it is read
    /// (`convert_api.rs::normalize_input`): NULs dropped and every line ending reduced to a bare
    /// newline.
    /// </summary>
    internal static string NormalizeInput(string html) =>
        html.Replace("\r\n", "\n").Replace("\r", "\n").Replace("\0", "");

    private static string Convert(string html, HtmlStructureCollector? structure, bool plainText = false)
    {
        html = NormalizeInput(html);
        string prepared = StripHiddenElements(StripScriptAndStyleTags(html));
        var root = HtmlDom.Parse(prepared);
        if (HasCustomElementTags(prepared) || HasInlineBlockMisnest(root))
        {
            MarkCanonicalAttributes(root);
            DropLeadingDocumentWhitespace(root);
        }

        var sb = new StringBuilder();
        string front = ExtractFrontmatter(root);
        sb.Append(front);

        var ctx = new Ctx { Structure = structure };
        foreach (var child in root.Children)
            WalkNode(child, sb, ctx);

        if (plainText) return HtmlPlainText.Extract(root);

        string outp = sb.ToString();
        outp = TrimLineEndWhitespace(outp);
        outp = CollapseExcessBlankLines(outp);
        return outp;
    }

    // ── html5ever repair (converter/main.rs) ────────────────────────────────
    /// <summary>
    /// Whether the source holds a custom element — a tag name with a hyphen in it. Upstream
    /// re-parses such a document with html5ever (`has_custom_element_tags`), because its own
    /// parser cannot place unknown elements.
    /// </summary>
    internal static bool HasCustomElementTags(string html)
    {
        for (int i = 0; i < html.Length; i++)
        {
            if (html[i] != '<') continue;
            int j = i + 1;
            if (j < html.Length && html[j] == '/') j++;
            while (j < html.Length && char.IsWhiteSpace(html[j])) j++;
            int start = j;
            while (j < html.Length)
            {
                char c = html[j];
                if (c is '>' or '/' || char.IsWhiteSpace(c))
                {
                    if (html.AsSpan(start, j - start).IndexOf('-') >= 0) return true;
                    break;
                }
                j++;
            }
            i = j - 1;
        }
        return false;
    }

    /// <summary>
    /// Whether a block-level element sits under an inline ancestor, or a table cell under a
    /// paragraph (`has_inline_block_misnest`). Either shape means the lenient parse disagrees
    /// with the HTML5 tree, and upstream re-parses the document with html5ever.
    /// </summary>
    internal static bool HasInlineBlockMisnest(HNode root)
    {
        foreach (var node in Descendants(root))
        {
            if (node.Tag is null) continue;
            if (node.Tag is "td" or "tr" or "th" && HasParagraphAncestor(node)) return true;
            if (!IsBlockLevelName(node.Tag)) continue;

            bool preformatted = false;
            for (HNode? n = node; n is not null; n = n.Parent)
                if (n.Tag is "pre" or "code") { preformatted = true; break; }
            if (preformatted) continue;

            for (HNode? parent = node.Parent; parent is not null; parent = parent.Parent)
                if (parent.Tag is not null && IsInlineElement(parent.Tag)
                    && parent.Tag is not ("a" or "ins" or "del")) return true;
        }
        return false;
    }

    /// <summary>
    /// Whether a <c>&lt;p&gt;</c> encloses the node with no table boundary in between — the
    /// shape an unclosed <c>&lt;p&gt;</c> inside a cell leaves behind.
    /// </summary>
    private static bool HasParagraphAncestor(HNode node)
    {
        for (HNode? parent = node.Parent; parent is not null; parent = parent.Parent)
        {
            if (parent.Tag == "p") return true;
            if (parent.Tag is "table" or "body" or "html") return false;
        }
        return false;
    }

    /// <summary>Every node under this one. Iterative: a page can nest deeply enough to matter.</summary>
    private static IEnumerable<HNode> Descendants(HNode node)
    {
        var stack = new Stack<HNode>();
        for (int i = node.Children.Count - 1; i >= 0; i--) stack.Push(node.Children[i]);
        int visited = 0;
        while (stack.Count > 0)
        {
            Cancel.Every(visited++, 0x3FF);
            var current = stack.Pop();
            yield return current;
            for (int i = current.Children.Count - 1; i >= 0; i--) stack.Push(current.Children[i]);
        }
    }

    /// <summary>Whether this document reaches the walk through the html5ever repair.</summary>
    internal static bool NeedsCanonicalAttributes(string preparedHtml)
        => HasCustomElementTags(preparedHtml) || HasInlineBlockMisnest(HtmlDom.Parse(preparedHtml));

    /// <summary>
    /// The repair re-parses the document and writes it back out through html5ever's serializer,
    /// which resolves every character reference in an attribute and re-escapes the special ones.
    /// That spelling is what the second parse — and so every handler — sees.
    /// </summary>
    private static void MarkCanonicalAttributes(HNode root)
    {
        foreach (var node in Descendants(root)) node.CanonicalAttrs = true;
    }

    /// <summary>The characters the HTML5 tokenizer counts as whitespace.</summary>
    private static readonly char[] Html5Whitespace = { ' ', '\t', '\n', '\f', '\r' };

    /// <summary>
    /// Drop the whitespace that opens the document. The HTML5 tree builder ignores whitespace
    /// character tokens in its initial, before-html and before-head insertion modes, so a
    /// document whose first content follows a comment or a leading blank line reaches the walk
    /// starting at its first non-whitespace character. A repaired document is re-parsed from
    /// that tree, so the walk never sees the whitespace that came first.
    /// </summary>
    private static void DropLeadingDocumentWhitespace(HNode root)
    {
        while (root.Children.Count > 0)
        {
            var first = root.Children[0];
            if (first.Tag is not null) return;

            string trimmed = first.Text.TrimStart(Html5Whitespace);
            if (trimmed.Length > 0)
            {
                first.Text = trimmed;
                return;
            }

            root.Children.RemoveAt(0);
            for (int i = 0; i < root.Children.Count; i++) root.Children[i].Index = i;
        }
    }

    /// <summary>
    /// An attribute value as html5ever's serializer writes it: every character reference
    /// resolved, then <c>&amp;</c>, <c>&lt;</c>, <c>&gt;</c>, <c>"</c> and a no-break space
    /// written back as named entities.
    /// </summary>
    /// <summary>
    /// The spelling text reaches the walk with on the html5ever repair path: references
    /// resolved by the parse, then the ones its serializer writes back put back. A `"` is not
    /// among them in text — only in an attribute value — so this is not
    /// <see cref="CanonicalizeAttrValue"/>.
    /// </summary>
    internal static string CanonicalizeText(string value)
    {
        string decoded = HtmlWalker.DecodeEntitiesFull(value, true);
        int i = decoded.AsSpan().IndexOfAny("&<>\u00a0");
        if (i < 0) return decoded;
        var sb = new StringBuilder(decoded.Length + 8);
        sb.Append(decoded, 0, i);
        for (; i < decoded.Length; i++)
        {
            switch (decoded[i])
            {
                case '&': sb.Append("&amp;"); break;
                case '<': sb.Append("&lt;"); break;
                case '>': sb.Append("&gt;"); break;
                case '\u00a0': sb.Append("&nbsp;"); break;
                default: sb.Append(decoded[i]); break;
            }
        }
        return sb.ToString();
    }

    internal static string CanonicalizeAttrValue(string value)
    {
        string decoded = HtmlWalker.DecodeEntitiesFull(value, true);
        int i = decoded.AsSpan().IndexOfAny("&<>\"\u00a0");
        if (i < 0) return decoded;
        var sb = new StringBuilder(decoded.Length + 8);
        sb.Append(decoded, 0, i);
        for (; i < decoded.Length; i++)
        {
            switch (decoded[i])
            {
                case '&': sb.Append("&amp;"); break;
                case '<': sb.Append("&lt;"); break;
                case '>': sb.Append("&gt;"); break;
                case '"': sb.Append("&quot;"); break;
                case '\u00a0': sb.Append("&nbsp;"); break;
                default: sb.Append(decoded[i]); break;
            }
        }
        return sb.ToString();
    }

    /// <summary>`is_block_level_name` (utility/content.rs).</summary>
    internal static bool IsBlockLevelName(string tag) => tag switch
    {
        "address" or "article" or "aside" or "blockquote" or "canvas" or "dd" or "div" or "dl"
        or "dt" or "fieldset" or "figcaption" or "figure" or "footer" or "form" or "h1" or "h2"
        or "h3" or "h4" or "h5" or "h6" or "header" or "hr" or "li" or "main" or "nav" or "ol"
        or "p" or "pre" or "section" or "table" or "tfoot" or "ul" => !IsInlineElement(tag),
        _ => false,
    };

    // ── pre-parse stripping (utility/preprocessing.rs) ──────────────────────
    /// <summary>
    /// Removes <c>&lt;script&gt;</c> and <c>&lt;style&gt;</c> elements from the source text
    /// (`strip_script_and_style_tags`). A removed block leaves a space behind when it stood
    /// between two non-space characters, which is what keeps the words around it apart:
    /// `&lt;span&gt;&lt;style&gt;…&lt;/style&gt;&lt;cite&gt;` renders with that space. JSON-LD
    /// scripts are kept, since their payload is document metadata, and nothing inside an
    /// <c>&lt;svg&gt;</c> is touched — there a style element is part of the drawing.
    /// </summary>
    internal static string StripScriptAndStyleTags(string html)
    {
        if (html.IndexOf('<') < 0) return html;

        StringBuilder? output = null;
        int last = 0, idx = 0, n = html.Length, svgDepth = 0;
        while (idx < n)
        {
            Cancel.Every(idx, 0xFFFF);
            if (html[idx] != '<' || idx + 1 >= n) { idx++; continue; }

            if (MatchesTagStart(html, idx + 1, "svg"))
            {
                int openEnd = FindTagEndQuoted(html, idx + 1 + 3);
                if (openEnd > 0) { svgDepth++; idx = openEnd; continue; }
            }
            else if (html[idx + 1] == '/' && MatchesTagStart(html, idx + 2, "svg"))
            {
                int closeEnd = FindTagEndQuoted(html, idx + 2 + 3);
                if (closeEnd > 0)
                {
                    if (svgDepth > 0) svgDepth--;
                    idx = closeEnd;
                    continue;
                }
            }

            if (svgDepth > 0) { idx++; continue; }

            if (html[idx + 1] == '/' && idx + 2 < n)
            {
                if (idx + 9 <= n && string.Compare(html, idx, "</script>", 0, 9, StringComparison.OrdinalIgnoreCase) == 0)
                { idx += 9; continue; }
                if (idx + 8 <= n && string.Compare(html, idx, "</style>", 0, 8, StringComparison.OrdinalIgnoreCase) == 0)
                { idx += 8; continue; }
            }

            string? name = null;
            if (idx + 7 < n && string.Compare(html, idx, "<script", 0, 7, StringComparison.OrdinalIgnoreCase) == 0
                && html[idx + 7] is '>' or ' ' or '\t' or '\n' or '\r') name = "script";
            else if (idx + 6 < n && string.Compare(html, idx, "<style", 0, 6, StringComparison.OrdinalIgnoreCase) == 0
                && html[idx + 6] is '>' or ' ' or '\t' or '\n' or '\r') name = "style";
            if (name is null) { idx++; continue; }

            int tagEnd = html.IndexOf('>', idx + name.Length + 1);
            if (tagEnd < 0) { idx++; continue; }
            tagEnd++;

            if (name == "script" && IsJsonLdScriptOpenTag(html[idx..tagEnd])) { idx++; continue; }

            int closeIdx = FindCloseTag(html, tagEnd, name);
            if (closeIdx < 0) { idx++; continue; }

            output ??= new StringBuilder(html.Length);
            output.Append(html, last, idx - last);
            if (idx > 0 && closeIdx < n && !char.IsWhiteSpace(html[idx - 1]) && !char.IsWhiteSpace(html[closeIdx]))
                output.Append(' ');
            last = closeIdx;
            idx = closeIdx;
        }

        if (output is null) return html;
        if (last < n) output.Append(html, last, n - last);
        return output.ToString();
    }

    /// <summary>Whether the named tag starts at <paramref name="start"/> and its name ends there.</summary>
    private static bool MatchesTagStart(string html, int start, string tag)
    {
        if (start + tag.Length >= html.Length) return false;
        if (string.Compare(html, start, tag, 0, tag.Length, StringComparison.OrdinalIgnoreCase) != 0) return false;
        char after = html[start + tag.Length];
        return char.IsWhiteSpace(after) || after is '>' or '/';
    }

    /// <summary>
    /// Whether a <c>&lt;script&gt;</c> open tag declares the JSON-LD media type, which is
    /// structured metadata rather than code and so survives the strip.
    /// </summary>
    private static bool IsJsonLdScriptOpenTag(string tag)
    {
        for (int idx = 0; idx + 4 <= tag.Length; idx++)
        {
            if (string.Compare(tag, idx, "type", 0, 4, StringComparison.OrdinalIgnoreCase) != 0) continue;
            bool beforeOk = idx == 0 || char.IsWhiteSpace(tag[idx - 1]) || tag[idx - 1] is '<' or '/';
            if (!beforeOk || idx + 4 >= tag.Length) continue;
            char afterCh = tag[idx + 4];
            if (!char.IsWhiteSpace(afterCh) && afterCh != '=') continue;

            int i = idx + 4;
            while (i < tag.Length && char.IsWhiteSpace(tag[i])) i++;
            if (i >= tag.Length || tag[i] != '=') continue;
            i++;
            while (i < tag.Length && char.IsWhiteSpace(tag[i])) i++;
            if (i >= tag.Length) return false;

            int valueStart, valueEnd;
            if (tag[i] is '"' or '\'')
            {
                char quote = tag[i];
                valueStart = i + 1;
                valueEnd = valueStart;
                while (valueEnd < tag.Length && tag[valueEnd] != quote) valueEnd++;
            }
            else
            {
                valueStart = i;
                valueEnd = i;
                while (valueEnd < tag.Length && !char.IsWhiteSpace(tag[valueEnd]) && tag[valueEnd] != '>') valueEnd++;
            }

            string value = tag[valueStart..valueEnd];
            int semi = value.IndexOf(';');
            string mediaType = (semi < 0 ? value : value[..semi]).Trim();
            return mediaType.Equals("application/ld+json", StringComparison.OrdinalIgnoreCase);
        }
        return false;
    }

    /// <summary>
    /// Removes every element carrying a <c>hidden</c> attribute along with everything up to its
    /// first matching close tag (`strip_hidden_elements`). This runs over the source text before
    /// anything is parsed, and the attribute is looked for by scanning the whole open tag — so a
    /// quoted value that happens to contain the word (a category link titled "…with hidden
    /// wikidata") takes its element with it.
    /// </summary>
    internal static string StripHiddenElements(string html)
    {
        if (html.IndexOf('<') < 0) return html;

        StringBuilder? output = null;
        int last = 0, idx = 0, n = html.Length;
        while (idx < n)
        {
            Cancel.Every(idx, 0xFFFF);
            if (html[idx] != '<' || idx + 1 >= n || html[idx + 1] == '/' || html[idx + 1] == '!')
            {
                idx++;
                continue;
            }
            int tagEnd = FindTagEndQuoted(html, idx + 1);
            if (tagEnd < 0) break;

            if (!TagHasHiddenAttribute(html, idx, tagEnd) && !TagHasHiddenStyle(html, idx, tagEnd))
            { idx++; continue; }

            int nameEnd = idx + 1;
            while (nameEnd < n && !char.IsWhiteSpace(html[nameEnd]) && html[nameEnd] != '>' && html[nameEnd] != '/')
                nameEnd++;
            string name = html[(idx + 1)..nameEnd];

            bool selfClosing = html.AsSpan(idx, tagEnd - idx).EndsWith("/>")
                || name.Equals("br", StringComparison.OrdinalIgnoreCase)
                || name.Equals("hr", StringComparison.OrdinalIgnoreCase)
                || name.Equals("img", StringComparison.OrdinalIgnoreCase)
                || name.Equals("input", StringComparison.OrdinalIgnoreCase);
            int removeEnd = selfClosing ? tagEnd : FindCloseTag(html, tagEnd, name);
            if (removeEnd < 0) removeEnd = tagEnd;

            output ??= new StringBuilder(html.Length);
            output.Append(html, last, idx - last);
            last = removeEnd;
            idx = removeEnd;
        }

        if (output is null) return html;
        if (last < n) output.Append(html, last, n - last);
        return output.ToString();
    }

    /// <summary>The index just past the <c>&gt;</c> that ends a tag, ignoring quoted values.</summary>
    private static int FindTagEndQuoted(string html, int idx)
    {
        char quote = '\0';
        for (; idx < html.Length; idx++)
        {
            char c = html[idx];
            if (c is '"' or '\'')
            {
                if (quote == c) quote = '\0';
                else if (quote == '\0') quote = c;
            }
            else if (c == '>' && quote == '\0') return idx + 1;
        }
        return -1;
    }

    /// <summary>The index just past the first <c>&lt;/name&gt;</c> at or after <paramref name="start"/>.</summary>
    private static int FindCloseTag(string html, int start, string name)
    {
        for (int i = start; i < html.Length; i++)
        {
            i = html.IndexOf('<', i);
            if (i < 0) return -1;
            if (i + 2 + name.Length >= html.Length || html[i + 1] != '/') continue;
            if (string.Compare(html, i + 2, name, 0, name.Length, StringComparison.OrdinalIgnoreCase) != 0) continue;
            char after = html[i + 2 + name.Length];
            if (after != '>' && !char.IsWhiteSpace(after)) continue;
            int close = html.IndexOf('>', i + 2 + name.Length);
            if (close < 0) return -1;
            return close + 1;
        }
        return -1;
    }

    /// <summary>
    /// Whether an open tag carries a <c>hidden</c> attribute, judged the way upstream judges it:
    /// the word, whitespace-delimited, anywhere after the tag name. Quoting is not considered, so
    /// <c>data-hidden</c> and <c>aria-hidden</c> are excluded but a value's own words are not.
    /// </summary>
    /// <summary>
    /// Whether an opening tag carries the <c>hidden</c> attribute.
    /// </summary>
    /// <remarks>
    /// Walks name=value pairs rather than scanning the tag's raw text for the word: a plain
    /// substring scan also matches inside a quoted <em>value</em>, so
    /// <c>&lt;div title="… hidden from search engines"&gt;</c> read as hidden and took the whole
    /// visible element with it. <c>data-hidden</c> and <c>aria-hidden</c> are different names and
    /// do not match.
    /// </remarks>
    private static bool TagHasHiddenAttribute(string html, int start, int end)
    {
        foreach (var (name, _) in TagAttributes(html, start, end))
            if (name.Equals("hidden", StringComparison.OrdinalIgnoreCase)) return true;
        return false;
    }

    /// <summary>Walk an opening tag's name=value pairs, skipping the element name.</summary>
    private static IEnumerable<(string Name, string? Value)> TagAttributes(string html, int start, int end)
    {
        int i = start;
        while (i < end && !char.IsWhiteSpace(html[i]) && html[i] != '>') i++;
        while (i < end)
        {
            while (i < end && (char.IsWhiteSpace(html[i]) || html[i] == '/')) i++;
            if (i >= end || html[i] == '>') yield break;

            int nameStart = i;
            while (i < end && !char.IsWhiteSpace(html[i]) && html[i] != '=' && html[i] != '>' && html[i] != '/') i++;
            string name = html[nameStart..i];

            while (i < end && char.IsWhiteSpace(html[i])) i++;
            string? value = null;
            if (i < end && html[i] == '=')
            {
                i++;
                while (i < end && char.IsWhiteSpace(html[i])) i++;
                if (i < end && (html[i] == '"' || html[i] == '\''))
                {
                    char quote = html[i++];
                    int vs = i;
                    while (i < end && html[i] != quote) i++;
                    value = html[vs..i];
                    if (i < end) i++;
                }
                else
                {
                    int vs = i;
                    while (i < end && !char.IsWhiteSpace(html[i]) && html[i] != '>') i++;
                    value = html[vs..i];
                }
            }
            yield return (name, value);
        }
    }

    /// <summary>
    /// Whether an opening tag's inline <c>style</c> hides the element with
    /// <c>display: none</c> or <c>visibility: hidden</c>.
    /// </summary>
    /// <remarks>
    /// A targeted declaration scan, not a CSS parser. Within one declaration block the
    /// <em>last</em> declaration for a property wins, so <c>display:none; display:block</c> is
    /// visible; <c>!important</c> is a priority flag rather than part of the value; and a
    /// <c>/* … */</c> comment before the property name would otherwise shift the first
    /// <c>:</c> and silently defeat the whole check.
    /// </remarks>
    private static bool TagHasHiddenStyle(string html, int start, int end)
    {
        string? style = null;
        foreach (var (name, value) in TagAttributes(html, start, end))
            if (name.Equals("style", StringComparison.OrdinalIgnoreCase)) { style = value; break; }
        if (style is null) return false;

        bool displayHides = false, visibilityHides = false;
        foreach (var raw in style.Split(';'))
        {
            string cleaned = StripCssComments(raw);
            int colon = cleaned.IndexOf(':');
            if (colon < 0) continue;
            string property = cleaned[..colon].Trim();
            string value = cleaned[(colon + 1)..].Split('!')[0].Trim();
            if (property.Equals("display", StringComparison.OrdinalIgnoreCase))
                displayHides = value.Equals("none", StringComparison.OrdinalIgnoreCase);
            else if (property.Equals("visibility", StringComparison.OrdinalIgnoreCase))
                visibilityHides = value.Equals("hidden", StringComparison.OrdinalIgnoreCase);
        }
        return displayHides || visibilityHides;
    }

    /// <summary>Remove <c>/* … */</c> comments from one CSS declaration.</summary>
    private static string StripCssComments(string declaration)
    {
        if (!declaration.Contains("/*", StringComparison.Ordinal)) return declaration;
        var sb = new StringBuilder(declaration.Length);
        string rest = declaration;
        while (true)
        {
            int start = rest.IndexOf("/*", StringComparison.Ordinal);
            if (start < 0) break;
            sb.Append(rest, 0, start);
            int end = rest.IndexOf("*/", start + 2, StringComparison.Ordinal);
            if (end < 0) { rest = ""; break; }
            rest = rest[(end + 2)..];
        }
        sb.Append(rest);
        return sb.ToString();
    }

    // ── context (mirrors converter::Context) ────────────────────────────────
    internal sealed record Ctx
    {
        public bool ConvertAsInline { get; init; }
        public bool InTableCell { get; init; }
        public bool InListItem { get; init; }
        public bool InList { get; init; }
        public bool InOrderedList { get; init; }
        public int ListCounter { get; init; }
        public int ListDepth { get; init; }
        /// <summary>
        /// Cumulative width of every ancestor <c>&lt;li&gt;</c>'s own marker — the column at
        /// which this item's content starts. An ordered marker is wider than a bullet
        /// (<c>"10. "</c> is 4 columns), and a nested list has to be indented to its parent
        /// marker's actual content column or CommonMark reads it as a sibling rather than
        /// nested content.
        /// </summary>
        public int ListIndentColumns { get; init; }
        public int UlDepth { get; init; }
        public bool LooseList { get; init; }
        public bool PrevItemHadBlocks { get; init; }
        public bool InHeading { get; init; }
        public bool InCode { get; init; }
        public bool InParagraph { get; init; }
        public int BlockContentStart { get; init; }
        public int BlockquoteDepth { get; init; }
        public int InlineDepth { get; init; }
        public bool InStrong { get; init; }
        public bool InLink { get; init; }
        public bool MeasureWidthOnly { get; init; }
        // When set, every <table> handled contributes a 2D cell grid to this sink (mirrors the
        // crate's structure collector `push_table_data`): nested tables are emitted before their
        // parent, in the order the two cell-walks (render + collect) encounter them.
        public Action<List<List<string>>>? TableEmit { get; init; }
        // When set, every <img> handled reports its (alt, src) to this sink (mirrors the crate's
        // structure collector `push_image`), so images inside table cells become image nodes.
        public Action<string?, string>? ImageEmit { get; init; }
        // When set, the block handlers report what they emit to this collector as they emit it,
        // which is how a node's text comes to be the markdown that block produced.
        public HtmlStructureCollector? Structure { get; init; }
    }

    // ── table grid emission (structure collector) ────────────────────────────
    /// <summary>
    /// Walk a parsed &lt;table&gt; node and emit a 2D cell grid for it and every nested table,
    /// mirroring the html-to-markdown crate's document-structure collector. Each grid is passed
    /// to <paramref name="emit"/>; nested tables are emitted before their enclosing table.
    /// </summary>
    public static void EmitTableTree(HNode table, Action<List<List<string>>> emit,
        Action<string?, string>? imageEmit = null)
    {
        var ctx = new Ctx { TableEmit = emit, ImageEmit = imageEmit };
        var dummy = new StringBuilder();
        HandleTableWithContext(table, dummy, ctx);
    }

    /// <summary>
    /// collect_table_grid: the table's cells on a 2D grid, each rendered under an in-cell context.
    /// <para>
    /// Column positions are resolved by <see cref="Tables.GridFlatten"/> rather than by advancing
    /// through each row independently. Advancing per row ignores the columns a rowspan started
    /// earlier still covers, which slides every cell beneath one leftwards and out from under its
    /// header.
    /// </para>
    /// </summary>
    private static List<List<string>> CollectGrid(HNode table, Ctx ctx)
    {
        var rows = new List<IReadOnlyList<GridCellSpan>>();
        foreach (var row in TableRows(table))
        {
            var cells = new List<GridCellSpan>();
            foreach (var cell in CollectTableCells(row))
            {
                var (colspan, rowspan) = GetColspanRowspan(cell);
                cells.Add(new GridCellSpan(RenderCellForGrid(cell, ctx), colspan, rowspan));
            }
            rows.Add(cells);
        }

        return Tables.GridFlatten.FlattenSpannedRows(rows, c => c.ColSpan, c => c.RowSpan, c => c.Content);
    }

    private readonly record struct GridCellSpan(string Content, int ColSpan, int RowSpan);

    // Cell content for the grid: walk children under in-cell context, normalize (keep newlines),
    // trim. Mirrors collect_grid_row's `normalize_whitespace(walk_node(children)).trim()`.
    private static string RenderCellForGrid(HNode cell, Ctx ctx)
    {
        var buf = new StringBuilder();
        // The collectors are detached for this walk. It runs after the table has already been
        // rendered — and recorded — and exists only to fill the grid returned from the text it
        // builds; leaving them attached records every link, image and nested table in a cell a
        // second time, into the very structure this walk is building.
        var cctx = ctx with { InTableCell = true, Structure = null, ImageEmit = null, TableEmit = null };
        foreach (var c in cell.Children) WalkNode(c, buf, cctx);
        return NormalizeWhitespaceKeepNewlines(buf.ToString()).Trim();
    }

    // ── frontmatter (head_metadata.rs + main_helpers::extract_head_metadata) ─
    private static string ExtractFrontmatter(HNode root)
    {
        var head = FindHead(root);
        if (head is null) return "";
        var metadata = new SortedDictionary<string, string>(StringComparer.Ordinal);
        foreach (var child in head.Children)
        {
            if (child.Tag is null) continue;
            switch (child.Tag)
            {
                case "meta":
                {
                    string? name = child.Attr("name");
                    string? property = child.Attr("property");
                    string? content = child.Attr("content");
                    if (content is not null)
                    {
                        if (name is not null) metadata[$"meta-{name}"] = content;
                        if (property is not null) metadata[$"meta-{property}"] = content;
                    }
                    break;
                }
                case "title":
                {
                    var t = new StringBuilder();
                    foreach (var tc in child.Children)
                        if (tc.Tag is null && !tc.IsComment) t.Append(tc.Text);
                    // Read as written. Only a document that reaches the walk through the
                    // html5ever repair arrives with its references resolved, and there the
                    // serializer's own spelling is what the converter sees.
                    string title = (child.CanonicalAttrs ? CanonicalizeText(t.ToString()) : t.ToString()).Trim();
                    if (title.Length > 0) metadata["title"] = title;
                    break;
                }
                case "link":
                {
                    string? rel = child.Attr("rel");
                    string? href = child.Attr("href");
                    if (rel is not null && href is not null && rel.Contains("canonical", StringComparison.Ordinal))
                        metadata["canonical"] = href;
                    break;
                }
                case "base":
                {
                    string? href = child.Attr("href");
                    if (href is not null) metadata["base"] = href;
                    break;
                }
            }
        }
        if (metadata.Count == 0) return "";
        var sb = new StringBuilder("---\n");
        foreach (var (k, v) in metadata) sb.Append(k).Append(": ").Append(v).Append('\n');
        sb.Append("---\n");
        return sb.ToString();
    }

    private static HNode? FindHead(HNode node)
    {
        // Depth-first, document order, first non-empty <head> preferred but we
        // take the first head that yields metadata (mirrors extract_head_metadata's
        // "keep searching for a later one" only when empty; simplified: first head).
        if (node.Tag == "head") return node;
        foreach (var c in node.Children)
        {
            if (c.Tag is null) continue;
            var r = FindHead(c);
            if (r is not null) return r;
        }
        return null;
    }

    // ── main walker (converter/main.rs walk_node) ────────────────────────────
    internal static void WalkNode(HNode node, StringBuilder output, Ctx ctx)
    {
        if (node.IsComment) return;
        if (node.Tag is null)
        {
            ProcessTextNode(node, output, ctx);
            return;
        }

        string tag = node.Tag;

        // should_drop_for_preprocessing (Standard preset, remove_navigation + remove_forms)
        if (ShouldDrop(tag, node))
        {
            TrimTrailingWhitespace(output);
            return;
        }

        // hidden attribute stripping (strip_hidden_elements runs pre-parse in Rust)
        if (node.Attr("hidden") is not null) return;

        switch (tag)
        {
            case "h1": case "h2": case "h3": case "h4": case "h5": case "h6":
                HandleHeading(tag, node, output, ctx);
                break;
            case "p":
                HandleParagraph(node, output, ctx);
                break;
            case "strong": case "b":
                HandleStrong(node, output, ctx);
                break;
            case "em": case "i":
                HandleEmphasis(node, output, ctx);
                break;
            case "code":
                HandleInlineCode(node, output, ctx);
                break;
            case "kbd": case "samp":
                HandleKbdSamp(node, output, ctx);
                break;
            case "del": case "s": case "strike":
                HandleStrikethrough(node, output, ctx);
                break;
            case "mark":     // HighlightStyle::DoubleEqual (default) → ==…==
            case "ins":      // inserted text → ==…== (marks.rs handle_inserted)
                HandleHighlight(node, output, ctx);
                break;
            case "u": case "small": case "bdi": case "bdo":
            case "rb": case "rtc":
                WalkChildren(node, output, ctx);
                break;
            case "abbr":
                HandleAbbr(node, output, ctx);
                break;
            case "sub": case "sup":
                HandleSubSup(node, output, ctx);
                break;
            // `<cite>` has a handler in `semantic/attributes.rs` that italicizes it, but
            // `converter/main.rs` never dispatches to it — the tag falls through to the unknown
            // handler, so a citation keeps its own inline markup and gains none.
            case "var": case "dfn":
                HandleEmphasis(node, output, ctx);
                break;
            case "q":
                HandleQ(node, output, ctx);
                break;
            case "span":
                HandleSpan(node, output, ctx);
                break;
            case "a":
                HandleLink(node, output, ctx);
                break;
            case "img":
                HandleImg(node, output, ctx);
                break;
            case "br":
                HandleBr(output, ctx);
                break;
            case "hr":
                HandleHr(node, output, ctx);
                break;
            case "div":
                HandleDiv(node, output, ctx);
                break;
            case "pre":
                HandlePre(node, output, ctx);
                break;
            case "blockquote":
                HandleBlockquote(node, output, ctx);
                break;
            case "table":
                // During an outer table's width-measurement pre-pass, don't recurse into the
                // nested table handler (which would emit its grid and launch its own pre-pass);
                // fall back to descendant text content, matching the crate (issue #406).
                if (ctx.MeasureWidthOnly) { output.Append(TextContent(node)); break; }
                HandleTableWithContext(node, output, ctx);
                break;
            case "caption":
                break; // handled inside table
            case "ul":
                HandleList(node, output, ctx, ordered: false);
                break;
            case "ol":
                HandleList(node, output, ctx, ordered: true);
                break;
            case "li":
                HandleLi(node, output, ctx);
                break;
            case "dl":
                HandleDl(node, output, ctx);
                break;
            case "dt":
                HandleDt(node, output, ctx);
                break;
            case "dd":
                HandleDd(node, output, ctx);
                break;
            case "article": case "section": case "aside": case "header":
            case "footer": case "main": case "nav":
                HandleSectioning(node, output, ctx);
                break;
            case "figure":
                HandleFigure(node, output, ctx);
                break;
            case "figcaption":
                HandleFigcaption(node, output, ctx);
                break;
            case "details": case "dialog":
                HandleDetails(node, output, ctx);
                break;
            case "summary":
                HandleSummary(node, output, ctx);
                break;
            case "time": case "data":
                WalkChildren(node, output, ctx);
                break;
            case "wbr": case "thead": case "tbody": case "tfoot": case "tr": case "th": case "td":
            case "source": case "track": case "param": case "col": case "colgroup":
                break; // no-op outside table context
            case "head": case "script": case "style":
                break; // metadata / non-content
            // `<template>` is an inert, unrendered document fragment per the HTML spec, and
            // `<noscript>` content only renders with scripting disabled — never true for a
            // Markdown conversion, which mirrors a scripting-enabled browser. Neither may reach
            // the output, even though a no-JS fallback `<img>` is sometimes the page's only copy
            // of that image. A stray body `title` still has no arm of its own upstream: it
            // reaches the unknown handler, which renders its children.
            case "template": case "noscript":
                break; // inert / scripting-disabled-only content
            case "meta": case "link": case "base":
                break; // void metadata elements: no children to render
            case "html": case "body":
                WalkChildren(node, output, ctx);
                break;
            case "math":
                HandleMath(node, output, ctx);
                break;
            case "svg":
                HandleSvg(node, output, ctx);
                break;
            case "audio": case "video": case "picture": case "iframe":
                HandleMedia(tag, node, output, ctx);
                break;
            case "object": case "embed": case "canvas": case "map": case "area":
                HandleUnknown(node, output, ctx);  // no arm upstream either — the unknown handler
                break;
            case "form": case "fieldset": case "legend": case "label": case "input":
            case "textarea": case "select": case "option": case "optgroup": case "button":
            case "progress": case "meter": case "output": case "datalist":
                HandleFormElement(tag, node, output, ctx);
                break;
            default:
                HandleUnknown(node, output, ctx);
                break;
        }
    }

    private static void WalkChildren(HNode node, StringBuilder output, Ctx ctx)
    {
        foreach (var c in node.Children) WalkNode(c, output, ctx);
    }

    // ── preprocessing drop (preprocessing_helpers::should_drop_for_preprocessing) ─
    private static bool ShouldDrop(string tag, HNode node) => ShouldDropForPreprocessing(
        tag, node.Attr("role"), node.Attr("aria-label"), node.Attr("class"), node.Attr("id"));

    private static readonly string[] NavKeywords =
    {
        "nav", "navigation", "navbar", "breadcrumbs", "breadcrumb", "toc", "sidebar",
        "sidenav", "menu", "menubar", "mainmenu", "subnav", "tabs", "tablist", "toolbar",
        "pager", "pagination", "skipnav", "skip-link", "skiplinks", "site-nav", "site-menu",
        "site-header", "site-footer", "topbar", "bottombar", "masthead", "vector-nav",
        "vector-header", "vector-footer",
    };

    private static bool HasNavigationHint(HNode node) =>
        HasNavigationHint(node.Attr("role"), node.Attr("aria-label"), node.Attr("class"), node.Attr("id"));

    /// <summary>
    /// Whether an element's attributes mark it as navigation chrome. Shared with the metadata
    /// scanner, which sees attribute strings rather than nodes but has to drop exactly what
    /// preprocessing drops — otherwise a sidebar's heading is collected as a document heading.
    /// </summary>
    internal static bool HasNavigationHint(string? role, string? ariaLabel, string? cls, string? id)
    {
        if (role is not null && role is "navigation" or "menubar" or "tablist" or "toolbar") return true;
        if (ariaLabel is not null)
        {
            string lower = ariaLabel.ToLowerInvariant();
            foreach (var kw in new[] { "navigation", "menu", "contents", "table of contents", "toc" })
                if (lower.Contains(kw, StringComparison.Ordinal)) return true;
        }
        return AttrTokenMatches(cls) || AttrTokenMatches(id);
    }

    /// <summary>
    /// Whether preprocessing removes this element outright: every form and every nav, and a
    /// header, footer or aside that carries a navigation hint.
    /// </summary>
    internal static bool ShouldDropForPreprocessing(string tag, string? role, string? ariaLabel, string? cls, string? id)
    {
        if (tag is "form" or "nav") return true;
        if (tag is "header" or "footer" or "aside") return HasNavigationHint(role, ariaLabel, cls, id);
        return false;
    }

    private static bool AttrTokenMatches(string? value)
    {
        if (value is null) return false;
        foreach (var token in value.Split(new[] { ' ', '\t', '\n', '\r', '\f' }, StringSplitOptions.RemoveEmptyEntries))
        {
            var sb = new StringBuilder(token.Length);
            foreach (char c in token) sb.Append(c is '_' or ':' or '.' or '/' ? '-' : char.ToLowerInvariant(c));
            if (Array.IndexOf(NavKeywords, sb.ToString()) >= 0) return true;
        }
        return false;
    }

    // ── text node (converter/text_node.rs, Normalized whitespace mode, no escaping) ─
    private static void ProcessTextNode(HNode node, StringBuilder output, Ctx ctx)
    {
        string text = HtmlWalker.DecodeEntitiesFull(node.Text, node.CanonicalAttrs);
        if (text.Length == 0) return;

        bool hadNewlines = text.Contains('\n');

        if (text.Trim().Length == 0)
        {
            if (ctx.InCode) { output.Append(text); return; }

            if (hadNewlines)
            {
                if (output.Length == 0) return;
                if (!EndsWith(output, "\n\n"))
                {
                    string? nextTag = NextSiblingTag(node);
                    if (nextTag is not null && IsInlineElement(nextTag))
                    {
                        if (!EndsWith(output, " ") && !EndsWith(output, "\n")) output.Append(' ');
                    }
                }
                return;
            }

            if (PrevSiblingIsInlineTag(node) && NextSiblingIsInlineTag(node))
            {
                if (HasMoreThanOneChar(text))
                {
                    if (!EndsWith(output, " ")) output.Append(' ');
                }
                else output.Append(text);
            }
            else output.Append(text);
            return;
        }

        string processed;
        if (ctx.InCode)
        {
            processed = text;
        }
        else if (ctx.InTableCell)
        {
            // A table cell cannot hold a hard line break, so unlike the block-level
            // normalizer this folds `\n` and `\r` into the run before collapsing.
            string normalized = NormalizeCellWhitespace(text);
            processed = EscapeCellText(normalized);
        }
        else
        {
            bool hasDoubleNewline = text.Contains("\n\n");
            bool hasTrailingSingleNewline = text.EndsWith('\n') && !text.EndsWith("\n\n");

            string normalized = NormalizeWhitespaceKeepNewlines(text);
            var (prefix, suffix, core) = Chomp(normalized);

            bool skipPrefix = EndsWith(output, "\n\n")
                || EndsWith(output, "* ")
                || EndsWith(output, "- ")
                || EndsWith(output, ". ")
                || EndsWith(output, "] ")
                || (EndsWith(output, "\n") && prefix == " ")
                || (EndsWith(output, " ") && prefix == " " && !PrevSiblingIsInlineTag(node));

            var final = new StringBuilder();
            if (!skipPrefix && prefix.Length > 0) final.Append(prefix);
            final.Append(core); // no escaping with default options
            if (suffix.Length > 0) final.Append(suffix);
            else if (hasTrailingSingleNewline)
            {
                int safeStart = Math.Min(ctx.BlockContentStart, output.Length);
                bool atParagraphBreak = EndsWithAt(output, safeStart, "\n\n");
                if (!atParagraphBreak)
                {
                    if (hasDoubleNewline) final.Append('\n');
                    else
                    {
                        string? nextTag = NextSiblingTag(node);
                        if (nextTag == "span") { }
                        else if (ctx.InlineDepth > 0 || ctx.ConvertAsInline || ctx.InParagraph) final.Append(' ');
                        else final.Append('\n');
                    }
                }
            }
            processed = final.ToString();
        }

        if (ctx.InListItem && processed.Contains("\n\n"))
        {
            string indent = new string(' ', 4 * ctx.ListDepth);
            bool first = true;
            foreach (var part in processed.Split("\n\n"))
            {
                if (!first) { output.Append("\n\n"); output.Append(indent); }
                first = false;
                output.Append(part.Trim());
            }
        }
        else output.Append(processed);
    }

    // did the current block (starting at blockStart) end with pattern?
    private static bool EndsWithAt(StringBuilder sb, int blockStart, string pattern)
    {
        int len = sb.Length - blockStart;
        if (len < pattern.Length) return false;
        return EndsWith(sb, pattern);
    }

    // ── heading (block/heading.rs) ────────────────────────────────────────────
    private static void HandleHeading(string tag, HNode node, StringBuilder output, Ctx ctx)
    {
        int level = tag[^1] - '0';
        if (level is < 1 or > 6) level = 1;

        bool needsLeadingSep = !ctx.InTableCell && !ctx.InListItem && !ctx.ConvertAsInline
            && ctx.BlockquoteDepth == 0 && output.Length > 0 && !EndsWith(output, "\n\n");
        if (needsLeadingSep)
        {
            TrimTrailingWhitespace(output);
            output.Append("\n\n");
        }

        var text = new StringBuilder();
        var hctx = ctx with { InHeading = true, ConvertAsInline = true };
        foreach (var c in node.Children) WalkNode(c, text, hctx);

        string trimmed = text.ToString().Trim();
        if (trimmed.Length == 0) return;
        string normalized = NormalizeHeadingText(trimmed);
        PushHeading(output, ctx, level, normalized);

        // A heading in a table cell is part of the cell's content, not a heading of the document.
        if (!ctx.InTableCell) ctx.Structure?.PushHeading((byte)level, normalized);
    }

    private static string NormalizeHeadingText(string text)
    {
        if (!text.Contains('\n') && !text.Contains('\r')) return text;
        var sb = new StringBuilder(text.Length);
        bool pendingSpace = false;
        foreach (char ch in text)
        {
            if (ch is '\n' or '\r') { if (sb.Length > 0) pendingSpace = true; }
            else if ((ch == ' ' || ch == '\t') && pendingSpace) { }
            else
            {
                if (pendingSpace)
                {
                    if (sb.Length == 0 || sb[^1] != ' ') sb.Append(' ');
                    pendingSpace = false;
                }
                sb.Append(ch);
            }
        }
        return sb.ToString();
    }

    internal static void PushHeading(StringBuilder output, Ctx ctx, int level, string text)
    {
        if (text.Length == 0) return;
        if (ctx.ConvertAsInline) { output.Append(text); return; }
        if (ctx.InTableCell)
        {
            bool isTableContinuation = output.Length > 0 && !EndsWith(output, "|") && !EndsWith(output, " ") && !EndsWith(output, "<br>");
            if (isTableContinuation) output.Append("<br>");
            output.Append(text);
            return;
        }
        if (ctx.InListItem)
        {
            if (EndsWith(output, "\n"))
            {
                int indentLevel = ctx.ListDepth > 0 ? 4 * ctx.ListDepth : 0;
                if (indentLevel > 0) output.Append(' ', indentLevel);
            }
            else if (!EndsWith(output, " ") && output.Length > 0) output.Append(' ');
        }
        else if (output.Length > 0 && !EndsWith(output, "\n\n"))
        {
            if (EndsWith(output, "\n")) output.Append('\n');
            else { TrimTrailingWhitespace(output); output.Append("\n\n"); }
        }

        string suffix = ctx.InListItem || ctx.BlockquoteDepth > 0 ? "\n" : "\n\n";
        output.Append('#', level).Append(' ').Append(text).Append(suffix);
    }

    // ── paragraph (block/paragraph.rs) ────────────────────────────────────────
    private static void HandleParagraph(HNode node, StringBuilder output, Ctx ctx)
    {
        int contentStart = output.Length;

        bool isTableContinuation = ctx.InTableCell && output.Length > 0 && !EndsWith(output, "|") && !EndsWith(output, "<br>");
        bool isListContinuation = ctx.InListItem && output.Length > 0
            && !EndsWith(output, "* ") && !EndsWith(output, "- ") && !EndsWith(output, ". ");
        bool afterCodeBlock = EndsWith(output, "```\n");
        // Inside a blockquote, sibling blocks (heading, list, table, pre) manage their own
        // trailing spacing and self-terminate without a blank line. The case that still needs
        // a separator is a paragraph straight after bare inline text, which leaves no trailing
        // newline at all — without one the "> " prefixing pass merges the two into a single
        // line. Requiring *no* trailing newline (rather than no blank line) keeps the compact
        // heading-then-paragraph style intact.
        bool needsLeadingSep = !ctx.InTableCell && !ctx.InListItem && !ctx.ConvertAsInline
            && output.Length > 0 && !afterCodeBlock
            && (ctx.BlockquoteDepth > 0 ? !EndsWith(output, "\n") : !EndsWith(output, "\n\n"));

        if (isTableContinuation)
        {
            EmitTableCellBreak(output);
        }
        else if (isListContinuation)
        {
            if (!EndsWith(output, " ") && !EndsWith(output, "\n")) output.Append(' ');
            // The column this item's own content starts at, not a uniform per-depth offset
            // that ignores how wide an ordered marker is.
            output.Append(' ', ctx.ListIndentColumns);
        }
        else if (needsLeadingSep)
        {
            TrimTrailingWhitespace(output);
            output.Append("\n\n");
        }

        var pctx = ctx with { InParagraph = true, BlockContentStart = output.Length };

        var children = node.Children;
        for (int i = 0; i < children.Count; i++)
        {
            var child = children[i];
            if (child.Tag is null && !child.IsComment && child.Text.Trim().Length == 0
                && i > 0 && i < children.Count - 1
                && IsEmptyInlineElement(children[i - 1]) && IsEmptyInlineElement(children[i + 1]))
                continue;
            WalkNode(child, output, pctx);
        }

        bool hasContent = output.Length > contentStart;
        if (hasContent && !ctx.ConvertAsInline && !ctx.InTableCell)
            output.Append("\n\n");

        if (hasContent && !ctx.InTableCell && !ctx.InListItem && !ctx.ConvertAsInline)
            ctx.Structure?.PushParagraph(output.ToString(contentStart, output.Length - contentStart).Trim());
    }

    private static bool IsEmptyInlineElement(HNode n) =>
        n.Tag is "br" or "hr" or "img" or "input" or "meta" or "link";

    // ── div (block/div.rs) ───────────────────────────────────────────────────
    private static void HandleDiv(HNode node, StringBuilder output, Ctx ctx)
    {
        if (ctx.ConvertAsInline) { WalkChildren(node, output, ctx); return; }

        int contentStart = output.Length;

        bool isTableContinuation = ctx.InTableCell && output.Length > 0 && !EndsWith(output, "|") && !EndsWith(output, "<br>");
        bool isListContinuation = ctx.InListItem && output.Length > 0
            && !EndsWith(output, "* ") && !EndsWith(output, "- ") && !EndsWith(output, ". ");
        bool needsLeadingSep = !ctx.InTableCell && !ctx.InListItem && !ctx.ConvertAsInline
            && output.Length > 0 && !EndsWith(output, "\n\n");

        if (isTableContinuation)
        {
            EmitTableCellBreak(output);
        }
        else if (isListContinuation)
        {
            if (!EndsWith(output, "\n")) output.Append('\n');
            output.Append(' ', 2 * ctx.ListDepth);
        }
        else if (needsLeadingSep)
        {
            TrimTrailingWhitespace(output);
            output.Append("\n\n");
        }

        WalkChildren(node, output, ctx);

        bool hasContent = output.Length > contentStart;
        if (!hasContent) return;

        if (contentStart == 0 && StartsWith(output, "\n") && !StartsWith(output, "\n\n"))
            output.Remove(0, 1);
        TrimTrailingWhitespace(output);

        if (ctx.InTableCell) { }
        else if (ctx.InListItem)
        {
            if (isListContinuation)
            {
                if (!EndsWith(output, "\n")) output.Append('\n');
            }
            else if (!EndsWith(output, "\n\n"))
            {
                if (EndsWith(output, "\n")) output.Append('\n');
                else output.Append("\n\n");
            }
        }
        else if (!ctx.ConvertAsInline)
        {
            if (EndsWith(output, "\n\n")) { }
            else if (EndsWith(output, "\n")) output.Append('\n');
            else output.Append("\n\n");
        }
    }

    // ── inline: strong / em (inline/emphasis.rs) ─────────────────────────────
    private static void HandleStrong(HNode node, StringBuilder output, Ctx ctx)
    {
        if (ctx.InCode) { WalkChildren(node, output, ctx); return; }
        var content = new StringBuilder();
        var sctx = ctx with { InlineDepth = ctx.InlineDepth + 1, InStrong = true };
        foreach (var c in node.Children) WalkNode(c, content, sctx);

        var (prefix, suffix, trimmed) = ChompInline(content.ToString());
        if (content.ToString().Trim().Length > 0)
        {
            output.Append(prefix);
            if (ctx.InStrong) output.Append(trimmed);
            else output.Append("**").Append(trimmed).Append("**");
            AppendInlineSuffix(output, suffix, trimmed.Length > 0, node);
        }
        else if (content.Length > 0)
        {
            output.Append(prefix);
            AppendInlineSuffix(output, suffix, false, node);
        }
    }

    private static void HandleEmphasis(HNode node, StringBuilder output, Ctx ctx)
    {
        if (ctx.InCode) { WalkChildren(node, output, ctx); return; }
        var content = new StringBuilder();
        var ectx = ctx with { InlineDepth = ctx.InlineDepth + 1 };
        foreach (var c in node.Children) WalkNode(c, content, ectx);

        var (prefix, suffix, trimmed) = ChompInline(content.ToString());
        if (content.ToString().Trim().Length > 0)
        {
            output.Append(prefix).Append('*').Append(trimmed).Append('*');
            AppendInlineSuffix(output, suffix, trimmed.Length > 0, node);
        }
        else if (content.Length > 0)
        {
            output.Append(prefix);
            AppendInlineSuffix(output, suffix, false, node);
        }
        else
        {
            string? cls = node.Attr("class");
            if (cls is not null && cls.Contains("caret", StringComparison.Ordinal) && !EndsWith(output, " "))
                output.Append(" > ");
        }
    }

    private static void HandleStrikethrough(HNode node, StringBuilder output, Ctx ctx)
    {
        if (ctx.InCode) { WalkChildren(node, output, ctx); return; }
        var content = new StringBuilder();
        foreach (var c in node.Children) WalkNode(c, content, ctx);
        var (prefix, suffix, trimmed) = ChompInline(content.ToString());
        if (content.ToString().Trim().Length > 0)
        {
            output.Append(prefix).Append("~~").Append(trimmed).Append("~~");
            AppendInlineSuffix(output, suffix, trimmed.Length > 0, node);
        }
        else if (content.Length > 0)
        {
            output.Append(prefix);
            AppendInlineSuffix(output, suffix, false, node);
        }
    }

    // ins / mark → ==content== (marks.rs handle_inserted / handle_mark, DoubleEqual default)
    private static void HandleHighlight(HNode node, StringBuilder output, Ctx ctx)
    {
        if (ctx.InCode) { WalkChildren(node, output, ctx); return; }
        var content = new StringBuilder();
        foreach (var c in node.Children) WalkNode(c, content, ctx);
        var (prefix, suffix, trimmed) = ChompInline(content.ToString());
        if (content.ToString().Trim().Length > 0)
        {
            output.Append(prefix).Append("==").Append(trimmed).Append("==");
            AppendInlineSuffix(output, suffix, trimmed.Length > 0, node);
        }
        else if (content.Length > 0)
        {
            output.Append(prefix);
            AppendInlineSuffix(output, suffix, false, node);
        }
    }

    // sub/sup with empty symbols → passthrough with chomp (typography.rs)
    private static void HandleSubSup(HNode node, StringBuilder output, Ctx ctx)
    {
        var content = new StringBuilder();
        foreach (var c in node.Children) WalkNode(c, content, ctx);
        if (ctx.InCode) { output.Append(content); return; }
        var (prefix, suffix, trimmed) = ChompInline(content.ToString());
        if (content.ToString().Trim().Length > 0)
        {
            output.Append(prefix).Append(trimmed);
            AppendInlineSuffix(output, suffix, trimmed.Length > 0, node);
        }
        else if (content.Length > 0)
        {
            output.Append(prefix);
            AppendInlineSuffix(output, suffix, false, node);
        }
    }

    private static void HandleQ(HNode node, StringBuilder output, Ctx ctx)
    {
        var content = new StringBuilder();
        foreach (var c in node.Children) WalkNode(c, content, ctx);
        string trimmed = content.ToString().Trim();
        if (trimmed.Length > 0) output.Append('"').Append(trimmed).Append('"');
    }

    private static void HandleSpan(HNode node, StringBuilder output, Ctx ctx)
    {
        // An hOCR word carries no whitespace of its own, so one is put back between words.
        if ((node.Attr("class") ?? "").Contains("ocrx_word", StringComparison.Ordinal)
            && output.Length > 0 && !EndsWith(output, " ") && !EndsWith(output, "\t") && !EndsWith(output, "\n"))
            output.Append(' ');

        // Whitespace normalization: pop a single trailing newline (typography.rs handle_span).
        // A hard break ("  \n" or "\\\n") and a table row's "|\n" are structure, not stray
        // whitespace — popping either glues this span onto the line before it.
        if (!ctx.InCode && EndsWith(output, "\n") && !EndsWith(output, "\n\n")
            && !EndsWith(output, "  \n") && !EndsWith(output, "\\\n") && !EndsWith(output, "|\n"))
            output.Remove(output.Length - 1, 1);
        WalkChildren(node, output, ctx);
    }

    // ── inline code (inline/code.rs) ─────────────────────────────────────────
    private static void HandleInlineCode(HNode node, StringBuilder output, Ctx ctx)
    {
        if (ctx.InCode) { WalkChildren(node, output, ctx); return; }
        var content = new StringBuilder();
        var cctx = ctx with { InCode = true };
        foreach (var c in node.Children) WalkNode(c, content, cctx);
        string s = content.ToString();
        if (s.Trim().Length == 0) return;
        RenderCodeWithEscaping(s, output);
    }

    private static void HandleKbdSamp(HNode node, StringBuilder output, Ctx ctx)
    {
        var content = new StringBuilder();
        var cctx = ctx.InCode ? ctx : ctx with { InCode = true };
        foreach (var c in node.Children) WalkNode(c, content, cctx);
        string normalized = NormalizeWhitespaceKeepNewlines(content.ToString());
        var (prefix, suffix, trimmed) = ChompInline(normalized);
        if (normalized.Trim().Length > 0)
        {
            output.Append(prefix).Append('`').Append(trimmed).Append('`');
            AppendInlineSuffix(output, suffix, trimmed.Length > 0, node);
        }
        else if (normalized.Length > 0)
        {
            output.Append(prefix);
            AppendInlineSuffix(output, suffix, false, node);
        }
    }

    /// <summary>Length of the longest consecutive run of <paramref name="marker"/>.</summary>
    private static int LongestConsecutiveRun(string content, char marker)
    {
        int max = 0, cur = 0;
        foreach (char c in content)
        {
            if (c == marker) { cur++; if (cur > max) max = cur; }
            else cur = 0;
        }
        return max;
    }

    /// <summary>Minimum length of a Markdown code fence per CommonMark.</summary>
    private const int MinFenceLength = 3;

    /// <summary>
    /// The smallest backtick-run length, from 1 up, that does not occur as a run inside
    /// <paramref name="content"/>.
    /// </summary>
    /// <remarks>
    /// CommonMark closes an inline code span at the next backtick string of the <em>same</em>
    /// length as the opening delimiter (6.1) — a longer or shorter run never matches — so the
    /// delimiter only has to avoid a run length that actually appears. Taking the longest run
    /// plus one over-escapes: content holding one length-2 run and no length-1 run is fine with
    /// a single backtick. A fenced block is the other rule, and needs the longest run plus one,
    /// because its close matches on <em>any</em> run at least as long as the fence.
    /// </remarks>
    private static int MinSafeCodeSpanDelimiterLength(string content)
    {
        var runLengths = new HashSet<int>();
        int cur = 0;
        foreach (char c in content)
        {
            if (c == '`') cur++;
            else { if (cur > 0) runLengths.Add(cur); cur = 0; }
        }
        if (cur > 0) runLengths.Add(cur);

        int candidate = 1;
        while (runLengths.Contains(candidate)) candidate++;
        return candidate;
    }

    private static void RenderCodeWithEscaping(string trimmed, StringBuilder output)
    {
        bool containsBacktick = trimmed.Contains('`');
        char? first = trimmed.Length > 0 ? trimmed[0] : null;
        char? last = trimmed.Length > 0 ? trimmed[^1] : null;
        bool allSpaces = trimmed.Length > 0 && trimmed.All(c => c == ' ');
        bool needsDelimiterSpaces = allSpaces
            || first == '`' || last == '`'
            || (first == ' ' && last == ' ' && containsBacktick);

        int numBackticks = containsBacktick ? MinSafeCodeSpanDelimiterLength(trimmed) : 1;

        output.Append('`', numBackticks);
        if (needsDelimiterSpaces) output.Append(' ');
        output.Append(trimmed);
        if (needsDelimiterSpaces) output.Append(' ');
        output.Append('`', numBackticks);
    }

    // ── link (handlers/link.rs) ──────────────────────────────────────────────
    private static void HandleLink(HNode node, StringBuilder output, Ctx ctx)
    {
        string? hrefRaw = node.Attr("href");
        string? title = node.Attr("title");

        if (hrefRaw is null)
        {
            WalkChildren(node, output, ctx);
            return;
        }

        string href = SanitizeMarkdownUrl(HtmlWalker.DecodeEntitiesFull(hrefRaw));

        if (ctx.InLink) { WalkChildren(node, output, ctx); return; }

        var (inlineLabel, sawBlock) = CollectLinkLabelText(node);
        // Without block descendants that sweep visited exactly the nodes the whole-subtree text
        // would, and decoded them the same way, so its text is reused rather than walking the
        // `<a>` a second time.
        string rawText = NormalizeWhitespaceKeepNewlines(sawBlock ? TextContent(node) : inlineLabel).Trim();

        bool isAutolink = href.Length > 0 && HasUriScheme(href)
            && (rawText == href || (href.StartsWith("mailto:", StringComparison.Ordinal) && rawText == href[7..]));
        if (isAutolink)
        {
            output.Append('<');
            output.Append(href.StartsWith("mailto:", StringComparison.Ordinal) && rawText == href[7..] ? rawText : href);
            output.Append('>');
            return;
        }

        // single heading child → heading wrapping the link
        var single = FindSingleHeadingChild(node);
        if (single is not null)
        {
            var (level, heading) = single.Value;
            var htext = new StringBuilder();
            var hctx = ctx with { InHeading = true, ConvertAsInline = true };
            WalkNode(heading, htext, hctx);
            string trimmedHeading = htext.ToString().Trim();
            if (trimmedHeading.Length > 0)
            {
                var linkBuf = new StringBuilder();
                AppendMarkdownLink(linkBuf, EscapeLinkLabel(trimmedHeading), href, title);
                PushHeading(output, ctx, level, linkBuf.ToString());
                return;
            }
        }

        string label;
        if (sawBlock)
        {
            var content = new StringBuilder();
            var lctx = ctx with { InlineDepth = ctx.InlineDepth + 1, ConvertAsInline = true, InLink = true };
            foreach (var child in node.Children)
            {
                var childBuf = new StringBuilder();
                WalkNode(child, childBuf, lctx);
                if (childBuf.ToString().Trim().Length > 0 && content.Length > 0
                    && !char.IsWhiteSpace(content[^1])
                    && childBuf.Length > 0 && !char.IsWhiteSpace(childBuf[0]))
                    content.Append(' ');
                content.Append(childBuf);
            }
            label = content.ToString().Trim().Length == 0
                ? NormalizeLinkLabel(inlineLabel)
                : NormalizeLinkLabel(content.ToString());
        }
        else
        {
            var content = new StringBuilder();
            var lctx = ctx with { InlineDepth = ctx.InlineDepth + 1, InLink = true };
            foreach (var child in node.Children) WalkNode(child, content, lctx);
            label = NormalizeLinkLabel(content.ToString());
        }

        // `rawText` is already the whole-subtree text when `sawBlock`, so this one fallback
        // covers both the block and the inline case.
        if (label.Length == 0 && rawText.Length > 0)
            label = NormalizeLinkLabel(rawText);
        if (label.Length == 0 && href.Length > 0 && node.Children.Count > 0)
            label = href;

        if (label == "^" && href.StartsWith('#')) label = "↑";

        AppendMarkdownLink(output, EscapeLinkLabel(label), href, title);
    }

    private static void AppendMarkdownLink(StringBuilder output, string label, string href, string? title)
    {
        output.Append('[').Append(label).Append("](");
        AppendUrlDestination(output, href);
        if (title is not null)
        {
            output.Append(" \"");
            AppendEscapedMarkdownTitle(output, title);
            output.Append('"');
        }
        output.Append(')');
    }

    /// <summary>
    /// Whether every <c>)</c> in a destination is matched by a preceding <c>(</c>, and every
    /// <c>(</c> is closed. A raw Markdown destination may hold parentheses only as a properly
    /// nested balanced pair (CommonMark 6.3) — counting the two separately calls <c>")("</c>
    /// balanced, which it is not.
    /// </summary>
    private static bool ParensAreBalanced(string href)
    {
        int depth = 0;
        foreach (char c in href)
        {
            if (c == '(') depth++;
            else if (c == ')' && --depth < 0) return false;
        }
        return depth == 0;
    }

    /// <summary>
    /// Append a Markdown destination — the <c>(...)</c> body, without the parens themselves.
    /// Shared by the link and image handlers so a destination is treated the same whichever
    /// element produced it.
    /// </summary>
    private static void AppendUrlDestination(StringBuilder output, string dest)
    {
        if (dest.Length == 0) { output.Append("<>"); return; }
        if (dest.Contains(' ') || dest.Contains('\n'))
        {
            // An angle-bracket destination may hold raw parentheses, but a raw `<`, `>` or an
            // unescaped `\` — which would otherwise merge with the next escaped character and
            // un-escape it — closes the wrap early, so all three are escaped inside it.
            output.Append('<');
            foreach (char c in dest)
            {
                if (c == '\\') output.Append("\\\\");
                else if (c == '<') output.Append("\\<");
                else if (c == '>') output.Append("\\>");
                else output.Append(c);
            }
            output.Append('>');
            return;
        }
        if (ParensAreBalanced(dest)) output.Append(dest);
        else output.Append(dest.Replace("(", "\\(").Replace(")", "\\)"));
    }

    /// <summary>
    /// Escape a title for interpolation into a double-quoted <c>"..."</c>. Backslashes go first:
    /// a title ending in a literal <c>\</c> would otherwise make the closing <c>"</c> read as an
    /// escaped quote, letting the title — and the destination after it — run into the rest of
    /// the document.
    /// </summary>
    private static void AppendEscapedMarkdownTitle(StringBuilder output, string text)
    {
        if (!text.Contains('\\') && !text.Contains('"')) { output.Append(text); return; }
        output.Append(text.Replace("\\", "\\\\").Replace("\"", "\\\""));
    }

    private static (int level, HNode node)? FindSingleHeadingChild(HNode node)
    {
        (int, HNode)? found = null;
        foreach (var child in node.Children)
        {
            if (child.IsComment) return null;
            if (child.Tag is null)
            {
                if (child.Text.Trim().Length > 0) return null;
                continue;
            }
            int level = child.Tag switch
            {
                "h1" => 1, "h2" => 2, "h3" => 3, "h4" => 4, "h5" => 5, "h6" => 6, _ => 0,
            };
            if (level == 0) return null;
            if (found is not null) return null;
            found = (level, child);
        }
        return found;
    }

    // collect_link_label_text: inline text of children skipping block descendants
    private static readonly HashSet<string> BlockLevelForLabel = new(StringComparer.Ordinal)
    {
        "p", "div", "blockquote", "pre", "table", "ul", "ol", "li", "dl", "dt", "dd",
        "h1", "h2", "h3", "h4", "h5", "h6", "section", "article", "aside", "header",
        "footer", "main", "nav", "figure", "figcaption", "hr",
    };

    private static (string text, bool sawBlock) CollectLinkLabelText(HNode node)
    {
        var text = new StringBuilder();
        bool sawBlock = false;
        var stack = new Stack<HNode>();
        for (int i = node.Children.Count - 1; i >= 0; i--) stack.Push(node.Children[i]);
        while (stack.Count > 0)
        {
            var n = stack.Pop();
            if (n.IsComment) continue;
            if (n.Tag is null) { text.Append(HtmlWalker.DecodeEntitiesFull(n.Text, n.CanonicalAttrs)); continue; }
            if (BlockLevelForLabel.Contains(n.Tag)) { sawBlock = true; continue; }
            for (int i = n.Children.Count - 1; i >= 0; i--) stack.Push(n.Children[i]);
        }
        return (text.ToString(), sawBlock);
    }

    internal static string NormalizeLinkLabel(string label)
    {
        string collapsed = label.Replace('\n', ' ').Replace('\r', ' ');
        return NormalizeWhitespaceKeepNewlines(collapsed).Trim();
    }

    private static string EscapeLinkLabel(string text)
    {
        if (text.Length == 0) return "";
        var result = new StringBuilder(text.Length);
        int backslashCount = 0;
        int bracketDepth = 0;
        foreach (char ch in text)
        {
            if (ch == '\\') { result.Append('\\'); backslashCount++; continue; }
            bool isEscaped = backslashCount % 2 == 1;
            backslashCount = 0;
            if (ch == '[' && !isEscaped) { bracketDepth++; result.Append('['); }
            else if (ch == ']' && !isEscaped)
            {
                if (bracketDepth == 0) result.Append('\\');
                else bracketDepth--;
                result.Append(']');
            }
            else result.Append(ch);
        }
        return result.ToString();
    }

    internal static bool HasUriScheme(string href)
    {
        if (href.Length == 0 || !char.IsAsciiLetter(href[0])) return false;
        for (int i = 1; i < href.Length; i++)
        {
            char c = href[i];
            if (c == ':') return true;
            if (!(char.IsAsciiLetterOrDigit(c) || c is '+' or '-' or '.')) return false;
        }
        return false;
    }

    private static string SanitizeMarkdownUrl(string url)
    {
        int mid = url.IndexOf("](", StringComparison.Ordinal);
        if (mid < 0) return url;
        if (!url[..mid].Contains('[')) return url;
        int parenStart = mid + 2;
        int relEnd = url.IndexOf(')', parenStart);
        if (relEnd < 0 || parenStart >= relEnd) return url;
        return url[parenStart..relEnd];
    }

    // ── MathML (media/svg.rs) ────────────────────────────────────────────────
    /// <summary>
    /// Emit a `&lt;math&gt;` element as its serialized source in an HTML comment followed by its
    /// text content, which is what the extractor reads the equations back out of.
    /// </summary>
    private static void HandleMath(HNode node, StringBuilder output, Ctx ctx)
    {
        string textContent = CollectRawText(node).Trim();
        if (textContent.Length == 0) return;

        bool isDisplayBlock = node.Attr("display") == "block";
        bool separated = isDisplayBlock && !ctx.InParagraph && !ctx.ConvertAsInline;

        if (separated) output.Append("\n\n");
        output.Append("<!-- MathML: ").Append(SerializeElement(node)).Append(" --> ").Append(textContent);
        if (separated) output.Append("\n\n");
    }

    /// <summary>
    /// An abbreviation spells itself out: its text, then its <c>title</c> in parentheses
    /// (`inline/semantic/typography.rs::handle_abbreviation`).
    /// </summary>
    private static void HandleAbbr(HNode node, StringBuilder output, Ctx ctx)
    {
        var content = new StringBuilder();
        foreach (var c in node.Children) WalkNode(c, content, ctx);
        string trimmed = content.ToString().Trim();
        if (trimmed.Length == 0) return;
        output.Append(trimmed);
        string title = (node.Attr("title") ?? "").Trim();
        if (title.Length > 0) output.Append(" (").Append(title).Append(')');
    }

    // ── embedded media (media/embedded.rs) ───────────────────────────────────
    /// <summary>
    /// A media element markdown cannot embed becomes a link to its source, plus whatever
    /// fallback content it wrapped; a <c>&lt;picture&gt;</c> reduces to the first
    /// <c>&lt;img&gt;</c> it holds (`converter/media/embedded.rs`).
    /// </summary>
    private static void HandleMedia(string tag, HNode node, StringBuilder output, Ctx ctx)
    {
        if (tag == "picture")
        {
            foreach (var child in node.Children)
            {
                if (child.Tag != "img") continue;
                WalkNode(child, output, ctx);
                return;
            }
            return;
        }

        string rawSrc = node.Attr("src") ?? "";
        if (tag != "iframe" && rawSrc.Length == 0)
        {
            foreach (var child in node.Children)
            {
                if (child.Tag != "source") continue;
                rawSrc = child.Attr("src") ?? "";
                break;
            }
        }
        string src = SanitizeMarkdownUrl(rawSrc);

        if (src.Length > 0)
        {
            // The src doubles as the label, so it goes through the same escaping an `<a href>`
            // gets in both positions rather than being spliced in raw.
            AppendMarkdownLink(output, EscapeLinkLabel(src), src, null);
            if (!ctx.InParagraph && !ctx.ConvertAsInline) output.Append("\n\n");
        }

        if (tag == "iframe") return;

        // Everything that is not a `<source>` is the element's no-support fallback.
        var fallback = new StringBuilder();
        foreach (var child in node.Children)
        {
            if (child.Tag == "source") continue;
            WalkNode(child, fallback, ctx);
        }
        if (fallback.Length > 0)
        {
            output.Append(fallback.ToString().Trim());
            if (!ctx.InParagraph && !ctx.ConvertAsInline) output.Append("\n\n");
        }
    }

    // ── form elements (form/elements.rs) ─────────────────────────────────────
    /// <summary>
    /// A form control has no markdown of its own, but the text and images inside it do, so each
    /// handler renders its children and differs only in the spacing it adds
    /// (`converter/form/elements.rs::handle`). `&lt;form&gt;` itself never reaches here —
    /// preprocessing drops it — but the rest are ordinary page furniture.
    /// </summary>
    private static void HandleFormElement(string tag, HNode node, StringBuilder output, Ctx ctx)
    {
        switch (tag)
        {
            // Collected, trimmed and set off by blank lines; an empty one disappears.
            case "form":
            case "fieldset":
            {
                if (ctx.ConvertAsInline) { WalkChildren(node, output, ctx); return; }
                var content = new StringBuilder();
                foreach (var c in node.Children) WalkNode(c, content, ctx);
                string trimmed = content.ToString().Trim();
                if (trimmed.Length == 0) return;
                if (output.Length > 0 && !EndsWith(output, "\n\n")) output.Append("\n\n");
                output.Append(trimmed).Append("\n\n");
                return;
            }

            // A fieldset's caption reads as bold text.
            case "legend":
            {
                var content = new StringBuilder();
                var legendCtx = ctx.ConvertAsInline ? ctx : ctx with { InStrong = true };
                foreach (var c in node.Children) WalkNode(c, content, legendCtx);
                string trimmed = content.ToString().Trim();
                if (trimmed.Length == 0) return;
                if (ctx.ConvertAsInline) output.Append(trimmed);
                else output.Append("**").Append(trimmed).Append("**").Append("\n\n");
                return;
            }

            case "label":
            {
                var content = new StringBuilder();
                foreach (var c in node.Children) WalkNode(c, content, ctx);
                string trimmed = content.ToString().Trim();
                if (trimmed.Length == 0) return;
                output.Append(trimmed);
                if (!ctx.ConvertAsInline) output.Append("\n\n");
                return;
            }

            // An <input> carries no text content, so it contributes nothing.
            case "input":
                return;

            // A group's label reads as bold text, then its options follow.
            case "optgroup":
            {
                string label = node.Attr("label") ?? "";
                if (label.Length > 0) output.Append("**").Append(label).Append("**").Append('\n');
                WalkChildren(node, output, ctx);
                return;
            }

            case "option":
            {
                bool selected = node.Attr("selected") is not null;
                var text = new StringBuilder();
                foreach (var c in node.Children) WalkNode(c, text, ctx);
                string trimmed = text.ToString().Trim();
                if (trimmed.Length == 0) return;
                if (selected && !ctx.ConvertAsInline) output.Append("* ");
                output.Append(trimmed);
                if (!ctx.ConvertAsInline) output.Append('\n');
                return;
            }

            // The rest render in place and only differ in the separator they leave behind.
            default:
            {
                int startLen = output.Length;
                WalkChildren(node, output, ctx);
                if (ctx.ConvertAsInline || output.Length == startLen) return;
                output.Append(tag is "select" or "datalist" ? "\n" : "\n\n");
                return;
            }
        }
    }

    // ── svg (media/svg.rs) ───────────────────────────────────────────────────
    /// <summary>
    /// An inline <c>&lt;svg&gt;</c> becomes an image whose source is the serialized subtree as a
    /// base64 data URI (`media/svg.rs::handle_svg`); in inline context only its title is kept.
    /// </summary>
    /// <summary>
    /// The image markdown one <c>&lt;svg&gt;</c> element's markup converts to, for the metadata
    /// scanner, which sees source text rather than a tree.
    /// </summary>
    internal static string RenderSvgImage(string svgMarkup)
    {
        var root = HtmlDom.Parse(svgMarkup);
        foreach (var child in root.Children)
        {
            if (child.Tag != "svg") continue;
            var sb = new StringBuilder();
            HandleSvg(child, sb, new Ctx());
            return sb.ToString();
        }
        return "";
    }

    private static void HandleSvg(HNode node, StringBuilder output, Ctx ctx)
    {
        string title = "SVG Image";
        foreach (var child in node.Children)
        {
            if (child.Tag == "title") { title = TextContent(child).Trim(); break; }
        }

        if (ctx.ConvertAsInline) { output.Append(title); return; }

        string svgHtml = SerializeElement(node);
        string base64 = System.Convert.ToBase64String(Encoding.UTF8.GetBytes(svgHtml));
        output.Append("![").Append(EscapeLinkLabel(title)).Append("](data:image/svg+xml;base64,").Append(base64).Append(')');
    }

    /// <summary>The concatenated text of a node's descendants, entity references resolved.</summary>
    private static string CollectRawText(HNode node)
    {
        var sb = new StringBuilder();
        void Walk(HNode n)
        {
            foreach (var c in n.Children)
            {
                if (c.IsComment) continue;
                if (c.Tag is null) sb.Append(HtmlWalker.DecodeEntitiesFull(c.Text, c.CanonicalAttrs));
                else Walk(c);
            }
        }
        Walk(node);
        return sb.ToString();
    }

    /// <summary>
    /// Re-escape a serialized text node: `&amp;`, a no-break space, `&lt;` and `&gt;` are written
    /// back as references, everything else verbatim. Quotes are left alone.
    /// </summary>
    private static string EscapeSerializedText(string text)
    {
        int first = -1;
        for (int i = 0; i < text.Length; i++)
        {
            char c = text[i];
            if (c == '&' || c == '<' || c == '>' || c == '\u00A0') { first = i; break; }
        }
        if (first < 0) return text;

        var sb = new StringBuilder(text.Length + 8);
        sb.Append(text, 0, first);
        for (int i = first; i < text.Length; i++)
        {
            switch (text[i])
            {
                case '&': sb.Append("&amp;"); break;
                case '\u00A0': sb.Append("&nbsp;"); break;
                case '<': sb.Append("&lt;"); break;
                case '>': sb.Append("&gt;"); break;
                default: sb.Append(text[i]); break;
            }
        }
        return sb.ToString();
    }

    /// <summary>
    /// Re-serialize an element and its subtree. A childless element closes itself with
    /// <c>&lt;tag /&gt;</c>, matching the crate's serializer.
    /// </summary>
    private static string SerializeElement(HNode node)
    {
        // Text is serialized with its character references already resolved, which is how the
        // MathML in the comment comes out reading `⁡` rather than `&ApplyFunction;`, and then
        // re-escaped: the four characters that would otherwise change the markup's shape go
        // back out as references, so `&#x3E;` and a literal `>` both serialize as `&gt;`.
        if (node.Tag is null)
            return node.IsComment ? "" : EscapeSerializedText(HtmlWalker.DecodeEntitiesFull(node.Text, node.CanonicalAttrs));

        var sb = new StringBuilder(256);
        sb.Append('<').Append(node.Tag);
        // Attributes come out sorted by name: the parser upstream serializes from keeps them in
        // a sorted map, so `stretchy="false" scriptlevel="+1"` is written the other way round.
        foreach (var (key, value) in HtmlWalker.EnumerateAttributes(node.AttrString)
                     .OrderBy(a => a.Key, StringComparer.Ordinal))
        {
            sb.Append(' ').Append(SvgAttrs.Canonical(key) ?? key);
            // A bare attribute and `attr=""` are HTML5-equivalent; upstream writes both bare.
            // A `"` in the value is the one character re-escaped on the way out: written raw it
            // would close the attribute early.
            if (!string.IsNullOrEmpty(value))
                sb.Append("=\"").Append(value!.Contains('"') ? value.Replace("\"", "&quot;") : value).Append('"');
        }

        if (node.Children.Count == 0) { sb.Append(" />"); return sb.ToString(); }

        sb.Append('>');
        foreach (var child in node.Children) sb.Append(SerializeElement(child));
        sb.Append("</").Append(node.Tag).Append('>');
        return sb.ToString();
    }

    // ── image (handlers/image.rs) ────────────────────────────────────────────
    private static void HandleImg(HNode node, StringBuilder output, Ctx ctx)
    {
        string src = SanitizeMarkdownUrl(node.Attr("src") ?? "");
        string alt = node.Attr("alt") ?? "";
        string? title = node.Attr("title");

        // Structure-collector side effect: report every <img> so cell images become nodes
        // (the crate's push_image runs unconditionally, once per handler invocation).
        ctx.ImageEmit?.Invoke(alt.Length == 0 ? null : alt, src);
        ctx.Structure?.PushImage(src, alt);

        bool shouldUseAltText = ctx.ConvertAsInline || ctx.InHeading;
        if (shouldUseAltText) { output.Append(alt); return; }

        // The alt text is escaped like a link label: an inert `alt` holding `]` and `(` would
        // otherwise close the image early and open a second, attacker-controlled link.
        output.Append("![").Append(EscapeLinkLabel(alt)).Append("](");
        AppendUrlDestination(output, src);
        if (title is not null)
        {
            output.Append(" \"");
            AppendEscapedMarkdownTitle(output, title);
            output.Append('"');
        }
        output.Append(')');
    }

    /// <summary>
    /// Emit a line break for a <c>&lt;br&gt;</c>, <c>&lt;div&gt;</c> or <c>&lt;p&gt;</c>
    /// continuation inside a table cell. A cell cannot contain a hard line break — neither
    /// newline style is valid there, and a raw newline splits the row's pipe syntax across
    /// physical lines — so the newline style is never consulted: source whitespace before the
    /// break is trimmed and the continuation collapses to a single space (this port's options
    /// leave <c>br_in_tables</c> off). The emptiness guard suppresses a leading space when the
    /// continuation is the cell's first content.
    /// </summary>
    private static void EmitTableCellBreak(StringBuilder output)
    {
        TrimTrailingWhitespace(output);
        if (output.Length > 0) output.Append(' ');
    }

    // ── br / hr ──────────────────────────────────────────────────────────────
    private static void HandleBr(StringBuilder output, Ctx ctx)
    {
        if (ctx.InHeading)
        {
            TrimTrailingWhitespace(output);
            output.Append("  ");
        }
        else if (ctx.InTableCell)
        {
            // Shared with div/p continuations inside a cell.
            EmitTableCellBreak(output);
        }
        else if (output.Length == 0 || EndsWith(output, "\n")) output.Append('\n');
        else output.Append("  \n");
    }

    private static void HandleHr(HNode node, StringBuilder output, Ctx ctx)
    {
        if (output.Length > 0)
        {
            string? prevTag = PrevSiblingTag(node);
            string lastLine = LastNonEmptyLine(output);
            bool lastLineIsBlockquote = lastLine.TrimStart().StartsWith('>');
            bool needsBlankLine = !ctx.InParagraph && prevTag != "blockquote" && !lastLineIsBlockquote;

            if (prevTag == "blockquote" && EndsWith(output, "\n\n"))
                output.Remove(output.Length - 1, 1);
            else if (ctx.InParagraph || !needsBlankLine)
            {
                if (!EndsWith(output, "\n")) output.Append('\n');
            }
            else
            {
                TrimTrailingWhitespace(output);
                if (EndsWith(output, "\n"))
                {
                    if (!EndsWith(output, "\n\n")) output.Append('\n');
                }
                else output.Append("\n\n");
            }
        }
        output.Append("---\n");
    }

    private static string LastNonEmptyLine(StringBuilder sb)
    {
        string s = sb.ToString();
        int end = s.Length;
        while (end > 0)
        {
            int start = s.LastIndexOf('\n', end - 1);
            string line = s[(start + 1)..end];
            if (line.Trim().Length > 0) return line;
            if (start < 0) break;
            end = start;
        }
        return "";
    }

    // ── pre (handlers/code_block.rs) ─────────────────────────────────────────
    // `block/preformatted.rs` carries a second, subtly different copy of this handler, but
    // nothing dispatches to it — `converter/main.rs` routes `<pre>` to `handlers::handle_pre`.
    private static void HandlePre(HNode node, StringBuilder output, Ctx ctx)
    {
        var cctx = ctx with { InCode = true };
        string? language = ExtractLanguageFromPre(node);

        var content = new StringBuilder();
        foreach (var c in node.Children) WalkNode(c, content, cctx);
        string s = content.ToString();
        if (s.Length == 0) return;

        int leading = 0; while (leading < s.Length && s[leading] == '\n') leading++;
        int trailing = 0; while (trailing < s.Length && s[s.Length - 1 - trailing] == '\n') trailing++;
        string core = s.Trim('\n');
        bool whitespaceOnly = core.Trim().Length == 0;

        string coreText = DedentCodeBlock(core);
        string processed;
        if (whitespaceOnly)
        {
            processed = new string('\n', leading) + coreText + new string('\n', trailing);
        }
        else
        {
            processed = coreText + new string('\n', trailing);
        }

        // Backticks style
        if (!ctx.ConvertAsInline && output.Length > 0 && !EndsWith(output, "\n\n"))
        {
            if (EndsWith(output, "\n")) output.Append('\n');
            else output.Append("\n\n");
        }
        // The fence has to be strictly longer than the longest backtick run inside the content,
        // or it terminates early and the rest of the document is swallowed into the block
        // (CommonMark 4.5).
        int fenceLength = Math.Max(LongestConsecutiveRun(processed, '`') + 1, MinFenceLength);
        output.Append('`', fenceLength);
        if (language is not null) output.Append(language);
        output.Append('\n');
        output.Append(processed.TrimEnd('\n'));
        output.Append('\n');
        output.Append('`', fenceLength).Append("\n\n");

        ctx.Structure?.PushCode(processed, language);
    }

    private static string? ExtractLanguageFromPre(HNode node)
    {
        string? FromClass(string? cls)
        {
            if (cls is null) return null;
            foreach (var c in cls.Split(' ', StringSplitOptions.RemoveEmptyEntries))
            {
                if (c.StartsWith("language-", StringComparison.Ordinal)) return c["language-".Length..];
                if (c.StartsWith("lang-", StringComparison.Ordinal)) return c["lang-".Length..];
            }
            return null;
        }
        var fromPre = FromClass(node.Attr("class"));
        if (fromPre is not null) return fromPre;
        foreach (var child in node.Children)
        {
            if (child.Tag == "code")
                return FromClass(child.Attr("class"));
        }
        return null;
    }

    /// <summary>
    /// Strip the common leading whitespace from every line of a code block
    /// (`converter/text/processing.rs::dedent_code_block`). A whitespace-only line is kept
    /// verbatim rather than blanked: the markdown output has its line ends trimmed globally
    /// afterwards, but the structure collector's Code node keeps those spaces.
    /// </summary>
    private static string DedentCodeBlock(string content)
    {
        var lines = content.Split('\n');
        int minIndent = int.MaxValue;
        foreach (var line in lines)
        {
            if (line.Trim().Length == 0) continue;
            int ind = 0;
            while (ind < line.Length && char.IsWhiteSpace(line[ind])) ind++;
            if (ind < minIndent) minIndent = ind;
        }
        if (minIndent == int.MaxValue) minIndent = 0;
        var sb = new StringBuilder(content.Length);
        for (int i = 0; i < lines.Length; i++)
        {
            if (i > 0) sb.Append('\n');
            var line = lines[i];
            if (line.Trim().Length == 0) { sb.Append(line); continue; }
            int cut = 0, remaining = minIndent;
            while (remaining > 0 && cut < line.Length && char.IsWhiteSpace(line[cut])) { cut++; remaining--; }
            sb.Append(line[cut..]);
        }
        return sb.ToString();
    }

    // ── blockquote (handlers/blockquote.rs) ──────────────────────────────────
    private static void HandleBlockquote(HNode node, StringBuilder output, Ctx ctx)
    {
        if (ctx.ConvertAsInline) { WalkChildren(node, output, ctx); return; }

        var bctx = ctx with { BlockquoteDepth = ctx.BlockquoteDepth + 1 };
        var content = new StringBuilder();
        foreach (var c in node.Children) WalkNode(c, content, bctx);

        string trimmedContent = content.ToString().Trim();
        if (trimmedContent.Length == 0) return;

        if (ctx.BlockquoteDepth > 0)
        {
            if (output.Length > 0)
            {
                while (output.Length > 0 && output[^1] == '\n') output.Length--;
                output.Append("\n\n");
            }
        }
        else if (output.Length > 0)
        {
            if (EndsWith(output, "\n\n")) output.Remove(output.Length - 1, 1);
            else if (!EndsWith(output, "\n")) output.Append("\n\n");
        }

        // Blank out whitespace-only lines, but keep the leading whitespace on a real content
        // line — code-block indentation and nested list markers are what makes a quoted block
        // child still read as that block.
        foreach (var line in trimmedContent.Split('\n'))
        {
            output.Append("> ");
            if (line.Trim().Length > 0) output.Append(line);
            output.Append('\n');
        }
        output.Append('\n');
    }

    // ── lists (list/*.rs) ────────────────────────────────────────────────────
    private static void HandleList(HNode node, StringBuilder output, Ctx ctx, bool ordered)
    {
        AddListLeadingSeparator(output, ctx);

        int nestedDepth = ctx.InList && !ctx.InListItem ? ctx.ListDepth + 1 : ctx.ListDepth;
        bool isLoose = IsLooseList(node);

        int start = 1;
        if (ordered && int.TryParse(node.Attr("start"), out var s)) start = s;

        // process_list_children
        int counter = start;
        var listCtx = ctx with
        {
            InOrderedList = ordered,
            ListCounter = ordered ? counter : 0,
            InList = true,
            ListDepth = nestedDepth,
            UlDepth = ordered ? ctx.UlDepth : ctx.UlDepth + 1,
            LooseList = isLoose,
            PrevItemHadBlocks = false,
        };

        if (!ctx.InTableCell) ctx.Structure?.PushListStart(ordered);

        foreach (var child in node.Children)
        {
            if (child.Tag is null && !child.IsComment && child.Text.Trim().Length == 0) continue;
            if (ordered) listCtx = listCtx with { ListCounter = counter };
            WalkNode(child, output, listCtx);
            if (ordered && child.Tag == "li") counter++;
        }

        if (!ctx.InTableCell) ctx.Structure?.PushListEnd();

        AddNestedListTrailingSeparator(output, ctx);
    }

    private static void AddListLeadingSeparator(StringBuilder output, Ctx ctx)
    {
        if (ctx.InTableCell)
        {
            bool isTableContinuation = output.Length > 0 && !EndsWith(output, "|") && !EndsWith(output, " ") && !EndsWith(output, "<br>");
            if (isTableContinuation) output.Append("<br>");
            return;
        }
        if (output.Length > 0 && !ctx.InList)
        {
            bool needsNewline = !EndsWith(output, "\n\n") && !EndsWith(output, "* ")
                && !EndsWith(output, "- ") && !EndsWith(output, ". ");
            if (needsNewline) output.Append("\n\n");
            return;
        }
        if (ctx.InListItem && output.Length > 0)
        {
            bool needsNewline = !EndsWith(output, "\n") && !EndsWith(output, "* ")
                && !EndsWith(output, "- ") && !EndsWith(output, ". ");
            if (needsNewline)
            {
                TrimTrailingWhitespace(output);
                output.Append('\n');
            }
        }
    }

    private static void AddNestedListTrailingSeparator(StringBuilder output, Ctx ctx)
    {
        if (!ctx.InListItem) return;
        if (ctx.LooseList)
        {
            if (!EndsWith(output, "\n\n"))
            {
                if (!EndsWith(output, "\n")) output.Append('\n');
                output.Append('\n');
            }
        }
        else if (!EndsWith(output, "\n")) output.Append('\n');
    }

    private static bool IsLooseList(HNode node)
    {
        foreach (var child in node.Children)
        {
            if (child.Tag != "li") continue;
            foreach (var liChild in child.Children)
                if (liChild.Tag == "p") return true;
        }
        return false;
    }

    private static void HandleLi(HNode node, StringBuilder output, Ctx ctx)
    {
        if (ctx.ListDepth > 0)
            output.Append(' ', ctx.ListIndentColumns);

        bool hasBlockChildren = false;
        foreach (var child in node.Children)
        {
            if (child.Tag is "p" or "div" or "blockquote" or "pre" or "table" or "hr" or "dl")
            {
                hasBlockChildren = true;
                break;
            }
        }

        // task lists: find checkbox
        var checkbox = FindCheckbox(node);

        // This item's own marker width, which is what descendants — nested lists and
        // continuation content — indent by. A bullet or task marker is always 2 columns
        // ("- "); an ordered marker's width follows its counter's digit count ("1. " is 3,
        // "10. " is 4). The configured indent width is a floor, not the literal width.
        int ownMarkerWidth = checkbox is not null || !ctx.InOrderedList
            ? 2
            : Math.Max(2, $"{ctx.ListCounter}. ".Length);

        var liCtx = ctx with
        {
            InListItem = true,
            ListDepth = ctx.ListDepth + 1,
            ListIndentColumns = ctx.ListIndentColumns + ownMarkerWidth,
        };
        int itemStart;
        if (checkbox is not null)
        {
            output.Append("- ").Append(checkbox.Value.check ? "[x]" : "[ ]");
            var taskText = new StringBuilder();
            RenderLiContentSkippingCheckbox(node, taskText, liCtx, checkbox.Value.node);
            output.Append(' ');
            itemStart = output.Length;
            string trimmedTask = taskText.ToString().Trim();
            if (trimmedTask.Length > 0) output.Append(trimmedTask);
        }
        else
        {
            if (ctx.InTableCell)
            {
                // A GFM pipe cell cannot hold block content, so sibling `<li>`s inside one lose
                // their marker (below) and would otherwise run together with no separator at
                // all. The same leading-separator helper that runs before the enclosing
                // `<ul>`/`<ol>` gives each item the `<br>` boundary already established for
                // `<p>`/`<div>` siblings in a cell.
                AddListLeadingSeparator(output, ctx);
            }
            else if (ctx.InOrderedList) output.Append(ctx.ListCounter).Append(". ");
            else
            {
                const string bullets = "-*+";
                int idx = ctx.UlDepth > 0 ? (ctx.UlDepth - 1) % bullets.Length : 0;
                output.Append(bullets[idx]).Append(' ');
            }

            itemStart = output.Length;
            foreach (var child in node.Children)
                WalkNode(child, output, liCtx);

            TrimTrailingWhitespace(output);
        }

        if (!ctx.InTableCell && itemStart <= output.Length)
            ctx.Structure?.PushListItem(output.ToString(itemStart, output.Length - itemStart).Trim());

        if (!ctx.InTableCell)
        {
            if (hasBlockChildren || ctx.LooseList || ctx.PrevItemHadBlocks)
            {
                if (!EndsWith(output, "\n\n"))
                {
                    if (EndsWith(output, "\n")) output.Append('\n');
                    else output.Append("\n\n");
                }
            }
            else if (!EndsWith(output, "\n")) output.Append('\n');
        }
    }

    private static (bool check, HNode node)? FindCheckbox(HNode node)
    {
        if (node.Tag == "input" && node.Attr("type") == "checkbox")
            return (node.Attr("checked") is not null, node);
        foreach (var c in node.Children)
        {
            if (c.Tag is null) continue;
            var r = FindCheckbox(c);
            if (r is not null) return r;
        }
        return null;
    }

    private static void RenderLiContentSkippingCheckbox(HNode node, StringBuilder output, Ctx ctx, HNode checkbox)
    {
        foreach (var child in node.Children)
        {
            if (ReferenceEquals(child, checkbox)) continue;
            if (ContainsNode(child, checkbox))
                RenderLiContentSkippingCheckbox(child, output, ctx, checkbox);
            else WalkNode(child, output, ctx);
        }
    }

    private static bool ContainsNode(HNode node, HNode target)
    {
        if (ReferenceEquals(node, target)) return true;
        foreach (var c in node.Children)
            if (ContainsNode(c, target)) return true;
        return false;
    }

    // ── definition lists (semantic/definition_list.rs) ───────────────────────
    private static void HandleDl(HNode node, StringBuilder output, Ctx ctx)
    {
        if (ctx.ConvertAsInline) { WalkChildren(node, output, ctx); return; }
        var content = new StringBuilder();
        foreach (var c in node.Children) WalkNode(c, content, ctx);
        string trimmed = content.ToString().Trim();
        if (trimmed.Length == 0) return;
        if (output.Length > 0 && !EndsWith(output, "\n\n")) output.Append("\n\n");
        output.Append(trimmed).Append("\n\n");
    }

    private static void HandleDt(HNode node, StringBuilder output, Ctx ctx)
    {
        var content = new StringBuilder();
        foreach (var c in node.Children) WalkNode(c, content, ctx);
        string trimmed = content.ToString().Trim();
        if (trimmed.Length == 0) return;
        output.Append(trimmed);
        if (!ctx.ConvertAsInline) output.Append('\n');
    }

    private static void HandleDd(HNode node, StringBuilder output, Ctx ctx)
    {
        var content = new StringBuilder();
        foreach (var c in node.Children) WalkNode(c, content, ctx);
        string trimmed = content.ToString().Trim();
        if (trimmed.Length == 0) return;
        output.Append(trimmed);
        if (!ctx.ConvertAsInline) output.Append("\n\n");
    }

    // ── sectioning / figure / details (semantic/*.rs) ────────────────────────
    private static void HandleSectioning(HNode node, StringBuilder output, Ctx ctx)
    {
        if (ctx.ConvertAsInline) { WalkChildren(node, output, ctx); return; }
        var content = new StringBuilder();
        foreach (var c in node.Children) WalkNode(c, content, ctx);
        string s = content.ToString();
        if (s.Trim().Length == 0) return;
        if (output.Length > 0 && !EndsWith(output, "\n\n")) output.Append("\n\n");
        output.Append(s);
        if (s.EndsWith('\n') && !s.EndsWith("\n\n")) output.Append('\n');
        else if (!s.EndsWith('\n')) output.Append("\n\n");
    }

    private static void HandleFigure(HNode node, StringBuilder output, Ctx ctx)
    {
        if (ctx.ConvertAsInline) { WalkChildren(node, output, ctx); return; }
        if (output.Length > 0 && !EndsWith(output, "\n\n")) output.Append("\n\n");
        var content = new StringBuilder();
        foreach (var c in node.Children) WalkNode(c, content, ctx);
        string s = content.ToString().Replace("\n![", "![").Replace(" ![", "![");
        string trimmed = s.Trim(' ', '\t', '\n');
        if (trimmed.Length == 0) return;
        output.Append(trimmed);
        if (!EndsWith(output, "\n")) output.Append('\n');
        if (!EndsWith(output, "\n\n")) output.Append('\n');
    }

    private static void HandleFigcaption(HNode node, StringBuilder output, Ctx ctx)
    {
        var text = new StringBuilder();
        foreach (var c in node.Children) WalkNode(c, text, ctx);
        string trimmed = text.ToString().Trim();
        if (trimmed.Length == 0) return;
        if (output.Length > 0)
        {
            if (EndsWith(output, "```\n")) output.Append('\n');
            else
            {
                TrimTrailingWhitespace(output);
                if (EndsWith(output, "\n") && !EndsWith(output, "\n\n")) output.Append('\n');
                else if (!EndsWith(output, "\n")) output.Append("\n\n");
            }
        }
        output.Append('*').Append(trimmed).Append("*\n\n");
    }

    private static void HandleDetails(HNode node, StringBuilder output, Ctx ctx)
    {
        if (ctx.ConvertAsInline) { WalkChildren(node, output, ctx); return; }
        var content = new StringBuilder();
        foreach (var c in node.Children) WalkNode(c, content, ctx);
        string trimmed = content.ToString().Trim();
        if (trimmed.Length == 0) return;
        if (output.Length > 0 && !EndsWith(output, "\n\n")) output.Append("\n\n");
        output.Append(trimmed).Append("\n\n");
    }

    private static void HandleSummary(HNode node, StringBuilder output, Ctx ctx)
    {
        var content = new StringBuilder();
        var sctx = ctx with { InStrong = true };
        foreach (var c in node.Children) WalkNode(c, content, sctx);
        string trimmed = content.ToString().Trim();
        if (trimmed.Length == 0) return;
        if (ctx.ConvertAsInline) output.Append(trimmed);
        else output.Append("**").Append(trimmed).Append("**\n\n");
    }

    // ── unknown elements (block/unknown.rs) ──────────────────────────────────
    private static void HandleUnknown(HNode node, StringBuilder output, Ctx ctx)
    {
        int lenBefore = output.Length;
        bool hadTrailingSpace = EndsWith(output, " ");
        WalkChildren(node, output, ctx);
        int lenAfter = output.Length;
        if (lenAfter > lenBefore)
        {
            string added = output.ToString(lenBefore, lenAfter - lenBefore);
            bool isCodeBlock = added.StartsWith("    ", StringComparison.Ordinal)
                || added.StartsWith("```", StringComparison.Ordinal)
                || added.StartsWith("~~~", StringComparison.Ordinal);
            if (added.Trim().Length == 0 && !isCodeBlock)
            {
                output.Length = lenBefore;
                if (!hadTrailingSpace && added.Contains(' ')) output.Append(' ');
            }
        }
    }

    // ── tables (block/table/*.rs) ────────────────────────────────────────────
    internal sealed class TableScan
    {
        public readonly List<int> RowCounts = new();
        public bool HasSpan, HasHeader, HasCaption, HasText;
        public int NestedTableCount, LinkCount;
    }

    /// <summary>
    /// Scan a table for the signals the layout-table heuristic reads.
    /// </summary>
    /// <remarks>
    /// The two groups come from different subtrees. <c>HasText</c>, <c>LinkCount</c>,
    /// <c>HasHeader</c> and <c>HasCaption</c> answer "is there any semantic content here at
    /// all", so they are gathered from the whole subtree, nested tables included.
    /// <c>RowCounts</c>, <c>NestedTableCount</c> and <c>HasSpan</c> feed the layout decision
    /// and describe only <em>this</em> table's own structure — a straight chain of
    /// one-nested-table-per-cell tables is not a layout table, and counting the inner
    /// tables' rows as if they were this one's is what used to make it look like one.
    /// </remarks>
    private static TableScan ScanTable(HNode node)
    {
        var scan = new TableScan();
        ScanOwnStructure(node, scan);
        AccumulateContent(node, scan);
        return scan;
    }

    /// <summary>
    /// Collect the table's own direct row/cell structure: per-row cell counts, whether any
    /// cell spans, and how many <c>&lt;table&gt;</c> elements are nested directly inside it.
    /// A nested table's own subtree is never walked — none of these fields count content past
    /// that boundary anyway.
    /// </summary>
    private static void ScanOwnStructure(HNode root, TableScan scan)
    {
        var work = new Stack<HNode>();
        for (int i = root.Children.Count - 1; i >= 0; i--) work.Push(root.Children[i]);
        while (work.Count > 0)
        {
            var n = work.Pop();
            if (n.IsComment || n.Tag is null) continue;
            if (n.Tag == "table") { scan.NestedTableCount++; continue; }
            if (n.Tag == "tr")
            {
                int cellCount = 0;
                foreach (var child in n.Children)
                {
                    if (child.Tag is "td" or "th")
                    {
                        cellCount += GetColspan(child);
                        if (child.Attr("colspan") is not null || child.Attr("rowspan") is not null)
                            scan.HasSpan = true;
                    }
                }
                scan.RowCounts.Add(cellCount);
                // Still descend into the row's cells — not their counts, already taken above —
                // so a `<table>` inside a `<td>` is found and counted. Only a nested `<table>`
                // tag itself stops this walk.
            }
            for (int i = n.Children.Count - 1; i >= 0; i--) work.Push(n.Children[i]);
        }
    }

    /// <summary>Fold the whole subtree's semantic content — text, links, headers, caption —
    /// into the scan, crossing nested-table boundaries.</summary>
    private static void AccumulateContent(HNode node, TableScan scan)
    {
        if (node.IsComment) return;
        if (node.Tag is null)
        {
            if (!scan.HasText && HtmlWalker.DecodeEntitiesFull(node.Text, node.CanonicalAttrs).Trim().Length > 0)
                scan.HasText = true;
            return;
        }
        switch (node.Tag)
        {
            case "a": scan.LinkCount++; break;
            case "caption": scan.HasCaption = true; break;
            case "th": scan.HasHeader = true; break;
            case "img":
                if (node.Attr("src") is not null || node.Attr("alt") is not null) scan.HasText = true;
                break;
        }
        foreach (var c in node.Children) AccumulateContent(c, scan);
    }

    private static int GetColspan(HNode cell)
    {
        if (int.TryParse(cell.Attr("colspan"), out var v) && v >= 0)
            return v == 0 ? 1 : Math.Min(v, 1000);
        return 1;
    }

    private static (int colspan, int rowspan) GetColspanRowspan(HNode cell)
    {
        int cs = 1, rs = 1;
        if (int.TryParse(cell.Attr("colspan"), out var c) && c >= 0) cs = c == 0 ? 1 : Math.Min(c, 1000);
        if (int.TryParse(cell.Attr("rowspan"), out var r) && r >= 0) rs = r == 0 ? 1 : Math.Min(r, 1000);
        return (cs, rs);
    }

    private static List<HNode> CollectTableCells(HNode row)
    {
        var cells = new List<HNode>();
        foreach (var c in row.Children)
            if (c.Tag is "td" or "th") cells.Add(c);
        return cells;
    }

    private static IEnumerable<HNode> TableRows(HNode table)
    {
        foreach (var child in table.Children)
        {
            if (child.Tag is "thead" or "tbody" or "tfoot")
            {
                foreach (var row in child.Children)
                    if (row.Tag == "tr") yield return row;
            }
            else if (child.Tag == "tr") yield return child;
        }
    }

    private static void HandleTableWithContext(HNode node, StringBuilder output, Ctx ctx)
    {
        var tableOutput = new StringBuilder();
        HandleTable(node, tableOutput, ctx);

        // Structure-collector side effect: after rendering (which already emitted any nested
        // tables encountered in cells), collect this table's grid (re-walking cells emits those
        // nested tables a second time), then emit this table — nested-before-parent order.
        if (ctx.TableEmit is not null)
        {
            var grid = CollectGrid(node, ctx);
            ctx.TableEmit(grid);
        }
        if (ctx.Structure is not null) ctx.Structure.PushTable(CollectGrid(node, ctx));

        if (ctx.InListItem)
        {
            bool hasCaption = StartsWith(tableOutput, "*");
            if (!hasCaption)
            {
                TrimTrailingWhitespace(output);
                if (output.Length > 0 && !EndsWith(output, "\n")) output.Append('\n');
            }
            output.Append(IndentTableForList(tableOutput.ToString(), ctx.ListDepth));
        }
        else
        {
            if (output.Length > 0 && !EndsWith(output, "\n\n"))
            {
                if (EndsWith(output, "\n")) output.Append('\n');
                else output.Append("\n\n");
            }
            output.Append(tableOutput);
        }

        if (!EndsWith(output, "\n")) output.Append('\n');
    }

    private static string IndentTableForList(string table, int listDepth)
    {
        string indent = new string(' ', 2 * (listDepth > 0 ? 2 * listDepth - 1 : 0));
        var sb = new StringBuilder();
        foreach (var line in table.Split('\n'))
        {
            if (line.Length > 0) sb.Append(indent).Append(line);
            sb.Append('\n');
        }
        // Remove the final extra newline that Split introduced
        if (sb.Length > 0 && sb[^1] == '\n') sb.Length--;
        return sb.ToString();
    }

    /// <summary>
    /// Whether the handler renders this table as a list of its rows rather than as a Markdown
    /// table. A layout table's cells are walked inline, which is what makes an image inside one
    /// degrade to its alt text — so the metadata collector has to reach the same verdict.
    /// </summary>
    internal static bool RendersAsLayoutList(HNode node, TableScan scan)
    {
        var distinctCounts = scan.RowCounts.Where(c => c > 0).Distinct().ToList();
        bool hasBorderZero = node.Attr("border") == "0";
        bool looksLikeLayout = scan.NestedTableCount > 1 || distinctCounts.Count > 1 || (scan.HasSpan && hasBorderZero);
        bool isBlankTable = !scan.HasText;
        int rowCount = scan.RowCounts.Count;
        return !scan.HasHeader && !scan.HasCaption
            && (looksLikeLayout || isBlankTable || (rowCount <= 2 && scan.LinkCount >= 3));
    }

    /// <summary>Whether the `&lt;table&gt;` this markup opens renders as a layout list.</summary>
    internal static bool TableMarkupRendersAsLayoutList(string tableHtml)
    {
        var root = HtmlDom.Parse(tableHtml);
        var table = FindFirstTable(root);
        return table is not null && RendersAsLayoutList(table, ScanTable(table));
    }

    private static HNode? FindFirstTable(HNode node)
    {
        foreach (var child in Descendants(node))
            if (child.Tag == "table") return child;
        return null;
    }

    private static void HandleTable(HNode node, StringBuilder output, Ctx ctx)
    {
        var scan = ScanTable(node);
        bool isBlankTable = !scan.HasText;

        if (RendersAsLayoutList(node, scan))
        {
            if (isBlankTable && scan.LinkCount == 0) return;
            foreach (var child in node.Children)
            {
                if (child.Tag is "thead" or "tbody" or "tfoot")
                {
                    foreach (var row in child.Children)
                        if (row.Tag == "tr") AppendLayoutRow(row, output, ctx);
                }
                else if (child.Tag == "tr") AppendLayoutRow(child, output, ctx);
                else if (child.Tag is "colgroup" or "col") { }
                else if (child.Tag is not null) WalkNode(child, output, ctx);
            }
            if (!EndsWith(output, "\n")) output.Append('\n');
            return;
        }

        int totalCols = TableTotalColumns(node);

        // width pre-pass
        var colWidths = new List<int>();
        var prepassRowspan = new int?[totalCols];
        // The pre-pass runs with its collectors detached: measuring a column's width is an
        // internal detail and must not show up in the result. Upstream keeps them here only when
        // it can reuse this pass's markdown verbatim in the render, which it cannot while a
        // structure collector is installed — and this port's options always install one.
        var prepassCtx = ctx with
        {
            MeasureWidthOnly = true,
            Structure = null,
            ImageEmit = null,
            TableEmit = null,
        };
        foreach (var row in TableRows(node))
            CollectRowCellWidths(row, prepassCtx, colWidths, prepassRowspan);

        var rowspanTracker = new int?[totalCols];
        int rowIndex = 0;
        int? firstRowCols = null;

        foreach (var child in node.Children)
        {
            switch (child.Tag)
            {
                case "caption":
                {
                    var text = new StringBuilder();
                    foreach (var gc in child.Children) WalkNode(gc, text, ctx);
                    string trimmedCap = text.ToString().Trim();
                    if (trimmedCap.Length > 0)
                    {
                        output.Append('*').Append(trimmedCap.Replace("-", "\\-")).Append("*\n\n");
                    }
                    break;
                }
                case "thead": case "tbody": case "tfoot":
                {
                    foreach (var row in child.Children)
                    {
                        if (row.Tag != "tr") continue;
                        if (firstRowCols is null)
                        {
                            int cols = CollectTableCells(row).Sum(GetColspan);
                            firstRowCols = Math.Clamp(cols, 1, 1000);
                        }
                        ConvertTableRow(row, output, ctx, rowIndex, scan.HasSpan, rowspanTracker,
                            totalCols, firstRowCols.Value, colWidths);
                        rowIndex++;
                    }
                    break;
                }
                case "tr":
                {
                    if (firstRowCols is null)
                    {
                        int cols = CollectTableCells(child).Sum(GetColspan);
                        firstRowCols = Math.Clamp(cols, 1, 1000);
                    }
                    ConvertTableRow(child, output, ctx, rowIndex, scan.HasSpan, rowspanTracker,
                        totalCols, firstRowCols.Value, colWidths);
                    rowIndex++;
                    break;
                }
                case "colgroup": case "col":
                    break;
                default:
                    if (child.Tag is not null) WalkNode(child, output, ctx);
                    break;
            }
        }
    }

    private static int TableTotalColumns(HNode table)
    {
        int maxCols = 0;
        foreach (var row in TableRows(table))
        {
            int colCount = CollectTableCells(row).Sum(GetColspan);
            if (colCount > maxCols) maxCols = colCount;
        }
        return Math.Clamp(maxCols, 1, 1000);
    }

    private static void AppendLayoutRow(HNode row, StringBuilder output, Ctx ctx)
    {
        var rowText = new StringBuilder();
        foreach (var cell in row.Children)
        {
            if (cell.Tag is not ("td" or "th")) continue;
            var cellText = new StringBuilder();
            var cellCtx = ctx with { ConvertAsInline = true };
            foreach (var c in cell.Children) WalkNode(c, cellText, cellCtx);
            string content = NormalizeWhitespaceKeepNewlines(cellText.ToString());
            if (content.Trim().Length > 0)
            {
                if (rowText.Length > 0) rowText.Append(' ');
                rowText.Append(content.Trim());
            }
        }
        string trimmed = rowText.ToString().Trim();
        if (trimmed.Length == 0) return;
        if (output.Length > 0 && !EndsWith(output, "\n")) output.Append('\n');
        string formatted = trimmed.StartsWith("- ", StringComparison.Ordinal) ? trimmed[2..].TrimStart() : trimmed;
        output.Append("- ").Append(formatted).Append('\n');
    }

    private static void CollectRowCellWidths(HNode row, Ctx ctx, List<int> colWidths, int?[] rowspanTracker)
    {
        var cells = CollectTableCells(row);
        int col = 0;
        int cellIdx = 0;
        while (true)
        {
            while (col < rowspanTracker.Length && rowspanTracker[col] is int remaining && remaining > 0)
            {
                rowspanTracker[col] = remaining - 1 == 0 ? null : remaining - 1;
                col++;
            }
            if (cellIdx >= cells.Count) break;
            var cell = cells[cellIdx++];
            string text = CellTextContent(cell, ctx);
            int width = Math.Min(text.Length, 200); // char count
            while (colWidths.Count <= col) colWidths.Add(0);
            if (width > colWidths[col]) colWidths[col] = width;
            var (colspan, rowspan) = GetColspanRowspan(cell);
            if (rowspan > 1 && col < rowspanTracker.Length)
                rowspanTracker[col] = rowspan - 1;
            col += colspan;
        }
    }

    private static string CellTextContent(HNode cell, Ctx ctx)
    {
        string text = RenderCellContent(cell, ctx);
        text = text.Trim();
        return text.Contains('\n') ? text.Replace('\n', ' ') : text;
    }

    private static string RenderCellContent(HNode cell, Ctx ctx)
    {
        // Always walk children under an in-cell context (mirrors the crate's convert_table_cell,
        // which routes every cell through walk_node). Text nodes decode entities and escape
        // *_| there, so `&nbsp;`-only cells collapse to empty exactly like the grid path.
        var buf = new StringBuilder();
        var cctx = ctx with { InTableCell = true };
        foreach (var c in cell.Children) WalkNode(c, buf, cctx);
        return buf.ToString();
    }

    internal static string EscapeCellText(string text)
    {
        // Always escape * and _ inside table cells; also escape | (escape_misc=false path).
        var sb = new StringBuilder(text.Length);
        foreach (char c in text)
        {
            if (c is '*' or '_' or '|') sb.Append('\\');
            sb.Append(c);
        }
        return sb.ToString();
    }

    private static void ConvertTableRow(HNode row, StringBuilder output, Ctx ctx, int rowIndex,
        bool hasSpan, int?[] rowspanTracker, int totalCols, int headerCols, List<int> colWidths)
    {
        var rowText = new StringBuilder();
        var cells = CollectTableCells(row);
        var cellCtx = ctx with { InTableCell = true };

        if (hasSpan)
        {
            int colIndex = 0;
            int cellIdx = 0;
            while (true)
            {
                if (colIndex < totalCols && rowspanTracker[colIndex] is int remaining && remaining > 0)
                {
                    rowText.Append(' ');
                    if (colIndex < colWidths.Count) rowText.Append(' ', colWidths[colIndex]);
                    rowText.Append(" |");
                    rowspanTracker[colIndex] = remaining - 1 == 0 ? null : remaining - 1;
                    colIndex++;
                    continue;
                }
                if (cellIdx >= cells.Count) break;
                var cell = cells[cellIdx++];
                int? colWidth = colIndex < colWidths.Count ? colWidths[colIndex] : null;
                ConvertTableCell(cell, rowText, cellCtx, colWidth);
                var (colspan, rowspan) = GetColspanRowspan(cell);
                if (rowspan > 1 && colIndex < totalCols)
                    rowspanTracker[colIndex] = rowspan - 1;
                colIndex += colspan;
            }
        }
        else
        {
            for (int i = 0; i < cells.Count; i++)
            {
                int? colWidth = i < colWidths.Count ? colWidths[i] : null;
                ConvertTableCell(cells[i], rowText, cellCtx, colWidth);
            }
        }

        output.Append('|').Append(rowText).Append('\n');

        if (rowIndex == 0)
        {
            int cols = Math.Clamp(headerCols, 1, 1000);
            output.Append("| ");
            for (int i = 0; i < cols; i++)
            {
                if (i > 0) output.Append(" | ");
                int dashCount = Math.Max(i < colWidths.Count ? colWidths[i] : 0, 3);
                output.Append('-', dashCount);
            }
            output.Append(" |\n");
        }
    }

    private static void ConvertTableCell(HNode cell, StringBuilder output, Ctx cellCtx, int? colWidth)
    {
        string text = RenderCellContent(cell, cellCtx).Trim();
        string textForOutput = text.Contains('\n') ? text.Replace('\n', ' ') : text;
        int colspan = GetColspan(cell);
        output.Append(' ').Append(textForOutput);
        if (colWidth is int w)
        {
            int len = textForOutput.Length;
            if (len < w) output.Append(' ', w - len);
        }
        for (int i = 0; i < colspan; i++) output.Append(" |");
    }

    // ── sibling helpers ──────────────────────────────────────────────────────
    private static string? NextSiblingTag(HNode node)
    {
        var parent = node.Parent;
        if (parent is null) return null;
        var siblings = parent.Children;
        for (int i = node.Index + 1; i < siblings.Count; i++)
        {
            var s = siblings[i];
            if (s.IsComment) continue;
            if (s.Tag is not null) return s.Tag;
            if (s.Text.Trim().Length > 0) return null;
        }
        return null;
    }

    private static string? PrevSiblingTag(HNode node)
    {
        var parent = node.Parent;
        if (parent is null) return null;
        var siblings = parent.Children;
        for (int i = node.Index - 1; i >= 0; i--)
        {
            var s = siblings[i];
            if (s.IsComment) continue;
            if (s.Tag is not null) return s.Tag;
            if (s.Text.Trim().Length > 0) return null;
        }
        return null;
    }

    private static bool PrevSiblingIsInlineTag(HNode node)
    {
        var parent = node.Parent;
        if (parent is null) return false;
        var siblings = parent.Children;
        for (int i = node.Index - 1; i >= 0; i--)
        {
            var s = siblings[i];
            if (s.IsComment) continue;
            if (s.Tag is not null) return IsInlineElement(s.Tag);
            if (s.Text.Trim().Length > 0) return false;
        }
        return false;
    }

    private static bool NextSiblingIsInlineTag(HNode node)
    {
        var parent = node.Parent;
        if (parent is null) return false;
        var siblings = parent.Children;
        for (int i = node.Index + 1; i < siblings.Count; i++)
        {
            var s = siblings[i];
            if (s.IsComment) continue;
            if (s.Tag is not null) return IsInlineElement(s.Tag);
            if (s.Text.Trim().Length > 0) return false;
        }
        return false;
    }

    private static bool NextSiblingIsWhitespaceText(HNode node)
    {
        var parent = node.Parent;
        if (parent is null) return false;
        var siblings = parent.Children;
        for (int i = node.Index + 1; i < siblings.Count; i++)
        {
            var s = siblings[i];
            if (s.IsComment) continue;
            if (s.Tag is not null) return false;
            return s.Text.Trim().Length == 0;
        }
        return false;
    }

    private static void AppendInlineSuffix(StringBuilder output, string suffix, bool hasCoreContent, HNode node)
    {
        if (suffix.Length == 0) return;
        if (suffix == " " && hasCoreContent && NextSiblingIsWhitespaceText(node)) return;
        output.Append(suffix);
    }

    internal static bool IsInlineElement(string tag) => tag switch
    {
        "a" or "abbr" or "b" or "bdi" or "bdo" or "br" or "cite" or "code" or "data" or "dfn"
        or "em" or "i" or "kbd" or "mark" or "q" or "rp" or "rt" or "ruby" or "s" or "samp"
        or "small" or "span" or "strong" or "sub" or "sup" or "time" or "u" or "var" or "wbr"
        or "del" or "ins" or "img" or "map" or "area" or "audio" or "video" or "picture"
        or "source" or "track" or "embed" or "object" or "param" or "input" or "label"
        or "button" or "select" or "textarea" or "output" or "progress" or "meter" => true,
        _ => false,
    };

    // ── text helpers ─────────────────────────────────────────────────────────
    private static string TextContent(HNode node)
    {
        var sb = new StringBuilder();
        AppendTextContent(node, sb);
        return HtmlWalker.DecodeEntitiesFull(sb.ToString());
    }

    private static void AppendTextContent(HNode node, StringBuilder sb)
    {
        foreach (var c in node.Children)
        {
            if (c.IsComment) continue;
            if (c.Tag is null) sb.Append(c.Text);
            else AppendTextContent(c, sb);
        }
    }

    private static bool HasMoreThanOneChar(string s) => s.Length > 1;

    // normalize_whitespace: collapse spaces/tabs/unicode spaces, preserve newlines
    internal static string NormalizeWhitespaceKeepNewlines(string text)
    {
        var sb = new StringBuilder(text.Length);
        bool prevWasSpace = false;
        foreach (char ch in text)
        {
            bool isSpace = ch == ' ' || ch == '\t' || IsUnicodeSpace(ch);
            if (isSpace)
            {
                if (!prevWasSpace) { sb.Append(' '); prevWasSpace = true; }
            }
            else { sb.Append(ch); prevWasSpace = false; }
        }
        return sb.ToString();
    }

    /// <summary>
    /// Normalize whitespace inside a Markdown table cell. A cell cannot contain a hard line
    /// break, so unlike <see cref="NormalizeWhitespaceKeepNewlines"/> — which keeps <c>\n</c>
    /// for block-level rendering — this folds <c>\n</c> and <c>\r</c> into the run before
    /// collapsing consecutive whitespace to one ASCII space.
    /// </summary>
    internal static string NormalizeCellWhitespace(string text)
    {
        if (!text.Contains('\n') && !text.Contains('\r')) return NormalizeWhitespaceKeepNewlines(text);
        return NormalizeWhitespaceKeepNewlines(text.Replace('\n', ' ').Replace('\r', ' '));
    }

    private static bool IsUnicodeSpace(char ch) => ch is '\u00A0' or '\u1680'
        or (>= '\u2000' and <= '\u200A') or '\u202F' or '\u205F' or '\u3000';

    // text::chomp
    private static (string prefix, string suffix, string core) Chomp(string text)
    {
        if (text.Length == 0) return ("", "", "");
        string prefix = char.IsWhiteSpace(text[0]) ? " " : "";
        string suffix;
        string core;
        if (text.EndsWith("\n\n", StringComparison.Ordinal))
        {
            suffix = "\n\n";
            core = text.TrimEnd('\n').Trim();
            // Rust trims "\n\n" suffix then trims — same effect
        }
        else if (text.EndsWith(' ') || text.EndsWith('\t'))
        {
            suffix = " ";
            core = text.Trim();
        }
        else
        {
            suffix = "";
            core = text.Trim();
        }
        return (prefix, suffix, core);
    }

    // utility::content::chomp_inline
    private static (string prefix, string suffix, string core) ChompInline(string text)
    {
        if (text.Length == 0) return ("", "", "");
        string prefix = text[0] is ' ' or '\t' ? " " : "";
        bool hasTrailingLinebreak = text.EndsWith("  \n", StringComparison.Ordinal) || text.EndsWith("\\\n", StringComparison.Ordinal);
        string suffix = hasTrailingLinebreak
            ? (text.EndsWith("  \n", StringComparison.Ordinal) ? "  \n" : "\\\n")
            : (text.EndsWith(' ') || text.EndsWith('\t') ? " " : "");
        string trimmed;
        if (hasTrailingLinebreak)
        {
            string stripped = text.EndsWith("  \n", StringComparison.Ordinal) ? text[..^3]
                : text.EndsWith("\\\n", StringComparison.Ordinal) ? text[..^2] : text;
            trimmed = stripped.Trim();
        }
        else trimmed = text.Trim();
        return (prefix, suffix, trimmed);
    }

    internal static void TrimTrailingWhitespace(StringBuilder sb)
    {
        while (sb.Length > 0 && (sb[^1] == ' ' || sb[^1] == '\t')) sb.Length--;
    }

    private static bool EndsWith(StringBuilder sb, string s)
    {
        if (sb.Length < s.Length) return false;
        for (int i = 0; i < s.Length; i++)
            if (sb[sb.Length - s.Length + i] != s[i]) return false;
        return true;
    }

    private static bool StartsWith(StringBuilder sb, string s)
    {
        if (sb.Length < s.Length) return false;
        for (int i = 0; i < s.Length; i++)
            if (sb[i] != s[i]) return false;
        return true;
    }

    // main_helpers::trim_line_end_whitespace — preserve exactly-two-space hard breaks
    internal static string TrimLineEndWhitespace(string output)
    {
        if (output.Length == 0) return output;
        var cleaned = new StringBuilder(output.Length);
        var lines = output.Split('\n');
        foreach (var lineRaw in lines)
        {
            string line = lineRaw;
            string suffixNl = "\n";
            if (line.EndsWith("  ", StringComparison.Ordinal))
            {
                line = line[..^2];
                suffixNl = "  \n";
            }
            cleaned.Append(line.TrimEnd(' ', '\t')).Append(suffixNl);
        }
        string result = cleaned.ToString();
        string trimmed = result.TrimEnd('\n');
        if (trimmed.Length == 0) return "";
        return trimmed + "\n";
    }

    // main_helpers::collapse_excess_blank_lines
    internal static string CollapseExcessBlankLines(string output)
    {
        if (!output.Contains("\n\n\n")) return output;
        var cleaned = new StringBuilder(output.Length);
        int consecutive = 0;
        foreach (char ch in output)
        {
            if (ch == '\n')
            {
                consecutive++;
                if (consecutive <= 2) cleaned.Append(ch);
            }
            else { consecutive = 0; cleaned.Append(ch); }
        }
        return cleaned.ToString();
    }

    /// <summary>
    /// Port of xberg's `normalize_html_markdown` (extractors/html.rs): setext → ATX,
    /// strip trailing whitespace, blank line before ATX headings, single trailing newline.
    /// </summary>
    public static string NormalizeHtmlMarkdown(string raw)
    {
        var lines = SplitLines(raw);
        var pass1 = new List<string>(lines.Count);
        int i = 0;
        while (i < lines.Count)
        {
            string line = lines[i];
            string lineTrimmed = line.TrimEnd();
            if (i + 1 < lines.Count)
            {
                string next = lines[i + 1].Trim();
                bool isSetextH1 = next.Length > 0 && next.All(c => c == '=');
                bool isSetextH2 = next.Length > 0 && next.All(c => c == '-')
                    && lineTrimmed.Trim().Length > 0 && !lineTrimmed.Trim().StartsWith('|');
                if (isSetextH1)
                {
                    pass1.Add("# " + lineTrimmed.Trim());
                    i += 2;
                    continue;
                }
                if (isSetextH2)
                {
                    pass1.Add("## " + lineTrimmed.Trim());
                    i += 2;
                    continue;
                }
            }
            pass1.Add(lineTrimmed);
            i++;
        }

        var result = new StringBuilder(raw.Length);
        for (int idx = 0; idx < pass1.Count; idx++)
        {
            string line = pass1[idx];
            bool isAtx = line.StartsWith('#');
            if (isAtx && idx > 0 && pass1[idx - 1].Length > 0) result.Append('\n');
            result.Append(line).Append('\n');
        }

        string s = result.ToString();
        string trimmedEnd = s.TrimEnd();
        if (trimmedEnd.Length == 0) return "";
        return trimmedEnd + "\n";
    }

    // Rust `str::lines()`: split on \n, dropping a trailing empty segment; \r\n handled too.
    private static List<string> SplitLines(string s)
    {
        var list = new List<string>();
        int start = 0;
        for (int i = 0; i < s.Length; i++)
        {
            if (s[i] == '\n')
            {
                int end = i > start && s[i - 1] == '\r' ? i - 1 : i;
                list.Add(s[start..end]);
                start = i + 1;
            }
        }
        if (start < s.Length) list.Add(s[start..]);
        return list;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Minimal lenient HTML DOM (mirrors the `tl` parser semantics used by the crate:
// no HTML5 tree corrections, unclosed tags nest until parent close, li/dt/dd
// implied-close normalization applied as in `normalize_unclosed_list_items`).
// ─────────────────────────────────────────────────────────────────────────────

internal sealed class HNode
{
    public string? Tag;                   // null → text node
    public string Text = "";              // raw (entities NOT decoded)
    public string AttrString = "";
    public bool IsComment;
    public readonly List<HNode> Children = new();
    public HNode? Parent;
    public int Index;

    private Dictionary<string, string?>? _attrCache;

    /// <summary>
    /// Set on documents the html5ever repair re-serializes, whose attribute values reach the
    /// converter in canonical form.
    /// </summary>
    public bool CanonicalAttrs;

    /// <summary>
    /// Look up an attribute by exact name.
    /// </summary>
    /// <remarks>
    /// Case-sensitive on the ordinary path, because the converter this ports is: the lenient
    /// parser feeding it keeps attribute names as written and every lookup is an exact match,
    /// so <c>&lt;A HREF=…&gt;</c> reaches the link handler with no href at all and degrades to
    /// its label. A repaired document is the exception — html5ever's serializer writes every
    /// name back lowercase, so there the match ignores case to stand in for that rewrite.
    /// </remarks>
    public string? Attr(string name)
    {
        if (Tag is null) return null;
        _attrCache ??= new Dictionary<string, string?>(
            CanonicalAttrs ? StringComparer.OrdinalIgnoreCase : StringComparer.Ordinal);
        if (_attrCache.TryGetValue(name, out var cached)) return cached;
        // Attribute values are otherwise left as written: the lenient parser hands the handlers
        // the source bytes, and the markdown writer escapes what it emits.
        string? v = CanonicalAttrs
            ? HtmlWalker.ExtractAttr(AttrString, name)
            : HtmlWalker.ExtractAttrExact(AttrString, name);
        if (v is not null && CanonicalAttrs) v = HtmlToMarkdown.CanonicalizeAttrValue(v);
        _attrCache[name] = v;
        return v;
    }
}

internal static class HtmlDom
{
    private static readonly HashSet<string> VoidTags = new(StringComparer.Ordinal)
    {
        "area", "base", "br", "col", "embed", "hr", "img", "input",
        "link", "meta", "param", "source", "track", "wbr",
    };

    private static readonly HashSet<string> RawTextTags = new(StringComparer.Ordinal)
    {
        "script", "style", "textarea",
    };

    public static HNode Parse(string src)
    {
        var root = new HNode { Tag = "#root" };
        var stack = new List<HNode> { root };
        int pos = 0;
        int n = src.Length;

        void AddChild(HNode child)
        {
            var parent = stack[^1];
            child.Parent = parent;
            child.Index = parent.Children.Count;
            parent.Children.Add(child);
        }

        while (pos < n)
        {
            // A `<` only opens markup when a letter, `/`, `!` or `?` follows it (the HTML
            // tag-open state); otherwise it is character data — `a <- filter(x, y > 0)` is R
            // source, not a tag that swallows everything up to the next `>`.
            if (src[pos] != '<' || !HtmlWalker.OpensTag(src, pos))
            {
                int lt = pos;
                if (src[lt] == '<') lt++;
                while (lt < n && !(src[lt] == '<' && HtmlWalker.OpensTag(src, lt))) lt++;
                AddChild(new HNode { Tag = null, Text = src[pos..lt] });
                pos = lt;
                continue;
            }

            // comment
            if (pos + 3 < n && src[pos + 1] == '!' && src[pos + 2] == '-' && src[pos + 3] == '-')
            {
                pos = HtmlWalker.CommentEnd(src, pos);
                continue;
            }
            // A processing instruction is not markup to the reference parser. `parse_tag`
            // (astral-tl `parser/base.rs`) steps over the `<`, finds no identifier after it and
            // gives up, leaving the stream just past the `<` — so `?xml version="1.0"?>` comes
            // back out as character data rather than being swallowed to the next `>`.
            if (pos + 1 < n && src[pos + 1] == '?')
            {
                pos++;
                continue;
            }
            // doctype
            if (pos + 1 < n && src[pos + 1] == '!')
            {
                int gtd = src.IndexOf('>', pos);
                pos = gtd < 0 ? n : gtd + 1;
                continue;
            }

            int gt = FindTagEnd(src, pos, out int stop);
            if (gt < 0)
            {
                // `parse_tag` bailed without finding the tag's `>`. astral-tl's `parse_single`
                // ignores that failure and loops from wherever the stream stopped, so the
                // malformed tag is dropped and everything after it is still parsed. Treating the
                // remainder as one text node instead costs the whole document: on
                // office/regression/000_000061.html an unclosed `<input … value="" </span>` was
                // hiding every heading and paragraph that followed it.
                if (stop > pos && stop < n) { pos = stop; continue; }
                AddChild(new HNode { Tag = null, Text = src[pos..] });
                break;
            }
            string tagContent = src[(pos + 1)..gt];
            pos = gt + 1;

            bool isClosing = tagContent.StartsWith('/');
            string content = isClosing ? tagContent[1..] : tagContent;
            bool selfClosing = content.TrimEnd().EndsWith('/');
            content = content.TrimEnd('/').Trim();
            // An END tag is `read_end` in astral-tl: the whole slice up to `>` is the name.
            // A START tag is `read_ident`, which stops at the first non-identifier byte.
            var (name, attrs) = isClosing
                ? HtmlWalker.SplitTagName(content)
                : SplitIdentTagName(content);
            string tag = name.ToLowerInvariant();
            if (tag.Length == 0) continue;

            if (isClosing)
            {
                // pop to matching open tag if present
                for (int i = stack.Count - 1; i >= 1; i--)
                {
                    if (stack[i].Tag == tag)
                    {
                        stack.RemoveRange(i, stack.Count - i);
                        break;
                    }
                }
                continue;
            }

            // `<body>` closes an unterminated `<head>`. Without this the body nests inside the
            // head, and head content is deliberately not content — the whole document is lost.
            if (tag is "body")
            {
                for (int i = stack.Count - 1; i >= 1; i--)
                    if (stack[i].Tag == "head") { stack.RemoveRange(i, stack.Count - i); break; }
            }

            // implied close of li/dt/dd (normalize_unclosed_list_items) and p-in-p leniency
            if (tag is "li")
            {
                for (int i = stack.Count - 1; i >= 1; i--)
                {
                    if (stack[i].Tag == "li") { stack.RemoveRange(i, stack.Count - i); break; }
                    if (stack[i].Tag is "ul" or "ol") break;
                }
            }
            else if (tag is "dt" or "dd")
            {
                for (int i = stack.Count - 1; i >= 1; i--)
                {
                    if (stack[i].Tag is "dt" or "dd") { stack.RemoveRange(i, stack.Count - i); break; }
                    if (stack[i].Tag == "dl") break;
                }
            }
            else if (tag is "td" or "th" or "tr" or "thead" or "tbody" or "tfoot")
            {
                // Table sections close each other the way the HTML5 insertion modes say, and a
                // cell that opens with no row around it gets one. Old hand-written pages write
                // `<table><td>…</td><td>…</td></table>`, and without the implied row every cell
                // hangs off the table where no consumer looks for it — on one fixture that is the
                // table holding 99% of the document.
                for (int i = stack.Count - 1; i >= 1; i--)
                {
                    string? open = stack[i].Tag;
                    if (open is "table") break;
                    if (tag is "td" or "th")
                    {
                        if (open is "tr") break;
                        if (open is "td" or "th") { stack.RemoveRange(i, stack.Count - i); break; }
                    }
                    else if (open is "tr" or "td" or "th"
                             || (tag is "thead" or "tbody" or "tfoot" && open is "thead" or "tbody" or "tfoot"))
                    {
                        stack.RemoveRange(i, stack.Count - i);
                    }
                    else break;
                }

                if (tag is "td" or "th" && stack[^1].Tag is not "tr"
                    && stack[^1].Tag is "table" or "thead" or "tbody" or "tfoot")
                {
                    var impliedRow = new HNode { Tag = "tr", AttrString = "" };
                    AddChild(impliedRow);
                    stack.Add(impliedRow);
                }
            }

            var node = new HNode { Tag = tag, AttrString = attrs };
            AddChild(node);

            if (VoidTags.Contains(tag) || selfClosing) continue;

            if (RawTextTags.Contains(tag))
            {
                // consume raw text until matching close tag
                string closeStart = "</" + tag;
                int searchFrom = pos;
                int close;
                while (true)
                {
                    close = src.IndexOf(closeStart, searchFrom, StringComparison.OrdinalIgnoreCase);
                    if (close < 0) { close = n; break; }
                    int k = close + closeStart.Length;
                    while (k < n && char.IsWhiteSpace(src[k])) k++;
                    if (k < n && src[k] == '>') break;
                    searchFrom = close + 1;
                }
                if (tag == "textarea" && close > pos)
                {
                    var text = new HNode { Tag = null, Text = src[pos..Math.Min(close, n)] };
                    text.Parent = node;
                    text.Index = 0;
                    node.Children.Add(text);
                }
                if (close >= n) { pos = n; }
                else
                {
                    int gtc = src.IndexOf('>', close);
                    pos = gtc < 0 ? n : gtc + 1;
                }
                continue;
            }

            stack.Add(node);
        }

        return root;
    }

    /// <summary>
    /// Find the <c>&gt;</c> terminating the tag that opens at <paramref name="lt"/>, or -1.
    /// </summary>
    /// <remarks>
    /// Ported from astral-tl's <c>parse_tag</c> / <c>parse_attributes</c> / <c>read_end</c>
    /// (<c>src/parser/base.rs</c>), the tokenizer the reference converter parses with. The rule
    /// that matters here is positional: a quote opens an attribute VALUE only where a value may
    /// start — immediately after an attribute name's <c>=</c>, past any spaces or newlines.
    /// A <c>"</c> anywhere else is an ordinary character. Treating every quote as a delimiter
    /// instead lets one stray quote in an attribute NAME — <c>&lt;KSHIM NAME="a" B}"&gt;</c> in a
    /// PDF's XML listing — hide the tag's own <c>&gt;</c> and swallow the document up to the next
    /// quote, which is text loss rather than a mis-parse.
    /// </remarks>
    private static int FindTagEnd(string src, int lt) => FindTagEnd(src, lt, out _);

    /// <param name="stop">
    /// Where the scan gave up when it found no <c>&gt;</c>. `parse_single` ignores `parse_tag`'s
    /// failure and carries on from the stream position it left behind, so a malformed tag costs
    /// the tag and not the rest of the document.
    /// </param>
    private static int FindTagEnd(string src, int lt, out int stop)
    {
        int n = src.Length;
        stop = -1;

        // `read_end`: an end tag runs to the next `>`; it parses no attributes and honours no
        // quotes.
        if (lt + 1 < n && src[lt + 1] == '/') return src.IndexOf('>', lt + 1);

        int i = SkipTagSpace(src, lt + 1);
        i = SkipIdent(src, i);                    // `read_ident`: the tag name

        // `parse_attributes`
        while (i < n)
        {
            i = SkipTagSpace(src, i);
            if (i >= n) { stop = i; return -1; }
            if (src[i] is '/' or '>') break;      // `is_closing`

            int nameEnd = SkipIdent(src, i);
            if (nameEnd == i) { i++; continue; }  // no identifier here — skip the character

            i = SkipTagSpace(src, nameEnd);
            if (i >= n || src[i] != '=') continue;   // a valueless attribute
            i = SkipTagSpace(src, i + 1);
            if (i >= n) { stop = i; return -1; }

            char quote = src[i];
            if (quote is '"' or '\'')
            {
                int close = src.IndexOf(quote, i + 1);
                i = close < 0 ? n : close;        // `read_to` stops ON the closing quote
            }
            else
            {
                // `read_to3([b' ', b'\n', b'>'])`: an unquoted value ends at a space, a
                // newline or the tag's own `>`.
                while (i < n && src[i] is not (' ' or '\n' or '>')) i++;
            }

            // "Only advance past the delimiter if we read a value."
            if (i < n && src[i] is not ('/' or '>')) i++;
        }

        if (i < n && src[i] == '/') i++;          // `is_self_closing`
        if (i < n && src[i] == '>') return i;
        stop = i;
        return -1;
    }

    /// <summary>
    /// Split a start tag's source into its element name and the rest, following astral-tl's
    /// <c>read_ident</c>: the name is the leading run of identifier bytes, not everything up to
    /// the first space. A PDF hex dump whose ASCII column renders as <c>&lt;p..."</c> is a
    /// <c>p</c> element carrying junk attributes to the reference parser; splitting on whitespace
    /// instead names the element <c>p..."</c>, which matches nothing and drops the block break
    /// the paragraph carries.
    /// </summary>
    private static (string Name, string Attrs) SplitIdentTagName(string content)
    {
        int end = SkipIdent(content, 0);
        return (content[..end], content[end..]);
    }

    /// <summary>astral-tl's <c>skip_whitespaces</c>: spaces and newlines only.</summary>
    private static int SkipTagSpace(string src, int i)
    {
        while (i < src.Length && src[i] is ' ' or '\n') i++;
        return i;
    }

    /// <summary>
    /// astral-tl's <c>read_ident</c> run (<c>util::is_ident</c>): ASCII alphanumerics plus
    /// <c>- _ : + /</c>.
    /// </summary>
    private static int SkipIdent(string src, int i)
    {
        while (i < src.Length && IsIdent(src[i])) i++;
        return i;
    }

    private static bool IsIdent(char c) =>
        char.IsAsciiLetterOrDigit(c) || c is '-' or '_' or ':' or '+' or '/';
}
