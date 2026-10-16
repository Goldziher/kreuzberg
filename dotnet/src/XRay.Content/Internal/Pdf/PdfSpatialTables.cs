// Ruling-line table detection: the tier upstream reaches through pdf_oxide's
// `structure::spatial_table_detector`, which xberg drives twice per page — once
// strict (`extract_tables_native`) and once relaxed (`extract_tables_bordered`).
// Ported here is the intersection pipeline those two configurations share:
//
//   edges → snap/merge → intersections → cells → groups → grid → text
//
// The text-strategy tiers are not part of this file: the port already reaches
// borderless grids through `PdfLayoutTables` (the geometric fallback), and the
// reference outputs are generated with the layout detector off.
using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using XRay.Content.Internal.PdfOxide;
using XRay.Content.Internal.PdfOxide.Layout;
using XRay.Content.Types;

namespace XRay.Content.Internal.Pdf;

/// <summary>Knobs shared by the strict (native) and relaxed (bordered) passes.</summary>
internal sealed class TableDetectionConfig
{
    public double ColumnTolerance;
    public double RowTolerance;
    public int MinTableCells;
    public int MinTableColumns;
    public int MaxTableColumns;

    /// <summary>pdf_oxide `TableDetectionConfig::strict()`, as `extract_tables_native` uses it.</summary>
    public static TableDetectionConfig Strict() => new()
    {
        ColumnTolerance = 2.0,
        RowTolerance = 1.0,
        MinTableCells = 6,
        MinTableColumns = 3,
        MaxTableColumns = 12,
    };

    /// <summary>The relaxed Lines-strategy pass `extract_tables_bordered` runs.</summary>
    public static TableDetectionConfig Bordered() => new()
    {
        ColumnTolerance = 3.0,
        RowTolerance = 2.0,
        MinTableCells = 4,
        MinTableColumns = 2,
        MaxTableColumns = 15,
    };
}

/// <summary>One word-sized piece of page text, as the detector sees it.</summary>
internal sealed class TableSpan
{
    public string Text = "";
    public PathRect Bbox;
    public double FontSize;
    public double CenterX => Bbox.CenterX;
    public double CenterY => Bbox.CenterY;
}

internal sealed class GridCell
{
    public string Text = "";
    public List<int> SpanIndices = new();
    public PathRect? Bbox;
    public bool IsHeader;
}

internal sealed class GridRow
{
    public bool IsHeader;
    public List<GridCell> Cells = new();
    public GridRow(bool isHeader) { IsHeader = isHeader; }
    public GridRow Clone() => new(IsHeader)
    {
        Cells = Cells.Select(c => new GridCell
        {
            Text = c.Text, SpanIndices = new List<int>(c.SpanIndices), Bbox = c.Bbox, IsHeader = c.IsHeader,
        }).ToList(),
    };
}

/// <summary>
/// A table the ruling-line tier read, with the grid geometry it was read from.
/// </summary>
/// <param name="Table">The table as the pipeline emits it.</param>
/// <param name="ColumnBoundaries">
/// The x of every gridline bounding its columns, or null where the tier inferred the columns
/// rather than reading them off the page.
/// </param>
/// <param name="IsContinuationCandidate">
/// Whether the band was too small to stand on its own and is offered only as the continuation
/// of a table on a neighbouring page.
/// </param>
internal readonly record struct RuledTable(
    Table Table, double[]? ColumnBoundaries, bool IsContinuationCandidate);

internal sealed class GridTable
{
    public List<GridRow> Rows = new();
    public bool HasHeader;
    public int ColCount;
    public PathRect? Bbox;

    /// <summary>
    /// The x of every gridline bounding this table's columns, where it came from a grid that
    /// has them. Null from the cluster tier, which infers columns rather than reading them.
    /// </summary>
    public double[]? ColumnBoundaries;

    /// <summary>
    /// A well-formed grid too small to stand on its own. Emitted only as a candidate
    /// continuation of a table on the page before or after — see
    /// <see cref="PdfTableContinuation"/> — and dropped if nothing claims it.
    /// </summary>
    public bool BelowThreshold;
}

internal static partial class PdfSpatialTables
{
    private const double SnapTol = 3.0;

    /// <summary>How far a whole snapped group may spread, however tightly it chains.</summary>
    private const double SnapSpreadTol = 2.0 * SnapTol;
    private const double JoinTol = 3.0;
    private const double MinEdgeLen = 5.0;
    private const int DottedMinSegments = 3;
    private const double DottedMinSpan = 50.0;
    private const double DottedCoordSnap = 10.0;
    private const double LineAxisTol = 2.0;

    /// <summary>Line/rect path count above which a page is a drawing, not a ruled table.</summary>
    private const int MaxTableEdges = 1500;

    /// <summary>
    /// Filled cells a band below <see cref="TableDetectionConfig.MinTableCells"/> still needs
    /// before it is worth offering as the continuation of a table on the next page. Two is a
    /// row of a two-column table, which is the smallest thing a page break can leave behind.
    /// </summary>
    private const int MinContinuationCells = 2;

    /// <summary>
    /// Candidate gridlines scanned past the nearest one when closing a cell. A merged cell
    /// covering more than this many of the grid's own lines does not occur in a real table,
    /// and the bound keeps the search linear on a page whose corners never close.
    /// </summary>
    private const int MaxSpannedGridLines = 64;

    /// <summary>
    /// Fraction of a horizontal stroke that must run over glyphs before it is read as text
    /// decoration rather than a ruling.
    /// </summary>
    private const double DecorationMinTextCoverage = 0.5;

    /// <summary>
    /// How far inside a word's own box a stroke must sit to be crossing it. A ruling line sits
    /// on the row boundary, which is the very edge of the adjacent words' boxes (measured on
    /// tracked-changes documents: the descender edge, 0.96 and up); a strikethrough sits near
    /// the middle (0.25 - 0.35). The band between those two is wide, so the margin is chosen
    /// well clear of both ends rather than tuned to either.
    /// </summary>
    private const double DecorationBandMargin = 0.15;

    /// <summary>Height of one bucket of the word index the decoration filter probes.</summary>
    private const double DecorationBucketHeight = 8.0;

    /// <summary>Horizontal slack for the fragment consolidation's X-start and width match.</summary>
    /// <remarks>
    /// The consolidation these tolerances serve is not reachable yet, and deliberately so.
    /// pdf_oxide calls it in one place only — inside the horizontal-rule-bounded branch, over
    /// that branch's fragments and with a vertical tolerance scaled to their median row height
    /// (`spatial_table_detector.rs:3809`). That branch is unported. Running it over the
    /// Lines/Lines output at the abutting-fragment default instead merges tables upstream keeps
    /// apart: measured, it cost 22 `ok` and 24 `tables` on the PDF corpus. The helpers stay for
    /// whoever ports `detect_tables_from_horizontal_rules`; the call does not.
    /// </remarks>
    private const double FragmentMergeXTol = 2.0;

    /// <summary>Vertical gap a pair of abutting fragments may leave between them.</summary>
    private const double FragmentMergeYTol = 3.0;
    private const double SectionDividerWidthRatio = 0.80;
    private const double AdjacentTableMergeGap = 20.0;
    private const int MergeColDiffTolerance = 2;

    /// <summary>
    /// The words <c>extract_words</c> produces from a page's spans, in the shape the
    /// detector consumes. The clustering, the whitespace split and the adjacency merge all
    /// live in <see cref="OxWordExtraction"/>; this only re-shapes the result.
    /// </summary>
    /// <remarks>
    /// The spans reaching here are the word path's own — postprocessed and ordered by
    /// <see cref="XRay.Content.Internal.PdfOxide.Layout.OxPageOrder.PageReadingOrder"/> — so the
    /// page /Rotate mapping and the rotated reading frame have already been applied to their
    /// geometry. This is not the plain-text path's span list.
    /// </remarks>
    public static List<TableSpan> WordsFromOxSpans(IReadOnlyList<OxTextSpan> spans)
    {
        var words = new List<TableSpan>();
        foreach (var w in OxWordExtraction.ExtractWords(spans))
        {
            words.Add(new TableSpan
            {
                Text = w.Text,
                Bbox = new PathRect(w.Bbox.X, w.Bbox.Y, w.Bbox.Width, w.Bbox.Height),
                FontSize = w.AvgFontSize,
            });
        }
        return words;
    }

    /// <summary>
    /// Split Tj-run spans into word-sized ones. The fallback for the pages the ported
    /// extractor did not run on, which have no per-glyph geometry to cluster: widths are
    /// apportioned by UTF-8 byte offset instead.
    /// </summary>
    public static List<TableSpan> SpansToWords(List<TextSpan> spans)
    {
        var words = new List<TableSpan>();
        foreach (var span in spans)
        {
            string text = span.Text;
            if (text.Trim().Length == 0) continue;

            bool hasInteriorWhitespace = false;
            string trimmed = text.Trim();
            foreach (char c in trimmed) if (char.IsWhiteSpace(c)) { hasInteriorWhitespace = true; break; }
            if (!hasInteriorWhitespace)
            {
                words.Add(new TableSpan
                {
                    Text = trimmed,
                    Bbox = new PathRect(span.X, span.Y, span.Width, span.Height),
                    FontSize = span.FontSize,
                });
                continue;
            }

            byte[] full = Encoding.UTF8.GetBytes(text);
            double totalBytes = full.Length;
            if (totalBytes <= 0) continue;
            int searchStart = 0;
            foreach (string word in text.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries))
            {
                byte[] wb = Encoding.UTF8.GetBytes(word);
                int offset = IndexOfBytes(full, wb, searchStart);
                if (offset < 0) continue;
                searchStart = offset + wb.Length;
                words.Add(new TableSpan
                {
                    Text = word,
                    Bbox = new PathRect(
                        span.X + span.Width * (offset / totalBytes),
                        span.Y,
                        span.Width * (wb.Length / totalBytes),
                        span.Height),
                    FontSize = span.FontSize,
                });
            }
        }
        return MergeAdjacentWordPieces(words);
    }

    /// <summary>
    /// Glue word pieces that only exist because the producer emitted them as separate
    /// text-showing operators. pdf_oxide's `extract_words` builds words from glyphs, so
    /// `1,011` reaches the detector whole; our spans arrive already cut at every Tj, and
    /// the cell writer joins spans with a space.
    /// </summary>
    private static List<TableSpan> MergeAdjacentWordPieces(List<TableSpan> words)
    {
        if (words.Count < 2) return words;
        var merged = new List<TableSpan>(words.Count) { words[0] };
        for (int i = 1; i < words.Count; i++)
        {
            var prev = merged[^1];
            var cur = words[i];
            double gap = cur.Bbox.X - (prev.Bbox.X + prev.Bbox.Width);
            double fontSize = Math.Max(Math.Max(prev.FontSize, cur.FontSize), 1.0);
            bool sameLine = Math.Abs(prev.Bbox.Y - cur.Bbox.Y) <= 2.0;
            if (sameLine && gap <= fontSize * 0.15 && gap > -fontSize)
            {
                double left = Math.Min(prev.Bbox.X, cur.Bbox.X);
                double right = Math.Max(prev.Bbox.X + prev.Bbox.Width, cur.Bbox.X + cur.Bbox.Width);
                merged[^1] = new TableSpan
                {
                    Text = prev.Text + cur.Text,
                    Bbox = new PathRect(left, Math.Min(prev.Bbox.Y, cur.Bbox.Y), right - left,
                        Math.Max(prev.Bbox.Height, cur.Bbox.Height)),
                    FontSize = fontSize,
                };
                continue;
            }
            merged.Add(cur);
        }
        return merged;
    }

    private static int IndexOfBytes(byte[] haystack, byte[] needle, int start)
    {
        if (needle.Length == 0) return -1;
        for (int i = start; i + needle.Length <= haystack.Length; i++)
        {
            bool match = true;
            for (int j = 0; j < needle.Length; j++)
                if (haystack[i + j] != needle[j]) { match = false; break; }
            if (match) return i;
        }
        return -1;
    }

    /// <summary>
    /// One page's ruled tables, in the shape xberg's `Table` type wants.
    /// Mirrors `extract_tables_native` / `extract_tables_bordered` after their
    /// shared `detect_tables_with_lines` + `convert_extracted_table` steps.
    /// </summary>
    public static List<Table> DetectPageTables(
        List<TableSpan> spans, List<PdfPath> paths, uint pageNumber, TableDetectionConfig config)
    {
        var result = new List<Table>();
        foreach (var ruled in DetectPageRuledTables(spans, paths, pageNumber, config))
            if (!ruled.IsContinuationCandidate) result.Add(ruled.Table);
        return result;
    }

    /// <summary>
    /// One page's ruled tables with the grid geometry behind them, plus the well-formed bands
    /// that fell below the size floor. The geometry is what lets
    /// <see cref="PdfTableContinuation"/> tell a table's continuation on the next page from an
    /// unrelated table that merely starts there; the small bands are what a page break leaves
    /// of a table whose rows are all on the other side of it.
    /// </summary>
    public static List<RuledTable> DetectPageRuledTables(
        List<TableSpan> spans, List<PdfPath> paths, uint pageNumber, TableDetectionConfig config)
    {
        var result = new List<RuledTable>();
        if (spans.Count == 0) return result;

        // A page carrying thousands of line/rect paths is a drawing or a chart, not a ruled
        // table; the collinear-join and intersection sweeps that follow are O(E²), and no real
        // ruled table has more than a few hundred edges.
        int edgeCount = 0;
        foreach (var p in paths)
            if (IsHorizontalLine(p, LineAxisTol) || IsVerticalLine(p, LineAxisTol) || IsRectangle(p))
                edgeCount++;
        if (edgeCount > MaxTableEdges) return result;

        var lines = new List<PdfPath>();
        foreach (var p in paths) if (p.IsTablePrimitive()) lines.Add(p);
        lines = WithoutTextDecoration(lines, spans);
        if (lines.Count == 0) return result;

        var detected = DetectTablesWithLines(spans, lines, config);
        foreach (var t in detected)
        {
            // document.rs applies the same prose-rejection filter this public API gets. The
            // size-shaped half of it — two rows, two filled cells to a row — is what a
            // continuation candidate is exempt from, since being too small is what makes it
            // one; the prose rejection still applies to both.
            if (LooksLikeProseTable(t)) continue;
            if (!t.BelowThreshold && !IsRealGrid(t)) continue;
            if (t.Rows.Count == 0 || t.ColCount == 0) continue;

            var (cells, markdown) = ConvertExtractedTable(t, spans);
            if (cells.Count == 0 || markdown.Trim().Length == 0) continue;
            if (!t.BelowThreshold && (cells.Count < 2 || cells.All(r => r.Count < 2))) continue;
            if (t.BelowThreshold && cells.All(r => r.Count < 2)) continue;

            result.Add(new RuledTable(
                new Table
                {
                    Cells = cells,
                    Markdown = markdown,
                    PageNumber = pageNumber,
                    // Single precision: the reference's far edges are the f32 sums of f32 edges.
                    BoundingBox = t.Bbox is { } bb
                        ? new BoundingBox
                        {
                            X0 = bb.X,
                            Y0 = bb.Y,
                            X1 = (float)bb.X + (float)bb.Width,
                            Y1 = (float)bb.Y + (float)bb.Height,
                        }
                        : null,
                },
                t.ColumnBoundaries,
                t.BelowThreshold));
        }
        return result;
    }

    /// <summary>The Lines/Lines branch of `detect_tables_with_lines`.</summary>
    private static List<GridTable> DetectTablesWithLines(
        List<TableSpan> spans, List<PdfPath> lines, TableDetectionConfig config)
    {
        var tables = DetectTablesFromIntersections(spans, lines, config);
        // Continuation candidates count for nothing here. Whether the intersection tier found
        // a grid at all, and so whether the clustering below runs, is decided on the real
        // tables exactly as it was before candidates existed — a band too small to be a table
        // is also too small to stand in for one. They ride along with whichever branch wins.
        var candidates = tables.Where(t => t.BelowThreshold && IsValidTable(t)).ToList();
        var real = tables.Where(t => !t.BelowThreshold).ToList();
        if (real.Count > 0)
        {
            var kept = real.Where(IsValidTable).ToList();
            kept.AddRange(candidates);
            return kept;
        }

        // A table can be ruled without its rules ever meeting: separate horizontal and
        // vertical strokes that stop short of each other leave no corner for intersection
        // detection to find. Clustering the rules' own coordinates still yields the grid.
        var clustered = DetectTablesInClusters(spans, lines, config);
        clustered.AddRange(candidates);
        return clustered;
    }

    // ── Edges ────────────────────────────────────────────────────────────────

    private struct Edge
    {
        /// <summary>For H edges the shared y; for V edges the shared x.</summary>
        public double Coord;
        public double Start;
        public double End;
    }

    /// <summary>
    /// Drop the horizontal strokes that are text decoration rather than ruling: a
    /// strikethrough is a thin line like any other, and one per struck-out cell is enough to
    /// shred a grid.
    /// </summary>
    /// <remarks>
    /// A tracked-changes document draws a strikethrough over every deleted cell, and those
    /// strokes reach the same edge list as the table's rules. They cross a column rule often
    /// enough to raise an intersection, so the grid gains a row boundary at every struck-out
    /// line; the rows that boundary invents are mostly empty, and the emptiness filters then
    /// discard the table itself. The discriminator is geometric and needs no heuristic about
    /// what a table looks like: <em>a ruling line does not pass through glyphs</em>. A stroke
    /// most of whose length runs over words, at a height inside those words' own boxes rather
    /// than at their edge, is decoration.
    /// <para>
    /// An underline is deliberately left alone: it sits where a rule sits — on the text's
    /// bottom edge — so the same test cannot separate the two, and dropping a real rule costs
    /// more than keeping a stray underline.
    /// </para>
    /// </remarks>
    private static List<PdfPath> WithoutTextDecoration(List<PdfPath> lines, List<TableSpan> spans)
    {
        if (spans.Count == 0) return lines;

        // Words bucketed by height, so each stroke only measures itself against the words it
        // could possibly cross rather than against the whole page.
        var buckets = new Dictionary<int, List<int>>();
        for (int i = 0; i < spans.Count; i++)
        {
            var b = spans[i].Bbox;
            if (b.Bottom <= b.Top) continue;
            int lo = (int)Math.Floor(b.Top / DecorationBucketHeight);
            int hi = (int)Math.Floor(b.Bottom / DecorationBucketHeight);
            for (int k = lo; k <= hi; k++)
            {
                if (!buckets.TryGetValue(k, out var list)) buckets[k] = list = new List<int>();
                list.Add(i);
            }
        }

        var kept = new List<PdfPath>(lines.Count);
        foreach (var path in lines)
        {
            if (IsHorizontalLine(path, LineAxisTol) && IsTextDecoration(path, spans, buckets)) continue;
            kept.Add(path);
        }
        return kept;
    }

    private static bool IsTextDecoration(
        PdfPath path, List<TableSpan> spans, Dictionary<int, List<int>> buckets)
    {
        var r = path.RenderedBbox();
        double left = r.Left, right = r.Right, y = r.CenterY;
        double length = right - left;
        if (length < MinEdgeLen) return false;
        if (!buckets.TryGetValue((int)Math.Floor(y / DecorationBucketHeight), out var candidates)) return false;

        // The x-intervals over which the stroke runs across a word at a height inside that
        // word's box, merged so overlapping words are not counted twice.
        var covered = new List<(double Start, double End)>();
        foreach (int i in candidates)
        {
            var b = spans[i].Bbox;
            double height = b.Bottom - b.Top;
            if (height <= 0) continue;
            double inset = height * DecorationBandMargin;
            if (y <= b.Top + inset || y >= b.Bottom - inset) continue;
            double start = Math.Max(left, b.Left), end = Math.Min(right, b.Right);
            if (end > start) covered.Add((start, end));
        }
        if (covered.Count == 0) return false;

        covered.Sort((a, b) => a.Start.CompareTo(b.Start));
        double total = 0.0, runStart = covered[0].Start, runEnd = covered[0].End;
        for (int i = 1; i < covered.Count; i++)
        {
            if (covered[i].Start <= runEnd) runEnd = Math.Max(runEnd, covered[i].End);
            else { total += runEnd - runStart; runStart = covered[i].Start; runEnd = covered[i].End; }
        }
        total += runEnd - runStart;
        return total / length >= DecorationMinTextCoverage;
    }

    private static (List<Edge> H, List<Edge> V) ExtractEdges(List<PdfPath> lines)
    {
        var h = new List<Edge>();
        var v = new List<Edge>();
        foreach (var path in lines)
        {
            var bbox = path.Bbox;
            if (IsHorizontalLine(path, LineAxisTol))
            {
                // Rendered extents, so a stroke-width-encoded rule contributes the edge
                // its drawn bar covers rather than its geometric speck.
                var r = path.RenderedBbox();
                h.Add(new Edge { Coord = r.CenterY, Start = r.Left, End = r.Right });
            }
            else if (IsVerticalLine(path, LineAxisTol))
            {
                var r = path.RenderedBbox();
                v.Add(new Edge { Coord = r.CenterX, Start = r.Top, End = r.Bottom });
            }
            else if (IsRectangle(path))
            {
                double l = bbox.Left, rr = bbox.Right, t = bbox.Top, b = bbox.Bottom;
                h.Add(new Edge { Coord = t, Start = l, End = rr });
                h.Add(new Edge { Coord = b, Start = l, End = rr });
                v.Add(new Edge { Coord = l, Start = t, End = b });
                v.Add(new Edge { Coord = rr, Start = t, End = b });
            }
        }
        return (h, v);
    }

    private static bool IsHorizontalLine(PdfPath p, double tolerance)
    {
        if (!p.IsStraightLine && !IsRectangle(p)) return false;
        var r = p.RenderedBbox();
        return Math.Abs(p.Bbox.Height) < tolerance && Math.Abs(r.Width) >= Math.Abs(r.Height);
    }

    private static bool IsVerticalLine(PdfPath p, double tolerance)
    {
        if (!p.IsStraightLine && !IsRectangle(p)) return false;
        var r = p.RenderedBbox();
        return Math.Abs(p.Bbox.Width) < tolerance && Math.Abs(r.Height) >= Math.Abs(r.Width);
    }

    private static bool IsRectangle(PdfPath p)
    {
        var ops = p.Operations;
        if (ops.Count == 1 && ops[0].Kind == PathOpKind.Rectangle) return true;
        if ((ops.Count == 5 && ops[4].Kind == PathOpKind.ClosePath) || ops.Count == 4)
        {
            if (ops[0].Kind != PathOpKind.MoveTo || ops[1].Kind != PathOpKind.LineTo
                || ops[2].Kind != PathOpKind.LineTo || ops[3].Kind != PathOpKind.LineTo)
                return false;
            const double tol = 0.1;
            bool s1 = Math.Abs(ops[0].X1 - ops[1].X1) < tol || Math.Abs(ops[0].Y1 - ops[1].Y1) < tol;
            bool s2 = Math.Abs(ops[1].X1 - ops[2].X1) < tol || Math.Abs(ops[1].Y1 - ops[2].Y1) < tol;
            bool s3 = Math.Abs(ops[2].X1 - ops[3].X1) < tol || Math.Abs(ops[2].Y1 - ops[3].Y1) < tol;
            return s1 && s2 && s3;
        }
        return false;
    }

    private static void SnapAndMerge(List<Edge> edges)
    {
        SnapEdges(edges);
        JoinCollinearEdges(edges);
        ReconstituteDottedLines(edges);
    }

    /// <summary>Sort by coord and snap nearby coordinates onto the first of each group.</summary>
    /// <remarks>
    /// A group extends while each coordinate is within <see cref="SnapTol"/> of the one before
    /// it, not only of the group's first, and stops at <see cref="SnapSpreadTol"/> overall.
    /// Measuring from the first alone splits a cluster that straddles the tolerance: a rule at
    /// 70.5 and the shading rectangle drawn against it at 70.8 belong together, but with a
    /// third edge at 67.5 sorting ahead of them the rule joins that one and the shading, 3.3
    /// away from it, becomes a column boundary of its own — which is a phantom column down the
    /// whole table. Chaining from the previous coordinate keeps the cluster whole, and the
    /// spread bound is what stops a page of closely spaced strokes from chaining into one.
    /// </remarks>
    private static void SnapEdges(List<Edge> edges)
    {
        if (edges.Count == 0) return;
        edges.Sort((a, b) => a.Coord.CompareTo(b.Coord));
        int i = 0;
        while (i < edges.Count)
        {
            double baseCoord = edges[i].Coord;
            double previous = baseCoord;
            int j = i + 1;
            while (j < edges.Count
                   && edges[j].Coord - previous <= SnapTol
                   && edges[j].Coord - baseCoord <= SnapSpreadTol)
            {
                previous = edges[j].Coord;
                var e = edges[j];
                e.Coord = baseCoord;
                edges[j] = e;
                j++;
            }
            i = j;
        }
    }

    /// <summary>Merge overlapping or adjacent collinear segments into single edges.</summary>
    private static void JoinCollinearEdges(List<Edge> edges)
    {
        if (edges.Count == 0) return;
        edges.Sort((a, b) =>
        {
            int c = a.Coord.CompareTo(b.Coord);
            return c != 0 ? c : a.Start.CompareTo(b.Start);
        });

        var merged = new List<Edge>();
        foreach (var edge in edges)
        {
            // Compare coords with SnapTol, not epsilon: edges snapped from slightly
            // different originals must still join.
            bool shouldMerge = merged.Count > 0
                && Math.Abs(merged[^1].Coord - edge.Coord) <= SnapTol
                && edge.Start <= merged[^1].End + JoinTol;
            if (shouldMerge)
            {
                var prev = merged[^1];
                prev.End = Math.Max(prev.End, edge.End);
                merged[^1] = prev;
            }
            else merged.Add(edge);
        }

        edges.Clear();
        edges.AddRange(merged);
    }

    /// <summary>
    /// Replace a run of short collinear segments with the long edge they trace, and
    /// discard short segments that do not qualify.
    /// </summary>
    private static void ReconstituteDottedLines(List<Edge> edges)
    {
        var dotted = new Dictionary<int, List<Edge>>();
        var longEdges = new List<Edge>();
        foreach (var edge in edges)
        {
            if (edge.End - edge.Start >= MinEdgeLen) longEdges.Add(edge);
            else
            {
                int key = (int)Math.Round(edge.Coord * DottedCoordSnap, MidpointRounding.AwayFromZero);
                if (!dotted.TryGetValue(key, out var list)) dotted[key] = list = new List<Edge>();
                list.Add(edge);
            }
        }

        // Sorted key order keeps the reconstituted edges — and so the cell and table
        // order downstream — independent of hash iteration order.
        var keys = dotted.Keys.ToList();
        keys.Sort();
        foreach (int key in keys)
        {
            var segments = dotted[key];
            if (segments.Count < DottedMinSegments) continue;
            double minStart = segments.Min(e => e.Start);
            double maxEnd = segments.Max(e => e.End);
            if (maxEnd - minStart >= DottedMinSpan)
                longEdges.Add(new Edge { Coord = segments[0].Coord, Start = minStart, End = maxEnd });
        }

        edges.Clear();
        edges.AddRange(longEdges);
    }

    /// <summary>
    /// Drop edges with no plausible counterpart on the other axis. Purely an X-range
    /// overlap check: the extended grid projects V positions across all H positions
    /// regardless of whether they share a Y range.
    /// </summary>
    private static void FilterEdgesByCoverage(List<Edge> hEdges, List<Edge> vEdges)
    {
        double allXMin = double.PositiveInfinity, allXMax = double.NegativeInfinity;
        foreach (var e in hEdges) { allXMin = Math.Min(allXMin, e.Start); allXMax = Math.Max(allXMax, e.End); }
        foreach (var e in vEdges) { allXMin = Math.Min(allXMin, e.Coord); allXMax = Math.Max(allXMax, e.Coord); }
        double xSpan = Math.Max(allXMax - allXMin, 1.0);
        double xTol = xSpan * 0.5;

        var vSnapshot = new List<Edge>(vEdges);
        hEdges.RemoveAll(h => !vSnapshot.Any(v => v.Coord >= h.Start - xTol && v.Coord <= h.End + xTol));
        var hSnapshot = new List<Edge>(hEdges);
        vEdges.RemoveAll(v => !hSnapshot.Any(h => v.Coord >= h.Start - xTol && v.Coord <= h.End + xTol));
    }

    private readonly struct Intersection
    {
        public readonly double X, Y;
        public Intersection(double x, double y) { X = x; Y = y; }
    }

    private static List<Intersection> FindIntersections(List<Edge> hEdges, List<Edge> vEdges)
    {
        var pts = new List<Intersection>();
        int hIndex = 0;
        foreach (var h in hEdges)
        {
            Cancel.Every(hIndex++, 0x3F);
            foreach (var v in vEdges)
                if (v.Coord >= h.Start - SnapTol && v.Coord <= h.End + SnapTol
                    && h.Coord >= v.Start - SnapTol && h.Coord <= v.End + SnapTol)
                    pts.Add(new Intersection(v.Coord, h.Coord));
        }

        pts.Sort((a, b) =>
        {
            int c = a.X.CompareTo(b.X);
            return c != 0 ? c : a.Y.CompareTo(b.Y);
        });
        var deduped = new List<Intersection>();
        foreach (var p in pts)
        {
            if (deduped.Count > 0
                && Math.Abs(p.X - deduped[^1].X) <= SnapTol && Math.Abs(p.Y - deduped[^1].Y) <= SnapTol)
                continue;
            deduped.Add(p);
        }
        return deduped;
    }

    private readonly struct IntersectionCell
    {
        public readonly double X1, Y1, X2, Y2;
        public IntersectionCell(double x1, double y1, double x2, double y2) { X1 = x1; Y1 = y1; X2 = x2; Y2 = y2; }
    }

    private static List<double> UniqueSorted(IEnumerable<double> values)
    {
        var list = values.ToList();
        list.Sort();
        var outList = new List<double>();
        foreach (double v in list)
            if (outList.Count == 0 || Math.Abs(v - outList[^1]) > SnapTol) outList.Add(v);
        return outList;
    }

    /// <summary>
    /// A cell is the smallest rectangle of gridlines whose four corners are all present.
    /// </summary>
    /// <remarks>
    /// Closing a cell against the <em>nearest</em> line on each axis only finds the cells of a
    /// fully ruled grid. Where cells are merged it finds nothing at all: a row rule that stops
    /// at the column it splits leaves the neighbouring tall cell's nearest bottom-right corner
    /// missing, so the tall cell is never built and every word inside it is dropped on the
    /// floor. Scanning on to the next candidate line recovers exactly those cells — a rowspan
    /// or a colspan — and changes nothing where the nearest lines already close, which is the
    /// ordinary case.
    /// </remarks>
    private static List<IntersectionCell> BuildCellsFromIntersections(List<Intersection> pts)
    {
        var xs = UniqueSorted(pts.Select(p => p.X));
        var ys = UniqueSorted(pts.Select(p => p.Y));
        int nx = xs.Count, ny = ys.Count;
        var present = new HashSet<int>();
        foreach (var p in pts)
        {
            int xi = xs.FindIndex(c => Math.Abs(c - p.X) <= SnapTol);
            int yi = ys.FindIndex(c => Math.Abs(c - p.Y) <= SnapTol);
            if (xi >= 0 && yi >= 0) present.Add(yi * nx + xi);
        }

        bool Has(int xi, int yi) => present.Contains(yi * nx + xi);

        // The cell closes at the nearest gridline on each axis wherever it can. Where it
        // cannot, the missing corner means the cell is merged across one axis, and the search
        // runs on along that axis alone: a rowspan keeps the nearest right edge and looks
        // further down, a colspan keeps the nearest bottom edge and looks further right.
        // Neither axis is extended without the other held fixed — a rectangle free to grow on
        // both would swallow whatever short band sits beside it.
        (int X, int Y)? ClosingCorner(int xi, int yi)
        {
            int right = -1;
            for (int k = xi + 1; k < nx; k++) if (Has(k, yi)) { right = k; break; }
            int bottom = -1;
            for (int m = yi + 1; m < ny; m++) if (Has(xi, m)) { bottom = m; break; }
            if (right < 0 || bottom < 0) return null;
            if (Has(right, bottom)) return (right, bottom);

            int steps = 0;
            for (int m = bottom + 1; m < ny && steps < MaxSpannedGridLines; m++)
            {
                if (!Has(xi, m)) continue;
                steps++;
                if (Has(right, m)) return (right, m);
            }
            steps = 0;
            for (int k = right + 1; k < nx && steps < MaxSpannedGridLines; k++)
            {
                if (!Has(k, yi)) continue;
                steps++;
                if (Has(k, bottom)) return (k, bottom);
            }
            return null;
        }

        var cells = new List<IntersectionCell>();
        for (int yi = 0; yi < ny; yi++)
        {
            for (int xi = 0; xi < nx; xi++)
            {
                if (!Has(xi, yi)) continue;
                if (ClosingCorner(xi, yi) is { } corner)
                    cells.Add(new IntersectionCell(xs[xi], ys[yi], xs[corner.X], ys[corner.Y]));
            }
        }
        return cells;
    }

    /// <summary>
    /// Cartesian product of H-edge Y positions and V-edge X positions, for pages whose
    /// horizontal and vertical rules do not physically cross.
    /// </summary>
    private static List<IntersectionCell> BuildExtendedGridCells(List<Edge> hEdges, List<Edge> vEdges)
    {
        var ys = UniqueSorted(hEdges.Select(e => e.Coord));
        var xs = UniqueSorted(vEdges.Select(e => e.Coord));
        var cells = new List<IntersectionCell>();
        if (xs.Count < 2 || ys.Count < 2) return cells;
        for (int yi = 0; yi < ys.Count - 1; yi++)
            for (int xi = 0; xi < xs.Count - 1; xi++)
                cells.Add(new IntersectionCell(xs[xi], ys[yi], xs[xi + 1], ys[yi + 1]));
        return cells;
    }

    private sealed class UnionFind
    {
        private readonly int[] _parent;
        public UnionFind(int n) { _parent = new int[n]; for (int i = 0; i < n; i++) _parent[i] = i; }
        public int Find(int i) { while (_parent[i] != i) { _parent[i] = _parent[_parent[i]]; i = _parent[i]; } return i; }
        public void Union(int a, int b) { int ra = Find(a), rb = Find(b); if (ra != rb) _parent[rb] = ra; }
    }

    /// <summary>Group cells that share an edge into tables.</summary>
    private static List<List<int>> GroupCellsIntoTables(List<IntersectionCell> cells)
    {
        var groups = new List<List<int>>();
        if (cells.Count == 0) return groups;
        int n = cells.Count;
        var uf = new UnionFind(n);

        // Both adjacency tests need the cells' y-extents to touch within SnapTol, so
        // sweeping in ascending-y1 order lets the inner loop break once a candidate's
        // y1 clears ci.y2 + SnapTol. Union is order-independent, so the partition is
        // identical to the full quadratic scan.
        var order = Enumerable.Range(0, n).ToList();
        order.Sort((a, b) => cells[a].Y1.CompareTo(cells[b].Y1));
        for (int a = 0; a < n; a++)
        {
            Cancel.Every(a, 0x3F);
            int i = order[a];
            var ci = cells[i];
            double yLimit = ci.Y2 + SnapTol;
            for (int bIdx = a + 1; bIdx < n; bIdx++)
            {
                int j = order[bIdx];
                var cj = cells[j];
                if (cj.Y1 > yLimit) break;
                bool sharesEdge =
                    ((Math.Abs(ci.X2 - cj.X1) <= SnapTol || Math.Abs(ci.X1 - cj.X2) <= SnapTol)
                     && Math.Abs(ci.Y1 - cj.Y1) <= SnapTol && Math.Abs(ci.Y2 - cj.Y2) <= SnapTol)
                    || ((Math.Abs(ci.Y2 - cj.Y1) <= SnapTol || Math.Abs(ci.Y1 - cj.Y2) <= SnapTol)
                        && Math.Abs(ci.X1 - cj.X1) <= SnapTol && Math.Abs(ci.X2 - cj.X2) <= SnapTol);
                if (sharesEdge) uf.Union(i, j);
            }
        }

        var byRoot = new Dictionary<int, List<int>>();
        for (int i = 0; i < n; i++)
        {
            int r = uf.Find(i);
            if (!byRoot.TryGetValue(r, out var list)) byRoot[r] = list = new List<int>();
            list.Add(i);
        }
        groups.AddRange(byRoot.Values);
        groups.Sort((a, b) => (a.Count > 0 ? a[0] : int.MaxValue).CompareTo(b.Count > 0 ? b[0] : int.MaxValue));
        return groups;
    }

    // ── Grid assembly ────────────────────────────────────────────────────────

    private static List<(List<IntersectionCell> Cells, List<double> Xs, List<double> Ys, int NumCols)>
        BuildGridFromLines(List<PdfPath> lines, TableDetectionConfig config)
    {
        var result = new List<(List<IntersectionCell>, List<double>, List<double>, int)>();
        var (hEdges, vEdges) = ExtractEdges(lines);
        SnapAndMerge(hEdges);
        SnapAndMerge(vEdges);
        if (hEdges.Count < 2 || vEdges.Count < 2) return result;

        var intersections = FindIntersections(hEdges, vEdges);

        // Sparse intersections mean the grid is being inferred, so drop orphan edges
        // (decorative lines far from the table) before projecting the extended grid.
        if (intersections.Count < 4)
        {
            FilterEdgesByCoverage(hEdges, vEdges);
            if (hEdges.Count < 2 || vEdges.Count < 2) return result;
        }

        List<IntersectionCell> cells;
        if (intersections.Count >= 4)
        {
            cells = BuildCellsFromIntersections(intersections);
            if (cells.Count == 0) cells = BuildExtendedGridCells(hEdges, vEdges);
        }
        else cells = BuildExtendedGridCells(hEdges, vEdges);
        if (cells.Count == 0) return result;

        foreach (var group in GroupCellsIntoTables(cells))
        {
            var groupCells = group.Select(i => cells[i]).ToList();
            var xs = UniqueSorted(groupCells.SelectMany(c => new[] { c.X1, c.X2 }));
            var ys = UniqueSorted(groupCells.SelectMany(c => new[] { c.Y1, c.Y2 }));
            if (xs.Count < 2 || ys.Count < 2) continue;
            int numCols = xs.Count - 1;
            if (numCols < config.MinTableColumns || numCols > config.MaxTableColumns) continue;
            result.Add((groupCells, xs, ys, numCols));
        }
        return result;
    }

    /// <summary>Interval containing <paramref name="point"/>; internal boundaries belong to the right.</summary>
    private static int GridIntervalForPoint(double point, List<double> boundaries)
    {
        int intervalCount = boundaries.Count - 1;
        if (intervalCount <= 0 || double.IsNaN(point) || double.IsInfinity(point)) return -1;
        if (point < boundaries[0]) return boundaries[0] - point <= SnapTol ? 0 : -1;
        if (point > boundaries[intervalCount])
            return point - boundaries[intervalCount] <= SnapTol ? intervalCount - 1 : -1;
        for (int index = 0; index < intervalCount; index++)
            if (point >= boundaries[index]
                && (point < boundaries[index + 1] || (index + 1 == intervalCount && point <= boundaries[index + 1])))
                return index;
        return -1;
    }

    private static (List<GridRow> Rows, List<List<List<int>>> SpanIndices)? AssignSpansToIntersectionGrid(
        List<IntersectionCell> groupCells, List<double> xs, List<double> ys, int numCols, List<TableSpan> spans)
    {
        int numRows = ys.Count - 1;
        if (numRows < 1) return null;

        int ColOf(double x) { for (int c = 0; c < numCols; c++) if (Math.Abs(xs[c] - x) <= SnapTol) return c; return -1; }
        int RowOf(double y) { for (int r = 0; r < numRows; r++) if (Math.Abs(ys[r] - y) <= SnapTol) return r; return -1; }
        int XBound(double x) { for (int c = 0; c < xs.Count; c++) if (Math.Abs(xs[c] - x) <= SnapTol) return c; return -1; }
        int YBound(double y) { for (int r = 0; r < ys.Count; r++) if (Math.Abs(ys[r] - y) <= SnapTol) return r; return -1; }

        // A merged cell covers several of the grid's row or column intervals, but holds its
        // text once. `anchor` maps every interval a cell covers back to the cell's first
        // interval in reading order — its top-left, which in these bottom-origin coordinates
        // is the *highest* y it touches — so a word sitting in the lower half of a vertically
        // merged cell is filed under the cell rather than dropped for landing where the grid
        // has no cell of its own. The origin keeping the text while the intervals it spans
        // stay empty is the placement rule
        // <see cref="XRay.Content.Internal.Tables.GridFlatten"/> applies to every other
        // format's merged cells.
        var gridHasCell = new bool[numRows, numCols];
        var anchor = new (int Row, int Col)?[numRows, numCols];
        foreach (var c in groupCells)
        {
            int ci = ColOf(c.X1), ciEnd = XBound(c.X2);
            int riLow = RowOf(c.Y1), riEnd = YBound(c.Y2);
            if (ci < 0 || riLow < 0) continue;
            // Both far edges are snapped coordinates, so a cell thinner than the snap
            // tolerance has its two edges on the same boundary. Such a cell still occupies the
            // one interval it starts in, and clamping is what keeps its anchor inside itself
            // rather than one row above, where it would shadow the row that really is there.
            ciEnd = Math.Max(ciEnd, ci + 1);
            riEnd = Math.Max(riEnd, riLow + 1);
            int riTop = Math.Clamp(riEnd - 1, riLow, numRows - 1);
            gridHasCell[riTop, ci] = true;
            for (int r = riLow; r < riEnd && r < numRows; r++)
                for (int col = ci; col < ciEnd && col < numCols; col++)
                    anchor[r, col] ??= (riTop, ci);
        }

        var gridSpans = new List<int>[numRows, numCols];
        for (int r = 0; r < numRows; r++) for (int c = 0; c < numCols; c++) gridSpans[r, c] = new List<int>();
        for (int idx = 0; idx < spans.Count; idx++)
        {
            int ci = GridIntervalForPoint(spans[idx].CenterX, xs);
            int ri = GridIntervalForPoint(spans[idx].CenterY, ys);
            if (ci < 0 || ri < 0) continue;
            if (anchor[ri, ci] is not { } at) continue;
            gridSpans[at.Row, at.Col].Add(idx);
        }

        // Higher y is higher on the page, so rows read top-to-bottom means descending y.
        var rowOrder = Enumerable.Range(0, numRows).ToList();
        rowOrder.Sort((a, b) => ys[b].CompareTo(ys[a]));

        var rows = new List<GridRow>();
        var rowCellSpanIndices = new List<List<List<int>>>();
        foreach (int ri in rowOrder)
        {
            var row = new GridRow(false);
            var cellIndicesForRow = new List<List<int>>();
            for (int ci = 0; ci < numCols; ci++)
            {
                var bbox = new PathRect(xs[ci], ys[ri], xs[ci + 1] - xs[ci], ys[ri + 1] - ys[ri]);
                if (!gridHasCell[ri, ci])
                {
                    // Still emit the cell so the column count stays consistent.
                    row.Cells.Add(new GridCell { Text = "", Bbox = bbox });
                    cellIndicesForRow.Add(new List<int>());
                    continue;
                }
                row.Cells.Add(new GridCell
                {
                    Text = ExtractCellText(gridSpans[ri, ci], spans),
                    SpanIndices = new List<int>(gridSpans[ri, ci]),
                    Bbox = bbox,
                });
                cellIndicesForRow.Add(new List<int>(gridSpans[ri, ci]));
            }
            rows.Add(row);
            rowCellSpanIndices.Add(cellIndicesForRow);
        }
        return (rows, rowCellSpanIndices);
    }

    private static List<GridTable> DetectTablesFromIntersections(
        List<TableSpan> spans, List<PdfPath> lines, TableDetectionConfig config)
    {
        var tables = new List<GridTable>();
        foreach (var (groupCells, xs, ys, numCols) in BuildGridFromLines(lines, config))
        {
            var assigned = AssignSpansToIntersectionGrid(groupCells, xs, ys, numCols, spans);
            if (assigned is not { } a) continue;
            tables.AddRange(FinalizeIntersectionTables(a.Rows, a.SpanIndices, spans, config, numCols, xs));
        }

        // A candidate is offered to the cross-page join and to nothing else. Holding it back
        // from the merge and the divider split is what keeps it from changing how the page's
        // real tables assemble — which band merges into which is sensitive to what else is in
        // the list, and a band that is not a table has no business deciding that.
        var candidates = tables.Where(t => t.BelowThreshold).ToList();
        tables.RemoveAll(t => t.BelowThreshold);

        MergeVerticallyAdjacentTables(tables);

        // Split at section dividers using merged H-edges but only snapped V-edges:
        // joining V-edges would fuse per-section segments and hide the boundary.
        var (hEdges, vEdges) = ExtractEdges(lines);
        SnapAndMerge(hEdges);
        SnapEdges(vEdges);
        var split = SplitTablesAtSectionDividers(tables, hEdges, vEdges, config);
        split.AddRange(candidates);
        return split;
    }

    private static bool RowIsEmpty(GridRow r) => r.Cells.All(c => c.Text.Length == 0);

    private static PathRect? RowsBbox(List<GridRow> rows)
    {
        double minX = double.PositiveInfinity, minY = double.PositiveInfinity;
        double maxX = double.NegativeInfinity, maxY = double.NegativeInfinity;
        foreach (var r in rows)
            foreach (var c in r.Cells)
                if (c.Bbox is { } b)
                {
                    minX = Math.Min(minX, b.Left); minY = Math.Min(minY, b.Top);
                    maxX = Math.Max(maxX, b.Right); maxY = Math.Max(maxY, b.Bottom);
                }
        if (double.IsInfinity(minX)) return null;
        return new PathRect(minX, minY, maxX - minX, maxY - minY);
    }

    private static List<GridTable> FinalizeIntersectionTables(
        List<GridRow> rows, List<List<List<int>>> rowCellSpanIndices,
        List<TableSpan> spans, TableDetectionConfig config, int numCols, List<double> xs)
    {
        // A row holding text at several distinct Y positions has no horizontal rules
        // between its lines; split it on the text's own Y clustering.
        var tableRows = SplitRowsByTextPositions(rows, rowCellSpanIndices, spans, config);
        StripFormNumberingArtifacts(tableRows);

        var tables = new List<GridTable>();
        int subStart = 0;
        while (subStart < tableRows.Count)
        {
            if (RowIsEmpty(tableRows[subStart])) { subStart++; continue; }
            int subEnd = subStart + 1;
            while (subEnd < tableRows.Count && !RowIsEmpty(tableRows[subEnd])) subEnd++;
            var subRows = tableRows.GetRange(subStart, subEnd - subStart);
            int filled = subRows.SelectMany(r => r.Cells).Count(c => c.Text.Length > 0);
            // Below the floor the band is not a table on its own — but a table's first rows
            // can be all that fits before a page break, and a header row alone at the foot of
            // a page is exactly that shape. Keep it as a candidate continuation; it is dropped
            // unless a table on the neighbouring page claims it.
            if (filled >= config.MinTableCells || filled >= MinContinuationCells)
                tables.Add(new GridTable
                {
                    Rows = subRows,
                    ColCount = numCols,
                    Bbox = RowsBbox(subRows),
                    ColumnBoundaries = xs.ToArray(),
                    BelowThreshold = filled < config.MinTableCells,
                });
            subStart = subEnd;
        }
        return tables;
    }

    private static List<GridRow> SplitRowsByTextPositions(
        List<GridRow> rows, List<List<List<int>>> rowCellSpanIndices,
        List<TableSpan> spans, TableDetectionConfig config)
    {
        var clustersPerRow = new List<double>?[rows.Count];
        int multiBaseline = 0, stacked = 0;
        for (int rowIdx = 0; rowIdx < rows.Count; rowIdx++)
        {
            var clusters = BaselineClusters(rowCellSpanIndices[rowIdx], spans, config);
            if (clusters is null) continue;
            multiBaseline++;
            if (!ClustersLookLikeSeparateRows(rowCellSpanIndices[rowIdx], clusters, spans)) continue;
            stacked++;
            clustersPerRow[rowIdx] = clusters;
        }

        // Splitting is for the table that is under-ruled — one whose bands hold stacked
        // records because the rules between them were never drawn. A table whose bands mostly
        // hold wrapped text is not that table, and the odd band of its own that happens to
        // look stacked (a header wrapping to two lines in every column at once is the common
        // one) is far more likely to be wrapped too. So the decision is the table's, not the
        // band's: a minority of stacked-looking bands leaves every band alone.
        if (stacked * 2 < multiBaseline) return new List<GridRow>(rows);

        var result = new List<GridRow>();
        for (int rowIdx = 0; rowIdx < rows.Count; rowIdx++)
        {
            var row = rows[rowIdx];
            var cellIndices = rowCellSpanIndices[rowIdx];

            if (clustersPerRow[rowIdx] is not { } yClusters) { result.Add(row); continue; }

            // Descending: higher y is the top of the page, and reads first.
            yClusters.Sort((a, b) => b.CompareTo(a));

            int numCols = row.Cells.Count;
            foreach (double clusterY in yClusters)
            {
                var newRow = new GridRow(row.IsHeader);
                for (int ci = 0; ci < numCols; ci++)
                {
                    var matching = new List<int>();
                    foreach (int idx in cellIndices[ci])
                    {
                        if (idx < 0 || idx >= spans.Count) continue;
                        double sy = spans[idx].CenterY;
                        double nearest = yClusters[0];
                        double best = Math.Abs(sy - nearest);
                        foreach (double cy in yClusters)
                        {
                            double d = Math.Abs(sy - cy);
                            if (d < best) { best = d; nearest = cy; }
                        }
                        if (Math.Abs(nearest - clusterY) < 0.01) matching.Add(idx);
                    }

                    PathRect? cellBbox = row.Cells[ci].Bbox;
                    if (matching.Count > 0)
                    {
                        var b = spans[matching[0]].Bbox;
                        for (int k = 1; k < matching.Count; k++)
                        {
                            var o = spans[matching[k]].Bbox;
                            double l = Math.Min(b.Left, o.Left), t = Math.Min(b.Top, o.Top);
                            double rr = Math.Max(b.Right, o.Right), bo = Math.Max(b.Bottom, o.Bottom);
                            b = new PathRect(l, t, rr - l, bo - t);
                        }
                        cellBbox = b;
                    }

                    newRow.Cells.Add(new GridCell
                    {
                        Text = ExtractCellText(matching, spans),
                        SpanIndices = matching,
                        Bbox = cellBbox,
                        IsHeader = row.IsHeader,
                    });
                }
                result.Add(newRow);
            }
        }
        return result;
    }

    /// <summary>
    /// The distinct text baselines inside one rule-bounded band, or null when it holds at most
    /// one and so cannot be several rows.
    /// </summary>
    private static List<double>? BaselineClusters(
        List<List<int>> cellIndices, List<TableSpan> spans, TableDetectionConfig config)
    {
        var allYs = new List<double>();
        foreach (var colSpans in cellIndices)
            foreach (int idx in colSpans)
                if (idx >= 0 && idx < spans.Count) allYs.Add(spans[idx].CenterY);
        if (allYs.Count <= 1) return null;

        allYs.Sort();
        var yClusters = new List<double>();
        foreach (double y in allYs)
        {
            if (yClusters.Count > 0 && Math.Abs(y - yClusters[^1]) < config.RowTolerance)
                yClusters[^1] = (yClusters[^1] + y) / 2.0;
            else yClusters.Add(y);
        }
        return yClusters.Count > 1 ? yClusters : null;
    }

    /// <summary>
    /// Whether a rule-bounded row's text baselines are several records stacked in one cell
    /// band, rather than one record whose longest cell wrapped.
    /// </summary>
    /// <remarks>
    /// Both shapes put text at more than one height inside the same band, so the baseline count
    /// alone cannot tell them apart — and splitting a wrapped row is destructive: each wrapped
    /// line becomes a row of its own with every other column empty, which the emptiness filters
    /// downstream then read as prose and reject, taking the whole table with it.
    /// <para>
    /// What separates them is which baselines run across the band. Stacked records line up:
    /// each is a baseline carrying text in several columns at once, because that is what makes
    /// it a row, so <b>two or more baselines must span at least two columns</b>. A wrapped cell
    /// has only one — its extra lines belong to one column, and the neighbouring columns sit at
    /// their own heights, centred against it.
    /// </para>
    /// <para>
    /// A baseline carrying one column is not disqualifying on its own: a label merged across
    /// the rows beside it sits centred between them, on a height of its own, and that is the
    /// ordinary look of a row group. It only disqualifies when its column also carries text on
    /// one of the spanning baselines — then the extra line is that column's own continuation,
    /// which is wrapping.
    /// </para>
    /// </remarks>
    private static bool ClustersLookLikeSeparateRows(
        List<List<int>> cellIndices, List<double> yClusters, List<TableSpan> spans)
    {
        int columns = cellIndices.Count;
        var occupied = new bool[yClusters.Count, columns];
        for (int ci = 0; ci < columns; ci++)
            foreach (int idx in cellIndices[ci])
            {
                if (idx < 0 || idx >= spans.Count) continue;
                int nearest = NearestClusterIndex(spans[idx].CenterY, yClusters);
                if (nearest >= 0) occupied[nearest, ci] = true;
            }

        var spansTwoColumns = new bool[yClusters.Count];
        int spanning = 0;
        for (int k = 0; k < yClusters.Count; k++)
        {
            int count = 0;
            for (int ci = 0; ci < columns; ci++) if (occupied[k, ci]) count++;
            if (count >= 2) { spansTwoColumns[k] = true; spanning++; }
        }
        if (spanning < 2) return false;

        for (int k = 0; k < yClusters.Count; k++)
        {
            if (spansTwoColumns[k]) continue;
            for (int ci = 0; ci < columns; ci++)
            {
                if (!occupied[k, ci]) continue;
                for (int other = 0; other < yClusters.Count; other++)
                    if (spansTwoColumns[other] && occupied[other, ci]) return false;
            }
        }
        return true;
    }

    private static int NearestClusterIndex(double y, List<double> yClusters)
    {
        int nearest = -1;
        double best = double.PositiveInfinity;
        for (int k = 0; k < yClusters.Count; k++)
        {
            double d = Math.Abs(y - yClusters[k]);
            if (d < best) { best = d; nearest = k; }
        }
        return nearest;
    }

    /// <summary>
    /// Strip form-template numbering artifacts and decorative separators: lone
    /// single-digit rows, single-digit prefixes on data cells, and dash/underscore rules.
    /// </summary>
    private static void StripFormNumberingArtifacts(List<GridRow> rows)
    {
        static bool IsLoneDigit(string t) => t.Length == 1 && t[0] >= '1' && t[0] <= '9';

        rows.RemoveAll(row =>
        {
            bool allEmptyOrDigit = row.Cells.All(c => c.Text.Trim().Length == 0 || IsLoneDigit(c.Text.Trim()));
            bool hasDigit = row.Cells.Any(c => IsLoneDigit(c.Text.Trim()));
            return allEmptyOrDigit && hasDigit;
        });

        foreach (var row in rows)
        {
            bool strippedAny = false;
            foreach (var cell in row.Cells)
            {
                string text = cell.Text.Trim();
                if (text.Length < 3) continue;
                if (text[0] >= '1' && text[0] <= '9' && text[1] == ' ')
                {
                    string rest = text.Substring(2).TrimStart();
                    if (rest.Length == 0) continue;
                    char first = rest[0];
                    bool looksLikeData = first == '$'
                        || char.IsAsciiDigit(first)
                        || (char.IsAsciiLetter(first)
                            && (rest.Contains('-') || rest.Contains('/') || rest.Contains(',')));
                    if (looksLikeData) { cell.Text = rest; strippedAny = true; }
                }
            }
            if (strippedAny)
                foreach (var cell in row.Cells)
                {
                    string t = cell.Text.Trim();
                    if (t.Length == 1 && char.IsAsciiDigit(t[0])) cell.Text = "";
                }
        }

        foreach (var row in rows)
            foreach (var cell in row.Cells)
            {
                string t = cell.Text.Trim();
                if (t.Length > 0 && t.All(c => c == '-' || c == '_')) cell.Text = "";
            }
    }

    /// <summary>
    /// Split each table at interior full-width horizontal edges that no vertical edge
    /// crosses — stacked bordered sections rather than ordinary row rules.
    /// </summary>
    private static List<GridTable> SplitTablesAtSectionDividers(
        List<GridTable> tables, List<Edge> hEdges, List<Edge> vEdges, TableDetectionConfig config)
    {
        var result = new List<GridTable>();
        foreach (var table in tables) result.AddRange(SplitTableAtSectionDividers(table, hEdges, vEdges, config));
        return result;
    }

    private static List<GridTable> SplitTableAtSectionDividers(
        GridTable table, List<Edge> hEdges, List<Edge> vEdges, TableDetectionConfig config)
    {
        var single = new List<GridTable> { table };
        if (table.Bbox is not { } bbox) return single;
        if (table.Rows.Count < 2) return single;

        double tableWidth = bbox.Right - bbox.Left;
        if (tableWidth <= 0.0) return single;

        double top = bbox.Top, bottom = bbox.Bottom;
        const double margin = 2.0;
        double tableLeft = bbox.Left, tableRight = bbox.Right;
        var relevantV = vEdges.Where(e => e.Coord >= tableLeft - SnapTol && e.Coord <= tableRight + SnapTol).ToList();

        var dividerYs = new List<double>();
        foreach (var edge in hEdges)
        {
            double overlap = Math.Min(edge.End, tableRight) - Math.Max(edge.Start, tableLeft);
            if (overlap < tableWidth * SectionDividerWidthRatio) continue;
            double y = edge.Coord;
            if (y <= top + margin || y >= bottom - margin) continue;
            const double crossMargin = SnapTol + 1.0;
            int crossings = relevantV.Count(v => v.Start < y - crossMargin && v.End > y + crossMargin);
            // A true section divider has no (or very few) verticals crossing it; ordinary
            // grid row boundaries have many.
            if (crossings <= 1) dividerYs.Add(y);
        }
        if (dividerYs.Count == 0) return single;

        dividerYs = UniqueSorted(dividerYs);

        var rowBounds = table.Rows.Select(row =>
        {
            double rmin = double.PositiveInfinity, rmax = double.NegativeInfinity;
            foreach (var c in row.Cells)
                if (c.Bbox is { } b) { rmin = Math.Min(rmin, b.Top); rmax = Math.Max(rmax, b.Bottom); }
            return double.IsInfinity(rmin) ? ((double, double)?)null : (rmin, rmax);
        }).ToList();

        var splitAfter = new List<int>();
        double tol = SnapTol + 2.0;
        foreach (double dy in dividerYs)
        {
            int bestIdx = -1;
            double bestDist = double.PositiveInfinity;
            for (int i = 0; i < rowBounds.Count; i++)
            {
                if (i >= table.Rows.Count - 1) continue;
                if (rowBounds[i] is not { } rb) continue;
                double distToBot = Math.Abs(dy - rb.Item2);
                double distToTop = Math.Abs(dy - rb.Item1);
                double minDist = Math.Min(distToBot, distToTop);
                if (minDist <= tol && minDist < bestDist)
                {
                    if (distToBot <= distToTop) bestIdx = i;
                    else if (i > 0) bestIdx = i - 1;
                    bestDist = minDist;
                }
            }
            if (bestIdx >= 0) splitAfter.Add(bestIdx);
        }
        splitAfter = splitAfter.Distinct().OrderBy(i => i).ToList();
        if (splitAfter.Count == 0) return single;

        var slices = new List<List<GridRow>>();
        int start = 0;
        foreach (int splitIdx in splitAfter)
        {
            int end = splitIdx + 1;
            if (end > start) slices.Add(table.Rows.GetRange(start, end - start));
            start = end;
        }
        if (start < table.Rows.Count) slices.Add(table.Rows.GetRange(start, table.Rows.Count - start));

        var result = new List<GridTable>();
        foreach (var subRows in slices)
        {
            int filled = subRows.SelectMany(r => r.Cells).Count(c => c.Text.Length > 0);
            if (filled < config.MinTableCells) continue;
            result.Add(new GridTable
            {
                Rows = subRows,
                ColCount = table.ColCount,
                Bbox = RowsBbox(subRows),
                ColumnBoundaries = table.ColumnBoundaries,
                BelowThreshold = table.BelowThreshold,
            });
        }
        // Don't lose data when every slice came out too small.
        return result.Count == 0 ? single : result;
    }

    /// <summary>
    /// Rejoin table fragments the line detector cut apart, before the grid-shape filters see
    /// them. A table ruled between every pair of rows is emitted one row-strip at a time, and
    /// each strip is too short to look like a grid; consolidated first, the whole table survives.
    /// </summary>
    private static List<GridTable> ConsolidateAdjacentTableFragments(List<GridTable> tables)
    {
        if (tables.Count < 2) return tables;

        // Top of the page first: PDF y grows upward, so that is descending `Y + Height`.
        var sorted = new List<GridTable>(tables);
        XRay.Content.Internal.PdfOxide.Layout.OxSpanCompare.SortStable(sorted, (a, b) =>
        {
            double aTop = a.Bbox is { } ab ? ab.Y + ab.Height : double.NegativeInfinity;
            double bTop = b.Bbox is { } bb ? bb.Y + bb.Height : double.NegativeInfinity;
            return SafeCmp(bTop, aTop);
        });

        var consolidated = new List<GridTable>(sorted.Count);
        foreach (var table in sorted)
        {
            if (consolidated.Count > 0
                && CanMergeFragments(consolidated[^1], table, FragmentMergeXTol, FragmentMergeYTol))
                MergeFragmentInto(consolidated[^1], table);
            else consolidated.Add(table);
        }
        return consolidated;
    }

    private static bool CanMergeFragments(GridTable upper, GridTable lower, double xTol, double yTol)
    {
        if (upper.Bbox is not { } u || lower.Bbox is not { } l) return false;
        if (upper.ColCount != lower.ColCount || upper.ColCount == 0) return false;
        if (Math.Abs(u.X - l.X) > xTol) return false;
        if (Math.Abs(u.Width - l.Width) > xTol) return false;

        // `upper` sits above `lower`, so upper's bottom edge is its `Y` and lower's top edge is
        // `Y + Height`. Ruling strokes have width, so the two may overlap slightly; only an
        // overlap deeper than half the shorter fragment means these are not one table.
        double gap = u.Y - (l.Y + l.Height);
        if (gap > yTol) return false;
        return -gap <= Math.Min(u.Height, l.Height) * 0.5;
    }

    private static void MergeFragmentInto(GridTable upper, GridTable lower)
    {
        if (upper.Bbox is { } ub && lower.Bbox is { } lb)
        {
            double x = Math.Min(ub.X, lb.X), y = Math.Min(ub.Y, lb.Y);
            double right = Math.Max(ub.X + ub.Width, lb.X + lb.Width);
            double top = Math.Max(ub.Y + ub.Height, lb.Y + lb.Height);
            upper.Bbox = new PathRect(x, y, right - x, top - y);
        }
        upper.Rows.AddRange(lower.Rows);
    }

    private static void MergeVerticallyAdjacentTables(List<GridTable> tables)
    {
        if (tables.Count < 2) return;
        tables.Sort((a, b) =>
        {
            double ay = a.Bbox?.Top ?? double.NegativeInfinity;
            double by = b.Bbox?.Top ?? double.NegativeInfinity;
            return ay.CompareTo(by);
        });

        var merged = new List<GridTable>();
        foreach (var table in tables)
        {
            bool shouldMerge = false;
            if (merged.Count > 0)
            {
                var prev = merged[^1];
                if (Math.Abs(prev.ColCount - table.ColCount) <= MergeColDiffTolerance
                    && prev.Bbox is { } pb && table.Bbox is { } tb)
                {
                    double gap = Math.Min(Math.Abs(tb.Top - pb.Bottom), Math.Abs(pb.Top - tb.Bottom));
                    shouldMerge = gap <= AdjacentTableMergeGap;
                }
            }

            if (!shouldMerge) { merged.Add(table); continue; }

            var target = merged[^1];
            int targetCols = Math.Max(target.ColCount, table.ColCount);
            if (target.ColCount < targetCols)
                foreach (var row in target.Rows)
                    for (int i = target.ColCount; i < targetCols; i++)
                        row.Cells.Add(new GridCell { IsHeader = row.IsHeader });
            if (table.ColCount < targetCols)
                foreach (var row in table.Rows)
                    for (int i = table.ColCount; i < targetCols; i++)
                        row.Cells.Add(new GridCell { IsHeader = row.IsHeader });

            target.Rows.AddRange(table.Rows);
            target.ColCount = targetCols;
            if (target.Bbox is { } tpb && table.Bbox is { } ttb)
            {
                double minX = Math.Min(tpb.Left, ttb.Left), minY = Math.Min(tpb.Top, ttb.Top);
                double maxX = Math.Max(tpb.Right, ttb.Right), maxY = Math.Max(tpb.Bottom, ttb.Bottom);
                target.Bbox = new PathRect(minX, minY, maxX - minX, maxY - minY);
            }
            target.HasHeader = target.HasHeader || table.HasHeader;
            // A candidate that merges into a real table is part of one, and stops being a
            // candidate; the merged columns are the wider of the two grids' own.
            target.BelowThreshold = target.BelowThreshold && table.BelowThreshold;
            if (target.ColumnBoundaries is null
                || (table.ColumnBoundaries is { } tc && tc.Length > target.ColumnBoundaries.Length))
                target.ColumnBoundaries = table.ColumnBoundaries;
        }

        tables.Clear();
        tables.AddRange(merged);
    }

    // ── Cell text ────────────────────────────────────────────────────────────

    /// <summary>
    /// The display text a span contributes to a table cell (table_extractor.rs:515).
    /// </summary>
    /// <remarks>
    /// A single positioned run reading <c>N.M</c> is sometimes not a decimal at all but two
    /// values in adjacent columns — a sailing score sheet draws <c>1</c> and <c>10</c> as one
    /// <c>Tj</c> whose glyphs straddle the column boundary. Upstream recognises this by the
    /// run's box being far wider than its glyph count can account for, and splits at the dot.
    /// The `char_widths` signal the Rust also tests cannot fire here: the detector is fed
    /// words, and the word-to-span conversion leaves that array empty
    /// (<c>document.rs:17846</c>), so the width estimate is the 0.50em-per-digit fallback.
    /// </remarks>
    private static string SpanTextForCell(TableSpan span)
    {
        string text = span.Text;
        int dot = text.IndexOf('.');
        if (dot <= 0 || dot >= text.Length - 1) return text;
        if (text.IndexOf('.', dot + 1) >= 0) return text;
        for (int i = 0; i < dot; i++) if (text[i] < '0' || text[i] > '9') return text;
        for (int i = dot + 1; i < text.Length; i++) if (text[i] < '0' || text[i] > '9') return text;

        float fontSize = (float)span.FontSize;
        if (!(fontSize > 0.0f)) return text;
        // Digits are narrower than the average glyph, so 0.50em per character keeps the
        // fallback from reporting a false positive on a run of ordinary width.
        int charCount = text.EnumerateRunes().Count();
        float expectedWidth = fontSize * 0.50f * charCount;
        float gap = (float)span.Bbox.Width - expectedWidth;
        if (gap > fontSize) return string.Concat(text.AsSpan(0, dot), " ", text.AsSpan(dot + 1));
        return text;
    }

    private static string ExtractCellText(List<int> cellSpanIndices, List<TableSpan> spans)
    {
        if (cellSpanIndices.Count == 0) return "";
        var entries = cellSpanIndices
            .Where(i => i >= 0 && i < spans.Count)
            .Select(i => (Y: spans[i].CenterY, Span: spans[i], Text: SpanTextForCell(spans[i])))
            .ToList();
        if (entries.Count == 0) return "";
        if (entries.Count == 1) return entries[0].Text;

        // Stable: spans sharing a baseline keep the order the cell collected them in, which
        // is the only thing that orders words within one line of a cell — nothing sorts by X.
        XRay.Content.Internal.PdfOxide.Layout.OxSpanCompare.SortStable(entries, (a, b) => SafeCmp(b.Y, a.Y));

        var lines = new List<List<(TableSpan Span, string Text)>>();
        var current = new List<(TableSpan, string)> { (entries[0].Span, entries[0].Text) };
        double currentY = entries[0].Y;
        for (int i = 1; i < entries.Count; i++)
        {
            if (Math.Abs(currentY - entries[i].Y) <= 2.0) current.Add((entries[i].Span, entries[i].Text));
            else
            {
                lines.Add(current);
                current = new List<(TableSpan, string)> { (entries[i].Span, entries[i].Text) };
                currentY = entries[i].Y;
            }
        }
        lines.Add(current);

        var sb = new StringBuilder();
        for (int li = 0; li < lines.Count; li++)
        {
            if (li > 0) sb.Append('\n');
            var line = lines[li];
            for (int i = 0; i < line.Count; i++)
            {
                if (i > 0) sb.Append(CellSpanSeparator(line[i - 1].Span, line[i].Span));
                sb.Append(line[i].Text);
            }
        }
        return sb.ToString();
    }

    /// <summary>
    /// Separator between two spans in one cell row: a space only when there is a real
    /// horizontal gap above the inter-glyph floor and the boundary is not CJK-adjacent.
    /// </summary>
    private static string CellSpanSeparator(TableSpan prev, TableSpan current)
    {
        if (prev.Text.EndsWith(' ') || current.Text.StartsWith(' ')) return "";

        double gap = current.Bbox.X - (prev.Bbox.X + prev.Bbox.Width);
        double fontSize = Math.Max(Math.Max(prev.FontSize, current.FontSize), 1.0);
        // Sub-em gap: the glyphs are touching, so they belong to one compound token.
        if (gap <= fontSize * 0.15) return "";

        if (prev.Text.Length > 0 && current.Text.Length > 0)
        {
            char p = prev.Text[^1], c = current.Text[0];
            bool pCjk = IsCjk(p), cCjk = IsCjk(c);
            if ((pCjk || IsFullwidthOperator(p)) && (cCjk || IsFullwidthOperator(c)) && (pCjk || cCjk)) return "";
        }
        return " ";
    }

    private static bool IsCjk(char c) =>
        (c >= '぀' && c <= 'ゟ') || (c >= '゠' && c <= 'ヿ')
        || (c >= '一' && c <= '鿿') || (c >= '가' && c <= '힯')
        || (c >= '㐀' && c <= '䶿');

    private static bool IsFullwidthOperator(char c) =>
        c == '＋' || c == '－' || c == '：' || c == '；'
        || (c >= '＜' && c <= '＞') || c == '≠' || c == '≈'
        || c == '≤' || c == '≥' || c == 'µ' || c == 'μ'
        || c == '±' || c == '×' || c == '÷';

    // ── Validity gates ───────────────────────────────────────────────────────

    private static bool IsValidTable(GridTable table)
    {
        if (table.Rows.Count == 0 || table.ColCount == 0) return false;

        int totalCells = Math.Max(table.Rows.Count * table.ColCount, 1);
        int emptyCells = table.Rows.SelectMany(r => r.Cells).Count(c => c.Text.Trim().Length == 0);
        if ((double)emptyCells / totalCells > 0.6) return false;

        // A 2-column "table" built from label/value rows with faint cell backgrounds
        // shows a continuation row: empty label beside a non-empty wrapped value.
        // Reject only that shape; legitimately sparse 2-column tables still validate.
        if (table.ColCount == 2
            && table.Rows.Any(r => r.Cells.Count == 2
                && r.Cells[0].Text.Trim().Length == 0 && r.Cells[1].Text.Trim().Length > 0))
            return false;

        return true;
    }

    private static bool IsRealGrid(GridTable table)
    {
        if (table.ColCount < 2 || table.Rows.Count < 2) return false;
        int rowsWithTwo = table.Rows.Count(r => r.Cells.Count(c => c.Text.Trim().Length > 0) >= 2);
        double ratio = (double)rowsWithTwo / table.Rows.Count;

        // Wide tables are high-risk false positives: prose split by decorative rules
        // yields highly variable row fill counts, where real wide data tables are dense.
        if (table.ColCount >= 8)
        {
            int minDense = Math.Max((int)(table.ColCount * 0.6), 2);
            int denseRows = table.Rows.Count(r => r.Cells.Count(c => c.Text.Trim().Length > 0) >= minDense);
            double denseRatio = (double)denseRows / table.Rows.Count;
            if (table.Rows.Count >= 3 && ratio >= 0.7 && denseRatio >= 0.70) return true;
            // A consolidated table mixes dense data rows with sparse header/label rows.
            int minAbsoluteDense = Math.Max(table.ColCount / 2, 3);
            return denseRows >= minAbsoluteDense && denseRatio >= 0.40;
        }
        return ratio >= 0.5;
    }

    private static bool LooksLikeProseTable(GridTable table)
    {
        int total = 0, sentenceTails = 0, lowerStarts = 0, leaderDots = 0;
        foreach (var row in table.Rows)
            foreach (var cell in row.Cells)
            {
                string trimmed = cell.Text.Trim();
                if (trimmed.Length == 0) continue;
                total++;
                char last = trimmed[^1];
                if (last == ',' || last == ';') sentenceTails++;
                if (char.IsAsciiLetterLower(trimmed[0])) lowerStarts++;
                // A cell of nothing but dots is a table-of-contents leader, not data.
                if (trimmed.All(c => c == '.' || c == ' ')) leaderDots++;
            }
        if (total < 10) return false;
        return (double)sentenceTails / total > 0.12
            || (double)lowerStarts / total > 0.25
            || (double)leaderDots / total > 0.10;
    }

    // ── Conversion to xberg's Table ──────────────────────────────────────────

    private static string CellTextInReadingOrder(GridCell cell, List<TableSpan> spans)
    {
        if (cell.SpanIndices.Count == 0) return cell.Text.Trim().Replace('\n', ' ');

        var sorted = cell.SpanIndices.Where(i => i >= 0 && i < spans.Count).Select(i => spans[i]).ToList();
        // Stable: two words sharing a baseline and a left edge keep the order the cell
        // collected them in, which is all that separates them.
        XRay.Content.Internal.PdfOxide.Layout.OxSpanCompare.SortStable(sorted, (a, b) =>
        {
            int c = b.Bbox.Y.CompareTo(a.Bbox.Y);
            return c != 0 ? c : a.Bbox.X.CompareTo(b.Bbox.X);
        });

        return string.Join(" ", sorted.Select(s => s.Text.Trim().Replace('\n', ' ')).Where(s => s.Length > 0));
    }

    private static (List<List<string>> Cells, string Markdown) ConvertExtractedTable(
        GridTable table, List<TableSpan> spans)
    {
        var cells = new List<List<string>>(table.Rows.Count);
        var markdown = new StringBuilder();
        bool foundHeader = false;

        for (int rowIdx = 0; rowIdx < table.Rows.Count; rowIdx++)
        {
            var row = table.Rows[rowIdx];
            var rowCells = row.Cells.Select(c => CellTextInReadingOrder(c, spans)).ToList();

            markdown.Append('|');
            foreach (string cell in rowCells) markdown.Append(' ').Append(cell).Append(" |");
            markdown.Append('\n');

            if ((row.IsHeader || rowIdx == 0) && !foundHeader)
            {
                foundHeader = true;
                markdown.Append('|');
                for (int i = 0; i < rowCells.Count; i++) markdown.Append(" --- |");
                markdown.Append('\n');
            }

            cells.Add(rowCells);
        }

        return (cells, markdown.ToString());
    }
}
