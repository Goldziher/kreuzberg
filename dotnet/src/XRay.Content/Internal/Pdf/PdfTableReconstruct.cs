// Ported from crates/xberg/src/table_core.rs, pdf/table_reconstruct.rs,
// pdf/oxide/table.rs (extract_tables_heuristic + region clustering), and
// pdf/structure/regions/tables.rs (compute_adaptive_column_gap).
//
// Text-layer table reconstruction with no OCR/graphics dependency: word
// bounding boxes (derived from font-metric segments) are clustered into
// vertically-contiguous regions, each region is reconstructed into a cell
// grid, then validated/cleaned by the same prose-rejection chain the Rust
// heuristic tier uses. The two ruling-line tiers that run before it
// (extract_tables_native/_bordered) live in PdfSpatialTables.
using System.Collections.Generic;
using System.Text;
using XRay.Content.Types;

namespace XRay.Content.Internal.Pdf;

/// <summary>A word with an integer pixel bounding box (mirrors Rust `HocrWord`).</summary>
internal struct HocrWord
{
    public string Text;
    public uint Left, Top, Width, Height;
    public double Confidence;

    public readonly uint Right => Left + Width;
    public readonly uint Bottom => Top + Height;
    public readonly double YCenter => Top + Height / 2.0;
}

internal static partial class PdfTableReconstruct
{
    private const int MaxRegionsPerPage = 20;

    // ── Public entry: heuristic table extraction across all pages ────────────

    /// <summary>Port of `pdf::oxide::table::extract_tables_heuristic`. Runs the
    /// text-edge heuristic on every page's segments and returns detected tables.</summary>
    /// <param name="skipPages">1-indexed pages a higher-priority tier already covered.</param>
    /// <param name="pagePaths">Per-page painted paths, used to count drawn rules. A page
    /// whose producer drew rules is one where a table candidate is credible on its face;
    /// without them the column boundaries have to prove they are whitespace.</param>
    public static List<Table> ExtractHeuristicTables(
        List<List<SegmentData>> allPageSegments, bool allowSingleColumn, HashSet<uint>? skipPages = null,
        List<List<PdfPath>>? pagePaths = null)
    {
        var tables = new List<Table>();
        for (int pageIdx = 0; pageIdx < allPageSegments.Count; pageIdx++)
        {
            Cancel.ThrowIfRequested();
            uint pageNumber = (uint)(pageIdx + 1);
            if (skipPages is not null && skipPages.Contains(pageNumber)) continue;
            var segments = allPageSegments[pageIdx];
            if (segments.Count == 0) continue;

            float pageHeight = 0f;
            foreach (var s in segments) pageHeight = Math.Max(pageHeight, s.Y + s.Height);
            pageHeight = Math.Max(pageHeight, 792.0f);

            var words = SegmentsToWords(segments, pageHeight);
            if (words.Count < 4) continue;

            var regions = ClusterWordsIntoVerticalRegions(words);
            if (regions.Count > MaxRegionsPerPage)
                regions = regions.GetRange(0, MaxRegionsPerPage);

            int horizontalRules = pagePaths is not null && pageIdx < pagePaths.Count
                ? PdfPath.CountRules(pagePaths[pageIdx]).Horizontal
                : 0;

            tables.AddRange(ReconstructPageRegionTables(
                regions, pageHeight, pageNumber, allowSingleColumn, horizontalRules));
        }
        return tables;
    }

    // ── Segment → word conversion (pdf/table_reconstruct.rs) ─────────────────

    private static uint RoundClamp(float v, float min) => (uint)Math.Max(MathF.Round(v, MidpointRounding.AwayFromZero), min);

    /// <summary>Port of `segments_to_words`.</summary>
    public static List<HocrWord> SegmentsToWords(List<SegmentData> segments, float pageHeight)
    {
        var perSegment = new List<List<HocrWord>>(segments.Count);
        foreach (var seg in segments)
        {
            var words = new List<HocrWord>();
            SplitSegmentToWords(seg, pageHeight, words);
            perSegment.Add(words);
        }
        return MergeTouchingSegmentBoundaries(segments, perSegment);
    }

    /// <summary>
    /// Rejoin a word split across two adjacent segments before cells are assigned, so the
    /// cell-text join never re-inserts a space the word split dropped by construction.
    /// </summary>
    /// <remarks>
    /// <see cref="HocrWord"/> is integer-rounded and carries neither font size nor baseline, so a
    /// sub-point gap like the reported 0.069 pt is not representable once words exist: the test
    /// has to run here, on the segments, one boundary at a time. Only the last word of one
    /// segment's group and the first of the next can be a split-word boundary, since a segment's
    /// own words came from whitespace-splitting its own text (xberg-io/xberg#1566).
    /// </remarks>
    private static List<HocrWord> MergeTouchingSegmentBoundaries(
        List<SegmentData> segments, List<List<HocrWord>> perSegment)
    {
        for (int boundary = 0; boundary + 1 < perSegment.Count; boundary++)
        {
            var left = perSegment[boundary];
            var right = perSegment[boundary + 1];
            if (left.Count == 0 || right.Count == 0) continue;
            if (!PdfStructure.SegmentsAreTouching(
                    segments[boundary], left[^1].Text, segments[boundary + 1], right[0].Text))
                continue;

            var prevWord = left[^1];
            left.RemoveAt(left.Count - 1);
            right[0] = MergeHocrWords(prevWord, right[0]);
        }

        var merged = new List<HocrWord>();
        foreach (var group in perSegment) merged.AddRange(group);
        return merged;
    }

    /// <summary>Combine two words that are one split word: text concatenated with no separator,
    /// bounding box the union of both, confidence the lower of the two.</summary>
    private static HocrWord MergeHocrWords(HocrWord prev, HocrWord next)
    {
        uint left = Math.Min(prev.Left, next.Left);
        uint top = Math.Min(prev.Top, next.Top);
        uint right = Math.Max(prev.Right, next.Right);
        uint bottom = Math.Max(prev.Bottom, next.Bottom);
        return new HocrWord
        {
            Text = prev.Text + next.Text,
            Left = left,
            Top = top,
            Width = right - left,
            Height = bottom - top,
            Confidence = Math.Min(prev.Confidence, next.Confidence),
        };
    }

    private static void SplitSegmentToWords(SegmentData seg, float pageHeight, List<HocrWord> outWords)
    {
        string trimmed = seg.Text.Trim();
        if (trimmed.Length == 0) return;

        // The segment's own upright frame, not raw page-space x/y: `advance` is the position
        // along the run's reading axis, which the per-word interpolation below advances along
        // via `fracStart * seg.Width`. For an unrotated segment this is the identity, so only
        // rotated runs are affected — but for those, raw x/y is the wrong axis entirely.
        var (advance, cross) = seg.UprightOrigin();
        uint topImage = RoundClamp(pageHeight - (cross + seg.Height), 0f);

        // Fast path: single word (no interior whitespace).
        bool hasWs = false;
        foreach (char c in trimmed) if (char.IsWhiteSpace(c)) { hasWs = true; break; }
        if (!hasWs)
        {
            outWords.Add(new HocrWord
            {
                // `segment_to_hocr_word` clones the segment's raw text: the whitespace
                // test is on the trimmed form, but the word keeps whatever padding the
                // segment carried, and the cell join adds a space of its own on top.
                Text = seg.Text,
                Left = RoundClamp(advance, 0f),
                Top = topImage,
                Width = RoundClamp(seg.Width, 0f),
                Height = RoundClamp(seg.Height, 0f),
                Confidence = 95.0,
            });
            return;
        }

        // Multi-word: proportional bbox per word by UTF-8 byte offset (matches Rust).
        byte[] full = Encoding.UTF8.GetBytes(seg.Text);
        float totalBytes = full.Length;
        if (totalBytes <= 0f) return;

        uint segHeight = RoundClamp(seg.Height, 0f);
        int searchStart = 0;
        foreach (string word in SplitWhitespace(seg.Text))
        {
            byte[] wb = Encoding.UTF8.GetBytes(word);
            int offset = IndexOf(full, wb, searchStart);
            if (offset < 0) continue;
            searchStart = offset + wb.Length;

            float fracStart = offset / totalBytes;
            float fracWidth = wb.Length / totalBytes;

            outWords.Add(new HocrWord
            {
                Text = word,
                Left = RoundClamp(advance + fracStart * seg.Width, 0f),
                Top = topImage,
                Width = RoundClamp(fracWidth * seg.Width, 1f),
                Height = segHeight,
                Confidence = 95.0,
            });
        }
    }

    private static IEnumerable<string> SplitWhitespace(string text)
    {
        int i = 0;
        while (i < text.Length)
        {
            while (i < text.Length && char.IsWhiteSpace(text[i])) i++;
            if (i >= text.Length) break;
            int start = i;
            while (i < text.Length && !char.IsWhiteSpace(text[i])) i++;
            yield return text.Substring(start, i - start);
        }
    }

    private static int IndexOf(byte[] haystack, byte[] needle, int start)
    {
        if (needle.Length == 0) return start;
        for (int i = start; i + needle.Length <= haystack.Length; i++)
        {
            bool ok = true;
            for (int j = 0; j < needle.Length; j++)
                if (haystack[i + j] != needle[j]) { ok = false; break; }
            if (ok) return i;
        }
        return -1;
    }

    // ── Region clustering (pdf/oxide/table.rs) ───────────────────────────────

    private static uint AbsDiff(uint a, uint b) => a > b ? a - b : b - a;

    private static List<List<HocrWord>> ClusterWordsIntoVerticalRegions(List<HocrWord> words)
    {
        if (words.Count < 4) return new List<List<HocrWord>>();

        var heights = words.Select(w => w.Height).OrderBy(h => h).ToList();
        uint medianHeight = Math.Max(heights[heights.Count / 2], 1u);
        uint rowTolerance = Math.Max(medianHeight / 2, 3u);
        uint rowGapSplit = (uint)(medianHeight * 1.8f);

        // Stable sort by y-center (top + height/2, integer).
        var sorted = words.OrderBy(w => w.Top + w.Height / 2).ToList();

        var regions = new List<List<HocrWord>>();
        var current = new List<HocrWord>();
        uint? lastRowYc = null;

        int idx = 0;
        while (idx < sorted.Count)
        {
            uint rowYc = sorted[idx].Top + sorted[idx].Height / 2;
            int end = idx + 1;
            while (end < sorted.Count)
            {
                uint yc = sorted[end].Top + sorted[end].Height / 2;
                if (AbsDiff(yc, rowYc) <= rowTolerance) end++;
                else break;
            }

            if (lastRowYc is uint prevYc && rowYc > prevYc && rowYc - prevYc > rowGapSplit && current.Count > 0)
            {
                regions.Add(current);
                current = new List<HocrWord>();
            }
            for (int k = idx; k < end; k++) current.Add(sorted[k]);
            lastRowYc = rowYc;
            idx = end;
        }
        if (current.Count > 0) regions.Add(current);

        AttachAlignedNumericHeaders(regions, medianHeight, rowTolerance);

        // retain: >=4 words, >=3 distinct rows, >=2 distinct x columns.
        var kept = new List<List<HocrWord>>();
        foreach (var r in regions)
        {
            if (r.Count < 4) continue;

            var rowYcs = r.Select(w => w.Top + w.Height / 2).OrderBy(v => v).ToList();
            int distinctRows = DedupCount(rowYcs, rowTolerance);
            if (distinctRows < 3) continue;

            var xs = r.Select(w => w.Left).OrderBy(v => v).ToList();
            int distinctXs = DedupCount(xs, 8u);
            if (distinctXs < 2) continue;

            kept.Add(r);
        }
        return kept;
    }

    // Count of runs after `dedup_by(|a,b| abs_diff <= tol)` on a sorted list.
    private static int DedupCount(List<uint> sorted, uint tol)
    {
        if (sorted.Count == 0) return 0;
        int count = 1;
        uint last = sorted[0];
        for (int i = 1; i < sorted.Count; i++)
        {
            if (AbsDiff(sorted[i], last) <= tol)
            {
                // merged into previous run; Rust dedup_by keeps the first element.
                continue;
            }
            count++;
            last = sorted[i];
        }
        return count;
    }

    // ── Adaptive column gap (pdf/structure/regions/tables.rs) ────────────────

    private static uint ComputeAdaptiveColumnGap(List<HocrWord> words, float tableWidth)
    {
        var gaps = new List<uint>();

        if (words.Count >= 4)
        {
            var heights = words.Select(w => w.Height).OrderBy(h => h).ToList();
            uint medianH = heights[heights.Count / 2];
            uint rowTolerance = Math.Max(medianH / 2, 3u);

            // (yc, left, right); stable sort by (yc, left).
            var sorted = words
                .Select(w => (Yc: w.Top + w.Height / 2, Left: w.Left, Right: w.Left + w.Width))
                .OrderBy(t => t.Yc).ThenBy(t => t.Left).ToList();

            int rowStart = 0;
            while (rowStart < sorted.Count)
            {
                uint rowYc = sorted[rowStart].Yc;
                int rowEnd = rowStart + 1;
                while (rowEnd < sorted.Count && AbsDiff(sorted[rowEnd].Yc, rowYc) <= rowTolerance) rowEnd++;

                for (int i = rowStart + 1; i < rowEnd; i++)
                {
                    uint prevRight = sorted[i - 1].Right;
                    uint currLeft = sorted[i].Left;
                    if (currLeft > prevRight) gaps.Add(currLeft - prevRight);
                }
                rowStart = rowEnd;
            }
        }

        if (gaps.Count >= 3)
        {
            gaps.Sort();
            var largeGaps = gaps.Where(g => g >= 40).ToList();
            if (largeGaps.Count > 0)
            {
                uint medianGap = largeGaps[largeGaps.Count / 2];
                return Math.Clamp(medianGap / 2, 20u, 60u);
            }
            else
            {
                uint medianGap = gaps[gaps.Count / 2];
                return Math.Clamp(medianGap * 3, 20u, 60u);
            }
        }

        if (tableWidth < 200.0f) return 10u;
        if (tableWidth < 400.0f) return 15u;
        if (tableWidth < 600.0f) return 20u;
        return 30u;
    }

    // ── Region → Table (pdf/oxide/table.rs reconstruct_region_table) ─────────

    /// <summary>
    /// Port of `reconstruct_page_region_tables`. Walks the page's regions, letting a run of
    /// label-heavy financial sections stitch itself into one table before falling back to
    /// per-region reconstruction.
    /// </summary>
    private static List<Table> ReconstructPageRegionTables(
        List<List<HocrWord>> regions, float pageHeight, uint pageNumber,
        bool allowSingleColumn, int horizontalRules)
    {
        var tables = new List<Table>();
        int index = 0;
        while (index < regions.Count)
        {
            var head = ReconstructLabelHeavyFinancialRegion(regions[index], pageHeight, pageNumber);
            if (head is null)
            {
                tables.AddRange(ReconstructRegionTables(
                    regions[index], pageHeight, pageNumber, allowSingleColumn, horizontalRules));
                index++;
                continue;
            }

            var (first, firstTracks, trackTolerance) = head.Value;
            var chain = new List<Table> { first };
            int chainEnd = index + 1;
            while (chainEnd < regions.Count)
            {
                var following = ReconstructLabelHeavyFinancialRegion(regions[chainEnd], pageHeight, pageNumber);
                if (following is null) break;
                var (next, nextTracks, nextTolerance) = following.Value;
                uint tolerance = Math.Max(trackTolerance, nextTolerance);
                bool tracksAlign = firstTracks.Zip(nextTracks, (l, r) => AbsDiff(l, r) <= tolerance).All(ok => ok);
                if (!tracksAlign || !LabelHeavyFinancialSectionsAreContiguous(
                        regions[chainEnd - 1], regions[chainEnd], nextTracks, nextTolerance))
                    break;
                chain.Add(next);
                chainEnd++;
            }

            if (chain.Count < 2)
            {
                tables.AddRange(ReconstructRegionTables(
                    regions[index], pageHeight, pageNumber, allowSingleColumn, horizontalRules));
                index++;
                continue;
            }

            tables.Add(StitchLabelHeavyFinancialSections(chain, pageNumber));
            index = chainEnd;
        }
        return tables;
    }

    /// <summary>
    /// Port of `reconstruct_region_tables`. A region wide enough to be two tables side by side
    /// is only split when both halves stand up as tables on their own.
    /// </summary>
    private static List<Table> ReconstructRegionTables(
        List<HocrWord> region, float pageHeight, uint pageNumber,
        bool allowSingleColumn, int horizontalRules)
    {
        var parent = ReconstructRegionTable(region, pageHeight, pageNumber, allowSingleColumn, horizontalRules);
        if (parent is null) return new List<Table>();
        if ((parent.Cells.Count > 0 ? parent.Cells[0].Count : 0) < SideBySideMinParentColumns)
            return new List<Table> { parent };

        var split = SplitSideBySideRegion(region);
        if (split is null) return new List<Table> { parent };

        var left = ReconstructSideBySideChild(split.Value.Left, pageHeight, pageNumber, allowSingleColumn, horizontalRules);
        if (left is null) return new List<Table> { parent };
        var right = ReconstructSideBySideChild(split.Value.Right, pageHeight, pageNumber, allowSingleColumn, horizontalRules);
        if (right is null) return new List<Table> { parent };
        if (!SideTablesHaveIndependentShape(left, right)) return new List<Table> { parent };

        var (normalizedLeft, normalizedRight) = NormalizeSideBySideFinancialTables(left, right);
        return new List<Table> { normalizedLeft, normalizedRight };
    }

    /// <summary>One half of a side-by-side split, reconstructed with a gap wide enough that the
    /// half's own columns survive: the parent's adaptive gap was measured across both halves.</summary>
    private static Table? ReconstructSideBySideChild(
        List<HocrWord> region, float pageHeight, uint pageNumber, bool allowSingleColumn, int horizontalRules)
    {
        if (region.Count == 0) return null;
        var heights = region.Select(w => w.Height).OrderBy(h => h).ToList();
        uint childGap = Math.Max(heights[heights.Count / 2], 1u) * SideBySideChildGapHeightMultiplier;
        return ReconstructRegionTableWithColumnGap(
            region, pageHeight, pageNumber, allowSingleColumn, childGap, horizontalRules);
    }

    private static Table? ReconstructRegionTable(
        List<HocrWord> region, float pageHeight, uint pageNumber, bool allowSingleColumn, int horizontalRules = 0)
    {
        uint regionLeft = region.Min(w => w.Left);
        uint regionRight = region.Max(w => w.Left + w.Width);
        float regionWidth = regionRight > regionLeft ? regionRight - regionLeft : 0f;
        uint colGap = HeuristicColumnGap(region, regionWidth);
        return ReconstructRegionTableWithColumnGap(
            region, pageHeight, pageNumber, allowSingleColumn, colGap, horizontalRules);
    }

    private static Table? ReconstructRegionTableWithColumnGap(
        List<HocrWord> region, float pageHeight, uint pageNumber,
        bool allowSingleColumn, uint colGap, int horizontalRules)
    {
        // The column positions come from the same call that built the grid.
        // ReconstructTable detects columns from post-merge cell tokens, not from the region
        // directly, so a separate DetectColumns(region, colGap) no longer agrees with the grid's
        // column count — and RepairSplitNumericTrack fails safe on a count mismatch, which would
        // silently switch the repair off rather than reporting anything.
        var (grid, columnPositions) = ReconstructTableWithColumns(region, colGap, 0.5);
        // Passed by reference so a successful repair drops the merged boundary from BOTH: the
        // repair collapses two grid columns into one, and a stale extra position would leave the
        // grid one column narrower than the positions describing it for every consumer below —
        // IsWellFormedBorderlessTable among them, which is handed both.
        RepairSplitNumericTrack(grid, region, columnPositions);
        if (grid.Count == 0 || grid[0].Count == 0) return null;

        var cleaned = PostProcessTable(grid, layoutGuided: true, allowSingleColumn);
        if (cleaned == null) return null;
        RepairHeuristicHeaderDelimiters(cleaned);
        if (cleaned.Count <= 1) return null;

        if (LooksLikeCodeListing(cleaned)) return null;

        // Beyond the text-statistics gate, reject candidates that have no drawn ruling lines
        // AND whose inferred column boundaries are not actually whitespace: continuous prose
        // bucketed into a wide grid by vertical-only clustering has words running across those
        // boundaries on most rows, which text statistics alone cannot see.
        if (!IsWellFormedBorderlessTable(cleaned, region, columnPositions, horizontalRules)) return null;

        var bbox = RegionBoundingBox(region, pageHeight);

        string markdown = TableToMarkdown(cleaned);
        if (markdown.Trim().Length == 0) return null;

        return new Table
        {
            Cells = cleaned,
            Markdown = markdown,
            PageNumber = pageNumber,
            BoundingBox = bbox,
        };
    }

    private static BoundingBox? RegionBoundingBox(List<HocrWord> region, float pageHeight)
    {
        double imgLeft = region.Min(w => (double)w.Left);
        double imgTop = region.Min(w => (double)w.Top);
        double imgRight = region.Max(w => (double)(w.Left + w.Width));
        double imgBottom = region.Max(w => (double)(w.Top + w.Height));
        if (imgRight <= imgLeft || imgBottom <= imgTop) return null;
        return new BoundingBox { X0 = imgLeft, Y0 = pageHeight - imgBottom, X1 = imgRight, Y1 = pageHeight - imgTop };
    }

    // ── Core grid reconstruction (table_core.rs) ─────────────────────────────

    private static List<uint> DetectColumns(List<HocrWord> words, uint columnThreshold)
    {
        if (words.Count == 0) return new List<uint>();
        var groups = new List<List<uint>>();
        foreach (var word in words)
        {
            uint x = word.Left;
            bool found = false;
            foreach (var group in groups)
            {
                if (group.Count > 0 && AbsDiff(x, group[0]) <= columnThreshold)
                {
                    group.Add(x);
                    found = true;
                    break;
                }
            }
            if (!found) groups.Add(new List<uint> { x });
        }
        var cols = new List<uint>();
        foreach (var group in groups)
        {
            if (group.Count == 0) continue;
            var s = group.OrderBy(v => v).ToList();
            cols.Add(s[s.Count / 2]);
        }
        cols.Sort();
        return cols;
    }

    private static List<uint> DetectRows(List<HocrWord> words, double rowThresholdRatio)
    {
        if (words.Count == 0) return new List<uint>();
        var heights = words.Select(w => w.Height).OrderBy(h => h).ToList();
        uint medianHeight = heights[heights.Count / 2];
        uint rowThreshold = (uint)(medianHeight * rowThresholdRatio);

        var groups = new List<List<double>>();
        foreach (var word in words)
        {
            double yc = word.YCenter;
            bool found = false;
            foreach (var group in groups)
            {
                if (group.Count > 0 && Math.Abs(yc - group[0]) <= rowThreshold)
                {
                    group.Add(yc);
                    found = true;
                    break;
                }
            }
            if (!found) groups.Add(new List<double> { yc });
        }
        var rows = new List<uint>();
        foreach (var group in groups)
        {
            if (group.Count == 0) continue;
            var s = group.OrderBy(v => v).ToList();
            rows.Add((uint)s[s.Count / 2]);
        }
        rows.Sort();
        return rows;
    }

    private static int? FindRowIndex(List<uint> rowPositions, HocrWord word)
    {
        uint yc = (uint)word.YCenter;
        int? best = null;
        uint bestDiff = 0;
        for (int i = 0; i < rowPositions.Count; i++)
        {
            uint d = AbsDiff(rowPositions[i], yc);
            if (best == null || d < bestDiff) { best = i; bestDiff = d; }
        }
        return best;
    }

    private static int? FindColumnIndex(List<uint> colPositions, HocrWord word)
    {
        uint x = word.Left;
        int? best = null;
        uint bestDiff = 0;
        for (int i = 0; i < colPositions.Count; i++)
        {
            uint d = AbsDiff(colPositions[i], x);
            if (best == null || d < bestDiff) { best = i; bestDiff = d; }
        }
        return best;
    }

    /// <summary>
    /// Which columns of a grid contain at least one non-empty cell.
    /// </summary>
    /// <remarks>
    /// Shared by <see cref="RemoveEmptyRowsAndColumns"/> and
    /// <see cref="ReconstructTableWithColumns"/>, which both drop the same set of all-empty
    /// columns — the latter also filters a parallel column-position list by the identical mask so
    /// positions keep lining up 1:1 with the grid's surviving columns.
    /// </remarks>
    private static bool[] NonEmptyColumnMask(List<List<string>> table)
    {
        int numCols = table.Count == 0 ? 0 : table[0].Count;
        var mask = new bool[numCols];
        foreach (var row in table)
            for (int c = 0; c < row.Count && c < numCols; c++)
                if (row[c].Trim().Length > 0) mask[c] = true;
        return mask;
    }

    private static List<List<string>> RemoveEmptyRowsAndColumns(List<List<string>> table)
    {
        if (table.Count == 0) return table;
        int numCols = table[0].Count;
        var nonEmptyCols = NonEmptyColumnMask(table);

        var result = new List<List<string>>();
        foreach (var row in table)
        {
            if (!row.Any(cell => cell.Trim().Length > 0)) continue;
            var newRow = new List<string>();
            for (int c = 0; c < row.Count; c++)
                if (c < numCols && nonEmptyCols[c]) newRow.Add(row[c]);
            result.Add(newRow);
        }
        return result;
    }

    /// <summary>
    /// Fraction of the median word height used as the maximum horizontal gap for merging two
    /// horizontally-adjacent words in the same detected row into one cell token before column
    /// detection.
    /// </summary>
    /// <remarks>
    /// Without this pre-merge, a cell containing more than one word puts its second and third
    /// words at x-positions matching no genuine column start, so <c>DetectColumns</c> mints a
    /// spurious near-empty column per extra word — exactly what the downstream structural
    /// validator then rejects, discarding a genuine table entirely (upstream #688: a scanned
    /// newspaper stock table emitted zero rows against seventeen in ground truth).
    /// <para>
    /// 0.6 is upstream's measured value: on their 215-word fixture 0.4 gave 12 columns, 0.6 the
    /// correct 10, 0.8 gave 9 and 1.0 gave 6.
    /// </para>
    /// </remarks>
    private const double CellMergeGapHeightRatio = 0.6;

    /// <summary>The median height of <paramref name="words"/>, or 0 when there are none.</summary>
    private static uint MedianWordHeight(List<HocrWord> words)
    {
        if (words.Count == 0) return 0;
        var heights = words.Select(w => w.Height).ToList();
        heights.Sort();
        return heights[heights.Count / 2];
    }

    /// <summary>
    /// Merge horizontally-adjacent words within the same detected row into single cell tokens, so
    /// a multi-word cell reaches column detection as one token. See
    /// <see cref="CellMergeGapHeightRatio"/>.
    /// </summary>
    private static List<HocrWord> MergeWordsIntoCellTokens(List<HocrWord> words, List<uint> rowPositions)
    {
        if (words.Count <= 1 || rowPositions.Count == 0) return words;

        double mergeGap = MedianWordHeight(words) * CellMergeGapHeightRatio;

        var rows = new List<HocrWord>[rowPositions.Count];
        for (int i = 0; i < rows.Length; i++) rows[i] = new List<HocrWord>();
        foreach (var word in words)
            if (FindRowIndex(rowPositions, word) is int rowIndex && rowIndex < rows.Length)
                rows[rowIndex].Add(word);

        var tokens = new List<HocrWord>(words.Count);
        foreach (var rowWords in rows)
        {
            rowWords.Sort((a, b) => a.Left.CompareTo(b.Left));

            HocrWord? current = null;
            foreach (var word in rowWords)
            {
                if (current is not { } token) { current = word; continue; }

                double gap = (double)word.Left - (token.Left + token.Width);
                if (gap <= mergeGap)
                {
                    uint newRight = Math.Max(word.Left + word.Width, token.Left + token.Width);
                    uint newBottom = Math.Max(word.Top + word.Height, token.Top + token.Height);
                    token.Top = Math.Min(token.Top, word.Top);
                    token.Width = newRight > token.Left ? newRight - token.Left : 0;
                    token.Height = newBottom > token.Top ? newBottom - token.Top : 0;
                    token.Text = token.Text + " " + word.Text;
                    current = token;
                }
                else
                {
                    tokens.Add(token);
                    current = word;
                }
            }
            if (current is { } last) tokens.Add(last);
        }

        return tokens;
    }

    /// <summary>
    /// Reconstruct a grid from positioned words. Rows come first, then adjacent words in a row are
    /// merged into cell tokens, then columns are detected from those tokens; the <em>original</em>
    /// words are what gets assigned to cells, so cell text is unchanged wherever a column was
    /// already detected correctly.
    /// </summary>
    /// <remarks>
    /// A caller that later indexes the column positions against the grid must use
    /// <see cref="ReconstructTableWithColumns"/>: columns are detected from post-merge tokens, so
    /// a separate <c>DetectColumns</c> call on the raw words no longer corresponds to this grid.
    /// </remarks>
    internal static List<List<string>> ReconstructTable(List<HocrWord> words, uint columnThreshold, double rowThresholdRatio) =>
        ReconstructTableWithColumns(words, columnThreshold, rowThresholdRatio).Grid;

    /// <summary>
    /// As <see cref="ReconstructTable"/>, but also returns the column x-positions actually used to
    /// build the grid, filtered to the columns that survived empty-column removal — so position
    /// <c>i</c> corresponds exactly to column <c>i</c> of every row.
    /// </summary>
    internal static (List<List<string>> Grid, List<uint> ColumnPositions) ReconstructTableWithColumns(
        List<HocrWord> words, uint columnThreshold, double rowThresholdRatio)
    {
        if (words.Count == 0) return (new List<List<string>>(), new List<uint>());
        var rowPositions = DetectRows(words, rowThresholdRatio);
        var cellTokens = MergeWordsIntoCellTokens(words, rowPositions);
        var colPositions = DetectColumns(cellTokens, columnThreshold);
        if (colPositions.Count == 0 || rowPositions.Count == 0)
            return (new List<List<string>>(), new List<uint>());

        int numRows = rowPositions.Count, numCols = colPositions.Count;
        var cells = new List<string>[numRows, numCols];
        for (int r = 0; r < numRows; r++)
            for (int c = 0; c < numCols; c++)
                cells[r, c] = new List<string>();

        foreach (var word in words)
        {
            int? r = FindRowIndex(rowPositions, word);
            int? c = FindColumnIndex(colPositions, word);
            if (r is int ri && c is int ci && ri < numRows && ci < numCols)
                cells[ri, ci].Add(word.Text);
        }

        var result = new List<List<string>>(numRows);
        for (int r = 0; r < numRows; r++)
        {
            var row = new List<string>(numCols);
            for (int c = 0; c < numCols; c++)
                row.Add(cells[r, c].Count == 0 ? "" : string.Join(" ", cells[r, c]));
            result.Add(row);
        }

        var keptMask = NonEmptyColumnMask(result);
        var keptColumnPositions = new List<uint>(colPositions.Count);
        for (int c = 0; c < colPositions.Count && c < keptMask.Length; c++)
            if (keptMask[c]) keptColumnPositions.Add(colPositions[c]);

        return (RemoveEmptyRowsAndColumns(result), keptColumnPositions);
    }

    /// <summary>
    /// Render a reconstructed grid as a GFM pipe table, through the same renderer every other
    /// source uses — so a PDF table serialises identically to one from a docx, and a ragged row
    /// is padded out to the widest row rather than emitting fewer columns than the header.
    /// </summary>
    public static string TableToMarkdown(List<List<string>> table) =>
        XRay.Content.Rendering.RenderCommon.RenderTableMarkdown(table);

    internal static List<List<string>>? PostProcessTable(List<List<string>> table, bool layoutGuided, bool allowSingleColumn)
    {
        int minColumns = allowSingleColumn ? 1 : (layoutGuided ? 2 : 3);
        return PostProcessTableInner(table, minColumns, layoutGuided);
    }

    private static int CharCount(string s)
    {
        // Rust chars().count() is a Unicode scalar count, not UTF-16 units.
        int n = 0;
        foreach (var _ in s.EnumerateRunes()) n++;
        return n;
    }

    // Rust `str::len()` is the UTF-8 byte length.
    private static int Utf8Len(string s) => Encoding.UTF8.GetByteCount(s);

    private static int WordCount(string s)
    {
        int n = 0;
        foreach (var _ in SplitWhitespace(s)) n++;
        return n;
    }

    /// <summary>
    /// Port of `pdf/table_reconstruct.rs:158` `merge_rows_columnwise`: collapse
    /// <paramref name="rows"/> into a single row whose i-th cell is the space-joined text of
    /// every non-empty i-th cell. Upstream has one such function with two callers — the
    /// header merge below, and the fragmented-table stitch in `PdfTableStitch` — so this is
    /// deliberately shared rather than inlined at either site.
    /// </summary>
    internal static List<string> MergeRowsColumnwise(List<List<string>> rows, int columnCount)
    {
        var merged = new List<string>(columnCount);
        for (int i = 0; i < columnCount; i++) merged.Add("");
        foreach (var row in rows)
        {
            int limit = Math.Min(row.Count, columnCount);
            for (int idx = 0; idx < limit; idx++)
            {
                string trimmed = row[idx].Trim();
                if (trimmed.Length == 0) continue;
                merged[idx] = merged[idx].Length == 0 ? trimmed : merged[idx] + " " + trimmed;
            }
        }
        return merged;
    }

    private static List<List<string>>? PostProcessTableInner(List<List<string>> table, int minColumns, bool layoutGuided)
    {
        table = table.Where(row => row.Any(cell => cell.Trim().Length > 0)).Select(r => new List<string>(r)).ToList();
        if (table.Count == 0) return null;

        int nonEmpty = 0, longCells = 0, totalChars = 0;
        foreach (var row in table)
            foreach (var cell in row)
            {
                string t = cell.Trim();
                if (t.Length == 0) continue;
                int cc = CharCount(t);
                nonEmpty++;
                totalChars += cc;
                if (cc > 60) longCells++;
            }

        if (nonEmpty > 0)
        {
            if (layoutGuided)
            {
                if (longCells > 0)
                {
                    int long100 = table.SelectMany(r => r).Count(cell =>
                    {
                        string t = cell.Trim();
                        return t.Length > 0 && CharCount(t) > 100;
                    });
                    if (long100 * 10 > nonEmpty * 7) return null;
                }
                if (totalChars / nonEmpty > 80) return null;
            }
            else
            {
                if (longCells * 2 > nonEmpty) return null;
                if (totalChars / nonEmpty > 50) return null;
            }
        }

        int colCount = table.Count > 0 ? table[0].Count : 0;
        if (colCount < minColumns) return null;

        int dataStart = FindDataStart(table, layoutGuided);

        var headerRows = dataStart > 0 ? table.GetRange(0, dataStart) : new List<List<string>>();
        var dataRows = table.GetRange(dataStart, table.Count - dataStart);

        if (headerRows.Count > 2)
        {
            // Keep the two-row header cap, but do not discard an unusually long prefix the
            // data-start inference produced: earlier rows are still table content. They are
            // demoted to data in their original order, and the two rows closest to the detected
            // boundary stay the header (xberg-io/xberg#1558).
            int surplus = headerRows.Count - 2;
            var demoted = headerRows.GetRange(0, surplus);
            headerRows.RemoveRange(0, surplus);
            demoted.AddRange(dataRows);
            dataRows = demoted;
        }

        if (headerRows.Count == 0)
        {
            if (dataRows.Count < 2) return null;
            headerRows.Add(dataRows[0]);
            dataRows = dataRows.GetRange(1, dataRows.Count - 1);
        }

        int columnCount = (headerRows.Count > 0 ? headerRows[0] : (dataRows.Count > 0 ? dataRows[0] : new List<string>())).Count;
        if (columnCount == 0) return null;

        var header = MergeRowsColumnwise(headerRows, columnCount);

        var processed = new List<List<string>> { header };
        processed.AddRange(dataRows);
        if (processed.Count <= 1) return null;

        // Remove header-only columns.
        int col = 0;
        while (col < processed[0].Count)
        {
            string headerText = processed[0][col].Trim();
            bool dataEmpty = true;
            for (int r = 1; r < processed.Count; r++)
            {
                if (col < processed[r].Count && processed[r][col].Trim().Length > 0) { dataEmpty = false; break; }
            }
            if (dataEmpty) MergeHeaderOnlyColumn(processed, col, headerText);
            else col++;

            if (processed.Count == 0 || processed[0].Count == 0) return null;
        }

        if (processed[0].Count < 2 || processed.Count <= 1) return null;

        PruneSpuriousInteriorColumn(processed, layoutGuided);

        int dataRowCount = processed.Count - 1;

        // Column sparsity check.
        if (dataRowCount > 0)
        {
            for (int c = 0; c < processed[0].Count; c++)
            {
                int emptyCount = 0;
                for (int r = 1; r < processed.Count; r++)
                    if (c >= processed[r].Count || processed[r][c].Trim().Length == 0) emptyCount++;
                bool tooSparse = layoutGuided ? emptyCount * 20 > dataRowCount * 19 : emptyCount * 4 > dataRowCount * 3;
                if (tooSparse) return null;
            }
        }

        // Overall density check.
        {
            int totalDataCells = dataRowCount * processed[0].Count;
            if (totalDataCells > 0)
            {
                int filled = 0;
                for (int r = 1; r < processed.Count; r++)
                    foreach (var cell in processed[r])
                        if (cell.Trim().Length > 0) filled++;
                bool tooSparse = layoutGuided ? filled * 20 < totalDataCells * 3 : filled * 5 < totalDataCells * 2;
                if (tooSparse) return null;
            }
        }

        // Single-word prose check (>=5 cols).
        if (processed[0].Count >= 5)
        {
            int singleWord = 0, nonEmptyCells = 0;
            for (int r = 1; r < processed.Count; r++)
                foreach (var cell in processed[r])
                {
                    string t = cell.Trim();
                    if (t.Length == 0) continue;
                    nonEmptyCells++;
                    if (WordCount(t) <= 2) singleWord++;
                }
            int threshold = layoutGuided ? 85 : 70;
            // A grid whose cells are values rather than words is tabular data however thin the
            // cells are: an invoice's line items are one or two words each and every one of them
            // is a number.
            bool denseNumericGrid = IsDenseNumericGrid(processed);
            bool denseScalarGrid = layoutGuided && IsDenseScalarGrid(processed);
            if (!denseNumericGrid
                && !denseScalarGrid
                && !IsPredominantlyNumericShortGrid(processed)
                && nonEmptyCells >= 6
                && singleWord * 100 > nonEmptyCells * threshold) return null;
        }

        // Column-text-flow check (col0 → col1).
        if (processed[0].Count >= 2)
        {
            int flowRows = 0, eligibleRows = 0;
            for (int r = 1; r < processed.Count; r++)
            {
                string col0 = processed[r].Count > 0 ? processed[r][0].Trim() : "";
                string col1 = processed[r].Count > 1 ? processed[r][1].Trim() : "";
                if (col0.Length == 0 || col1.Length == 0) continue;
                eligibleRows++;
                bool endsWithoutPunct = !col0.EndsWith('.') && !col0.EndsWith('?') && !col0.EndsWith('!') && !col0.EndsWith(':');
                bool startsLower = col1.Length > 0 && char.IsLower(col1[0]);
                if (endsWithoutPunct && startsLower) flowRows++;
            }
            if (eligibleRows >= 3 && flowRows * 10 > eligibleRows * 6) return null;
        }

        // Content asymmetry check.
        {
            int numCols = processed[0].Count;
            var colCharCounts = new long[numCols];
            for (int c = 0; c < numCols; c++)
            {
                long sum = 0;
                for (int r = 1; r < processed.Count; r++)
                    if (c < processed[r].Count) sum += Utf8Len(processed[r][c].Trim());
                colCharCounts[c] = sum;
            }
            long totalCharsAsym = colCharCounts.Sum();
            if (totalCharsAsym > 0)
            {
                double maxColShare = 0.0;
                foreach (var cc in colCharCounts) maxColShare = Math.Max(maxColShare, (double)cc / totalCharsAsym);
                double dominantThreshold = layoutGuided ? 0.92 : 0.85;
                if (maxColShare > dominantThreshold) return null;

                if (!layoutGuided)
                {
                    for (int c = 0; c < numCols; c++)
                    {
                        double charShare = (double)colCharCounts[c] / totalCharsAsym;
                        int emptyInCol = 0;
                        for (int r = 1; r < processed.Count; r++)
                            if (c >= processed[r].Count || processed[r][c].Trim().Length == 0) emptyInCol++;
                        double emptyRatio = (double)emptyInCol / dataRowCount;
                        if (charShare < 0.15 && emptyRatio > 0.5) return null;
                    }
                }
            }
        }

        // Row-to-row sentence continuation check.
        if (processed.Count > 3 && processed[0].Count >= 2)
        {
            int lastCol = processed[0].Count - 1;
            int continuation = 0, eligibleTransitions = 0;
            for (int r = 1; r + 1 < processed.Count; r++)
            {
                string prevLast = lastCol < processed[r].Count ? processed[r][lastCol].Trim() : "";
                string nextFirst = processed[r + 1].Count > 0 ? processed[r + 1][0].Trim() : "";
                if (prevLast.Length == 0 || nextFirst.Length == 0) continue;
                eligibleTransitions++;
                bool endsWithoutPunct = !prevLast.EndsWith('.') && !prevLast.EndsWith('?') && !prevLast.EndsWith('!') && !prevLast.EndsWith(':') && !prevLast.EndsWith(';');
                bool startsLower = nextFirst.Length > 0 && char.IsLower(nextFirst[0]);
                if (endsWithoutPunct && startsLower) continuation++;
            }
            if (eligibleTransitions >= 3 && continuation * 10 > eligibleTransitions * 4) return null;
        }

        // High-row low-column prose check.
        {
            int numCols = processed[0].Count;
            int numDataRows = processed.Count - 1;
            if (numDataRows > 20 && numCols <= 3)
            {
                int totalDataCells = numDataRows * numCols;
                int filledCells = 0;
                for (int r = 1; r < processed.Count; r++)
                    foreach (var cell in processed[r]) if (cell.Trim().Length > 0) filledCells++;
                // The density alone is not the signal: a 30-row ledger of three columns is dense
                // too. What separates prose is words per cell — a table cell holds a value, a
                // reflowed column holds a phrase.
                if (totalDataCells > 0
                    && filledCells * 100 > totalDataCells * 80
                    && LooksLikeProseInColumns(processed.GetRange(1, processed.Count - 1), numCols)) return null;
            }
        }

        // Uniform column width check.
        {
            int numCols = processed[0].Count;
            int numDataRows = processed.Count - 1;
            if (numCols >= 3 && numCols <= 5 && numDataRows >= 5)
            {
                var colAvg = new double[numCols];
                for (int c = 0; c < numCols; c++)
                {
                    int totalLen = 0, count = 0;
                    for (int r = 1; r < processed.Count; r++)
                    {
                        string cell = c < processed[r].Count ? processed[r][c].Trim() : "";
                        if (cell.Length > 0) { totalLen += Utf8Len(cell); count++; }
                    }
                    colAvg[c] = count > 0 ? (double)totalLen / count : 0.0;
                }
                var textColAvgs = colAvg.Where(a => a > 15.0).ToList();
                if (textColAvgs.Count >= 3)
                {
                    double minAvg = textColAvgs.Min();
                    double maxAvg = textColAvgs.Max();
                    if (minAvg > 0.0 && maxAvg <= minAvg * 2.0)
                    {
                        int totalDataCells = numDataRows * numCols;
                        int filledCells = 0;
                        for (int r = 1; r < processed.Count; r++)
                            foreach (var cell in processed[r]) if (cell.Trim().Length > 0) filledCells++;
                        double fillRate = (double)filledCells / totalDataCells;
                        if (fillRate > 0.75) return null;
                    }
                }
            }
        }

        // Normalize cells.
        for (int i = 0; i < processed[0].Count; i++)
            processed[0][i] = processed[0][i].Trim().Replace("  ", " ");
        // Gated per column rather than applied to every data cell: the dash and exponent
        // rewriting is right for a financial column (an em-dash cell means nil, `1.5E-05` is
        // scientific notation) and corrupts a prose one (`Functionaliteit—12`, a part code
        // `HRE - HReco`). Row 0 never reaches this loop (xberg-io/xberg#1582).
        var numericColumns = new bool[processed[0].Count];
        for (int c = 0; c < numericColumns.Length; c++)
            numericColumns[c] = ColumnIsNumericForNormalization(processed, c);

        for (int r = 1; r < processed.Count; r++)
            for (int c = 0; c < processed[r].Count; c++)
                processed[r][c] = c < numericColumns.Length && numericColumns[c]
                    ? NormalizeDataCell(processed[r][c])
                    : processed[r][c].Trim();

        return processed;
    }

    private static void MergeHeaderOnlyColumn(List<List<string>> table, int col, string headerText)
    {
        if (table.Count == 0 || table[0].Count == 0) return;
        string trimmed = headerText.Trim();

        if (trimmed.Length == 0 && table.Count > 1)
        {
            foreach (var row in table) if (col < row.Count) row.RemoveAt(col);
            return;
        }

        if (trimmed.Length > 0)
        {
            if (col > 0)
            {
                int target = col - 1;
                while (target > 0 && table[0][target].Trim().Length == 0) target--;
                if (table[0][target].Trim().Length > 0 || target == 0)
                {
                    if (table[0][target].Length > 0) table[0][target] += " ";
                    table[0][target] += trimmed;
                    foreach (var row in table) if (col < row.Count) row.RemoveAt(col);
                    return;
                }
            }
            if (col + 1 < table[0].Count)
            {
                if (table[0][col + 1].Trim().Length == 0)
                    table[0][col + 1] = trimmed;
                else
                    table[0][col + 1] = trimmed + " " + table[0][col + 1].Trim();
                foreach (var row in table) if (col < row.Count) row.RemoveAt(col);
                return;
            }
        }

        foreach (var row in table) if (col < row.Count) row.RemoveAt(col);
    }

    private static string NormalizeDataCell(string cell)
    {
        string trimmed = cell.Trim();
        if (trimmed.Length == 0) return "";

        string text = NormalizeDashGlyphsAndSpacing(trimmed);
        text = text.Replace("E-", "e-").Replace("E+", "e+");

        return text == "-" ? "" : text;
    }

    /// <summary>
    /// Rewrite em-dash, en-dash and minus-sign glyphs to an ASCII hyphen and collapse the
    /// whitespace around a leading or embedded one (<c>"- 3"</c> → <c>"-3"</c>), without the
    /// exponent lowercasing or lone-dash clearing that follow it.
    /// </summary>
    /// <remarks>
    /// Shared with <see cref="ColumnIsNumericForNormalization"/>, which needs the same
    /// dash-normalized preview to decide whether a cell is numeric <em>before</em> the full
    /// normalization runs: testing the raw text would miss <c>"- 3"</c>, which only reads as a
    /// number once this rewrite has run.
    /// </remarks>
    private static string NormalizeDashGlyphsAndSpacing(string text)
    {
        text = text.Replace('\u2014', '-').Replace('\u2013', '-').Replace('\u2212', '-');
        if (text.StartsWith("- ", StringComparison.Ordinal)) text = "-" + text.Substring(2).TrimStart();
        text = text.Replace("- ", "-");
        text = text.Replace(" -", "-");
        return text;
    }

    /// <summary>
    /// Minimum percentage of a column's non-ambiguous data cells that must parse as a bare
    /// numeric literal for the column to receive the dash and exponent rewriting.
    /// </summary>
    private const int NumericColumnMinNumericPercent = 60;

    /// <summary>
    /// Whether a column's data rows are predominantly bare numeric literals once dash glyphs are
    /// normalized — the gate that keeps the numeric rewriting off a prose column.
    /// </summary>
    /// <remarks>
    /// A cell that is nothing but a dash is nil-or-N/A and cannot decide the question on its own,
    /// so it is excluded from the vote and left to the column's other cells.
    /// </remarks>
    private static bool ColumnIsNumericForNormalization(List<List<string>> table, int col)
    {
        int evidence = 0, numeric = 0;
        for (int r = 1; r < table.Count; r++)
        {
            if (col >= table[r].Count) continue;
            string trimmed = table[r][col].Trim();
            if (trimmed.Length == 0 || IsLoneDashCell(trimmed)) continue;
            evidence++;
            if (LooksLikeNumericLiteral(NormalizeDashGlyphsAndSpacing(trimmed))) numeric++;
        }
        return evidence > 0 && numeric * 100 >= evidence * NumericColumnMinNumericPercent;
    }

    /// <summary>Whether the cell is nothing but one dash glyph — ambiguous nil-or-N/A content
    /// that carries no evidence either way.</summary>
    private static bool IsLoneDashCell(string text) =>
        text is "-" or "\u2014" or "\u2013" or "\u2212";

    /// <summary>
    /// Whether the text — already dash-normalized — is a bare numeric literal: an optional
    /// leading <c>-</c>, digits with at most one <c>.</c>, and an optional exponent. Anything
    /// carrying a letter outside the exponent marker, or no digits at all, is not a number.
    /// </summary>
    private static bool LooksLikeNumericLiteral(string text)
    {
        if (text.StartsWith('-')) text = text.Substring(1);
        int e = text.IndexOfAny(['e', 'E']);
        string mantissa = e < 0 ? text : text.Substring(0, e);
        if (!IsNumericMantissa(mantissa)) return false;
        if (e < 0) return true;

        string exponent = text.Substring(e + 1);
        if (exponent.StartsWith('-') || exponent.StartsWith('+')) exponent = exponent.Substring(1);
        return exponent.Length > 0 && exponent.All(char.IsAsciiDigit);
    }

    /// <summary>Whether the text is one or more ASCII digits with at most one <c>.</c>.</summary>
    private static bool IsNumericMantissa(string text)
    {
        bool seenDot = false, seenDigit = false;
        foreach (char c in text)
        {
            if (char.IsAsciiDigit(c)) seenDigit = true;
            else if (c == '.' && !seenDot) seenDot = true;
            else return false;
        }
        return seenDigit;
    }

    // ── header/column repair (pdf/table_reconstruct.rs) ─────────────────────

    private const int DefaultMinDataRowDigitCells = 3;
    private const int RepeatedDataRowCount = 3;
    private const int RowShapeMinOverlapPercent = 80;
    private const int SpuriousColumnMinDataRows = 20;
    private const int SpuriousColumnMinColumns = 6;
    private const int SpuriousColumnMinRetainedDensityPercent = 75;
    private const int FooterMinAlphaPercent = 70;

    /// <summary>
    /// The row the data starts on. The first row carrying enough digit cells usually is it, but a
    /// wide layout-guided table can spell its header over several lines — a units row in
    /// parentheses under multi-word labels — and the first of those lines already looks numeric.
    /// Where three consecutive rows of the same shape follow, that run is the data instead.
    /// </summary>
    private static int FindDataStart(List<List<string>> table, bool layoutGuided)
    {
        int firstNumericRow = 0;
        for (int i = 0; i < table.Count; i++)
            if (DigitCellCount(table[i]) >= DefaultMinDataRowDigitCells) { firstNumericRow = i; break; }

        int columnCount = table.Count > 0 ? table[0].Count : 0;
        if (!layoutGuided || columnCount < LargeTableMinColumns || table.Count < RepeatedDataRowCount)
            return firstNumericRow;

        int repeatedStart = -1;
        for (int i = 0; i + RepeatedDataRowCount <= table.Count; i++)
        {
            bool allNumeric = true, shapesMatch = true;
            for (int k = 0; k < RepeatedDataRowCount; k++)
                if (DigitCellCount(table[i + k]) < DefaultMinDataRowDigitCells) { allNumeric = false; break; }
            if (!allNumeric) continue;
            for (int k = 0; k + 1 < RepeatedDataRowCount; k++)
                if (!RowShapesMatch(table[i + k], table[i + k + 1])) { shapesMatch = false; break; }
            if (shapesMatch) { repeatedStart = i; break; }
        }

        if (repeatedStart < 0) return firstNumericRow;
        if (repeatedStart == firstNumericRow) return repeatedStart;

        var continuation = table.GetRange(firstNumericRow + 1, Math.Max(0, repeatedStart - firstNumericRow - 1));
        return LooksLikeMultilineNumericHeader(table[firstNumericRow], continuation)
            ? repeatedStart
            : firstNumericRow;
    }

    /// <summary>
    /// Whether a row that looks numeric is really the first line of a wrapped header: multi-word
    /// labels above a shorter continuation row that carries a parenthesised unit.
    /// </summary>
    private static bool LooksLikeMultilineNumericHeader(List<string> header, List<List<string>> continuationRows)
    {
        int filledHeaderCells = header.Count(c => c.Trim().Length > 0);
        int multiwordLabels = header.Count(c =>
        {
            string text = c.Trim();
            return WordCount(text) >= 2 && text.Any(char.IsLetter);
        });
        var continuationCells = continuationRows
            .SelectMany(r => r).Select(c => c.Trim()).Where(c => c.Length > 0).ToList();
        bool hasParenthesizedUnit = continuationCells.Any(c => c.StartsWith('(') && c.Contains(')'));

        return continuationRows.Count > 0
            && multiwordLabels >= 2
            && continuationCells.Count < filledHeaderCells
            && hasParenthesizedUnit;
    }

    private static int DigitCellCount(List<string> row) => row.Count(cell => cell.Any(char.IsAsciiDigit));

    /// <summary>Whether two rows fill the same columns, within a tolerance.</summary>
    private static bool RowShapesMatch(List<string> left, List<string> right)
    {
        int columnCount = Math.Max(left.Count, right.Count);
        int union = 0, intersection = 0;
        for (int c = 0; c < columnCount; c++)
        {
            bool leftFilled = c < left.Count && left[c].Trim().Length > 0;
            bool rightFilled = c < right.Count && right[c].Trim().Length > 0;
            if (leftFilled || rightFilled) union++;
            if (leftFilled && rightFilled) intersection++;
        }
        return union > 0 && intersection * 100 >= union * RowShapeMinOverlapPercent;
    }

    /// <summary>
    /// Drop one empty-header interior column that catches nothing but a stray footer word, in a
    /// large otherwise-dense table. Such a track appears when a word in the footer sits at an
    /// x-position the body never uses.
    /// </summary>
    private static bool PruneSpuriousInteriorColumn(List<List<string>> table, bool layoutGuided)
    {
        if (table.Count == 0) return false;
        int columnCount = table[0].Count;
        int dataRowCount = table.Count - 1;
        if (!layoutGuided || columnCount < SpuriousColumnMinColumns || dataRowCount < SpuriousColumnMinDataRows)
            return false;

        var candidates = new List<int>();
        for (int column = 1; column < columnCount - 1; column++)
        {
            if (table[0][column].Trim().Length != 0) continue;
            var populatedRows = new List<int>();
            for (int r = 1; r < table.Count; r++)
                if (column < table[r].Count && table[r][column].Trim().Length > 0) populatedRows.Add(r - 1);
            if (populatedRows.Count == 1 && populatedRows[0] == dataRowCount - 1 && LooksLikeFooterRow(table[^1]))
                candidates.Add(column);
        }
        if (candidates.Count != 1) return false;
        int spurious = candidates[0];

        int retainedCells = dataRowCount * (columnCount - 1);
        int retainedFilled = 0;
        for (int r = 1; r < table.Count; r++)
            for (int c = 0; c < table[r].Count; c++)
                if (c != spurious && table[r][c].Trim().Length > 0) retainedFilled++;
        if (retainedCells == 0
            || retainedFilled * 100 < retainedCells * SpuriousColumnMinRetainedDensityPercent) return false;

        MergeInteriorColumn(table, spurious);
        return true;
    }

    /// <summary>A row of mostly-alphabetic multi-word text, the shape of a page footer.</summary>
    private static bool LooksLikeFooterRow(List<string> row)
    {
        var nonEmpty = row.Select(c => c.Trim()).Where(c => c.Length > 0).ToList();
        if (nonEmpty.Count < 2 || !nonEmpty.Any(c => WordCount(c) >= 2)) return false;
        string text = string.Join(" ", nonEmpty);
        int alphanumeric = text.Count(char.IsLetterOrDigit);
        int alphabetic = text.Count(char.IsLetter);
        return alphanumeric > 0 && alphabetic * 100 >= alphanumeric * FooterMinAlphaPercent;
    }

    /// <summary>Remove a column, folding whatever text it held into whichever neighbour is fuller.</summary>
    private static void MergeInteriorColumn(List<List<string>> table, int column)
    {
        int leftOccupancy = 0, rightOccupancy = 0;
        for (int r = 1; r < table.Count; r++)
        {
            if (column - 1 < table[r].Count && table[r][column - 1].Trim().Length > 0) leftOccupancy++;
            if (column + 1 < table[r].Count && table[r][column + 1].Trim().Length > 0) rightOccupancy++;
        }
        bool mergeRight = rightOccupancy >= leftOccupancy;

        foreach (var row in table)
        {
            if (column >= row.Count) continue;
            string text = row[column].Trim();
            row.RemoveAt(column);
            if (text.Length == 0) continue;
            int target = mergeRight ? column : column - 1;
            if (target < 0 || target >= row.Count) continue;
            string existing = row[target].Trim();
            row[target] = existing.Length == 0
                ? text
                : mergeRight ? $"{text} {existing}" : $"{existing} {text}";
        }
    }

    private const int DenseScalarMinDataRows = 20;
    private const int DenseScalarMinColumns = 6;
    private const int DenseScalarMinFilledPercent = 75;
    private const int DenseScalarMinCompactPercent = 90;
    private const int DenseScalarMinDigitPercent = 25;
    private const int DenseScalarMaxCellChars = 24;
    private const int ShortNumericMinDataCells = 4;
    private const int ShortNumericMinCellPercent = 60;
    private const int ShortNumericMinRowOccupancyPercent = 85;

    /// <summary>
    /// A big grid of short cells, most of them carrying a digit — a schedule or a price list,
    /// whose one-and-two-word cells would otherwise read as shredded prose.
    /// </summary>
    private static bool IsDenseScalarGrid(List<List<string>> grid)
    {
        if (grid.Count == 0) return false;
        int dataRows = grid.Count - 1;
        if (grid[0].Count < DenseScalarMinColumns || dataRows < DenseScalarMinDataRows) return false;

        int totalCells = dataRows * grid[0].Count;
        int filled = 0, compact = 0, digit = 0;
        for (int r = 1; r < grid.Count; r++)
            foreach (var cell in grid[r])
            {
                string t = cell.Trim();
                if (t.Length == 0) continue;
                filled++;
                if (CharCount(t) <= DenseScalarMaxCellChars && WordCount(t) <= 2) compact++;
                if (t.Any(char.IsAsciiDigit)) digit++;
            }

        return totalCells > 0
            && filled * 100 >= totalCells * DenseScalarMinFilledPercent
            && compact * 100 >= filled * DenseScalarMinCompactPercent
            && digit * 100 >= filled * DenseScalarMinDigitPercent;
    }

    /// <summary>
    /// Whether the data cells are overwhelmingly numeric with no size floor — a borderless
    /// line-item table. Measured twice, over every data cell and over the substantially populated
    /// rows only, because a wrapped description row with the rest of its columns blank drags the
    /// pooled fraction below the bar. Either measurement clearing it grants the exemption.
    /// </summary>
    private static bool IsPredominantlyNumericShortGrid(List<List<string>> grid)
    {
        int width = grid.Count > 0 ? grid[0].Count : 0;
        return ShortGridNumericRatioMeetsBar(grid, false)
            || (width > 0 && ShortGridNumericRatioMeetsBar(grid, true));
    }

    private static bool ShortGridNumericRatioMeetsBar(List<List<string>> grid, bool substantiallyPopulatedRowsOnly)
    {
        int width = grid.Count > 0 ? grid[0].Count : 0;
        int nonEmpty = 0, numeric = 0;
        for (int r = 1; r < grid.Count; r++)
        {
            if (substantiallyPopulatedRowsOnly && !IsSubstantiallyPopulatedDataRow(grid[r], width)) continue;
            foreach (var cell in grid[r])
            {
                string t = cell.Trim();
                if (t.Length == 0) continue;
                nonEmpty++;
                if (IsNumericValueCell(t)) numeric++;
            }
        }
        return nonEmpty >= ShortNumericMinDataCells
            && numeric * 100 >= nonEmpty * ShortNumericMinCellPercent;
    }

    /// <summary>Whether the row fills enough of the inferred grid to be self-contained.</summary>
    private static bool IsSubstantiallyPopulatedDataRow(List<string> row, int width)
    {
        if (width == 0) return false;
        int populated = row.Take(width).Count(cell => cell.Trim().Length > 0);
        return populated * 100 >= width * ShortNumericMinRowOccupancyPercent;
    }

    /// <summary>
    /// Whether a dense grid of data rows is prose laid out in columns rather than a real table.
    /// The signal is words per cell: a table cell holds a value — a number, a code, a short label
    /// — while columned prose fills each cell with a phrase.
    /// </summary>
    private static bool LooksLikeProseInColumns(List<List<string>> dataRows, int numCols)
    {
        const double ProseWordsPerCell = 4.0;

        if (numCols < 2) return false;
        int proseRows = 0, eligibleRows = 0;
        foreach (var row in dataRows)
        {
            var cells = row.Select(c => c.Trim()).Where(c => c.Length > 0).ToList();
            if (cells.Count < 2) continue;
            if (cells.Sum(Utf8Len) < 15) continue;
            eligibleRows++;
            double avgWords = (double)cells.Sum(WordCount) / cells.Count;
            if (avgWords >= ProseWordsPerCell) proseRows++;
        }
        return eligibleRows >= 3 && proseRows * 2 > eligibleRows;
    }

    // ── is_well_formed_table (pdf/table_reconstruct.rs) ──────────────────────

    // ── is_well_formed_table (pdf/table_reconstruct.rs) ─────────────────────

    private const int DenseNumericMinDataRows = 6;
    private const int DenseNumericMinColumns = 6;
    private const int DenseNumericMinCellPercent = 75;
    private const int ShortWideMaxDataRows = 2;
    private const int ShortWideMinColumns = 6;
    private const int ShortWideMaxEmptyCellPercent = 35;
    private const int LargeTableMinColumns = 6;
    private const int DefaultMaxEmptyCellPercent = 40;

    /// <summary>
    /// Whether a reconstructed grid is a real table rather than multi-column prose, a repeated
    /// page element or low-vocabulary boilerplate. Ports <c>is_well_formed_table_core</c>.
    /// </summary>
    /// <param name="skipColumnarProseGuard">
    /// Drops only the uniform-column-length prose heuristic, for callers that have already vetted
    /// the region geometrically: a genuine key-value grid has the regular short columns that
    /// heuristic mistakes for wrapped prose. Every other guard still applies.
    /// </param>

    // ── Heuristic column gap and grid repairs (pdf/oxide/table.rs) ───────────

    private const uint DenseNumericColumnGapCap = 20;
    private const int DenseNumericMinRecurringRows = 5;
    private const int DenseNumericMinWordsPerRow = 3;
    private const int DenseNumericMinRowWordPercent = 60;
    private const int DenseNumericMinRecurringTracks = 4;
    private const int DenseNumericMinTrackRowPercent = 60;
    private const int NumericHeaderMaxRows = 2;
    private const int NumericHeaderMinTrackPercent = 60;
    private const int NumericHeaderMinAlphaPercent = 60;
    private const int SplitNumericTrackMinRowsPerSide = 2;
    private const int SplitNumericTrackMinTotalRows = 6;

    /// <summary>
    /// A ledger's columns sit closer together than ordinary prose, so a wide adaptive
    /// gap would merge two numeric tracks into one column.
    /// </summary>
    private static uint HeuristicColumnGap(List<HocrWord> region, float regionWidth)
    {
        uint adaptive = ComputeAdaptiveColumnGap(region, regionWidth);
        return IsDenseNumericRegion(region) ? Math.Min(adaptive, DenseNumericColumnGapCap) : adaptive;
    }

    private static bool IsNumericWord(string text)
    {
        int digits = 0, alphanumeric = 0;
        foreach (char c in text)
        {
            if (char.IsAsciiDigit(c)) digits++;
            if (char.IsLetterOrDigit(c)) alphanumeric++;
        }
        return digits > 0 && digits * 2 >= alphanumeric;
    }

    private static bool IsDenseNumericRegion(List<HocrWord> region)
    {
        if (region.Count == 0) return false;
        var heights = region.Select(w => w.Height).OrderBy(h => h).ToList();
        uint medianHeight = Math.Max(heights[heights.Count / 2], 1);
        uint rowTolerance = Math.Max(medianHeight / 2, 3);
        var rows = NumericRows(region, rowTolerance);
        var recurring = LongestRecurringNumericRun(rows);
        if (recurring is null) return false;
        return RecurringNumericTrackCenters(recurring, medianHeight).Count >= DenseNumericMinRecurringTracks;
    }

    private static List<List<HocrWord>> NumericRows(List<HocrWord> region, uint rowTolerance)
    {
        var sorted = region.OrderBy(w => w.Top + w.Height / 2).ToList();
        var rows = new List<List<HocrWord>>();
        foreach (var word in sorted)
        {
            uint center = word.Top + word.Height / 2;
            if (rows.Count > 0 && AbsDiff(rows[^1][0].Top + rows[^1][0].Height / 2, center) <= rowTolerance)
                rows[^1].Add(word);
            else rows.Add(new List<HocrWord> { word });
        }
        return rows;
    }

    private static bool IsNumericRow(List<HocrWord> row)
    {
        int numeric = row.Count(w => IsNumericWord(w.Text));
        return numeric >= DenseNumericMinWordsPerRow
            && numeric * 100 >= row.Count * DenseNumericMinRowWordPercent;
    }

    /// <summary>The longest unbroken run of numeric rows, if it is long enough to count.</summary>
    private static List<List<HocrWord>>? LongestRecurringNumericRun(List<List<HocrWord>> rows)
    {
        int bestStart = 0, bestLen = 0, start = 0;
        for (int index = 0; index < rows.Count; index++)
        {
            if (IsNumericRow(rows[index])) continue;
            if (index - start > bestLen) { bestStart = start; bestLen = index - start; }
            start = index + 1;
        }
        if (rows.Count - start > bestLen) { bestStart = start; bestLen = rows.Count - start; }
        return bestLen >= DenseNumericMinRecurringRows ? rows.GetRange(bestStart, bestLen) : null;
    }

    private static List<uint> RecurringNumericTrackCenters(List<List<HocrWord>> rows, uint medianHeight)
    {
        uint xTolerance = Math.Max(medianHeight * 2, 12);
        int minRowSupport = (rows.Count * DenseNumericMinTrackRowPercent + 99) / 100;

        var candidates = rows.SelectMany(r => r).Where(w => IsNumericWord(w.Text))
            .Select(w => w.Left + w.Width / 2).OrderBy(v => v).ToList();
        var deduped = new List<uint>();
        foreach (uint c in candidates)
            if (deduped.Count == 0 || AbsDiff(deduped[^1], c) > xTolerance) deduped.Add(c);

        return deduped.Where(candidate =>
            rows.Count(row => row.Any(w => IsNumericWord(w.Text) && AbsDiff(w.Left + w.Width / 2, candidate) <= xTolerance))
                >= minRowSupport).ToList();
    }

    /// <summary>
    /// Fold a compact caption region into the numeric block it labels. Vertical clustering
    /// splits on a blank line, so a table whose header sits one line above its data arrives
    /// as two regions — and the header half, being one or two rows, is dropped by the
    /// region filter before anything can use it. Merging first is what lets the column names
    /// reach the grid.
    /// </summary>
    private static void AttachAlignedNumericHeaders(
        List<List<HocrWord>> regions, uint medianHeight, uint rowTolerance)
    {
        int index = 1;
        while (index < regions.Count)
        {
            if (IsAlignedNumericHeader(regions[index - 1], regions[index], medianHeight, rowTolerance))
            {
                var data = regions[index];
                regions.RemoveAt(index);
                regions[index - 1].AddRange(data);
            }
            else index++;
        }
    }

    /// <summary>A caption belongs to the block below it when it is short, mostly words, sits
    /// close enough to touch, and its glyphs line up with the numeric tracks underneath.</summary>
    private static bool IsAlignedNumericHeader(
        List<HocrWord> header, List<HocrWord> data, uint medianHeight, uint rowTolerance)
    {
        var dataRows = NumericRows(data, rowTolerance);
        var recurring = LongestRecurringNumericRun(dataRows);
        if (recurring is null) return false;

        var tracks = RecurringNumericTrackCenters(recurring, medianHeight);
        int headerRows = NumericRows(header, rowTolerance).Count;
        if (tracks.Count < DenseNumericMinRecurringTracks || headerRows < 1 || headerRows > NumericHeaderMaxRows)
            return false;

        int alphaWords = header.Count(w => w.Text.Any(char.IsLetter));
        if (alphaWords * 100 < header.Count * NumericHeaderMinAlphaPercent) return false;

        uint headerBottom = header.Count == 0 ? 0 : header.Max(w => w.Top + w.Height);
        uint dataTop = data.Count == 0 ? uint.MaxValue : data.Min(w => w.Top);
        if (SaturatingSub(dataTop, headerBottom) > medianHeight * 2) return false;

        uint xTolerance = Math.Max(medianHeight * 2, 12u);
        int matched = tracks.Count(track =>
            header.Any(w => AbsDiff(w.Left + w.Width / 2, track) <= xTolerance));
        return matched * 100 >= tracks.Count * NumericHeaderMinTrackPercent;
    }

    private static uint SaturatingSub(uint a, uint b) => a > b ? a - b : 0u;

    /// <summary>
    /// Rejoin one numeric column that column detection split in two. The signature is a
    /// pair of adjacent columns that are never both filled on the same row, whose
    /// positions are closer together than a single numeric value is wide.
    /// </summary>
    private static bool RepairSplitNumericTrack(
        List<List<string>> grid, List<HocrWord> region, List<uint> columnPositions)
    {
        if (grid.Count == 0 || grid[0].Count != columnPositions.Count) return false;

        int found = -1;
        for (int column = 0; column + 1 < columnPositions.Count; column++)
        {
            if (!IsSplitNumericTrackCandidate(grid, region, columnPositions, column)) continue;
            if (found >= 0) return false;   // more than one candidate: not this shape
            found = column;
        }
        if (found < 0) return false;

        foreach (var row in grid)
        {
            if (found + 1 >= row.Count) continue;
            string right = row[found + 1];
            row.RemoveAt(found + 1);
            if (row[found].Trim().Length == 0) row[found] = right;
        }
        // The two tracks were nearly coincident by construction — that is what made this a split
        // rather than two real columns — so the surviving cluster keeps the left anchor and the
        // spurious boundary is dropped, mirroring the row.RemoveAt(found + 1) above exactly.
        if (found + 1 < columnPositions.Count) columnPositions.RemoveAt(found + 1);
        return true;
    }

    private static bool IsSplitNumericTrackCandidate(
        List<List<string>> grid, List<HocrWord> region, List<uint> columnPositions, int column)
    {
        int leftNumeric = 0, rightNumeric = 0;
        bool leftHeader = false, rightHeader = false;
        for (int rowIndex = 0; rowIndex < grid.Count; rowIndex++)
        {
            var row = grid[rowIndex];
            if (column + 1 >= row.Count) return false;
            string left = row[column].Trim(), right = row[column + 1].Trim();
            if (left.Length > 0 && right.Length > 0) return false;
            string text;
            bool onLeft;
            if (left.Length > 0) { text = left; onLeft = true; }
            else if (right.Length > 0) { text = right; onLeft = false; }
            else continue;

            if (IsNumericWord(text)) { if (onLeft) leftNumeric++; else rightNumeric++; }
            else if (rowIndex >= NumericHeaderMaxRows) return false;
            else if (onLeft) leftHeader = true;
            else rightHeader = true;
        }

        if (leftNumeric < SplitNumericTrackMinRowsPerSide || rightNumeric < SplitNumericTrackMinRowsPerSide
            || leftNumeric + rightNumeric < SplitNumericTrackMinTotalRows || leftHeader == rightHeader)
            return false;

        uint separation = AbsDiff(columnPositions[column], columnPositions[column + 1]);
        var numericWidths = new List<uint>();
        foreach (var word in region)
        {
            if (!IsNumericWord(word.Text)) continue;
            int nearest = 0;
            uint bestDiff = AbsDiff(columnPositions[0], word.Left);
            for (int i = 1; i < columnPositions.Count; i++)
            {
                uint d = AbsDiff(columnPositions[i], word.Left);
                if (d < bestDiff) { nearest = i; bestDiff = d; }
            }
            if (nearest == column || nearest == column + 1) numericWidths.Add(word.Width);
        }
        if (numericWidths.Count == 0) return false;
        numericWidths.Sort();
        uint medianWidth = numericWidths[numericWidths.Count / 2];
        return separation * 2 < medianWidth;
    }

    /// <summary>
    /// Move a closing bracket that column detection stranded at the head of the next
    /// header cell back onto the cell whose opener it closes.
    /// </summary>
    private static void RepairHeuristicHeaderDelimiters(List<List<string>> rows)
    {
        if (rows.Count == 0) return;
        var header = rows[0];
        for (int column = 0; column + 1 < header.Count; column++)
        {
            var closerAndRest = LeadingSingleCloser(header[column + 1]);
            if (closerAndRest is not { } pair) continue;
            if (UnmatchedOpener(header[column]) != MatchingOpener(pair.Closer)) continue;
            header[column] += pair.Closer;
            header[column + 1] = pair.Remainder;
        }
    }

    private static (char Closer, string Remainder)? LeadingSingleCloser(string text)
    {
        if (text.Length == 0) return null;
        char closer = text[0];
        if (MatchingOpener(closer) is null) return null;
        string remainder = text.Substring(1).TrimStart();
        if (remainder.Length == 0 || MatchingOpener(remainder[0]) is not null) return null;
        return (closer, remainder);
    }

    private static char? UnmatchedOpener(string text)
    {
        var openers = new List<char>();
        foreach (char character in text)
        {
            if (character is '(' or '[' or '{') { openers.Add(character); continue; }
            char? expected = MatchingOpener(character);
            if (expected is null) continue;
            if (openers.Count == 0 || openers[^1] != expected) return null;
            openers.RemoveAt(openers.Count - 1);
        }
        return openers.Count > 0 ? openers[^1] : null;
    }

    private static char? MatchingOpener(char closer) => closer switch
    {
        ')' => '(',
        ']' => '[',
        '}' => '{',
        _ => null,
    };

    /// <summary>Fraction of (column boundary, row) pairs a word's box runs across.</summary>
    /// <remarks>
    /// A column boundary is the *start* of the next column, not the midpoint between two
    /// column positions: `DetectColumns` returns each column's median left edge, so a
    /// midpoint falls inside the left column's own text and any word wider than half the
    /// column pitch would straddle it. Measured per row, because a column boundary is a
    /// band of whitespace that holds on every row — one long word on one row is not prose.
    /// </remarks>
    internal static double StraddledBoundaryRatio(List<HocrWord> region, List<uint> columnPositions)
    {
        if (columnPositions.Count < 2 || region.Count == 0) return 0.0;

        var rowPositions = DetectRows(region, StraddleRowThresholdRatio);
        if (rowPositions.Count == 0) return 0.0;

        var rows = new List<List<HocrWord>>(rowPositions.Count);
        for (int i = 0; i < rowPositions.Count; i++) rows.Add(new List<HocrWord>());
        foreach (var word in region)
        {
            uint yCenter = (uint)word.YCenter;
            int best = 0;
            uint bestDiff = AbsDiff(rowPositions[0], yCenter);
            for (int i = 1; i < rowPositions.Count; i++)
            {
                uint d = AbsDiff(rowPositions[i], yCenter);
                if (d < bestDiff) { best = i; bestDiff = d; }
            }
            rows[best].Add(word);
        }

        int total = 0, straddled = 0;
        for (int b = 1; b < columnPositions.Count; b++)
        {
            uint boundary = columnPositions[b];
            foreach (var row in rows)
            {
                total++;
                foreach (var word in row)
                    if (word.Left < boundary && word.Left + word.Width > boundary) { straddled++; break; }
            }
        }
        return total == 0 ? 0.0 : (double)straddled / total;
    }

    /// <summary>
    /// Well-formedness gate for the borderless heuristic path, the only caller holding raw
    /// word geometry and the page's drawn-rule count. Drawn rules are strong enough
    /// evidence on their own; without them, the inferred column boundaries have to be
    /// actual whitespace, which continuous prose bucketed into a wide grid is not.
    /// </summary>
    internal static bool IsWellFormedBorderlessTable(
        List<List<string>> grid, List<HocrWord> region, List<uint> columnPositions, int horizontalRules)
    {
        if (!IsWellFormedTable(grid)) return false;
        if (horizontalRules > 0) return true;
        return StraddledBoundaryRatio(region, columnPositions) < MaxStraddledBoundaryRatio;
    }

    private const double MaxStraddledBoundaryRatio = 0.60;

    /// <summary>Row-grouping tolerance matching what `ReconstructTable` itself uses.</summary>
    private const double StraddleRowThresholdRatio = 0.5;

    internal static bool IsWellFormedTable(List<List<string>> grid, bool skipColumnarProseGuard = false)
    {
        if (grid.Count < 2) return false;
        int numCols = grid[0].Count;
        if (numCols < 2) return false;

        bool denseNumericGrid = IsDenseNumericGrid(grid);

        // Check 0: cell density. A short, wide grid is allowed to be emptier — its shape is the
        // evidence, not its fill — unless it is already a dense numeric grid.
        int dataRowCount = grid.Count - 1;
        int maxEmptyCellPercent =
            dataRowCount <= ShortWideMaxDataRows && numCols >= ShortWideMinColumns && !denseNumericGrid
                ? ShortWideMaxEmptyCellPercent
                : DefaultMaxEmptyCellPercent;
        int maxCols = grid.Max(r => r.Count);
        int totalCells = grid.Count * maxCols;
        if (totalCells > 0)
        {
            int filled = grid.SelectMany(r => r).Count(cell => cell.Trim().Length > 0);
            int emptyCells = totalCells - filled;
            if (emptyCells * 100 > totalCells * maxEmptyCellPercent) return false;
        }

        var dataRows = grid.GetRange(1, grid.Count - 1);

        // Check 1: a wide grid of one or two rows, every one of them a clause of a shredded
        // semicolon-delimited list rather than table data.
        if (dataRows.Count is >= 1 and < 3 && numCols >= LargeTableMinColumns && !denseNumericGrid)
        {
            if (dataRows.All(row => LooksLikeShreddedProseRow(row, numCols))) return false;
        }

        // Check 2: a one- or two-row grid that is really wrapped prose split across columns.
        if (dataRows.Count > 0 && !denseNumericGrid && LooksLikeShortColumnedProse(dataRows, numCols))
            return false;

        // Check 3: row coherence (prose detection).
        if (dataRows.Count >= 3 && numCols >= 2)
        {
            int proseLike = 0, eligible = 0;
            foreach (var row in dataRows)
            {
                string concatenated = string.Join(" ", row.Select(c => c.Trim()).Where(c => c.Length > 0));
                if (Utf8Len(concatenated) < 15) continue;
                eligible++;
                if (AlphaRatio(concatenated) > 0.8) proseLike++;
            }
            if (eligible >= 3 && proseLike * 2 > eligible) return false;
        }

        // Check 4: column semantic uniformity.
        if (numCols >= 3 && dataRows.Count >= 4)
        {
            var colStats = new (double Mean, double StdDev)[numCols];
            for (int c = 0; c < numCols; c++)
            {
                var lengths = new List<double>();
                foreach (var row in dataRows)
                {
                    string cell = c < row.Count ? row[c].Trim() : "";
                    if (cell.Length > 0) lengths.Add(Utf8Len(cell));
                }
                if (lengths.Count == 0) { colStats[c] = (0.0, 0.0); continue; }
                double mean = lengths.Sum() / lengths.Count;
                double variance = lengths.Sum(l => (l - mean) * (l - mean)) / lengths.Count;
                colStats[c] = (mean, Math.Sqrt(variance));
            }
            var meaningful = colStats.Where(st => st.Mean > 3.0).ToList();
            if (meaningful.Count >= 3)
            {
                double minMean = meaningful.Min(st => st.Mean);
                double maxMean = meaningful.Max(st => st.Mean);
                bool columnsUniform = minMean > 0.0 && maxMean <= minMean * 2.0;
                bool lowVariance = meaningful.All(st => st.Mean > 0.0 && st.StdDev / st.Mean < 0.3);
                if (!skipColumnarProseGuard && !denseNumericGrid && columnsUniform && lowVariance) return false;
            }
        }

        // Check 5: minimum meaningful content (repetitive vocabulary).
        if (numCols >= 3)
        {
            var uniqueWords = new HashSet<string>(StringComparer.Ordinal);
            foreach (var row in dataRows)
                foreach (var cell in row)
                    foreach (var w in SplitWhitespace(cell))
                        uniqueWords.Add(w);
            int rowCount = dataRows.Count;
            if (!denseNumericGrid && rowCount >= 3 && uniqueWords.Count < rowCount * 2) return false;
        }

        // Check 6: repeated header detection.
        {
            var header = grid[0];
            int headerMatches = dataRows.Count(row =>
                row.Count == header.Count && row.Zip(header, (a, b) => a.Trim() == b.Trim()).All(x => x));
            if (headerMatches >= 2) return false;
        }

        return true;
    }

    /// <summary>Fraction of a string that is letters or whitespace, measured over UTF-8 bytes.</summary>
    private static double AlphaRatio(string text)
    {
        int len = Utf8Len(text);
        if (len == 0) return 0.0;
        int alpha = 0;
        foreach (var rune in text.EnumerateRunes())
            if (System.Text.Rune.IsLetter(rune) || System.Text.Rune.IsWhiteSpace(rune)) alpha++;
        return (double)alpha / len;
    }

    /// <summary>A cell holding at least one digit, and at least as many digits as other letters.</summary>
    private static bool IsNumericValueCell(string cell)
    {
        int digits = cell.Count(char.IsAsciiDigit);
        if (digits == 0) return false;
        int alphanumeric = cell.Count(char.IsLetterOrDigit);
        return digits * 2 >= alphanumeric;
    }

    /// <summary>A cell with letters in it that is not a numeric value — the signal of wrapped prose.</summary>
    private static bool IsWordCell(string cell) => !IsNumericValueCell(cell) && cell.Any(char.IsLetter);

    /// <summary>
    /// A large grid whose data cells are overwhelmingly numeric. Such a grid is exempt from the
    /// prose guards: a ledger's regular short columns look exactly like wrapped columnar prose to
    /// them.
    /// </summary>
    private static bool IsDenseNumericGrid(List<List<string>> grid)
    {
        if (grid.Count == 0) return false;
        if (grid[0].Count < DenseNumericMinColumns || grid.Count <= DenseNumericMinDataRows) return false;

        int nonEmpty = 0, numeric = 0;
        for (int r = 1; r < grid.Count; r++)
            foreach (var cell in grid[r])
            {
                string trimmed = cell.Trim();
                if (trimmed.Length == 0) continue;
                nonEmpty++;
                if (IsNumericValueCell(trimmed)) numeric++;
            }

        return nonEmpty > 0 && numeric * 100 >= nonEmpty * DenseNumericMinCellPercent;
    }

    private const int ShreddedProseMinFilledCells = 4;
    private const double ShreddedProseMaxAvgWordsPerCell = 2.5;
    private const int ShreddedProseMinRowTextLen = 30;

    /// <summary>
    /// Whether one data row reads as a clause of a word-shredded, semicolon-delimited prose list:
    /// most columns filled (a real wrapped-table row leaves many empty), few words per cell, a
    /// substantial run of text, and clause-terminal punctuation at the end.
    /// </summary>
    private static bool LooksLikeShreddedProseRow(List<string> row, int numCols)
    {
        var cells = row.Select(c => c.Trim()).Where(c => c.Length > 0).ToList();
        if (cells.Count < ShreddedProseMinFilledCells) return false;
        if (numCols == 0 || cells.Count <= numCols * 0.5) return false;
        if (cells.Sum(Utf8Len) < ShreddedProseMinRowTextLen) return false;

        double avgWords = (double)cells.Sum(c => SplitWhitespace(c).Count()) / cells.Count;
        if (avgWords > ShreddedProseMaxAvgWordsPerCell) return false;

        string last = cells[^1];
        return last.Length > 0 && last[^1] is ';' or ':' or '.' or ',';
    }

    private const double ShortProseWordsPerCell = 4.0;
    private const double ShortProseAlphaRatio = 0.8;
    private const int ShortProseMinConcatLen = 15;
    private const int ShortProseNumericExemptPercent = 30;
    private const int MinShreddedWordCells = 4;
    private const double ShreddedMaxAvgWords = 2.5;
    private const double ShreddedMinWordCellFraction = 0.6;

    /// <summary>
    /// Whether a short grid is a wrapped-prose passage split across columns rather than a table.
    /// Closes the hole the row-count-gated guards leave: with one or two data rows there is no
    /// cross-row evidence, so a reflowed paragraph reaches acceptance untouched.
    /// </summary>
    /// <remarks>
    /// Two prose shapes, each judged per row: cells averaging four or more words in an alphabetic
    /// row (columns of phrases), and a wide row of thin mostly-word cells (a line chopped into
    /// one-word columns). A grid that is at least 30% numeric-value cells is exempt outright —
    /// a real short table is mostly values, prose with a stray number is not.
    /// </remarks>
    private static bool LooksLikeShortColumnedProse(List<List<string>> dataRows, int numCols)
    {
        if (numCols < 2) return false;

        int filledCells = 0, numericValueCells = 0;
        foreach (var row in dataRows)
            foreach (var cell in row)
            {
                string trimmed = cell.Trim();
                if (trimmed.Length == 0) continue;
                filledCells++;
                if (IsNumericValueCell(trimmed)) numericValueCells++;
            }
        if (filledCells == 0) return false;
        if (numericValueCells * 100 >= filledCells * ShortProseNumericExemptPercent) return false;

        int eligibleRows = 0, proseRows = 0;
        foreach (var row in dataRows)
        {
            var cells = row.Select(c => c.Trim()).Where(c => c.Length > 0).ToList();
            if (cells.Count < 2) continue;
            string concatenated = string.Join(" ", cells);
            if (Utf8Len(concatenated) < ShortProseMinConcatLen) continue;
            eligibleRows++;

            double avgWords = (double)cells.Sum(c => SplitWhitespace(c).Count()) / cells.Count;
            bool isPhraseProse = avgWords >= ShortProseWordsPerCell && AlphaRatio(concatenated) > ShortProseAlphaRatio;

            int wordCells = cells.Count(IsWordCell);
            bool isShreddedProse = cells.Count >= MinShreddedWordCells
                && avgWords <= ShreddedMaxAvgWords
                && wordCells >= cells.Count * ShreddedMinWordCellFraction;

            if (isPhraseProse || isShreddedProse) proseRows++;
        }

        return eligibleRows >= 1 && proseRows * 2 > eligibleRows;
    }

    // ── looks_like_code_listing (pdf/table_reconstruct.rs) ───────────────────

    internal static bool LooksLikeCodeListing(List<List<string>> tableCells)
    {
        var nonEmpty = tableCells.SelectMany(r => r).Select(s => s.Trim()).Where(s => s.Length > 0).ToList();
        if (nonEmpty.Count == 0) return false;
        if (nonEmpty.Any(cell => cell == "{" || cell == "}")) return true;
        int braceCount = nonEmpty.Count(cell => cell.Contains('{') || cell.Contains('}'));
        return (double)braceCount / nonEmpty.Count >= 0.20 || LooksLikeDeclarationGrid(tableCells);
    }

    private readonly record struct ParameterRowEvidence(bool EndsWithComma, bool ClosesDeclaration, bool HasPointer);

    /// <summary>
    /// A wrapped C-style function signature: one call head on the first row, then one
    /// parameter per row. Brace-free on its own, so the brace fraction above cannot see it,
    /// yet the shape is exactly what column detection turns into a two-column grid. The
    /// terminal <c>);</c> or trailing comma on every parameter row is what keeps API-reference
    /// tables with incidental code punctuation out of this.
    /// </summary>
    private static bool LooksLikeDeclarationGrid(List<List<string>> tableCells)
    {
        if (tableCells.Count == 0) return false;
        var firstCells = tableCells[0].Select(cell => cell.Trim()).Where(cell => cell.Length > 0).ToList();
        if (firstCells.Count != 1 || !LooksLikeDeclarationHead(firstCells[0])) return false;

        var continuationRows = tableCells.Skip(1)
            .Where(row => row.Any(cell => cell.Trim().Length > 0)).ToList();
        var evidence = continuationRows.Select(ParameterRowEvidenceOf).Where(e => e is not null)
            .Select(e => e!.Value).ToList();
        if (evidence.Count < 2 || evidence.Count != continuationRows.Count) return false;

        bool hasPointer = evidence.Any(e => e.HasPointer);
        bool hasClosingDeclaration = evidence.Any(e => e.ClosesDeclaration);
        bool allTruncatedParameters = evidence.All(e => e.EndsWithComma);
        return hasPointer && (hasClosingDeclaration || allTruncatedParameters);
    }

    private static ParameterRowEvidence? ParameterRowEvidenceOf(List<string> row)
    {
        var cells = row.Select(cell => cell.Trim()).Where(cell => cell.Length > 0).ToList();
        if (cells.Count < 2) return null;
        string last = cells[^1];

        string parameterName;
        bool endsWithComma = false, closesDeclaration = false;
        if (last.EndsWith(",", StringComparison.Ordinal))
        {
            parameterName = last[..^1];
            endsWithComma = true;
        }
        else if (last.EndsWith(");", StringComparison.Ordinal))
        {
            parameterName = last[..^2];
            closesDeclaration = true;
        }
        else return null;

        if (!LooksLikeParameterName(parameterName)) return null;
        return new ParameterRowEvidence(endsWithComma, closesDeclaration, cells.Any(cell => cell.Contains('*')));
    }

    private static bool LooksLikeParameterName(string name)
    {
        name = name.Trim().TrimStart('*');
        return name.Length > 0
            && name.Any(char.IsLetter)
            && name.All(c => char.IsLetterOrDigit(c) || c is '_' or '[' or ']');
    }

    private static bool LooksLikeDeclarationHead(string head)
    {
        if (!head.EndsWith("(", StringComparison.Ordinal)) return false;
        return head[..^1].Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries)
            .Count(token => token.Any(char.IsLetter)) >= 2;
    }
}
