using System.Text;
using XRay.Content.Core;
using XRay.Content.Types;

namespace XRay.Content.Internal.Html;

/// <summary>
/// Byte-level HTML walker that emits directly into an <see cref="InternalDocumentBuilder"/>.
/// Ports the Rust `extraction/html/structure.rs` walker combined with the flat→element mapping
/// of `extractors/html.rs::walk_nodes`. Headings open nested "section" Groups (mirroring
/// html-to-markdown's DocumentStructure), so element depth and the derived JSON section tree
/// match the Rust output. The document <c>&lt;head&gt;</c> is skipped for content (metadata is
/// scanned separately).
/// </summary>
public sealed class HtmlWalker
{
    private readonly string _src;
    private int _pos;
    private readonly InternalDocumentBuilder _b;

    // Heading-section group stack (heading levels of open Groups).
    private readonly List<byte> _groupStack = new();

    // Paragraph / inline accumulation. _textBuf is kept whitespace-normalized incrementally
    // so annotation byte offsets align with the final (normalized) paragraph text.
    private readonly StringBuilder _textBuf = new();
    private bool _lastWasSpace = true;
    private readonly List<InlineSpan> _inlineStack = new();
    private readonly List<TextAnnotation> _annotations = new();

    // Container state
    private bool _inPre;
    private int _pDepth;                     // >0 while inside a <p> (paragraphs are emitted only from <p>)
    private PreBlock? _preBlock;
    private TableAccumulator? _table;
    // List handling: html-to-markdown emits a SEPARATE list node for every <ul>/<ol>
    // (all siblings under the current section), while nested lists are ALSO flattened into
    // their parent item's text with indented bullet/number markers. We buffer the whole
    // list subtree into a tree and emit it when the outermost list closes.
    private readonly List<LList> _listStack = new();   // currently-open lists
    private readonly List<LItem> _itemStack = new();   // currently-open <li> items
    private bool InListItem => _itemStack.Count > 0;
    private StringBuilder CurrentItemBuffer => _itemStack[^1].Inline;

    // Definition list
    private bool _inDl;
    private string? _dlTerm;
    private bool _inDt, _inDd;
    private readonly StringBuilder _dtText = new();
    private readonly StringBuilder _ddText = new();

    /// <summary>
    /// The hostile-input counters for this walk. Charged where upstream's node walk charges:
    /// one loop turn per tag, a descent per nested container, the emitted text of every block,
    /// and a table's cells before the table is built.
    /// </summary>
    private readonly SecurityBudget _budget;

    public HtmlWalker(string src, InternalDocumentBuilder builder, SecurityLimits? limits = null)
    {
        _src = src;
        _b = builder;
        _budget = new SecurityBudget(limits ?? new SecurityLimits());
    }

    /// <summary>
    /// The index just past the end of the comment that starts at <paramref name="start"/>, or
    /// the end of the input when it is never closed.
    /// </summary>
    /// <remarks>
    /// A comment normally ends at <c>--&gt;</c>. Upstream's converter ends it early when the
    /// comment reads like a self-closing tag: for a comment whose content opens with whitespace
    /// or a slash, it scans quote-aware to the first <c>&gt;</c>, and if that <c>&gt;</c> is
    /// preceded by <c>/</c> the comment stops there — leaking the rest of it into the document
    /// as text. A <c>&gt;</c> that is not preceded by a slash ends the scan without truncating,
    /// which is why a comment holding <c>&lt;a href="…"&gt;…&lt;img … /&gt;</c> survives intact
    /// while one holding <c>&lt;br /&gt;</c> does not.
    ///
    /// This is neither html5ever's behaviour nor `tl`'s — both were probed directly and parse
    /// such comments correctly — so it comes from the converter's own preprocessing. The rule was
    /// characterised by differential testing against the real converter: 243 generated shapes,
    /// including quoted attributes, conditional comments and multiple <c>/&gt;</c>, all agree.
    /// Eleven fixtures in the corpus contain a comment this applies to.
    /// </remarks>
    internal static int CommentEnd(string src, int start)
    {
        int contentStart = start + 4;
        int normalEnd = src.IndexOf("-->", contentStart, StringComparison.Ordinal);
        int end = normalEnd < 0 ? src.Length : normalEnd + 3;

        if (contentStart >= src.Length) return end;
        char first = src[contentStart];
        if (!char.IsWhiteSpace(first) && first != '/') return end;

        char quote = '\0';
        for (int i = contentStart; i < src.Length; i++)
        {
            char c = src[i];
            if (quote != '\0') { if (c == quote) quote = '\0'; continue; }
            if (c == '"' || c == '\'') { quote = c; continue; }
            // The first unquoted `>` settles it either way.
            if (c == '>') return i > contentStart && src[i - 1] == '/' ? i + 1 : end;
        }
        return end;
    }

    public void Walk()
    {
        while (_pos < _src.Length)
        {
            if (Starts("<!--"))
            {
                _pos = CommentEnd(_src, _pos);
                continue;
            }
            _budget.Step();
            if (_src[_pos] == '<' && OpensTag(_src, _pos)) HandleTag();
            else HandleText();
        }
        CloseParagraphContext();
        while (_groupStack.Count > 0) { _b.PushGroupEnd(); _groupStack.RemoveAt(_groupStack.Count - 1); }

        if (_b.NodeCount == 0 && _discarded.Length != 0)
            _b.PushParagraph(_discarded.ToString(), new(), null, null);
    }

    /// <summary>Charge a table's cells before it is handed to the builder.</summary>
    private void ChargeCells(List<List<string>> grid)
    {
        long count = 0;
        foreach (var row in grid) count += row.Count;
        _budget.AddCells(count);
    }

    private bool Starts(string s) => string.CompareOrdinal(_src, _pos, s, 0, s.Length) == 0 && _pos + s.Length <= _src.Length;

    // ── text ─────────────────────────────────────────────────────────────────
    /// <summary>
    /// Whether the <c>&lt;</c> at <paramref name="pos"/> opens markup. The HTML tag-open state
    /// takes only an ASCII letter, <c>/</c>, <c>!</c> or <c>?</c>; anything else — an R
    /// assignment arrow, a less-than sign in prose — is character data, and the <c>&lt;</c>
    /// stands for itself.
    /// </summary>
    internal static bool OpensTag(string src, int pos)
    {
        if (pos + 1 >= src.Length) return false;
        char c = src[pos + 1];
        // `</` and `<!` still need a name after them: `</=` and `<!5` are character data, not
        // markup, and a page that writes them (quoted-printable mail cut mid-tag, say) must keep
        // the text rather than lose everything up to the next `>`.
        if (c == '/' || c == '!')
        {
            if (pos + 2 >= src.Length) return false;
            char d = src[pos + 2];
            return char.IsAsciiLetter(d) || (c == '!' && d == '-');
        }
        return char.IsAsciiLetter(c) || c == '?';
    }

    private void HandleText()
    {
        int start = _pos;
        // A `<` that cannot open a tag is content: step over it so the scan does not stall.
        if (_pos < _src.Length && _src[_pos] == '<') _pos++;
        while (_pos < _src.Length && !(_src[_pos] == '<' && OpensTag(_src, _pos))) _pos++;
        string decoded = DecodeEntities(_src[start.._pos]);

        if (_table is not null) { _table.PushText(decoded); return; }
        if (_preBlock is not null) { _preBlock.Text.Append(decoded); return; }
        if (InListItem) { AppendNormalizedTo(CurrentItemBuffer, decoded); return; }
        if (_inDt) { _dtText.Append(decoded); return; }
        if (_inDd) { _ddText.Append(decoded); return; }
        AppendNormalized(decoded);
    }

    // Append text to the paragraph buffer, collapsing whitespace on the fly (mirrors
    // NormalizeWhitespace) and preserving the \x01 <br> sentinel so offsets stay stable.
    private void AppendNormalized(string s) => AppendNormalizedTo(_textBuf, s);

    // Boundary-aware whitespace normalization into `target`: collapses spaces/tabs, keeps
    // newlines INTERNAL to a text node, and reduces leading/trailing (element-boundary)
    // whitespace to a single pending word-break space. Shared _lastWasSpace state carries the
    // pending space across nodes/inline elements. Used for both paragraph and list-item text.
    private void AppendNormalizedTo(StringBuilder target, string s)
    {
        int n = s.Length, idx = 0;
        bool nodeContent = false;   // has content been seen within THIS text node yet
        while (idx < n)
        {
            char c = s[idx];
            if (c == '\r') { idx++; continue; }
            if (c == '\x01')
            {
                while (target.Length > 0 && target[^1] == ' ') target.Remove(target.Length - 1, 1);
                target.Append('\x01');
                _lastWasSpace = true;
                nodeContent = true;
                idx++;
                continue;
            }
            if (c is ' ' or '\t' or '\n' or '\f' or '\v')
            {
                var run = new StringBuilder();
                bool prevSp = false;
                int j = idx;
                while (j < n)
                {
                    char w = s[j];
                    if (w == '\r') { j++; continue; }
                    if (w == '\n') { run.Append('\n'); prevSp = false; j++; }
                    else if (w is ' ' or '\t' or '\f' or '\v') { if (!prevSp) { run.Append(' '); prevSp = true; } j++; }
                    else break;
                }
                bool trailing = j >= n;
                if (!nodeContent || trailing)
                {
                    _lastWasSpace = true;
                }
                else
                {
                    target.Append(run);
                    _lastWasSpace = run.Length > 0 && run[^1] == ' ';
                }
                idx = j;
                continue;
            }
            if (_lastWasSpace)
            {
                // A pending word-break space is dropped after a line break, including the `<br>`
                // sentinel: the source newline that follows `<br>` is layout, and keeping it
                // indents the next line by one space.
                if (target.Length > 0 && target[^1] is not (' ' or '\n' or '\x01')) target.Append(' ');
                _lastWasSpace = false;
            }
            target.Append(c);
            nodeContent = true;
            idx++;
        }
    }

    // Finalize buffered inline text: <br> sentinel → newline, then trim ends (internal newlines
    // preserved). Buffer is already space-collapsed by AppendNormalizedTo.
    private static string FinalizeInline(string s) => s.Replace('\x01', '\n').Trim();

    // Table cell text normalization: collapse spaces/tabs to a single space but PRESERVE
    // newlines (html-to-markdown renders <br> in a cell as " \n" and keeps them), then trim.
    private static string CellNormalize(string s)
    {
        var outp = new StringBuilder(s.Length);
        bool pendingSpace = false, seen = false;
        foreach (char c in s)
        {
            if (c == '\r') continue;
            if (c == '\n') { if (pendingSpace && seen) outp.Append(' '); pendingSpace = false; if (seen) outp.Append('\n'); }
            else if (c is ' ' or '\t' or '\f' or '\v') { if (seen) pendingSpace = true; }
            else { if (pendingSpace) { outp.Append(' '); pendingSpace = false; } outp.Append(c); seen = true; }
        }
        while (outp.Length > 0 && (outp[^1] == ' ' || outp[^1] == '\n')) outp.Remove(outp.Length - 1, 1);
        return outp.ToString();
    }

    // Append a literal inline string to whichever buffer is active (used by <q> quotes).
    private void AppendInline(string s)
    {
        if (_table is not null) { _table.PushText(s); return; }
        if (_preBlock is not null) { _preBlock.Text.Append(s); return; }
        if (InListItem) { AppendNormalizedTo(CurrentItemBuffer, s); return; }
        if (_inDt) { _dtText.Append(s); return; }
        if (_inDd) { _ddText.Append(s); return; }
        AppendNormalized(s);
    }

    private void ClearTextBuf() { _textBuf.Clear(); _lastWasSpace = true; }

    // ── tags ───────────────────────────────────────────────────────────────
    private int _tagStart;

    private void HandleTag()
    {
        _tagStart = _pos;
        int gt = _src.IndexOf('>', _pos);
        if (gt < 0) { _pos = _src.Length; return; }
        string tagContent = _src[(_pos + 1)..gt];
        _pos = gt + 1;

        if (tagContent.StartsWith('!') || tagContent.StartsWith('?')) return;

        bool isClosing = tagContent.StartsWith('/');
        string content = isClosing ? tagContent[1..] : tagContent;
        content = content.TrimEnd('/').Trim();

        var (tagName, attrs) = SplitTagName(content);
        string tag = tagName.ToLowerInvariant();

        if (isClosing) HandleClose(tag);
        else HandleOpen(tag, attrs, tagContent.EndsWith('/'));
    }

    private void HandleOpen(string tag, string attrs, bool selfClosing)
    {
        // Readability/boilerplate removal (html-to-markdown Standard preset): drop <nav>/<form>
        // unconditionally and <header>/<footer>/<aside> when they carry navigation hints. The
        // whole subtree is skipped so its chrome never reaches the InternalDocument.
        if (ShouldDropForPreprocessing(tag, attrs))
        {
            if (!selfClosing) SkipSubtree(tag);
            return;
        }

        switch (tag)
        {
            case "head":
            {
                // Skip the head — metadata is handled by a separate scan. `<body>` closes it
                // implicitly, which matters: a document with no `</head>` at all is not one
                // whose head runs to the last byte, and treating it that way swallowed the
                // whole file.
                int close = _src.IndexOf("</head>", _pos, StringComparison.OrdinalIgnoreCase);
                int body = _src.IndexOf("<body", _pos, StringComparison.OrdinalIgnoreCase);
                if (close >= 0 && (body < 0 || close < body)) _pos = close + "</head>".Length;
                else if (body >= 0) _pos = body;
                else _pos = _src.Length;
                break;
            }
            case "h1": case "h2": case "h3": case "h4": case "h5": case "h6":
                CloseParagraphContext();
                break;
            case "p":
                CloseParagraphContext();
                _pDepth++;
                break;
            case "br":
                if (_inPre || _preBlock is not null) { _preBlock?.Text.Append('\n'); }
                else if (_table is not null) _table.PushText(" \n");   // rendered as " \n" inside cells
                else if (InListItem) AppendNormalizedTo(CurrentItemBuffer, "\x01");
                else AppendNormalized("\x01");
                break;
            case "strong": case "b": PushInline(InlineKind.Bold, null, null); break;
            case "em": case "i": case "var": case "cite": case "dfn": PushInline(InlineKind.Italic, null, null); break;
            case "kbd": case "samp": PushInline(InlineKind.Code, null, null); break;
            case "code":
                if (_inPre)
                    _preBlock = new PreBlock { Language = ExtractLanguageFromClass(ExtractAttr(attrs, "class")) };
                else PushInline(InlineKind.Code, null, null);
                break;
            case "u": PushInline(InlineKind.Underline, null, null); break;
            case "ins": PushInline(InlineKind.Highlight, null, null); break;
            case "s": case "del": case "strike": PushInline(InlineKind.Strikethrough, null, null); break;
            case "sub": PushInline(InlineKind.Subscript, null, null); break;
            case "sup": PushInline(InlineKind.Superscript, null, null); break;
            case "mark": PushInline(InlineKind.Highlight, null, null); break;
            case "a":
            {
                string? href = ExtractAttr(attrs, "href");
                // href entities are decoded (matches html-to-markdown); the title attribute is
                // kept raw (html-to-markdown does not entity-decode link titles).
                PushInline(InlineKind.Link,
                    href is null ? "" : DecodeEntities(href),
                    ExtractAttr(attrs, "title"));
                break;
            }
            case "q":
                AppendInline("\"");
                break;
            case "pre":
                CloseParagraphContext();
                _inPre = true;
                _preBlock = new PreBlock();
                break;
            case "blockquote":
                CloseParagraphContext();
                _b.PushQuoteStart();
                break;
            case "ul":
                if (_listStack.Count == 0) CloseParagraphContext();
                OpenList(false, 1);
                break;
            case "ol":
                if (_listStack.Count == 0) CloseParagraphContext();
                OpenList(true, (int)ParseU32(ExtractAttr(attrs, "start"), 1));
                break;
            case "li":
                OpenItem();
                break;
            case "table":
                // Top-level tables: capture the raw subtree HTML, skip it in the stream, and
                // delegate grid building to the html-to-markdown converter so cell content is
                // rendered under an in-cell markdown context and nested tables are flattened as
                // separate tables (mirrors the crate's structure collector). Tables nested inside
                // list items / definition lists / <pre> keep the legacy accumulator path.
                if (_table is null && _preBlock is null && !InListItem && !_inDt && !_inDd && !selfClosing)
                {
                    CloseParagraphContext();
                    int end = FindMatchingClose("table", _pos);
                    string tableHtml = _src[_tagStart..end];
                    _pos = end;
                    EmitTablesFromHtml(tableHtml);
                }
                else
                {
                    CloseParagraphContext();
                    _table = new TableAccumulator();
                }
                break;
            case "tr": if (_table is not null) _table.OpenRow(); break;
            case "thead": case "tbody": case "tfoot": break;
            case "th": case "td":
                if (_table is not null)
                {
                    uint colSpan = ParseU32(ExtractAttr(attrs, "colspan"), 1);
                    uint rowSpan = ParseU32(ExtractAttr(attrs, "rowspan"), 1);
                    _table.OpenCell(colSpan, rowSpan, tag == "th");
                }
                break;
            case "img":
            {
                // html-to-markdown emits every <img> as its own image node (using its alt),
                // even inside <a>/<figure>; the enclosing link and any <figcaption> are ignored.
                if (!InListItem)
                {
                    CloseParagraphContext();
                    EmitImage(ExtractAttr(attrs, "alt"), ExtractAttr(attrs, "src"));
                }
                break;
            }
            case "dl":
                CloseParagraphContext();
                _inDl = true; _dlTerm = null;
                break;
            case "dt":
                FlushDefinitionItem();
                _inDt = true; _dtText.Clear();
                break;
            case "dd":
                _inDt = false;
                if (_inDl) { string term = NormalizeWhitespace(_dtText.ToString()); if (term.Length > 0) _dlTerm = term; }
                _dtText.Clear();
                _inDd = true; _ddText.Clear();
                break;
            case "script": case "style":
                SkipRawElement(tag); // raw block skipped for content
                break;
            case "video": case "audio":
            {
                int before = _pos;
                SkipRawElement(tag);
                if (_pos >= _src.Length && before < _src.Length) _pos = before; // no close tag: don't swallow rest
                break;
            }
            case "hr":
                CloseParagraphContext();
                break;
            case "div": case "section": case "article": case "main": case "aside":
            case "header": case "footer": case "nav": case "details": case "summary":
            case "figure": case "figcaption":
                CloseParagraphContext();
                break;
            default:
                break; // span/html/body/title/link and unknowns: passthrough
        }
    }

    private void HandleClose(string tag)
    {
        switch (tag)
        {
            case "h1": case "h2": case "h3": case "h4": case "h5": case "h6":
            {
                byte level = byte.TryParse(tag.AsSpan(1), out var l) ? l : (byte)1;
                string text = NormalizeWhitespace(_textBuf.ToString());
                if (text.Length > 0) EmitHeading(level, text);
                ClearTextBuf();
                _annotations.Clear();
                _inlineStack.Clear();
                break;
            }
            case "p": CloseParagraphContext(); break;
            case "strong": case "b": PopInline(InlineKind.Bold); break;
            case "em": case "i": case "var": case "cite": case "dfn": PopInline(InlineKind.Italic); break;
            case "kbd": case "samp": PopInline(InlineKind.Code); break;
            case "code": if (!_inPre) PopInline(InlineKind.Code); break;
            case "u": PopInline(InlineKind.Underline); break;
            case "ins": PopInline(InlineKind.Highlight); break;
            case "s": case "del": case "strike": PopInline(InlineKind.Strikethrough); break;
            case "sub": PopInline(InlineKind.Subscript); break;
            case "sup": PopInline(InlineKind.Superscript); break;
            case "mark": PopInline(InlineKind.Highlight); break;
            case "a": PopInlineLink(); break;
            case "q": AppendInline("\""); break;
            case "pre":
                if (_preBlock is not null)
                {
                    string text = _preBlock.Text.ToString().TrimEnd('\n');
                    if (text.Length > 0) _b.PushCode(text, _preBlock.Language, null, null);
                    _preBlock = null;
                }
                _inPre = false;
                break;
            case "blockquote":
                CloseParagraphContext();
                _b.PushQuoteEnd();
                break;
            case "ul": case "ol":
                CloseList();
                break;
            case "li": CloseItem(); break;
            case "table":
                if (_table is not null)
                {
                    _table.CloseCell(); _table.CloseRow();
                    if (_table.Rows.Count > 0) EmitTable(_table.Rows);
                    _table = null;
                }
                break;
            case "tr": if (_table is not null) { _table.CloseCell(); _table.CloseRow(); } break;
            case "th": case "td": if (_table is not null) _table.CloseCell(); break;
            case "dl": FlushDefinitionItem(); _inDl = false; break;
            case "dt": _inDt = false; break;
            case "dd": _inDd = false; FlushDefinitionItem(); break;
            case "div": case "section": case "article": case "main": case "aside":
            case "header": case "footer": case "nav": case "details": case "summary":
            case "figure": case "figcaption":
                CloseParagraphContext();
                break;
        }
    }

    // ── heading grouping ─────────────────────────────────────────────────────
    private void EmitHeading(byte level, string text)
    {
        while (_groupStack.Count > 0 && _groupStack[^1] >= level)
        {
            _b.PushGroupEnd();
            _groupStack.RemoveAt(_groupStack.Count - 1);
        }
        _b.PushGroupStart(null, null);
        _groupStack.Add(level);
        _budget.AccountText(Encoding.UTF8.GetByteCount(text));
        _b.PushHeading(level, text, null, null);
    }

    private void EmitImage(string? description, string? src)
    {
        string text = description ?? "";
        if (text.Length > 0 || !string.IsNullOrEmpty(src))
        {
            string display;
            if (!string.IsNullOrEmpty(src))
                display = text.Length == 0 ? $"![]({src})" : $"![{text}]({src})";
            else
                display = text;
            _budget.AccountText(Encoding.UTF8.GetByteCount(display));
            _b.PushParagraph(display, new(), null, null);
        }
        if (!string.IsNullOrEmpty(src))
            _b.PushUri(new ExtractedUri { Url = src!, Label = description, Kind = UriKind.Image });
    }

    // Find the index just past the `</tag>` that matches the already-opened `tag` (current _pos
    // is just after its opening `<tag ...>`), tracking same-tag nesting. Returns _src.Length if
    // no matching close is found.
    private int FindMatchingClose(string tag, int from)
    {
        int depth = 1;
        int p = from;
        while (p < _src.Length)
        {
            int lt = _src.IndexOf('<', p);
            if (lt < 0) return _src.Length;
            p = lt;
            if (string.CompareOrdinal(_src, p, "<!--", 0, 4) == 0)
            {
                p = CommentEnd(_src, p);
                continue;
            }
            int gt = _src.IndexOf('>', p);
            if (gt < 0) return _src.Length;
            string raw = _src[(p + 1)..gt];
            p = gt + 1;
            if (raw.StartsWith('!') || raw.StartsWith('?')) continue;
            bool closing = raw.StartsWith('/');
            string c = closing ? raw[1..] : raw;
            bool self = c.TrimEnd().EndsWith('/');
            c = c.TrimEnd('/').Trim();
            var (nm, _) = SplitTagName(c);
            if (!nm.Equals(tag, StringComparison.OrdinalIgnoreCase)) continue;
            if (closing) { depth--; if (depth == 0) return p; }
            else if (!self) depth++;
        }
        return _src.Length;
    }

    // Parse a captured <table>…</table> subtree and emit its grid (plus any nested tables, in the
    // crate's nested-before-parent order) as InternalDocument tables.
    private void EmitTablesFromHtml(string tableHtml)
    {
        var root = HtmlDom.Parse(tableHtml);
        var table = FindFirstTag(root, "table");
        if (table is null) return;
        HtmlToMarkdown.EmitTableTree(table, grid =>
        {
            if (grid.Count > 0) { ChargeCells(grid); _b.PushTableFromCells(grid, null, null); }
        }, (alt, src) => EmitImage(alt, src));
    }

    private static HNode? FindFirstTag(HNode node, string tag)
    {
        foreach (var c in node.Children)
        {
            if (c.Tag == tag) return c;
            if (c.Tag is not null)
            {
                var r = FindFirstTag(c, tag);
                if (r is not null) return r;
            }
        }
        return null;
    }

    private void EmitTable(List<List<CellMeta>> rows)
    {
        bool hasSpans = rows.Any(r => r.Any(c => c.ColSpan > 1 || c.RowSpan > 1));
        if (!hasSpans)
        {
            var simple = rows.Select(r => r.Select(c => CellNormalize(c.Text)).ToList()).ToList();
            ChargeCells(simple);
            _b.PushTableFromCells(simple, null, null);
            return;
        }
        // Advancing by colspan alone ignores the columns a rowspan from an earlier row still
        // covers, which slides every cell beneath one leftwards and out from under its header.
        var grid = Tables.GridFlatten.FlattenSpannedRows<CellMeta>(
            rows, c => (int)c.ColSpan, c => (int)c.RowSpan, c => CellNormalize(c.Text));
        ChargeCells(grid);
        _b.PushTableFromCells(grid, null, null);
    }

    // ── inline formatting ─────────────────────────────────────────────────────
    // html-to-markdown bakes inline markdown directly into the element text (the plain
    // renderer strips annotations, so text carries the formatting). Nested same-type spans
    // are de-duplicated: only the outermost open/close emits its marker.
    private static string Marker(InlineKind kind) => kind switch
    {
        InlineKind.Bold => "**",
        InlineKind.Italic => "*",
        InlineKind.Code => "`",
        InlineKind.Strikethrough => "~~",
        InlineKind.Highlight => "==",
        _ => "",
    };

    private void PushInline(InlineKind kind, string? href, string? title)
    {
        if (kind == InlineKind.Link)
        {
            AppendInline("[");
            _inlineStack.Add(new InlineSpan { Kind = kind, Href = href, Title = title, TextStart = ActiveInlineBuffer?.Length ?? 0 });
            return;
        }
        string mk = Marker(kind);
        // Only <strong>/<b> de-dupe when nested inside the same kind (crate `in_strong`);
        // <em>/<i>, strikethrough and highlight always emit their markers (no de-dupe).
        bool dedup = kind == InlineKind.Bold && _inlineStack.Exists(s => s.Kind == kind);
        bool emit = mk.Length > 0 && !dedup;
        if (emit) AppendInline(mk);
        _inlineStack.Add(new InlineSpan { Kind = kind, Emitted = emit });
    }

    private void PopInline(InlineKind expected)
    {
        int idx = _inlineStack.FindLastIndex(s => s.Kind == expected);
        if (idx < 0) return;
        var span = _inlineStack[idx];
        _inlineStack.RemoveAt(idx);
        if (span.Emitted) AppendInline(Marker(expected));
    }

    private void PopInlineLink()
    {
        int idx = _inlineStack.FindLastIndex(s => s.Kind == InlineKind.Link);
        if (idx < 0) return;
        var span = _inlineStack[idx];
        _inlineStack.RemoveAt(idx);
        var buf = ActiveInlineBuffer;
        if (buf is not null && span.TextStart <= buf.Length)
        {
            // Link labels are whitespace-trimmed (matches html-to-markdown's normalize+trim).
            while (buf.Length > span.TextStart && char.IsWhiteSpace(buf[^1])) buf.Remove(buf.Length - 1, 1);
            while (span.TextStart < buf.Length && char.IsWhiteSpace(buf[span.TextStart])) buf.Remove(span.TextStart, 1);
            // Wikipedia back-reference normalization: a bare "^" label with a fragment href
            // becomes "↑" to avoid clashing with markdown footnote syntax (link.rs).
            if (span.Href is not null && span.Href.StartsWith('#') &&
                buf.Length - span.TextStart == 1 && buf[span.TextStart] == '^')
                buf[span.TextStart] = '↑';

            // GFM autolink: when the label equals an absolute-scheme href, emit `<href>`
            // (mailto uses the address without the scheme). Matches html-to-markdown defaults.
            if (span.Href is not null && span.TextStart >= 1 && buf[span.TextStart - 1] == '[' && HasUriScheme(span.Href))
            {
                bool isMailto = span.Href.StartsWith("mailto:", StringComparison.Ordinal);
                string autoText = isMailto ? span.Href[7..] : span.Href;
                string label = buf.ToString(span.TextStart, buf.Length - span.TextStart);
                if (label == autoText)
                {
                    buf.Length = span.TextStart - 1;   // drop the "[" and label
                    buf.Append('<').Append(autoText).Append('>');
                    _lastWasSpace = false;
                    return;
                }
            }
        }
        // The label was just trimmed, so the closing "]" must follow it directly — drop any
        // pending word-break space left over from the label's trailing whitespace.
        _lastWasSpace = false;
        // A present-but-empty title attribute still renders the "" marker (matches html-to-markdown);
        // only an absent title (null) omits the quotes.
        string suffix = span.Title is null ? $"]({span.Href})" : $"]({span.Href} \"{span.Title}\")";
        AppendInline(suffix);
    }

    private bool ParagraphContext =>
        _table is null && _preBlock is null && !InListItem && !_inDt && !_inDd;

    // The StringBuilder that AppendInline currently targets (null for table cells / <pre>,
    // whose buffers aren't directly editable here).
    private StringBuilder? ActiveInlineBuffer =>
        _table is not null || _preBlock is not null ? null
        : InListItem ? CurrentItemBuffer
        : _inDt ? _dtText
        : _inDd ? _ddText
        : _textBuf;

    // ── flush helpers ─────────────────────────────────────────────────────────
    // Paragraphs are emitted ONLY from <p> elements — this mirrors html-to-markdown's
    // DocumentStructure, where the paragraph handler is the sole producer of Paragraph
    // nodes. Loose inline text and text directly inside <div>/<summary>/etc. is discarded.
    private void CloseParagraphContext()
    {
        if (_pDepth > 0) { EmitParagraph(); _pDepth = 0; }
        else DiscardParagraph();
    }

    private void EmitParagraph()
    {
        // The buffer is already space-collapsed and boundary-aware (AppendNormalized), so we only
        // convert the <br> sentinel to a newline and trim the ends — internal newlines are kept.
        string text = _textBuf.ToString().Replace('\x01', '\n').Trim();
        if (text.Length > 0)
        {
            var anns = new List<TextAnnotation>(_annotations);
            _budget.AccountText(Encoding.UTF8.GetByteCount(text));
            _b.PushParagraph(text, anns, null, null);
        }
        DiscardParagraph();
    }

    /// <summary>
    /// Loose text seen outside any `<p>`, kept only in case the document turns out to have no
    /// structure at all. Emitting it eagerly is measurably wrong — this walker buffers text in
    /// places upstream does not, and flushing every block boundary costs far more fixtures than
    /// it fixes — but a document that produces *nothing* has clearly lost everything, and the
    /// corpus has several: plain text under an .html name, and markdown whose only wrapper is a
    /// raw `<div>`.
    /// </summary>
    private readonly StringBuilder _discarded = new();

    private void DiscardParagraph()
    {
        string text = _textBuf.ToString().Replace('\x01', '\n').Trim();
        if (text.Length != 0)
        {
            if (_discarded.Length != 0) _discarded.Append('\n');
            _discarded.Append(text);
        }

        ClearTextBuf();
        _annotations.Clear();
        _inlineStack.Clear();
    }

    // ── list tree (buffered, then emitted on outermost </ul>/</ol>) ────────────
    private void OpenList(bool ordered, int start)
    {
        var lst = new LList { Ordered = ordered, Start = start };
        if (_itemStack.Count > 0) _itemStack[^1].Nested.Add(lst);
        _listStack.Add(lst);
    }

    private void OpenItem()
    {
        if (_listStack.Count == 0) return; // stray <li>
        var lst = _listStack[^1];
        if (lst.HasOpenItem) CloseItem();  // auto-close a previous sibling <li>
        lst.HasOpenItem = true;
        _itemStack.Add(new LItem());
        _lastWasSpace = true;   // fresh word-break state for the new item's text

    }

    private void CloseItem()
    {
        if (_itemStack.Count == 0 || _listStack.Count == 0) return;
        var item = _itemStack[^1];
        _itemStack.RemoveAt(_itemStack.Count - 1);
        var lst = _listStack[^1];
        lst.Items.Add(item);
        lst.HasOpenItem = false;
    }

    private void CloseList()
    {
        if (_listStack.Count == 0) return;
        if (_listStack[^1].HasOpenItem) CloseItem();
        var closed = _listStack[^1];
        _listStack.RemoveAt(_listStack.Count - 1);
        if (_listStack.Count == 0) EmitListNode(closed, 1, 0); // outermost list → emit whole subtree
    }

    // Emit each list as its own node (all siblings under the current section) in the same
    // pre-order html-to-markdown uses: the list itself, then the nested lists of its items.
    private void EmitListNode(LList lst, int nl, int parentUd)
    {
        int ud = parentUd + (lst.Ordered ? 0 : 1);
        _b.PushList(lst.Ordered);
        foreach (var item in lst.Items)
        {
            string text = ItemText(item, nl, ud);
            if (text.Length > 0)
            {
                _budget.AccountText(Encoding.UTF8.GetByteCount(text));
                _b.PushListItem(text, lst.Ordered, new(), null, null);
            }
        }
        _b.EndList();
        foreach (var item in lst.Items)
            foreach (var nested in item.Nested)
                EmitListNode(nested, nl + 1, ud);
    }

    // The structure text for a single list item: its own inline text plus each nested list
    // rendered as flattened markdown (excluding the item's own marker), then trimmed.
    private string ItemText(LItem item, int nl, int ud)
    {
        var sb = new StringBuilder(FinalizeInline(item.Inline.ToString()));
        foreach (var nested in item.Nested)
        {
            sb.Append('\n');
            sb.Append(RenderList(nested, nl + 1, ud));
        }
        return sb.ToString().Trim();
    }

    // Render a (nested) list to flattened markdown lines: `indent + marker + content`,
    // bullets cycling "-*+" by <ul> depth, indent = 2 spaces per nesting level.
    private string RenderList(LList lst, int nl, int parentUd)
    {
        int ud = parentUd + (lst.Ordered ? 0 : 1);
        string indent = new string(' ', 2 * (nl - 1));
        var lines = new List<string>();
        int counter = lst.Start;
        foreach (var item in lst.Items)
        {
            string marker = lst.Ordered ? $"{counter}. " : Bullet(ud) + " ";
            string inline = FinalizeInline(item.Inline.ToString());
            var child = new StringBuilder();
            foreach (var nested in item.Nested)
            {
                child.Append('\n');
                child.Append(RenderList(nested, nl + 1, ud));
            }
            string childStr = child.ToString();
            string body = inline.Length == 0 && childStr.Length > 0
                ? marker.TrimEnd() + childStr
                : marker + inline + childStr;
            lines.Add(indent + body);
            if (lst.Ordered) counter++;
        }
        return string.Join("\n", lines);
    }

    // A URI has a scheme when it starts with an ASCII letter followed by [A-Za-z0-9+-.]* then ':'.
    private static bool HasUriScheme(string href)
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

    private static char Bullet(int ulDepth)
    {
        const string bullets = "-*+";
        int idx = ulDepth <= 0 ? 0 : (ulDepth - 1) % bullets.Length;
        return bullets[idx];
    }

    private void FlushDefinitionItem()
    {
        if (_inDd)
        {
            _inDd = false;
            if (_inDl)
            {
                string definition = NormalizeWhitespace(_ddText.ToString());
                if (_dlTerm is not null)
                {
                    _b.PushDefinitionTerm(_dlTerm, null);
                    _b.PushDefinitionDescription(definition, null);
                    _dlTerm = null;
                }
            }
            _ddText.Clear();
        }
        if (_inDt)
        {
            _inDt = false;
            if (_inDl) { string term = NormalizeWhitespace(_dtText.ToString()); if (term.Length > 0) _dlTerm = term; }
            _dtText.Clear();
        }
    }

    private static int Utf8Len(StringBuilder sb) => Encoding.UTF8.GetByteCount(sb.ToString());
    private static uint ParseU32(string? s, uint fallback) => uint.TryParse(s, out var v) ? v : fallback;

    // ── preprocessing / boilerplate removal ──────────────────────────────────
    // Mirrors html-to-markdown's should_drop_for_preprocessing (Standard preset,
    // remove_navigation + remove_forms enabled).
    private static readonly string[] NavKeywords =
    {
        "nav", "navigation", "navbar", "breadcrumbs", "breadcrumb", "toc", "sidebar",
        "sidenav", "menu", "menubar", "mainmenu", "subnav", "tabs", "tablist", "toolbar",
        "pager", "pagination", "skipnav", "skip-link", "skiplinks", "site-nav", "site-menu",
        "site-header", "site-footer", "topbar", "bottombar", "masthead", "vector-nav",
        "vector-header", "vector-footer",
    };

    private static bool ShouldDropForPreprocessing(string tag, string attrs)
    {
        if (tag is "nav" or "form") return true;
        if (tag is "header" or "footer" or "aside") return HasNavigationHint(attrs);
        return false;
    }

    private static bool HasNavigationHint(string attrs)
    {
        if (AttrMatchesAny(attrs, "role", new[] { "navigation", "menubar", "tablist", "toolbar" }))
            return true;
        if (AttrContainsAny(attrs, "aria-label", new[] { "navigation", "menu", "contents", "table of contents", "toc" }))
            return true;
        return AttrMatchesAny(attrs, "class", NavKeywords) || AttrMatchesAny(attrs, "id", NavKeywords);
    }

    // Token-aware match: split on whitespace, map _:./ → -, lowercase, exact-equal a keyword.
    private static bool AttrMatchesAny(string attrs, string attr, string[] keywords)
    {
        string? value = ExtractAttr(attrs, attr);
        if (value is null) return false;
        foreach (var token in value.Split(new[] { ' ', '\t', '\n', '\r', '\f' }, StringSplitOptions.RemoveEmptyEntries))
        {
            var sb = new StringBuilder(token.Length);
            foreach (char c in token) sb.Append(c is '_' or ':' or '.' or '/' ? '-' : char.ToLowerInvariant(c));
            string norm = sb.ToString();
            if (norm.Length > 0 && Array.IndexOf(keywords, norm) >= 0) return true;
        }
        return false;
    }

    private static bool AttrContainsAny(string attrs, string attr, string[] keywords)
    {
        string? value = ExtractAttr(attrs, attr);
        if (value is null) return false;
        string lower = value.ToLowerInvariant();
        foreach (var kw in keywords) if (lower.Contains(kw, StringComparison.Ordinal)) return true;
        return false;
    }

    // Advance _pos past the closing tag of a raw-text element (script/style/…). Tolerates
    // whitespace inside the close tag (e.g. `</style\n>`) and matches case-insensitively.
    private void SkipRawElement(string tag)
    {
        string closeStart = "</" + tag;
        int i = _pos;
        while (true)
        {
            int c = _src.IndexOf(closeStart, i, StringComparison.OrdinalIgnoreCase);
            if (c < 0) { _pos = _src.Length; return; }
            int k = c + closeStart.Length;
            while (k < _src.Length && char.IsWhiteSpace(_src[k])) k++;
            if (k < _src.Length && _src[k] == '>') { _pos = k + 1; return; }
            i = c + 1;
        }
    }

    // Skip the entire subtree of an opening `tag` (already consumed) up to its matching close,
    // tracking same-tag nesting. Comments/PIs and self-closing same-tag elements are handled.
    private void SkipSubtree(string tag)
    {
        int depth = 1;
        while (_pos < _src.Length && depth > 0)
        {
            int lt = _src.IndexOf('<', _pos);
            if (lt < 0) { _pos = _src.Length; return; }
            _pos = lt;
            if (Starts("<!--"))
            {
                _pos = CommentEnd(_src, _pos);
                continue;
            }
            int gt = _src.IndexOf('>', _pos);
            if (gt < 0) { _pos = _src.Length; return; }
            string raw = _src[(_pos + 1)..gt];
            _pos = gt + 1;
            if (raw.StartsWith('!') || raw.StartsWith('?')) continue;
            bool closing = raw.StartsWith('/');
            string c = closing ? raw[1..] : raw;
            bool selfClose = c.TrimEnd().EndsWith('/');
            c = c.TrimEnd('/').Trim();
            var (nm, _) = SplitTagName(c);
            if (!nm.Equals(tag, StringComparison.OrdinalIgnoreCase)) continue;
            if (closing) depth--;
            else if (!selfClose) depth++;
        }
    }

    // ── static utilities (ported from structure.rs) ──────────────────────────
    internal static (string name, string attrs) SplitTagName(string content)
    {
        content = content.Trim();
        int sp = content.IndexOfAny(new[] { ' ', '\t', '\n', '\r', '\f' });
        return sp < 0 ? (content, "") : (content[..sp], content[(sp + 1)..]);
    }

    // Quote-aware attribute lookup: tokenizes the attribute string so a `name=` sequence inside
    // a quoted value (e.g. `?title=` in an href query string) is never mistaken for the attribute.
    // Returns null when absent, "" when present with an empty/omitted value.
    /// <summary>
    /// The element's attributes in source order, each as its name and its value (null when the
    /// attribute was written with no value at all).
    /// </summary>
    /// <summary>
    /// Drop the leading characters an attribute name cannot begin with.
    /// </summary>
    /// <remarks>
    /// The parser upstream discards them rather than keeping them in the name, which is what a
    /// stray character in a tag comes down to: <c>&lt;a href="…" \&gt;</c> has no third
    /// attribute at all, <c>&lt;a href="…"&lt;u&gt;</c> — a tag left unterminated, running the
    /// next element's bracket into its own attribute list — carries one named <c>u</c>, and
    /// <c>@click</c> records as <c>click</c>. A name may still start with <c>_</c>, <c>:</c>,
    /// <c>-</c> or a digit, all of which real documents use.
    /// </remarks>
    internal static string TrimAttributeNameStart(string key)
    {
        int i = 0;
        while (i < key.Length && !IsAttributeNameStart(key[i])) i++;
        return i == 0 ? key : key[i..];
    }

    private static bool IsAttributeNameStart(char c) =>
        (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9')
        || c == '_' || c == ':' || c == '-';

    internal static IEnumerable<(string Key, string? Value)> EnumerateAttributes(string attrs)
    {
        int i = 0, n = attrs.Length;
        while (i < n)
        {
            while (i < n && char.IsWhiteSpace(attrs[i])) i++;
            if (i >= n) break;
            int ks = i;
            while (i < n && attrs[i] != '=' && !char.IsWhiteSpace(attrs[i]) && attrs[i] != '>' && attrs[i] != '/') i++;
            string key = TrimAttributeNameStart(attrs[ks..i]);
            // Nothing that can start a name here — a stray `=` or `/` between attributes. Step
            // over that one character and look again; the `=` does not adopt what follows it as
            // its value, so `<a =` + `href=…` still yields the href.
            if (key.Length == 0) { i++; continue; }
            while (i < n && char.IsWhiteSpace(attrs[i])) i++;
            string? value = null;
            if (i < n && attrs[i] == '=')
            {
                i++;
                while (i < n && char.IsWhiteSpace(attrs[i])) i++;
                if (i < n && (attrs[i] == '"' || attrs[i] == '\''))
                {
                    char q = attrs[i++];
                    int vs = i;
                    while (i < n && attrs[i] != q) i++;
                    value = attrs[vs..i];
                    if (i < n) i++;
                }
                else
                {
                    int vs = i;
                    while (i < n && !char.IsWhiteSpace(attrs[i]) && attrs[i] != '>') i++;
                    value = attrs[vs..i];
                }
            }
            yield return (key, value);
        }
    }

    internal static string? ExtractAttr(string attrs, string name)
    {
        foreach (var (key, value) in EnumerateAttributes(attrs))
            if (key.Equals(name, StringComparison.OrdinalIgnoreCase)) return value ?? "";
        return null;
    }

    /// <summary>
    /// Attribute lookup by exact name, for the markdown converter — the crate it ports matches
    /// attribute names case-sensitively, so an uppercase <c>HREF</c> or <c>SRC</c> is simply not
    /// there as far as its handlers are concerned.
    /// </summary>
    internal static string? ExtractAttrExact(string attrs, string name)
    {
        foreach (var (key, value) in EnumerateAttributes(attrs))
            if (key.Equals(name, StringComparison.Ordinal)) return value ?? "";
        return null;
    }

    private static string? ExtractLanguageFromClass(string? cls)
    {
        if (cls is null) return null;
        foreach (var c in cls.Split(new[] { ' ', '\t', '\n', '\r', '\f' }, StringSplitOptions.RemoveEmptyEntries))
        {
            if (c.StartsWith("language-", StringComparison.Ordinal)) return c["language-".Length..];
            if (c.StartsWith("lang-", StringComparison.Ordinal)) return c["lang-".Length..];
        }
        return null;
    }

    /// <summary>
    /// Resolve every named character reference in the WHATWG table, plus numeric references.
    /// </summary>
    /// <remarks>
    /// This is the decoder the HTML-to-markdown converter needs: upstream runs that path through
    /// a real HTML5 parser, so a page writing <c>&amp;deg;</c> or <c>&amp;oacute;</c> arrives with
    /// the character already resolved. <see cref="DecodeEntities"/> stays deliberately small
    /// because the structure walker it serves ports a Rust function that knows only a few dozen
    /// names, and widening it there would diverge from the reference the other way.
    /// </remarks>
    internal static string DecodeEntitiesFull(string s) => DecodeEntitiesFull(s, false);

    /// <param name="html5NumericTable">
    /// Resolve a numeric reference the way the HTML5 tokenizer does, through the replacement
    /// table of §13.2.5.80. Only a document that reaches the walk through the html5ever repair
    /// gets this: the converter's own fast path decodes <c>&amp;#146;</c> to U+0092 and leaves it
    /// there, where the repair resolves it to U+2019.
    /// </param>
    internal static string DecodeEntitiesFull(string s, bool html5NumericTable)
    {
        if (!s.Contains('&')) return s;
        var outp = new StringBuilder(s.Length);
        int i = 0;
        while (i < s.Length)
        {
            char c = s[i++];
            if (c != '&') { outp.Append(c); continue; }

            int semi = s.IndexOf(';', i);
            if (semi < 0 || semi - i > HtmlEntityTable.MaxNameLength) { outp.Append('&'); continue; }
            string name = s[i..semi];
            if (name.Length == 0) { outp.Append('&'); continue; }

            if (HtmlEntityTable.Named.TryGetValue(name, out var mapped))
            {
                outp.Append(mapped);
                i = semi + 1;
                continue;
            }

            if (name[0] == '#' && DecodeNumericReference(name, html5NumericTable) is { } numeric)
            {
                outp.Append(numeric);
                i = semi + 1;
                continue;
            }

            outp.Append('&');
        }
        return outp.ToString();
    }

    /// <summary>
    /// The HTML5 tokenizer's numeric character reference replacement table (§13.2.5.80). A
    /// reference in the C1 range names a Windows-1252 character rather than the control it
    /// nominally points at, because that is what the pages doing it meant.
    /// </summary>
    private static readonly Dictionary<int, int> Html5NumericReplacements = new()
    {
        [0x00] = 0xFFFD, [0x80] = 0x20AC, [0x82] = 0x201A, [0x83] = 0x0192, [0x84] = 0x201E,
        [0x85] = 0x2026, [0x86] = 0x2020, [0x87] = 0x2021, [0x88] = 0x02C6, [0x89] = 0x2030,
        [0x8A] = 0x0160, [0x8B] = 0x2039, [0x8C] = 0x0152, [0x8E] = 0x017D, [0x91] = 0x2018,
        [0x92] = 0x2019, [0x93] = 0x201C, [0x94] = 0x201D, [0x95] = 0x2022, [0x96] = 0x2013,
        [0x97] = 0x2014, [0x98] = 0x02DC, [0x99] = 0x2122, [0x9A] = 0x0161, [0x9B] = 0x203A,
        [0x9C] = 0x0153, [0x9E] = 0x017E, [0x9F] = 0x0178,
    };

    private static string? DecodeNumericReference(string name, bool html5NumericTable = false)
    {
        string num = name[1..];
        int cp;
        if (num.StartsWith('x') || num.StartsWith('X'))
        {
            if (!int.TryParse(num[1..], System.Globalization.NumberStyles.HexNumber, null, out cp)) return null;
        }
        else if (!int.TryParse(num, out cp)) return null;

        if (cp < 0 || cp > 0x10FFFF) return null;
        if (html5NumericTable && Html5NumericReplacements.TryGetValue(cp, out int replacement))
            cp = replacement;
        try { return char.ConvertFromUtf32(cp); }
        catch (Exception) when (Cancel.NotCancelling) { return null; }
    }

    internal static string DecodeEntities(string s)
    {
        if (!s.Contains('&')) return s;
        var outp = new StringBuilder(s.Length);
        int i = 0;
        while (i < s.Length)
        {
            char c = s[i++];
            if (c != '&') { outp.Append(c); continue; }
            var entity = new StringBuilder();
            bool tooLong = false;
            bool terminated = false;
            while (i < s.Length)
            {
                char ec = s[i++];
                if (ec == ';') { terminated = true; break; }
                entity.Append(ec);
                if (entity.Length > 10) { outp.Append('&'); outp.Append(entity); tooLong = true; break; }
            }
            if (tooLong) continue;
            if (entity.Length == 0) continue;
            string e = entity.ToString();
            string? mapped = e switch
            {
                "amp" => "&", "lt" => "<", "gt" => ">", "quot" => "\"", "apos" => "'",
                "nbsp" => " ", "copy" => "©", "reg" => "®", "trade" => "™",
                "mdash" => "—", "ndash" => "–", "laquo" => "«", "raquo" => "»",
                "hellip" => "…", "eacute" => "é", "egrave" => "è", "ecirc" => "ê",
                "euml" => "ë", "aacute" => "á", "agrave" => "à", "acirc" => "â",
                "auml" => "ä", "iacute" => "í", "ocirc" => "ô", "ouml" => "ö",
                "uuml" => "ü", "ntilde" => "ñ", "ccedil" => "ç", "ldquo" => "“",
                "rdquo" => "”", "lsquo" => "‘", "rsquo" => "’", "bull" => "•",
                "middot" => "·", "euro" => "€", "pound" => "£", "yen" => "¥",
                "times" => "×", "divide" => "÷", "plusmn" => "±",
                _ => null,
            };
            if (mapped is not null) { outp.Append(mapped); continue; }
            if (e.StartsWith('#'))
            {
                string num = e[1..];
                int? cp = null;
                if (num.StartsWith('x') || num.StartsWith('X'))
                {
                    if (int.TryParse(num[1..], System.Globalization.NumberStyles.HexNumber, null, out var h)) cp = h;
                }
                else if (int.TryParse(num, out var d)) cp = d;
                if (cp is not null && cp >= 0 && cp <= 0x10FFFF)
                {
                    try { outp.Append(char.ConvertFromUtf32(cp.Value)); continue; } catch (Exception) when (Cancel.NotCancelling) { }
                }
            }
            // Unknown entity — preserve raw. Only re-emit the trailing ';' when the source
            // actually terminated the entity with one (matches the `tl` HTML parser used by
            // the golden; a bare '&' followed by non-entity text keeps no spurious ';').
            outp.Append('&'); outp.Append(e); if (terminated) outp.Append(';');
        }
        return outp.ToString();
    }

    // Collapse ALL runs of whitespace (including newlines) to a single space, trimming ends.
    // Used for list items, table cells, headings and definitions, where the raw buffered text
    // is not boundary-aware. Paragraph text preserves internal newlines via FinalizeParagraph.
    internal static string NormalizeWhitespace(string s)
    {
        var outp = new StringBuilder(s.Length);
        bool lastWasSpace = true;
        foreach (char c in s)
        {
            if (c == '\x01')
            {
                while (outp.Length > 0 && outp[^1] == ' ') outp.Remove(outp.Length - 1, 1);
                outp.Append('\n');
                lastWasSpace = true;
            }
            else if (c is ' ' or '\t' or '\n' or '\r' or '\f' or '\v')
            {
                if (!lastWasSpace) { outp.Append(' '); lastWasSpace = true; }
            }
            else { outp.Append(c); lastWasSpace = false; }
        }
        if (outp.Length > 0 && outp[^1] == ' ') outp.Remove(outp.Length - 1, 1);
        return outp.ToString();
    }

    // ── nested state types ─────────────────────────────────────────────────
    private enum InlineKind { Bold, Italic, Code, Underline, Strikethrough, Link, Subscript, Superscript, Highlight }

    private struct InlineSpan { public InlineKind Kind; public int TextStart; public string? Href; public string? Title; public bool Emitted; }

    private sealed class PreBlock { public string? Language; public readonly StringBuilder Text = new(); }

    private sealed class LList
    {
        public bool Ordered;
        public int Start = 1;
        public bool HasOpenItem;
        public readonly List<LItem> Items = new();
    }

    private sealed class LItem
    {
        public readonly StringBuilder Inline = new();
        public readonly List<LList> Nested = new();
    }

    internal sealed class CellMeta { public string Text = ""; public uint ColSpan = 1; public uint RowSpan = 1; public bool IsHeader; }


    private sealed class TableAccumulator
    {
        public readonly List<List<CellMeta>> Rows = new();
        private List<CellMeta> _currentRow = new();
        private readonly StringBuilder _currentCell = new();
        private uint _colSpan = 1, _rowSpan = 1;
        private bool _isHeader, _inRow, _inCell;

        public void OpenRow() { _currentRow = new(); _inRow = true; }
        public void CloseRow() { if (_inRow) { Rows.Add(_currentRow); _inRow = false; } }
        public void OpenCell(uint colSpan, uint rowSpan, bool isHeader)
        {
            _currentCell.Clear(); _colSpan = colSpan; _rowSpan = rowSpan; _isHeader = isHeader; _inCell = true;
        }
        public void CloseCell()
        {
            if (!_inCell) return;
            _currentRow.Add(new CellMeta { Text = _currentCell.ToString(), ColSpan = _colSpan, RowSpan = _rowSpan, IsHeader = _isHeader });
            _inCell = false; _colSpan = 1; _rowSpan = 1; _isHeader = false;
        }
        public void PushText(string t) { if (_inCell) _currentCell.Append(t); }
    }
}
