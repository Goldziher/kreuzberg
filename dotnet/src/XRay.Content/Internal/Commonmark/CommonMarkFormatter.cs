using System.Text;

namespace XRay.Content.Internal.Commonmark;

/// <summary>
/// Port of comrak's <c>format_commonmark</c> (<c>comrak-0.53.0/src/cm.rs</c>) specialised to the
/// <c>render.width == 0</c> configuration used by xberg's markdown renderer (no line wrapping,
/// <c>prefer_fenced = true</c>). All the wrapping / column-tracking machinery is elided because it
/// is dead code when <c>width == 0</c>. Everything else is a faithful line-for-line port so output
/// matches Rust byte-for-byte.
/// </summary>
internal sealed class CommonMarkFormatter
{
    private enum Escaping { Literal, Normal, Url, Title }

    private readonly StringBuilder _output = new();
    private readonly List<byte> _window = new(2);
    private readonly StringBuilder _prefix = new();
    private MdNode _node;
    private int _needCr;
    private bool _beginLine = true;
    private bool _beginContent = true;
    private bool _noLinebreaks;
    private bool _inTightListItem;
    private Func<MdNode, char, bool>? _customEscape;
    private uint _footnoteIx;
    private readonly List<int> _olStack = new();

    private CommonMarkFormatter(MdNode root) => _node = root;

    public static string Format(MdNode root)
    {
        var f = new CommonMarkFormatter(root);
        f.Run(root);
        return f._output.ToString();
    }

    // ---- low-level write -------------------------------------------------

    private void Write(string s)
    {
        if (s.Length == 0) return;
        byte[] bytes = Encoding.UTF8.GetBytes(s);
        if (bytes.Length == 0) return;

        if (bytes.Length > 1)
        {
            _window.Clear();
            _window.Add(bytes[bytes.Length - 2]);
            _window.Add(bytes[bytes.Length - 1]);
        }
        else
        {
            if (_window.Count == 2) _window.RemoveAt(0);
            _window.Add(bytes[0]);
        }

        bool lastWasCr = _window.Count > 0 && _window[^1] == (byte)'\n';
        _output.Append(s);

        if (lastWasCr)
        {
            _beginLine = true;
            _beginContent = true;
        }
    }

    private void WritePrefix()
    {
        if (_prefix.Length == 0) return;
        string p = _prefix.ToString();
        _output.Append(p);
        byte[] pb = Encoding.UTF8.GetBytes(p);
        _window.Clear();
        _window.Add(pb[pb.Length - 2]);
        _window.Add(pb[pb.Length - 1]);
    }

    private void Output(string s, Escaping escaping)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(s);

        if (_inTightListItem && _needCr > 1) _needCr = 1;

        int lastCrsConsume = 0;
        for (int k = _window.Count - 1; k >= 0 && _window[k] == (byte)'\n'; k--) lastCrsConsume++;

        while (_needCr > 0)
        {
            if (_window.Count == 0)
            {
                // nvm
            }
            else if (lastCrsConsume > 0)
            {
                lastCrsConsume--;
            }
            else
            {
                Write("\n");
                if (_needCr > 1) WritePrefix();
            }
            _needCr--;
        }

        int i = 0;
        while (i < bytes.Length)
        {
            int codepoint = DecodeUtf8(bytes, i, out int charLen);
            char? asChar = codepoint <= 0xFFFF ? (char)codepoint : null;

            if (_beginLine) WritePrefix();

            if (_customEscape is not null && codepoint <= 0xFFFF && _customEscape(_node, (char)codepoint))
                Write("\\");

            byte nextb = i + 1 < bytes.Length ? bytes[i + 1] : (byte)0;

            if (escaping == Escaping.Literal)
            {
                if (bytes[i] == (byte)'\n')
                {
                    Write("\n");
                }
                else
                {
                    Write(Utf8Slice(bytes, i, charLen));
                    _beginLine = false;
                    _beginContent = _beginContent && Ctype.IsDigit(bytes[i]);
                }
            }
            else
            {
                Outc(codepoint, escaping, nextb);
                _beginLine = false;
                _beginContent = _beginContent && Ctype.IsDigit(bytes[i]);
            }

            i += charLen;
        }
    }

    private void Outc(int c, Escaping escaping, byte nextb)
    {
        bool followsDigit = _window.Count > 0 && Ctype.IsDigit(_window[^1]);

        bool needsEscaping = c < 0x80 && escaping != Escaping.Literal &&
            ((escaping == Escaping.Normal &&
                (c < 0x20
                 || c == '*' || c == '_' || c == '[' || c == ']' || c == '#'
                 || c == '<' || c == '>' || c == '\\' || c == '`' || c == '~' || c == '!'
                 || (c == '&' && Ctype.IsAlpha(nextb))
                 || (c == '!' && nextb == 0x5b)
                 || (_beginContent && (c == '-' || c == '+' || c == '=') && !followsDigit)
                 || (_beginContent && (c == '.' || c == ')') && followsDigit && (nextb == 0 || Ctype.IsSpace(nextb)))))
             || (escaping == Escaping.Url &&
                (c == '`' || c == '<' || c == '>' || Ctype.IsSpaceChar(c) || c == '\\' || c == ')' || c == '('))
             || (escaping == Escaping.Title &&
                (c == '`' || c == '<' || c == '>' || c == '"' || c == '\\')));

        if (needsEscaping)
        {
            string outStr;
            if (escaping == Escaping.Url && Ctype.IsSpaceChar(c))
                outStr = "%" + c.ToString("X").PadLeft(2, ' ');
            else if (Ctype.IsPunctChar(c))
                outStr = "\\" + (char)c;
            else if (c == 0)
                outStr = "�";
            else
                outStr = "&#" + ((byte)c).ToString() + ";";
            Write(outStr);
        }
        else
        {
            Write(char.ConvertFromUtf32(c));
        }
    }

    private void Cr() => _needCr = Math.Max(_needCr, 1);
    private void Blankline() => _needCr = Math.Max(_needCr, 2);

    // Convenience: mirror `write!(self, ...)` / write_str -> output(s, Literal).
    private void W(string s) => Output(s, Escaping.Literal);

    // ---- driver ----------------------------------------------------------

    private void Run(MdNode root)
    {
        var stack = new Stack<(MdNode node, bool post)>();
        stack.Push((root, false));

        int visited = 0;
        while (stack.Count > 0)
        {
            Cancel.Every(visited++, 0x3FF);
            var (node, post) = stack.Pop();
            if (!post)
            {
                if (FormatNode(node, true))
                {
                    stack.Push((node, true));
                    foreach (var ch in node.ReverseChildren())
                        stack.Push((ch, false));
                }
            }
            else
            {
                FormatNode(node, false);
            }
        }

        if (_window.Count > 0 && _window[^1] != (byte)'\n')
            _output.Append('\n');
    }

    private bool GetInTightListItem(MdNode node)
    {
        var tmp = node.ContainingBlock();
        if (tmp is null) return false;

        if (tmp.Type is NodeType.Item or NodeType.TaskItem)
        {
            if (tmp.Parent is { Type: NodeType.List } l) return l.List.Tight;
            return false;
        }

        var parent = tmp.Parent;
        if (parent is null) return false;
        if (parent.Type is NodeType.Item or NodeType.TaskItem)
        {
            if (parent.Parent is { Type: NodeType.List } pl) return pl.List.Tight;
        }
        return false;
    }

    private bool FormatNode(MdNode node, bool entering)
    {
        _node = node;
        var parent = node.Parent;

        if (entering)
        {
            if (parent is not null && parent.Type is NodeType.Item or NodeType.TaskItem)
                _inTightListItem = GetInTightListItem(node);
        }
        else if (node.Type == NodeType.List)
        {
            _inTightListItem = parent is not null
                && parent.Type is NodeType.Item or NodeType.TaskItem
                && GetInTightListItem(node);
        }

        bool nextIsBlock = node.Next is null || node.Next.IsBlock;
        bool textInCell = node.Type == NodeType.Text && parent is { Type: NodeType.TableCell };

        switch (node.Type)
        {
            case NodeType.Document: break;
            case NodeType.BlockQuote: FormatBlockQuote(entering); break;
            case NodeType.List: FormatList(node, entering); break;
            case NodeType.Item: FormatItem(node, entering); break;
            case NodeType.Heading: FormatHeading(node.Heading, entering); break;
            case NodeType.CodeBlock: FormatCodeBlock(node, node.CodeBlock, entering); break;
            case NodeType.ThematicBreak: FormatThematicBreak(entering); break;
            case NodeType.Paragraph: FormatParagraph(entering); break;
            case NodeType.Text: FormatText(node.Literal, entering, !textInCell); break;
            case NodeType.LineBreak: FormatLineBreak(entering, nextIsBlock); break;
            case NodeType.SoftBreak: FormatSoftBreak(entering); break;
            case NodeType.Code: FormatCode(node.Code.Literal, entering); break;
            case NodeType.Raw: if (entering) W(node.Literal); break;
            case NodeType.Strong:
                if (parent is null || parent.Type != NodeType.Strong) W("**");
                break;
            case NodeType.Emph: FormatEmph(node); break;
            case NodeType.TaskItem: FormatTaskItem(node, entering); break;
            case NodeType.Strikethrough: W("~~"); break;
            case NodeType.Highlight: W("=="); break;
            case NodeType.Underline: W("__"); break;
            case NodeType.Subscript: W("~"); break;
            case NodeType.Superscript: W("^"); break;
            case NodeType.Link: return FormatLink(node, node.Link, entering);
            case NodeType.Image: FormatImage(node.Link, entering); break;
            case NodeType.Table: FormatTable(entering); break;
            case NodeType.TableRow: FormatTableRow(entering); break;
            case NodeType.TableCell: FormatTableCell(node, entering); break;
            case NodeType.FootnoteDefinition: FormatFootnoteDefinition(node.FootnoteDefinition.Name, entering); break;
            case NodeType.FootnoteReference: FormatFootnoteReference(node.FootnoteReference.Name, entering); break;
            case NodeType.Math: FormatMath(node.Math, entering); break;
            case NodeType.Alert: FormatAlert(node.Alert, entering); break;
        }
        return true;
    }

    private void FormatBlockQuote(bool entering)
    {
        if (entering)
        {
            W("> ");
            _beginContent = true;
            _prefix.Append("> ");
        }
        else
        {
            _prefix.Length -= 2;
            Blankline();
        }
    }

    private void FormatList(MdNode node, bool entering)
    {
        bool ordered = node.List.ListType == ListType.Ordered;
        int start = node.List.Start;

        if (entering)
        {
            if (ordered) _olStack.Add(start);
        }
        else
        {
            if (ordered && _olStack.Count > 0) _olStack.RemoveAt(_olStack.Count - 1);

            var next = node.Next;
            if (next is not null && next.Type is NodeType.CodeBlock or NodeType.List)
            {
                Cr();
                W("<!-- end list -->");
                Blankline();
            }
        }
    }

    private void FormatItem(MdNode node, bool entering)
    {
        var parentList = node.Parent!.List;

        int markerWidth;
        string listmarker = "";
        if (parentList.ListType == ListType.Bullet)
        {
            markerWidth = 2;
        }
        else
        {
            int listNumber;
            if (_olStack.Count > 0)
            {
                listNumber = _olStack[^1];
                if (entering) _olStack[^1] = listNumber + 1;
            }
            else
            {
                listNumber = node.Type == NodeType.Item ? node.List.Start : parentList.Start;
            }
            listmarker = listNumber.ToString() + (parentList.Delimiter == ListDelimType.Paren ? ")" : ".") + " ";
            // ol_width default is 0, so no padding is added.
            markerWidth = Encoding.UTF8.GetByteCount(listmarker);
        }

        if (entering)
        {
            if (parentList.ListType == ListType.Bullet)
                W("- "); // list_style default is Dash (0x2D)
            else
                W(listmarker);
            _beginContent = true;
            for (int k = 0; k < markerWidth; k++) _prefix.Append(' ');
        }
        else
        {
            int newLen = _prefix.Length > markerWidth ? _prefix.Length - markerWidth : 0;
            _prefix.Length = newLen;
            Cr();
        }
    }

    private void FormatHeading(NodeHeading nh, bool entering)
    {
        if (entering)
        {
            for (int k = 0; k < nh.Level; k++) W("#");
            W(" ");
            _beginContent = true;
            _noLinebreaks = true;
        }
        else
        {
            _noLinebreaks = false;
            Blankline();
        }
    }

    private void FormatCodeBlock(MdNode node, NodeCodeBlock ncb, bool entering)
    {
        if (!entering) return;

        bool firstInListItem = node.Prev is null &&
            node.Parent is not null && node.Parent.Type is NodeType.Item or NodeType.TaskItem;

        if (!firstInListItem) Blankline();

        byte[] info = Encoding.UTF8.GetBytes(ncb.Info);
        byte[] literal = Encoding.UTF8.GetBytes(ncb.Literal);

        bool indented = !(info.Length > 0
            || literal.Length <= 2
            || (literal.Length > 0 && Ctype.IsSpace(literal[0]))
            || firstInListItem
            || true /* prefer_fenced */
            || (literal.Length >= 2 && Ctype.IsSpace(literal[^1]) && Ctype.IsSpace(literal[^2])));

        if (indented)
        {
            W("    ");
            _prefix.Append("    ");
            W(ncb.Literal);
            _prefix.Length -= 4;
        }
        else
        {
            byte fenceByte = Array.IndexOf(info, (byte)'`') >= 0 ? (byte)'~' : (byte)'`';
            int numticks = Math.Max(3, LongestByteSequence(literal, fenceByte) + 1);
            for (int k = 0; k < numticks; k++) W(((char)fenceByte).ToString());
            if (info.Length > 0) W(ncb.Info);
            Cr();
            W(ncb.Literal);
            Cr();
            for (int k = 0; k < numticks; k++) W(((char)fenceByte).ToString());
        }
        Blankline();
    }

    private void FormatThematicBreak(bool entering)
    {
        if (entering)
        {
            Blankline();
            W("-----");
            Blankline();
        }
    }

    private void FormatParagraph(bool entering)
    {
        if (!entering) Blankline();
    }

    private void FormatText(string literal, bool entering, bool wrap)
    {
        if (entering) Output(literal, Escaping.Normal);
    }

    private void FormatLineBreak(bool entering, bool nextIsBlock)
    {
        if (entering)
        {
            if (!nextIsBlock) W("\\"); // hardbreaks = false
            Cr();
        }
    }

    private void FormatSoftBreak(bool entering)
    {
        if (entering)
        {
            if (!_noLinebreaks) Cr(); // width == 0, hardbreaks = false
            else Output(" ", Escaping.Literal);
        }
    }

    private void FormatCode(string literal, bool entering)
    {
        if (!entering) return;
        byte[] lb = Encoding.UTF8.GetBytes(literal);
        int numticks = ShortestUnusedSequence(lb, (byte)'`');
        for (int k = 0; k < numticks; k++) W("`");

        bool pad;
        if (lb.Length == 0)
        {
            pad = true;
        }
        else
        {
            bool allSpace = lb.All(c => c == (byte)' ' || c == (byte)'\r' || c == (byte)'\n');
            bool hasEdgeSpace = lb[0] == (byte)' ' || lb[^1] == (byte)' ';
            bool hasEdgeBacktick = lb[0] == (byte)'`' || lb[^1] == (byte)'`';
            pad = hasEdgeBacktick || (!allSpace && hasEdgeSpace);
        }

        if (pad) W(" ");
        Output(literal, Escaping.Literal);
        if (pad) W(" ");
        for (int k = 0; k < numticks; k++) W("`");
    }

    private void FormatEmph(MdNode node)
    {
        bool underscore = node.Parent is { Type: NodeType.Emph }
            && node.Next is null && node.Prev is null;
        W(underscore ? "_" : "*");
    }

    private void FormatTaskItem(MdNode node, bool entering)
    {
        if (node.Parent is { Type: NodeType.List }) FormatItem(node, entering);
        if (entering) W("[" + (node.TaskSymbol ?? ' ') + "] ");
    }

    private bool FormatLink(MdNode node, NodeLink nl, bool entering)
    {
        if (IsAutolink(node, nl))
        {
            if (entering)
            {
                W("<" + TrimStartMatch(nl.Url, "mailto:") + ">");
                return false;
            }
        }
        else if (entering)
        {
            W("[");
        }
        else
        {
            W("](");
            Output(nl.Url, Escaping.Url);
            if (!string.IsNullOrEmpty(nl.Title))
            {
                W(" \"");
                Output(nl.Title, Escaping.Title);
                W("\"");
            }
            W(")");
        }
        return true;
    }

    private void FormatImage(NodeLink nl, bool entering)
    {
        if (entering)
        {
            W("![");
        }
        else
        {
            W("](");
            Output(nl.Url, Escaping.Url);
            if (!string.IsNullOrEmpty(nl.Title))
            {
                Output(" \"", Escaping.Literal);
                Output(nl.Title, Escaping.Title);
                W("\"");
            }
            W(")");
        }
    }

    private void FormatTable(bool entering)
    {
        _customEscape = entering ? TableEscape : null;
        Blankline();
    }

    private void FormatTableRow(bool entering)
    {
        if (entering)
        {
            Cr();
            W("|");
        }
    }

    private void FormatTableCell(MdNode node, bool entering)
    {
        if (entering)
        {
            W(" ");
        }
        else
        {
            W(" |");
            bool inHeader = node.Parent!.TableRowHeader;
            if (inHeader && node.Next is null)
            {
                var alignments = node.Parent!.Parent!.Table.Alignments;
                Cr();
                W("|");
                foreach (var a in alignments)
                {
                    string sep = a switch
                    {
                        TableAlignment.Left => ":--",
                        TableAlignment.Center => ":-:",
                        TableAlignment.Right => "--:",
                        _ => "---",
                    };
                    W(" " + sep + " |");
                }
                Cr();
            }
        }
    }

    private void FormatFootnoteDefinition(string name, bool entering)
    {
        if (entering)
        {
            _footnoteIx += 1;
            W("[^" + name + "]:\n");
            _prefix.Append("    ");
        }
        else
        {
            _prefix.Length -= 4;
        }
    }

    private void FormatFootnoteReference(string r, bool entering)
    {
        if (entering)
        {
            W("[^");
            W(r);
            W("]");
        }
    }

    private void FormatMath(NodeMath math, bool entering)
    {
        if (!entering) return;
        string startFence = math.DollarMath ? (math.DisplayMath ? "$$" : "$") : "$`";
        string endFence = startFence == "$`" ? "`$" : startFence;
        Output(startFence, Escaping.Literal);
        Output(math.Literal, Escaping.Literal);
        Output(endFence, Escaping.Literal);
    }

    private void FormatAlert(NodeAlert alert, bool entering)
    {
        if (entering)
        {
            W("> [!" + alert.AlertType.DefaultTitle().ToUpperInvariant() + "]");
            if (alert.Title is not null) W(" " + alert.Title);
            W("\n");
            W("> ");
            _beginContent = true;
            _prefix.Append("> ");
        }
        else
        {
            _prefix.Length -= 2;
            Blankline();
        }
    }

    // ---- helpers ---------------------------------------------------------

    private static bool TableEscape(MdNode node, char c) => node.Type switch
    {
        NodeType.Table or NodeType.TableRow or NodeType.TableCell => false,
        _ => c == '|',
    };

    private static bool IsAutolink(MdNode node, NodeLink nl)
    {
        if (string.IsNullOrEmpty(nl.Url) || Scanners.Scheme(nl.Url) is null) return false;
        if (!string.IsNullOrEmpty(nl.Title)) return false;
        var child = node.FirstChild;
        if (child is null || child.Type != NodeType.Text) return false;
        return TrimStartMatch(nl.Url, "mailto:") == child.Literal;
    }

    private static string TrimStartMatch(string s, string prefix) =>
        s.StartsWith(prefix, StringComparison.Ordinal) ? s.Substring(prefix.Length) : s;

    private static int LongestByteSequence(byte[] buffer, byte ch)
    {
        int longest = 0, current = 0;
        foreach (var c in buffer)
        {
            if (c == ch) current++;
            else { if (current > longest) longest = current; current = 0; }
        }
        if (current > longest) longest = current;
        return longest;
    }

    private static int ShortestUnusedSequence(byte[] buffer, byte f)
    {
        var used = new HashSet<int>();
        int current = 0;
        foreach (var c in buffer)
        {
            if (c == f) current++;
            else { if (current > 0) used.Add(current); current = 0; }
        }
        if (current > 0) used.Add(current);
        int i = 1;
        while (used.Contains(i)) i++;
        return i;
    }

    private static int DecodeUtf8(byte[] bytes, int i, out int len)
    {
        byte b0 = bytes[i];
        if (b0 < 0x80) { len = 1; return b0; }
        if ((b0 & 0xE0) == 0xC0) { len = 2; return ((b0 & 0x1F) << 6) | (bytes[i + 1] & 0x3F); }
        if ((b0 & 0xF0) == 0xE0)
        {
            len = 3;
            return ((b0 & 0x0F) << 12) | ((bytes[i + 1] & 0x3F) << 6) | (bytes[i + 2] & 0x3F);
        }
        len = 4;
        return ((b0 & 0x07) << 18) | ((bytes[i + 1] & 0x3F) << 12) | ((bytes[i + 2] & 0x3F) << 6) | (bytes[i + 3] & 0x3F);
    }

    private static string Utf8Slice(byte[] bytes, int i, int len) =>
        Encoding.UTF8.GetString(bytes, i, len);
}
