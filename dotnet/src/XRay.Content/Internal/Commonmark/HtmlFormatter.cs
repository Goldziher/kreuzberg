using System.Text;

namespace XRay.Content.Internal.Commonmark;

/// <summary>
/// Port of comrak's <c>format_html</c> (<c>comrak-0.53.0/src/html.rs</c> + <c>html/context.rs</c>)
/// specialised to xberg's HTML renderer options: <c>unsafe = true</c>, <c>github_pre_lang = true</c>,
/// <c>full_info_string = true</c>, everything else default. Output matches Rust byte-for-byte.
/// </summary>
internal sealed class HtmlFormatter
{
    private enum ChildRendering { Html, Plain, Skip }

    private readonly StringBuilder _output = new();
    private bool _lastWasLf = true;
    private uint _footnoteIx;
    private uint _writtenFootnoteIx;

    private HtmlFormatter() { }

    public static string Format(MdNode root)
    {
        var f = new HtmlFormatter();
        f.Run(root);
        return f._output.ToString();
    }

    // ---- context ---------------------------------------------------------

    private void WriteStr(string s)
    {
        int l = s.Length;
        if (l > 0) _lastWasLf = s[l - 1] == '\n';
        _output.Append(s);
    }

    private void Cr()
    {
        if (!_lastWasLf) WriteStr("\n");
    }

    private void Lf() => WriteStr("\n");

    private void Escape(string buffer)
    {
        foreach (char c in buffer)
        {
            switch (c)
            {
                case '"': _output.Append("&quot;"); _lastWasLf = false; break;
                case '&': _output.Append("&amp;"); _lastWasLf = false; break;
                case '<': _output.Append("&lt;"); _lastWasLf = false; break;
                case '>': _output.Append("&gt;"); _lastWasLf = false; break;
                case '\0': _output.Append('�'); _lastWasLf = false; break;
                default: _output.Append(c); _lastWasLf = c == '\n'; break;
            }
        }
    }

    private static readonly bool[] HrefSafe = BuildHrefSafe();

    private static bool[] BuildHrefSafe()
    {
        var a = new bool[256];
        foreach (byte b in Encoding.ASCII.GetBytes("-_.+!*(),#@?=;:/,+$~")) a[b] = true;
        for (char c = 'a'; c <= 'z'; c++) a[c] = true;
        for (char c = 'A'; c <= 'Z'; c++) a[c] = true;
        for (char c = '0'; c <= '9'; c++) a[c] = true;
        return a;
    }

    private void EscapeHref(string buffer)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(buffer);
        int size = bytes.Length;
        int i = 0;
        // ipv6_url_start scanning skipped (relaxed_autolinks = false, rare case).
        while (i < size)
        {
            int org = i;
            while (i < size && HrefSafe[bytes[i]]) i++;
            if (i > org) _output.Append(Encoding.UTF8.GetString(bytes, org, i - org));
            if (i >= size) break;

            byte b = bytes[i];
            if (b == (byte)'&') _output.Append("&amp;");
            else if (b == (byte)'\'') _output.Append("&#x27;");
            else if (b == (byte)'%')
            {
                if (i + 2 < size && IsHex(bytes[i + 1]) && IsHex(bytes[i + 2]))
                {
                    _output.Append(Encoding.UTF8.GetString(bytes, i, 3));
                    i += 2;
                }
                else _output.Append("%25");
            }
            else if (b == 0) _output.Append("%EF%BF%BD");
            else _output.Append('%').Append(b.ToString("X2"));
            i++;
        }
        _lastWasLf = false;
    }

    private static bool IsHex(byte b) =>
        (b >= (byte)'0' && b <= (byte)'9') || (b >= (byte)'a' && b <= (byte)'f') || (b >= (byte)'A' && b <= (byte)'F');

    private void Finish()
    {
        if (_footnoteIx > 0)
        {
            WriteStr("</ol>");
            Lf();
            WriteStr("</section>");
            Lf();
        }
    }

    // ---- driver ----------------------------------------------------------

    private void Run(MdNode root)
    {
        var stack = new Stack<(MdNode node, ChildRendering cr, bool post)>();
        stack.Push((root, ChildRendering.Html, false));

        int visited = 0;
        while (stack.Count > 0)
        {
            Cancel.Every(visited++, 0x3FF);
            var (node, childRendering, post) = stack.Pop();
            if (!post)
            {
                ChildRendering newCr;
                if (childRendering == ChildRendering.Plain)
                {
                    switch (node.Type)
                    {
                        case NodeType.Text: Escape(node.Literal); break;
                        case NodeType.Code: Escape(node.Code.Literal); break;
                        case NodeType.LineBreak:
                        case NodeType.SoftBreak: WriteStr(" "); break;
                        case NodeType.Math: Escape(node.Math.Literal); break;
                    }
                    newCr = ChildRendering.Plain;
                }
                else
                {
                    stack.Push((node, ChildRendering.Html, true));
                    newCr = FormatNode(node, true);
                }

                if (newCr != ChildRendering.Skip)
                    foreach (var ch in node.ReverseChildren())
                        stack.Push((ch, newCr, false));
            }
            else
            {
                FormatNode(node, false);
            }
        }

        Finish();
    }

    private ChildRendering FormatNode(MdNode node, bool entering)
    {
        switch (node.Type)
        {
            case NodeType.Document: return ChildRendering.Html;
            case NodeType.BlockQuote: return RenderBlockQuote(entering);
            case NodeType.Code: return RenderCode(node, entering);
            case NodeType.CodeBlock: return RenderCodeBlock(node, entering);
            case NodeType.Emph: return RenderTag(entering, "<em>", "</em>");
            case NodeType.Heading: return RenderHeading(node, entering);
            case NodeType.Image: return RenderImage(node, entering);
            case NodeType.Item: return RenderItem(entering);
            case NodeType.LineBreak: return RenderLineBreak(entering);
            case NodeType.Link: return RenderLink(node, entering);
            case NodeType.List: return RenderList(node, entering);
            case NodeType.Paragraph: return RenderParagraph(node, entering);
            case NodeType.SoftBreak: return RenderSoftBreak(entering);
            case NodeType.Strong: return RenderStrong(node, entering);
            case NodeType.Text: if (entering) Escape(node.Literal); return ChildRendering.Html;
            case NodeType.ThematicBreak: return RenderThematicBreak(entering);
            case NodeType.FootnoteDefinition: return RenderFootnoteDefinition(node, entering);
            case NodeType.FootnoteReference: return RenderFootnoteReference(node, entering);
            case NodeType.Strikethrough: return RenderTag(entering, "<del>", "</del>");
            case NodeType.Highlight: return RenderTag(entering, "<mark>", "</mark>");
            case NodeType.Table: return RenderTable(node, entering);
            case NodeType.TableCell: return RenderTableCell(node, entering);
            case NodeType.TableRow: return RenderTableRow(node, entering);
            case NodeType.TaskItem: return RenderTaskItem(node, entering);
            case NodeType.Alert: return RenderAlert(node, entering);
            case NodeType.Math: return RenderMath(node, entering);
            case NodeType.Raw: if (entering) WriteStr(node.Literal); return ChildRendering.Html;
            case NodeType.Subscript: return RenderTag(entering, "<sub>", "</sub>");
            case NodeType.Superscript: return RenderTag(entering, "<sup>", "</sup>");
            case NodeType.Underline: return RenderTag(entering, "<u>", "</u>");
            default: return ChildRendering.Html;
        }
    }

    private ChildRendering RenderTag(bool entering, string open, string close)
    {
        WriteStr(entering ? open : close);
        return ChildRendering.Html;
    }

    private ChildRendering RenderBlockQuote(bool entering)
    {
        if (entering)
        {
            Cr(); WriteStr("<blockquote>"); Lf();
        }
        else
        {
            Cr(); WriteStr("</blockquote>"); Lf();
        }
        return ChildRendering.Html;
    }

    private ChildRendering RenderCode(MdNode node, bool entering)
    {
        if (entering)
        {
            WriteStr("<code>");
            Escape(node.Code.Literal);
            WriteStr("</code>");
        }
        return ChildRendering.Html;
    }

    private ChildRendering RenderCodeBlock(MdNode node, bool entering)
    {
        if (!entering) return ChildRendering.Html;

        var ncb = node.CodeBlock;
        string info = ncb.Info;
        byte[] infoBytes = Encoding.UTF8.GetBytes(info);
        int firstTag = 0;
        while (firstTag < infoBytes.Length && !Ctype.IsSpace(infoBytes[firstTag])) firstTag++;
        string lang = Encoding.UTF8.GetString(infoBytes, 0, firstTag);
        string meta = firstTag < infoBytes.Length
            ? Encoding.UTF8.GetString(infoBytes, firstTag, infoBytes.Length - firstTag).Trim()
            : "";

        if (lang == "math")
        {
            RenderMathCodeBlock(ncb.Literal);
            return ChildRendering.Html;
        }

        Cr();

        // github_pre_lang = true
        string preAttrs = "";
        if (info.Length > 0)
        {
            preAttrs = " lang=\"" + EscapeAttr(lang) + "\"";
            if (meta.Length > 0) // full_info_string = true
                preAttrs += " data-meta=\"" + EscapeAttr(meta.Trim()) + "\"";
        }

        WriteStr("<pre" + preAttrs + ">");
        WriteStr("<code>");
        Escape(ncb.Literal);
        WriteStr("</code></pre>");
        Lf();
        return ChildRendering.Html;
    }

    private void RenderMathCodeBlock(string literal)
    {
        Cr();
        WriteStr("<pre lang=\"math\" data-math-style=\"display\">");
        WriteStr("<code>");
        Escape(literal);
        WriteStr("</code></pre>");
        Lf();
    }

    private ChildRendering RenderHeading(MdNode node, bool entering)
    {
        int level = node.Heading.Level;
        if (entering)
        {
            Cr();
            WriteStr("<h" + level + ">");
        }
        else
        {
            WriteStr("</h" + level + ">");
            Lf();
        }
        return ChildRendering.Html;
    }

    private ChildRendering RenderImage(MdNode node, bool entering)
    {
        var nl = node.Link;
        if (entering)
        {
            WriteStr("<img src=\"");
            EscapeHref(nl.Url); // unsafe = true, so always emit
            WriteStr("\" alt=\"");
            return ChildRendering.Plain;
        }
        else
        {
            if (!string.IsNullOrEmpty(nl.Title))
            {
                WriteStr("\" title=\"");
                Escape(nl.Title);
            }
            WriteStr("\" />");
        }
        return ChildRendering.Html;
    }

    private ChildRendering RenderItem(bool entering)
    {
        if (entering)
        {
            Cr(); WriteStr("<li>");
        }
        else
        {
            WriteStr("</li>"); Lf();
        }
        return ChildRendering.Html;
    }

    private ChildRendering RenderLineBreak(bool entering)
    {
        if (entering)
        {
            WriteStr("<br />"); Lf();
        }
        return ChildRendering.Html;
    }

    private ChildRendering RenderLink(MdNode node, bool entering)
    {
        var nl = node.Link;
        if (entering)
        {
            WriteStr("<a href=\"");
            EscapeHref(nl.Url);
            if (!string.IsNullOrEmpty(nl.Title))
            {
                WriteStr("\" title=\"");
                Escape(nl.Title);
            }
            WriteStr("\">");
        }
        else
        {
            WriteStr("</a>");
        }
        return ChildRendering.Html;
    }

    private ChildRendering RenderList(MdNode node, bool entering)
    {
        var nl = node.List;
        if (entering)
        {
            Cr();
            if (nl.ListType == ListType.Bullet)
            {
                WriteStr("<ul>"); Lf();
            }
            else
            {
                if (nl.Start == 1) { WriteStr("<ol>"); Lf(); }
                else { WriteStr("<ol start=\"" + nl.Start + "\">"); Lf(); }
            }
        }
        else if (nl.ListType == ListType.Bullet)
        {
            WriteStr("</ul>"); Lf();
        }
        else
        {
            WriteStr("</ol>"); Lf();
        }
        return ChildRendering.Html;
    }

    private ChildRendering RenderParagraph(MdNode node, bool entering)
    {
        bool tight =
            (node.Parent?.Parent is { } gp && ((gp.Type == NodeType.List && gp.List.Tight)))
            || (node.Parent is { Type: NodeType.List } /* description fallbacks omitted */ && false);

        if (!tight)
        {
            if (entering)
            {
                Cr(); WriteStr("<p>");
            }
            else
            {
                if (node.Parent is { Type: NodeType.FootnoteDefinition } parent && node.Next is null)
                {
                    WriteStr(" ");
                    PutFootnoteBackref(parent.FootnoteDefinition);
                }
                WriteStr("</p>"); Lf();
            }
        }
        return ChildRendering.Html;
    }

    private ChildRendering RenderSoftBreak(bool entering)
    {
        if (entering) WriteStr("\n"); // hardbreaks = false
        return ChildRendering.Html;
    }

    private ChildRendering RenderStrong(MdNode node, bool entering)
    {
        // gfm_quirks = false, so always render.
        WriteStr(entering ? "<strong>" : "</strong>");
        return ChildRendering.Html;
    }

    private ChildRendering RenderThematicBreak(bool entering)
    {
        if (entering)
        {
            Cr(); WriteStr("<hr />"); Lf();
        }
        return ChildRendering.Html;
    }

    private ChildRendering RenderFootnoteDefinition(MdNode node, bool entering)
    {
        var nfd = node.FootnoteDefinition;
        if (entering)
        {
            if (_footnoteIx == 0)
            {
                WriteStr("<section class=\"footnotes\" data-footnotes>"); Lf();
                WriteStr("<ol>"); Lf();
            }
            _footnoteIx += 1;
            WriteStr("<li id=\"fn-");
            EscapeHref(nfd.Name);
            WriteStr("\">");
        }
        else
        {
            if (PutFootnoteBackref(nfd)) Lf();
            WriteStr("</li>"); Lf();
        }
        return ChildRendering.Html;
    }

    private ChildRendering RenderFootnoteReference(MdNode node, bool entering)
    {
        if (entering)
        {
            var nfr = node.FootnoteReference;
            string refId = "fnref-" + nfr.Name;
            if (nfr.RefNum > 1) refId = refId + "-" + nfr.RefNum;

            WriteStr("<sup class=\"footnote-ref\"><a href=\"#fn-");
            EscapeHref(nfr.Name);
            WriteStr("\" id=\"");
            EscapeHref(refId);
            WriteStr("\" data-footnote-ref>" + nfr.Ix + "</a></sup>");
        }
        return ChildRendering.Html;
    }

    private ChildRendering RenderTable(MdNode node, bool entering)
    {
        if (entering)
        {
            Cr(); WriteStr("<table>"); Lf();
        }
        else
        {
            var last = node.LastChild;
            var first = node.FirstChild;
            if (last is not null && first is not null && !ReferenceEquals(last, first))
            {
                Cr(); WriteStr("</tbody>"); Lf();
            }
            Cr(); WriteStr("</table>"); Lf();
        }
        return ChildRendering.Html;
    }

    private ChildRendering RenderTableCell(MdNode node, bool entering)
    {
        var rowNode = node.Parent!;
        bool inHeader = rowNode.TableRowHeader;
        var alignments = rowNode.Parent!.Table.Alignments;

        if (entering)
        {
            Cr();
            WriteStr(inHeader ? "<th" : "<td");

            int i = 0;
            var start = rowNode.FirstChild!;
            while (!ReferenceEquals(start, node)) { i++; start = start.Next!; }

            switch (alignments[i])
            {
                case TableAlignment.Left: WriteStr(" align=\"left\""); break;
                case TableAlignment.Right: WriteStr(" align=\"right\""); break;
                case TableAlignment.Center: WriteStr(" align=\"center\""); break;
            }
            WriteStr(">");
        }
        else
        {
            WriteStr(inHeader ? "</th>" : "</td>");
        }
        return ChildRendering.Html;
    }

    private ChildRendering RenderTableRow(MdNode node, bool entering)
    {
        bool thead = node.TableRowHeader;
        if (entering)
        {
            Cr();
            if (thead)
            {
                WriteStr("<thead>"); Lf();
            }
            else if (node.Prev is { Type: NodeType.TableRow, TableRowHeader: true })
            {
                WriteStr("<tbody>"); Lf();
            }
            WriteStr("<tr>");
        }
        else
        {
            Cr(); WriteStr("</tr>");
            if (thead) { Cr(); WriteStr("</thead>"); }
        }
        return ChildRendering.Html;
    }

    private ChildRendering RenderTaskItem(MdNode node, bool entering)
    {
        bool writeLi = node.Parent is { Type: NodeType.List };
        if (entering)
        {
            Cr();
            if (writeLi) WriteStr("<li>");
            WriteStr("<input type=\"checkbox\"");
            if (node.TaskSymbol is not null) WriteStr(" checked=\"\"");
            WriteStr(" disabled=\"\" /> ");
        }
        else if (writeLi)
        {
            WriteStr("</li>"); Lf();
        }
        return ChildRendering.Html;
    }

    private ChildRendering RenderAlert(MdNode node, bool entering)
    {
        var alert = node.Alert;
        if (entering)
        {
            Cr();
            WriteStr("<div class=\"markdown-alert ");
            WriteStr(alert.AlertType.CssClass());
            WriteStr("\">");
            Lf();
            WriteStr("<p class=\"markdown-alert-title\">");
            if (alert.Title is not null) Escape(alert.Title);
            else WriteStr(alert.AlertType.DefaultTitle());
            WriteStr("</p>");
            Lf();
        }
        else
        {
            Cr(); WriteStr("</div>"); Lf();
        }
        return ChildRendering.Html;
    }

    private ChildRendering RenderMath(MdNode node, bool entering)
    {
        if (entering)
        {
            var nm = node.Math;
            string styleAttr = nm.DisplayMath ? "display" : "inline";
            string tag = nm.DollarMath ? "span" : "code";
            WriteStr("<" + tag + " data-math-style=\"" + EscapeAttr(styleAttr) + "\">");
            Escape(nm.Literal);
            WriteStr("</" + tag + ">");
        }
        return ChildRendering.Html;
    }

    private bool PutFootnoteBackref(NodeFootnoteDefinition nfd)
    {
        if (_writtenFootnoteIx >= _footnoteIx) return false;
        _writtenFootnoteIx = _footnoteIx;

        string refSuffix = "";
        string superscript = "";
        for (int refNum = 1; refNum <= nfd.TotalReferences; refNum++)
        {
            if (refNum > 1)
            {
                refSuffix = "-" + refNum;
                superscript = "<sup class=\"footnote-ref\">" + refNum + "</sup>";
                WriteStr(" ");
            }
            WriteStr("<a href=\"#fnref-");
            EscapeHref(nfd.Name);
            uint fnix = _footnoteIx;
            WriteStr(refSuffix + "\" class=\"footnote-backref\" data-footnote-backref data-footnote-backref-idx=\""
                + fnix + refSuffix + "\" aria-label=\"Back to reference " + fnix + refSuffix + "\">↩" + superscript + "</a>");
        }
        return true;
    }

    // Attribute value escaping mirrors `write_opening_tag` -> `escape`.
    private static string EscapeAttr(string s)
    {
        var sb = new StringBuilder(s.Length);
        foreach (char c in s)
        {
            switch (c)
            {
                case '"': sb.Append("&quot;"); break;
                case '&': sb.Append("&amp;"); break;
                case '<': sb.Append("&lt;"); break;
                case '>': sb.Append("&gt;"); break;
                case '\0': sb.Append('�'); break;
                default: sb.Append(c); break;
            }
        }
        return sb.ToString();
    }
}
