using System.Text;

namespace XRay.Content.Internal.Djot;

/// <summary>
/// A pragmatic Djot markup parser that emits a <c>jotdown</c>-like flat event stream
/// (<see cref="DjotEvent"/>) consumed by <c>DjotExtractor</c>. It stands in for the Rust
/// <c>jotdown</c> crate used by <c>crates/xberg/src/extractors/djot_format/</c>.
///
/// It covers the block/inline constructs the content extractor cares about: paragraphs,
/// ATX headings, pipe tables (with caption lines suppressed), fenced code blocks, block
/// quotes, bullet/ordered/task lists, block math, raw blocks, links, images, footnotes, and
/// the inline emphasis/strong/verbatim/strikethrough spans. It is not a byte-exact jotdown
/// clone for adversarial inline edge cases, but matches jotdown for the common cases the
/// extractor relies on (notably: straight quotes are treated as smart-punctuation and dropped
/// from text runs, mirroring jotdown's distinct quote events).
/// </summary>
internal static class DjotParser
{
    public static List<DjotEvent> Parse(string input)
    {
        var lines = SplitLines(input);
        var events = new List<DjotEvent>();
        ParseBlocks(lines, 0, lines.Count, events);
        return events;
    }

    // ------------------------------------------------------------------
    // Block parsing
    // ------------------------------------------------------------------

    private static void ParseBlocks(List<string> lines, int start, int end, List<DjotEvent> ev)
    {
        int i = start;
        while (i < end)
        {
            Cancel.ThrowIfRequested();
            string line = lines[i];

            if (IsBlank(line)) { i++; continue; }

            string trimmed = line.TrimStart();

            // Caption line (^ ...) — suppressed (jotdown emits a Caption container the extractor ignores).
            if (IsCaption(trimmed)) { i = SkipParagraphLike(lines, i, end); continue; }

            // Thematic break — jotdown emits ThematicBreak, ignored by the extractor.
            if (IsThematicBreak(trimmed)) { i++; continue; }

            // Fenced code block ``` / ~~~
            if (TryFenceInfo(trimmed, out char fence, out int fenceLen, out string lang))
            {
                i = ParseCodeBlock(lines, i, end, fence, fenceLen, lang, ev);
                continue;
            }

            // Block math $$ ... $$
            if (trimmed.StartsWith("$$", StringComparison.Ordinal))
            {
                i = ParseDisplayMath(lines, i, end, ev);
                continue;
            }

            // Div fence ::: — parse inner blocks, drop the fences.
            if (IsDivFence(trimmed))
            {
                i = ParseDiv(lines, i, end, ev);
                continue;
            }

            // Table: consecutive pipe lines containing at least one delimiter row.
            if (trimmed.StartsWith("|", StringComparison.Ordinal))
            {
                int tblEnd = i;
                while (tblEnd < end && lines[tblEnd].TrimStart().StartsWith("|", StringComparison.Ordinal))
                    tblEnd++;
                if (HasDelimiterRow(lines, i, tblEnd))
                {
                    ParseTable(lines, i, tblEnd, ev);
                    i = tblEnd;
                    continue;
                }
                // no delimiter row → fall through and treat as paragraph
            }

            // Heading
            if (TryHeading(trimmed, out byte level, out string headingText))
            {
                i = ParseHeading(lines, i, end, level, headingText, ev);
                continue;
            }

            // Blockquote
            if (trimmed.StartsWith(">", StringComparison.Ordinal))
            {
                i = ParseBlockquote(lines, i, end, ev);
                continue;
            }

            // Lists
            if (IsListMarker(trimmed, out _, out _, out _))
            {
                i = ParseList(lines, i, end, ev);
                continue;
            }

            // Paragraph
            i = ParseParagraph(lines, i, end, ev);
        }
    }

    private static int ParseParagraph(List<string> lines, int start, int end, List<DjotEvent> ev)
    {
        var sb = new StringBuilder();
        int i = start;
        while (i < end)
        {
            Cancel.ThrowIfRequested();
            string line = lines[i];
            if (IsBlank(line)) break;
            string trimmed = line.TrimStart();
            if (i != start && StartsNewBlock(trimmed)) break;
            if (sb.Length > 0) sb.Append('\n');
            sb.Append(line.Trim());
            i++;
        }

        ev.Add(DjotEvent.Simple(DjotEventKind.StartParagraph));
        InlineParagraph(sb.ToString(), ev);
        ev.Add(DjotEvent.Simple(DjotEventKind.EndParagraph));
        return i;
    }

    private static int ParseHeading(List<string> lines, int start, int end, byte level, string firstText, List<DjotEvent> ev)
    {
        // Consume continuation lines (non-blank, no new block start) as part of the heading.
        var sb = new StringBuilder(firstText.Trim());
        int i = start + 1;
        while (i < end)
        {
            Cancel.ThrowIfRequested();
            string line = lines[i];
            if (IsBlank(line)) break;
            string trimmed = line.TrimStart();
            if (StartsNewBlock(trimmed)) break;
            sb.Append(' ');
            sb.Append(line.Trim());
            i++;
        }

        ev.Add(new DjotEvent { Kind = DjotEventKind.StartHeading, Level = level });
        InlineParagraph(sb.ToString(), ev);
        ev.Add(DjotEvent.Simple(DjotEventKind.EndHeading));
        return i;
    }

    private static int ParseCodeBlock(List<string> lines, int start, int end, char fence, int fenceLen, string lang, List<DjotEvent> ev)
    {
        var code = new StringBuilder();
        int i = start + 1;
        bool first = true;
        while (i < end)
        {
            Cancel.ThrowIfRequested();
            string line = lines[i];
            string t = line.TrimStart();
            if (IsClosingFence(t, fence, fenceLen)) { i++; break; }
            if (!first) code.Append('\n');
            code.Append(line);
            first = false;
            i++;
        }

        ev.Add(new DjotEvent { Kind = DjotEventKind.StartCodeBlock, Text = lang });
        if (code.Length > 0) ev.Add(DjotEvent.Text_(code.ToString()));
        ev.Add(DjotEvent.Simple(DjotEventKind.EndCodeBlock));
        return i;
    }

    private static int ParseDisplayMath(List<string> lines, int start, int end, List<DjotEvent> ev)
    {
        // $$ math ... $$ (single-line or fenced).
        string firstTrim = lines[start].TrimStart();
        var math = new StringBuilder();
        int i = start;
        string body = firstTrim.Substring(2);
        if (body.EndsWith("$$", StringComparison.Ordinal) && body.Length >= 2)
        {
            math.Append(body.Substring(0, body.Length - 2));
            i = start + 1;
        }
        else
        {
            if (body.Length > 0) math.Append(body);
            i = start + 1;
            while (i < end)
            {
                Cancel.ThrowIfRequested();
                string line = lines[i];
                if (IsBlank(line)) { i++; break; }
                if (line.TrimEnd().EndsWith("$$", StringComparison.Ordinal))
                {
                    string s = line.TrimEnd();
                    if (math.Length > 0) math.Append('\n');
                    math.Append(s.Substring(0, s.Length - 2));
                    i++;
                    break;
                }
                if (math.Length > 0) math.Append('\n');
                math.Append(line);
                i++;
            }
        }

        ev.Add(new DjotEvent { Kind = DjotEventKind.StartMath, Display = true });
        if (math.Length > 0) ev.Add(DjotEvent.Text_(math.ToString()));
        ev.Add(new DjotEvent { Kind = DjotEventKind.EndMath, Display = true });
        return i;
    }

    private static int ParseDiv(List<string> lines, int start, int end, List<DjotEvent> ev)
    {
        int i = start + 1;
        int innerStart = i;
        while (i < end && !IsDivFence(lines[i].TrimStart())) i++;
        ParseBlocks(lines, innerStart, i, ev);
        if (i < end) i++; // consume closing :::
        return i;
    }

    private static int ParseBlockquote(List<string> lines, int start, int end, List<DjotEvent> ev)
    {
        var inner = new List<string>();
        int i = start;
        while (i < end)
        {
            Cancel.ThrowIfRequested();
            string line = lines[i];
            string trimmed = line.TrimStart();
            if (trimmed.StartsWith(">", StringComparison.Ordinal))
            {
                string rest = trimmed.Substring(1);
                if (rest.StartsWith(" ", StringComparison.Ordinal)) rest = rest.Substring(1);
                inner.Add(rest);
                i++;
            }
            else if (IsBlank(line)) break;
            else break;
        }

        ev.Add(DjotEvent.Simple(DjotEventKind.StartBlockquote));
        ParseBlocks(inner, 0, inner.Count, ev);
        ev.Add(DjotEvent.Simple(DjotEventKind.EndBlockquote));
        return i;
    }

    private static int ParseList(List<string> lines, int start, int end, List<DjotEvent> ev)
    {
        IsListMarker(lines[start].TrimStart(), out bool ordered, out _, out _);
        ev.Add(new DjotEvent { Kind = DjotEventKind.StartList, Ordered = ordered });

        int i = start;
        while (i < end)
        {
            Cancel.ThrowIfRequested();
            string line = lines[i];
            if (IsBlank(line)) { i++; continue; }
            string trimmed = line.TrimStart();
            if (!IsListMarker(trimmed, out bool itemOrdered, out int markerLen, out _) || itemOrdered != ordered)
                break;

            // Collect this item's lines: the marker line + indented continuation lines.
            var itemText = new StringBuilder(trimmed.Substring(markerLen).Trim());
            i++;
            while (i < end)
            {
                Cancel.ThrowIfRequested();
                string cont = lines[i];
                if (IsBlank(cont)) { i++; break; }
                string contTrim = cont.TrimStart();
                if (IsListMarker(contTrim, out _, out _, out _)) break;
                if (StartsNewBlock(contTrim)) break;
                itemText.Append(' ');
                itemText.Append(cont.Trim());
                i++;
            }

            ev.Add(DjotEvent.Simple(DjotEventKind.StartListItem));
            InlineParagraph(itemText.ToString(), ev);
            ev.Add(DjotEvent.Simple(DjotEventKind.EndListItem));
        }

        ev.Add(DjotEvent.Simple(DjotEventKind.EndList));
        return i;
    }

    private static void ParseTable(List<string> lines, int start, int end, List<DjotEvent> ev)
    {
        ev.Add(DjotEvent.Simple(DjotEventKind.StartTable));
        for (int i = start; i < end; i++)
        {
            var cells = SplitRow(lines[i].TrimStart());
            if (cells.Count == 0) continue;
            if (IsDelimiterCells(cells)) continue; // separator row → not a data/header row

            ev.Add(DjotEvent.Simple(DjotEventKind.StartTableRow));
            foreach (var cell in cells)
            {
                ev.Add(DjotEvent.Simple(DjotEventKind.StartTableCell));
                InlineParagraph(cell.Trim(), ev);
                ev.Add(DjotEvent.Simple(DjotEventKind.EndTableCell));
            }
            ev.Add(DjotEvent.Simple(DjotEventKind.EndTableRow));
        }
        ev.Add(DjotEvent.Simple(DjotEventKind.EndTable));
    }

    // ------------------------------------------------------------------
    // Inline parsing
    // ------------------------------------------------------------------

    /// <summary>Tokenize inline content, treating soft line breaks (embedded '\n') as Softbreak
    /// events, matching how jotdown reports wrapped paragraph text.</summary>
    private static void InlineParagraph(string text, List<DjotEvent> ev)
    {
        int lineStart = 0;
        for (int i = 0; i <= text.Length; i++)
        {
            if (i == text.Length || text[i] == '\n')
            {
                InlineSpan(text.Substring(lineStart, i - lineStart), ev);
                if (i < text.Length)
                {
                    ev.Add(DjotEvent.Simple(DjotEventKind.Softbreak));
                    lineStart = i + 1;
                }
            }
        }
    }

    private static void InlineSpan(string s, List<DjotEvent> ev)
    {
        var buf = new StringBuilder();
        int i = 0;
        while (i < s.Length)
        {
            char c = s[i];

            // Escape
            if (c == '\\' && i + 1 < s.Length)
            {
                buf.Append(s[i + 1]);
                i += 2;
                continue;
            }

            // Verbatim (inline code) — literal content between matching backtick runs.
            if (c == '`')
            {
                int tickLen = 1;
                while (i + tickLen < s.Length && s[i + tickLen] == '`') tickLen++;
                int close = FindBacktickRun(s, i + tickLen, tickLen);
                if (close >= 0)
                {
                    Flush(buf, ev);
                    string content = s.Substring(i + tickLen, close - (i + tickLen));
                    // jotdown trims one optional surrounding space from verbatim spans.
                    if (content.Length >= 2 && content[0] == ' ' && content[^1] == ' ')
                        content = content.Substring(1, content.Length - 2);
                    ev.Add(DjotEvent.Simple(DjotEventKind.StartVerbatim));
                    if (content.Length > 0) ev.Add(DjotEvent.Text_(content));
                    ev.Add(DjotEvent.Simple(DjotEventKind.EndVerbatim));
                    i = close + tickLen;
                    continue;
                }
            }

            // Image ![alt](url)
            if (c == '!' && i + 1 < s.Length && s[i + 1] == '[')
            {
                if (TryLink(s, i + 1, out int labelStart, out int labelEnd, out string url, out int consumed))
                {
                    Flush(buf, ev);
                    ev.Add(new DjotEvent { Kind = DjotEventKind.StartImage, Url = url });
                    InlineSpan(s.Substring(labelStart, labelEnd - labelStart), ev);
                    ev.Add(new DjotEvent { Kind = DjotEventKind.EndImage, Url = url });
                    i = consumed;
                    continue;
                }
            }

            // Footnote reference [^label]
            if (c == '[' && i + 1 < s.Length && s[i + 1] == '^')
            {
                int rb = s.IndexOf(']', i + 2);
                if (rb > i + 1)
                {
                    Flush(buf, ev);
                    ev.Add(new DjotEvent { Kind = DjotEventKind.FootnoteReference, Text = s.Substring(i + 2, rb - (i + 2)) });
                    i = rb + 1;
                    continue;
                }
            }

            // Link [text](url)
            if (c == '[')
            {
                if (TryLink(s, i, out int labelStart, out int labelEnd, out string url, out int consumed))
                {
                    Flush(buf, ev);
                    ev.Add(new DjotEvent { Kind = DjotEventKind.StartLink, Url = url });
                    InlineSpan(s.Substring(labelStart, labelEnd - labelStart), ev);
                    ev.Add(new DjotEvent { Kind = DjotEventKind.EndLink, Url = url });
                    i = consumed;
                    continue;
                }
            }

            // Strong *...*
            if (c == '*')
            {
                int close = FindEmphasisClose(s, i, '*');
                if (close > i)
                {
                    Flush(buf, ev);
                    ev.Add(DjotEvent.Simple(DjotEventKind.StartStrong));
                    InlineSpan(s.Substring(i + 1, close - (i + 1)), ev);
                    ev.Add(DjotEvent.Simple(DjotEventKind.EndStrong));
                    i = close + 1;
                    continue;
                }
            }

            // Emphasis _..._
            if (c == '_')
            {
                int close = FindEmphasisClose(s, i, '_');
                if (close > i)
                {
                    Flush(buf, ev);
                    ev.Add(DjotEvent.Simple(DjotEventKind.StartEmphasis));
                    InlineSpan(s.Substring(i + 1, close - (i + 1)), ev);
                    ev.Add(DjotEvent.Simple(DjotEventKind.EndEmphasis));
                    i = close + 1;
                    continue;
                }
            }

            // Strikethrough {-...-}
            if (c == '{' && i + 1 < s.Length && s[i + 1] == '-')
            {
                int endIdx = s.IndexOf("-}", i + 2, StringComparison.Ordinal);
                if (endIdx > i)
                {
                    Flush(buf, ev);
                    ev.Add(DjotEvent.Simple(DjotEventKind.StartDelete));
                    InlineSpan(s.Substring(i + 2, endIdx - (i + 2)), ev);
                    ev.Add(DjotEvent.Simple(DjotEventKind.EndDelete));
                    i = endIdx + 2;
                    continue;
                }
            }

            // Smart punctuation: straight quotes become smart-quote events in jotdown, which are
            // dropped from Str runs. Reproduce by omitting the character from the text run.
            if (c == '\'' || c == '"')
            {
                i++;
                continue;
            }

            buf.Append(c);
            i++;
        }

        Flush(buf, ev);
    }

    private static void Flush(StringBuilder buf, List<DjotEvent> ev)
    {
        if (buf.Length > 0)
        {
            ev.Add(DjotEvent.Text_(buf.ToString()));
            buf.Clear();
        }
    }

    private static int FindBacktickRun(string s, int from, int len)
    {
        int i = from;
        while (i < s.Length)
        {
            if (s[i] == '`')
            {
                int run = 1;
                while (i + run < s.Length && s[i + run] == '`') run++;
                if (run == len) return i;
                i += run;
            }
            else i++;
        }
        return -1;
    }

    /// <summary>Finds the closing delimiter for an emphasis/strong span opened at <paramref name="open"/>.
    /// Opener must be followed by a non-space; closer must be preceded by a non-space (Djot rule).</summary>
    private static int FindEmphasisClose(string s, int open, char delim)
    {
        if (open + 1 >= s.Length) return -1;
        if (char.IsWhiteSpace(s[open + 1])) return -1;
        for (int j = open + 1; j < s.Length; j++)
        {
            if (s[j] == '\\') { j++; continue; }
            if (s[j] == delim && !char.IsWhiteSpace(s[j - 1]))
                return j;
        }
        return -1;
    }

    /// <summary>Parses a <c>[label](url)</c> starting at the '[' at <paramref name="lb"/>.</summary>
    private static bool TryLink(string s, int lb, out int labelStart, out int labelEnd, out string url, out int consumed)
    {
        labelStart = labelEnd = 0;
        url = "";
        consumed = 0;
        int depth = 0;
        int close = -1;
        for (int j = lb; j < s.Length; j++)
        {
            if (s[j] == '\\') { j++; continue; }
            if (s[j] == '[') depth++;
            else if (s[j] == ']') { depth--; if (depth == 0) { close = j; break; } }
        }
        if (close < 0) return false;
        if (close + 1 >= s.Length || s[close + 1] != '(') return false;
        int rp = s.IndexOf(')', close + 2);
        if (rp < 0) return false;

        labelStart = lb + 1;
        labelEnd = close;
        url = s.Substring(close + 2, rp - (close + 2)).Trim();
        consumed = rp + 1;
        return true;
    }

    // ------------------------------------------------------------------
    // Block classification helpers
    // ------------------------------------------------------------------

    private static bool StartsNewBlock(string trimmed)
    {
        if (trimmed.Length == 0) return false;
        if (trimmed.StartsWith("|", StringComparison.Ordinal)) return true;
        if (IsCaption(trimmed)) return true;
        if (IsThematicBreak(trimmed)) return true;
        if (TryFenceInfo(trimmed, out _, out _, out _)) return true;
        if (trimmed.StartsWith("$$", StringComparison.Ordinal)) return true;
        if (IsDivFence(trimmed)) return true;
        if (TryHeading(trimmed, out _, out _)) return true;
        if (trimmed.StartsWith(">", StringComparison.Ordinal)) return true;
        if (IsListMarker(trimmed, out _, out _, out _)) return true;
        return false;
    }

    private static bool IsCaption(string trimmed) =>
        trimmed.StartsWith("^ ", StringComparison.Ordinal) || trimmed == "^";

    private static bool IsThematicBreak(string trimmed)
    {
        if (trimmed.Length < 3) return false;
        char c = trimmed[0];
        if (c != '-' && c != '*' && c != '_') return false;
        foreach (char ch in trimmed)
            if (ch != c && ch != ' ') return false;
        int count = 0;
        foreach (char ch in trimmed) if (ch == c) count++;
        return count >= 3;
    }

    private static bool IsDivFence(string trimmed) =>
        trimmed.StartsWith(":::", StringComparison.Ordinal);

    private static bool TryFenceInfo(string trimmed, out char fence, out int fenceLen, out string lang)
    {
        fence = '\0';
        fenceLen = 0;
        lang = "";
        if (trimmed.Length < 3) return false;
        char c = trimmed[0];
        if (c != '`' && c != '~') return false;
        int n = 0;
        while (n < trimmed.Length && trimmed[n] == c) n++;
        if (n < 3) return false;
        fence = c;
        fenceLen = n;
        lang = trimmed.Substring(n).Trim();
        return true;
    }

    private static bool IsClosingFence(string trimmed, char fence, int fenceLen)
    {
        if (trimmed.Length < fenceLen) return false;
        int n = 0;
        while (n < trimmed.Length && trimmed[n] == fence) n++;
        return n >= fenceLen && trimmed.Substring(n).Trim().Length == 0;
    }

    private static bool TryHeading(string trimmed, out byte level, out string text)
    {
        level = 0;
        text = "";
        int n = 0;
        while (n < trimmed.Length && trimmed[n] == '#') n++;
        if (n == 0 || n > 6) return false;
        if (n >= trimmed.Length) { level = (byte)n; return true; } // bare '#'
        if (trimmed[n] != ' ') return false;
        level = (byte)n;
        text = trimmed.Substring(n + 1);
        return true;
    }

    private static bool IsListMarker(string trimmed, out bool ordered, out int markerLen, out bool task)
    {
        ordered = false;
        markerLen = 0;
        task = false;
        if (trimmed.Length < 2) return false;

        char c = trimmed[0];
        if ((c == '-' || c == '*' || c == '+') && trimmed[1] == ' ')
        {
            markerLen = 2;
            // Task list: "- [ ] " / "- [x] "
            string rest = trimmed.Substring(2).TrimStart();
            if (rest.StartsWith("[ ]", StringComparison.Ordinal) || rest.StartsWith("[x]", StringComparison.OrdinalIgnoreCase))
            {
                task = true;
                int bracket = trimmed.IndexOf(']', 2);
                if (bracket > 0) markerLen = bracket + 1;
            }
            return true;
        }

        // Ordered: digits then '.' or ')' then space
        int d = 0;
        while (d < trimmed.Length && char.IsDigit(trimmed[d])) d++;
        if (d > 0 && d + 1 < trimmed.Length && (trimmed[d] == '.' || trimmed[d] == ')') && trimmed[d + 1] == ' ')
        {
            ordered = true;
            markerLen = d + 2;
            return true;
        }

        return false;
    }

    // ------------------------------------------------------------------
    // Table helpers
    // ------------------------------------------------------------------

    private static bool HasDelimiterRow(List<string> lines, int start, int end)
    {
        for (int i = start; i < end; i++)
        {
            var cells = SplitRow(lines[i].TrimStart());
            if (cells.Count > 0 && IsDelimiterCells(cells)) return true;
        }
        return false;
    }

    private static bool IsDelimiterCells(List<string> cells)
    {
        foreach (var raw in cells)
        {
            string cell = raw.Trim();
            if (cell.Length == 0) return false;
            int k = 0;
            if (cell[k] == ':') k++;
            int dashes = 0;
            while (k < cell.Length && cell[k] == '-') { k++; dashes++; }
            if (k < cell.Length && cell[k] == ':') k++;
            if (k != cell.Length || dashes == 0) return false;
        }
        return cells.Count > 0;
    }

    /// <summary>Splits a pipe-table row into raw (untrimmed) cell strings, dropping the empty
    /// segments before the first and after the last pipe.</summary>
    private static List<string> SplitRow(string line)
    {
        var cells = new List<string>();
        var sb = new StringBuilder();
        // line starts with '|'; iterate honoring backslash escapes.
        int i = 0;
        if (i < line.Length && line[i] == '|') i++;
        for (; i < line.Length; i++)
        {
            char c = line[i];
            if (c == '\\' && i + 1 < line.Length)
            {
                sb.Append(c);
                sb.Append(line[i + 1]);
                i++;
                continue;
            }
            if (c == '|')
            {
                cells.Add(sb.ToString());
                sb.Clear();
            }
            else sb.Append(c);
        }
        // Any trailing content after the last pipe (should be empty for well-formed rows).
        string tail = sb.ToString();
        if (tail.Trim().Length > 0) cells.Add(tail);
        return cells;
    }

    // ------------------------------------------------------------------
    // Misc
    // ------------------------------------------------------------------

    private static bool IsBlank(string line) => line.Trim().Length == 0;

    private static int SkipParagraphLike(List<string> lines, int start, int end)
    {
        int i = start + 1;
        while (i < end && !IsBlank(lines[i])) i++;
        return i;
    }

    private static List<string> SplitLines(string input)
    {
        var lines = new List<string>();
        int start = 0;
        for (int i = 0; i < input.Length; i++)
        {
            if (input[i] == '\n')
            {
                int e = i;
                if (e > start && input[e - 1] == '\r') e--;
                lines.Add(input.Substring(start, e - start));
                start = i + 1;
            }
        }
        if (start <= input.Length)
        {
            int e = input.Length;
            if (e > start && input[e - 1] == '\r') e--;
            if (start < input.Length || lines.Count == 0)
                lines.Add(input.Substring(start, e - start));
        }
        return lines;
    }
}
