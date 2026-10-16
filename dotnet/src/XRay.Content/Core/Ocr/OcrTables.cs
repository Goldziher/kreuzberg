using System.Text;
using XRay.Content.Internal.Html;
using XRay.Content.Internal.Tables;

namespace XRay.Content.Core.Ocr;

/// <summary>
/// Recovers the tables a recognizer reports as HTML markup into cell grids, so the OCR pass can
/// put them in the document's own table model rather than leaving markup in a text element.
/// </summary>
/// <remarks>
/// <para>
/// PaddleOCR-VL reads a table region as OTSL and the pipeline converts that to an HTML
/// <c>&lt;table&gt;</c> — see <c>OtslTable.ToHtml</c> — so a recognised page arrives as plain
/// text with HTML tables embedded in it. Leaving that markup in the text makes every output
/// format wrong in its own way: Markdown emits raw HTML where a pipe table belongs, and plain
/// text emits the tags themselves.
/// </para>
/// <para>
/// Splitting the text here, before the document is rendered, is what keeps the port's rule that
/// the output format is a rendering concern: a recovered table becomes an ordinary
/// <see cref="Types.ElementKindTag.Table"/> element and each renderer writes it in its own
/// notation, exactly as a table from any other extractor.
/// </para>
/// <para>
/// Cells are placed by <see cref="GridFlatten"/>, the same rule every other
/// format's tables go through, so the merged cells OTSL does produce keep the rest of the row
/// lined up with its headers: the spanning cell's text sits at its origin and the columns it
/// covers stay empty.
/// </para>
/// <para>
/// A cell's text is taken plainly — entities decoded (<c>AT&amp;amp;T</c> back to
/// <c>AT&amp;T</c>), whitespace collapsed — rather than rendered as Markdown the way the HTML
/// extractor renders its cells. There is no inline markup to preserve: <c>OtslTable</c> encodes
/// the recognised text, so anything that looks like markup in a cell is text the camera saw.
/// Rendering it would escape those characters into the grid, and a grid is what the plain and
/// HTML renderers read too — one format's escaping would end up in all of them.
/// </para>
/// <para>
/// A table that yields no grid at all — unbalanced markup, or nothing but empty cells — is left
/// in the text verbatim. Markup a consumer can still read beats a table invented from a failed
/// parse.
/// </para>
/// </remarks>
internal static class OcrTables
{
    /// <summary>One run of a recognition: either text, or a table's cells.</summary>
    /// <param name="Text">The run's text; empty for a table.</param>
    /// <param name="Cells">The table's rows, or <c>null</c> when this run is text.</param>
    internal readonly record struct Segment(string Text, List<List<string>>? Cells);

    /// <summary>
    /// Splits recognised text into its text runs and the tables embedded in it, in order.
    /// </summary>
    /// <remarks>
    /// Text runs are trimmed and empty ones dropped, so a table on a line of its own — which is
    /// how the recognizer emits one — does not leave blank elements around it.
    /// </remarks>
    public static List<Segment> Split(string text)
    {
        var segments = new List<Segment>();
        int runStart = 0;
        int search = 0;

        while (search < text.Length)
        {
            int open = IndexOfTag(text, "<table", search);
            if (open < 0) break;

            int end = MatchingEnd(text, open);
            if (end < 0) break;

            var cells = TryReadGrid(text.Substring(open, end - open));
            if (cells is null)
            {
                // Not a table this port can put on a grid: leave the markup where it is, as part
                // of the surrounding text run, and carry on looking for the next one.
                search = end;
                continue;
            }

            AddText(segments, text.Substring(runStart, open - runStart));
            segments.Add(new Segment("", cells));
            runStart = search = end;
        }

        AddText(segments, text.Substring(runStart));
        return segments;
    }

    private static void AddText(List<Segment> segments, string text)
    {
        string trimmed = text.Trim();
        if (trimmed.Length > 0) segments.Add(new Segment(trimmed, null));
    }

    /// <summary>
    /// The index of the next <c>&lt;table</c> or <c>&lt;/table</c>, requiring the tag name to
    /// end where it should so that a <c>&lt;tablet&gt;</c> — or an OCR'd sentence about one —
    /// is not read as markup.
    /// </summary>
    private static int IndexOfTag(string text, string tag, int from)
    {
        for (int i = from; i < text.Length; i++)
        {
            int hit = text.IndexOf(tag, i, StringComparison.OrdinalIgnoreCase);
            if (hit < 0) return -1;
            int after = hit + tag.Length;
            if (after >= text.Length) return -1;
            char c = text[after];
            if (c == '>' || c == '/' || char.IsWhiteSpace(c)) return hit;
            i = hit;
        }
        return -1;
    }

    /// <summary>
    /// The index just past the <c>&lt;/table&gt;</c> that closes the element opening at
    /// <paramref name="open"/>, counting nested tables, or -1 when nothing closes it.
    /// </summary>
    private static int MatchingEnd(string text, int open)
    {
        int depth = 0;
        int p = open;

        while (p < text.Length)
        {
            int nextOpen = IndexOfTag(text, "<table", p);
            int nextClose = IndexOfTag(text, "</table", p);
            if (nextClose < 0) return -1;

            if (nextOpen >= 0 && nextOpen < nextClose)
            {
                depth++;
                p = nextOpen + "<table".Length;
                continue;
            }

            int gt = text.IndexOf('>', nextClose);
            if (gt < 0) return -1;
            if (--depth == 0) return gt + 1;
            p = gt + 1;
        }

        return -1;
    }

    /// <summary>
    /// The cell grid of a <c>&lt;table&gt;</c>, or <c>null</c> when it holds nothing worth
    /// putting on one.
    /// </summary>
    private static List<List<string>>? TryReadGrid(string html)
    {
        HNode? table;
        try
        {
            table = FirstTable(HtmlDom.Parse(html));
        }
        catch (Exception) when (Cancel.NotCancelling)
        {
            // OCR is never fatal: markup the parser chokes on stays in the text rather than
            // costing the document its recognition.
            return null;
        }

        if (table is null) return null;

        var rows = new List<IReadOnlyList<Cell>>();
        foreach (var row in Rows(table))
        {
            var cells = new List<Cell>();
            foreach (var cell in row.Children)
            {
                if (cell.Tag is not ("td" or "th")) continue;
                cells.Add(new Cell(Span(cell, "colspan"), Span(cell, "rowspan"), CellText(cell)));
            }
            rows.Add(cells);
        }

        if (rows.Count == 0) return null;

        var grid = GridFlatten.FlattenSpannedRows(
            rows, c => c.ColSpan, c => c.RowSpan, c => c.Text);

        if (grid.Count == 0) return null;
        if (grid.All(row => row.All(cell => cell.Length == 0))) return null;
        return grid;
    }

    private readonly record struct Cell(int ColSpan, int RowSpan, string Text);

    /// <summary>The table's rows, whether or not a section element wraps them.</summary>
    private static IEnumerable<HNode> Rows(HNode table)
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

    /// <summary>
    /// A span attribute, defaulting to 1 for anything that is not a positive number.
    /// <see cref="GridFlatten"/> owns the upper clamp, so this only has to keep a missing or
    /// nonsense value from reaching it as a zero or a negative.
    /// </summary>
    private static int Span(HNode cell, string name) =>
        int.TryParse(cell.Attr(name), out int value) && value > 0 ? value : 1;

    /// <summary>The cell's text: every text node under it, entities decoded and runs of
    /// whitespace — a line break in the markup included — collapsed to one space.</summary>
    private static string CellText(HNode cell)
    {
        var text = new StringBuilder();
        Collect(cell);
        return Collapse(text.ToString());

        void Collect(HNode node)
        {
            foreach (var child in node.Children)
            {
                if (child.IsComment) continue;
                if (child.Tag is null)
                    text.Append(HtmlWalker.DecodeEntitiesFull(child.Text, child.CanonicalAttrs));
                else if (child.Tag == "br") text.Append(' ');
                else Collect(child);
            }
        }
    }

    private static string Collapse(string text)
    {
        var sb = new StringBuilder(text.Length);
        bool pendingSpace = false;
        foreach (char c in text)
        {
            if (char.IsWhiteSpace(c)) { pendingSpace = sb.Length > 0; continue; }
            if (pendingSpace) { sb.Append(' '); pendingSpace = false; }
            sb.Append(c);
        }
        return sb.ToString();
    }

    private static HNode? FirstTable(HNode node)
    {
        foreach (var child in node.Children)
        {
            if (child.Tag == "table") return child;
            if (FirstTable(child) is { } nested) return nested;
        }
        return null;
    }
}
