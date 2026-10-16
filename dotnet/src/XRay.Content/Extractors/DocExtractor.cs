using System.Text;
using System.Text.Json;
using XRay.Content.Core;
using XRay.Content.Internal.Cfb;
using XRay.Content.Internal.Doc;
using XRay.Content.Types;

namespace XRay.Content.Extractors;

/// <summary>
/// Native Word 97-2003 binary (.doc) extractor. Ports <c>extractors/doc.rs</c> +
/// <c>extraction/doc/mod.rs</c>: opens the OLE/CFB container, reads the FIB from the
/// <c>WordDocument</c> stream, walks the piece table (CLX/PlcPcd) in the table stream, decodes
/// CP1252 / UTF-16LE text, and applies the heuristic short-line heading detection.
/// </summary>
public sealed class DocExtractor : IExtractor
{
    public IEnumerable<string> SupportedMimeTypes => new[] { "application/msword" };

    // Higher than the default so it wins over any generic handler for application/msword.
    public int Priority => 60;

    /// <summary>
    /// Index of the <c>fcClx</c>/<c>lcbClx</c> pair in the FIB's <c>FibRgFcLcb97</c> array.
    /// </summary>
    /// <remarks>
    /// [MS-DOC] 2.5.5 orders the array <c>fcStshfOrig</c>(0) … <c>fcSttbfAssoc</c>(32),
    /// <c>fcClx</c>(33). This read used index 66 (<c>fcBkdFtnOldOld</c>, an obsolete field Word
    /// writes as zero), so <c>fcClx == 0</c> held for every real document and the piece table was
    /// never walked — the whole <c>Clx</c> path was unreachable and only the contiguous fallback
    /// ever ran, reading <c>reserved5</c>/<c>reserved6</c> at <c>0x18</c>/<c>0x1C</c>, bytes
    /// [MS-DOC] says a reader must ignore (xberg-io/xberg#1551).
    /// </remarks>
    private const int FibFcLcbIdxClx = 33;

    // Indices into the FIB's `FibRgLw97` long-word array ([MS-DOC] 2.5.4).
    private const int FibLwIdxCcpText = 3;
    private const int FibLwIdxCcpFtn = 4;
    private const int FibLwIdxCcpHdd = 5;
    private const int FibLwIdxCcpAtn = 7;
    private const int FibLwIdxCcpEdn = 8;
    private const int FibLwIdxCcpTxbx = 9;
    private const int FibLwIdxCcpHdrTxbx = 10;

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        var comp = CompoundFile.Open(content);

        var meta = new OleUtil.OleMetadata();
        if (comp.TryReadStream("/\x05SummaryInformation") is { } si) OleUtil.ParseSummaryInfo(si, meta);
        if (comp.TryReadStream("/\x05DocumentSummaryInformation") is { } dsi) OleUtil.ParseSummaryInfo(dsi, meta);

        byte[] wordDoc = comp.TryReadStream("/WordDocument")
            ?? throw new InvalidDataException("Failed to open stream 'WordDocument'");
        if (wordDoc.Length < 12) throw new InvalidDataException("WordDocument stream too short");

        int wIdent = OleUtil.U16(wordDoc, 0);
        if (wIdent != 0xA5EC)
            throw new InvalidDataException($"Invalid DOC magic number: 0x{wIdent:X4}, expected 0xA5EC");

        int nFib = OleUtil.U16(wordDoc, 2);
        int flagsA = OleUtil.U16(wordDoc, 0x0A);
        bool use1Table = (flagsA & 0x0200) != 0;
        byte[] tableStream = comp.TryReadStream(use1Table ? "/1Table" : "/0Table") ?? Array.Empty<byte>();

        var extracted = nFib >= 101
            ? ExtractTextWord97(wordDoc, tableStream)
            : new DocText(ExtractTextWord6(wordDoc), new List<DocParagraph>());
        string text = extracted.Content;

        var doc = new InternalDocument("doc") { MimeType = mimeType };

        var additional = new Dictionary<string, JsonElement>();
        if (meta.RevisionNumber is { } rev) additional["revision"] = JsonStr(rev);
        additional["extraction_method"] = JsonStr("native_ole");

        List<string>? authors = meta.Author is { } a ? new List<string> { a } : null;
        doc.Metadata = new Metadata
        {
            Title = meta.Title,
            Subject = meta.Subject,
            Authors = authors,
            CreatedBy = meta.Author,
            ModifiedBy = meta.LastAuthor,
            Additional = additional,
        };

        // Elements follow Word's own paragraph structure, matching what the DOCX path does with
        // `w:p`. The blank-line fallback is only for documents carrying no paragraph properties
        // at all — Word 6/95, or the contiguous fallback — where there is nothing finer to use.
        if (extracted.Paragraphs.Count == 0)
        {
            PushBlankLineChunks(doc, text);
        }
        else
        {
            PushParagraphElements(doc, extracted.Paragraphs);
            PushSubdocumentSections(doc, extracted.Sections);
        }

        return doc;
    }

    /// <summary>
    /// Whether a chunk looks like a heading, by shape rather than by style.
    /// </summary>
    /// <remarks>
    /// Used only for documents whose style sheet declares no heading style at all. Deriving
    /// headings purely from <c>istd</c> would delete all 13 headings from one reporter document
    /// and all 12 from another, because both style their headings as bold <c>Normal</c>. Half the
    /// corpus does. Shape is the only signal those documents carry (xberg-io/xberg#1553).
    /// </remarks>
    private static bool LooksLikeHeading(string text, string? next) =>
        !text.Contains('\n')
        && text.Length <= 80
        && !text.EndsWith('.') && !text.EndsWith(':') && !text.EndsWith(';')
        && next is { Length: > 0 } && next.Length > text.Length;

    /// <summary>
    /// Emit the labelled footnote, header, comment and text-box sections after the body.
    /// </summary>
    /// <remarks>
    /// A deliberate deviation from upstream, which this port does not follow. Upstream assembles
    /// these sections into its <c>content</c> string but builds its element stream from the main
    /// paragraphs alone, and never marks the content pre-rendered — so once paragraph properties
    /// exist, every footnote, header, comment and text box silently disappears from the output
    /// (xberg-io/xberg#1550 against #77). Reproducing that would undo the subdocument extraction
    /// this port gained in the same pass, to no one's benefit.
    /// </remarks>
    private static void PushSubdocumentSections(InternalDocument doc, List<(string Label, string Text)> sections)
    {
        foreach (var (label, text) in sections)
        {
            doc.PushElement(InternalElement.TextElement(ElementKind.Heading(2), label, 0));
            foreach (var chunk in text.Split("\n\n"))
            {
                string trimmed = chunk.Trim();
                if (trimmed.Length > 0)
                    doc.PushElement(InternalElement.TextElement(ElementKind.Paragraph, trimmed, 0));
            }
        }
    }

    /// <summary>Emit one element per blank-line-separated chunk.</summary>
    /// <remarks>
    /// Only reachable for documents that carry no paragraph properties, where Word's own
    /// paragraph boundaries are not available. It merges any two paragraphs not separated by a
    /// blank line, which is why it is no longer the main path.
    /// </remarks>
    private static void PushBlankLineChunks(InternalDocument doc, string content)
    {
        var chunks = content.Split("\n\n");
        for (int i = 0; i < chunks.Length; i++)
        {
            string trimmed = chunks[i].Trim();
            if (trimmed.Length == 0) continue;
            string? next = i + 1 < chunks.Length ? chunks[i + 1].Trim() : null;
            var kind = LooksLikeHeading(trimmed, next) ? ElementKind.Heading(2) : ElementKind.Paragraph;
            doc.PushElement(InternalElement.TextElement(kind, trimmed, 0));
        }
    }

    /// <summary>
    /// Emit one element per Word paragraph, so a list-bound paragraph becomes a list item inside
    /// a list container — the shape the DOCX path already produces for <c>w:numPr</c>.
    /// </summary>
    /// <remarks>
    /// Consecutive bound paragraphs share a container. A change of nesting depth opens or closes
    /// nested containers, and a change of <em>kind</em> at the same depth closes and reopens: a
    /// document can move from a numbered run straight into a bulleted one at the same level, and
    /// merging those would label half the items wrongly.
    /// </remarks>
    private static void PushParagraphElements(InternalDocument doc, List<DocParagraph> paragraphs)
    {
        // The switch is "does this document EMIT styled headings", which is narrower than either
        // alternative that looks right.
        //
        // Not "does the style sheet define headings": nearly every Word style sheet defines
        // heading 1..9 whether or not the author applied one, so that answers yes almost always.
        //
        // And not merely "does any paragraph carry a heading style": a list-bound paragraph is
        // emitted as a list item regardless of its style, matching the DOCX path's handling of
        // `w:numPr`. `simple.doc` is the case — its one Heading 1 paragraph is also list-bound,
        // so counting it flipped the document into styled mode and suppressed every heading
        // while emitting none, leaving it with no heading structure at all.
        bool styledHeadings = paragraphs.Any(p => p.HeadingLevel is not null && p.List is null);

        // One entry per open container, holding whether it is ordered.
        var open = new List<bool>();

        for (int i = 0; i < paragraphs.Count; i++)
        {
            string text = paragraphs[i].Content.Trim();
            if (text.Length == 0) continue;

            if (paragraphs[i].List is not { } list)
            {
                CloseLists(doc, open, 0);
                var kind = HeadingKind(paragraphs, i, text, styledHeadings) is { } level
                    ? ElementKind.Heading(level)
                    : ElementKind.Paragraph;
                doc.PushElement(InternalElement.TextElement(kind, text, 0));
                continue;
            }

            int depth = list.Level + 1;
            CloseLists(doc, open, depth);
            if (open.Count == depth && open[^1] != list.Ordered) CloseLists(doc, open, depth - 1);
            while (open.Count < depth)
            {
                doc.PushElement(InternalElement.TextElement(
                    ElementKind.ListStart(list.Ordered), "", (ushort)Math.Min(open.Count, ushort.MaxValue)));
                open.Add(list.Ordered);
            }

            doc.PushElement(InternalElement.TextElement(
                ElementKind.ListItem(list.Ordered), text, (ushort)Math.Min(open.Count, ushort.MaxValue)));
        }

        CloseLists(doc, open, 0);
    }

    /// <summary>Decide whether a paragraph is a heading, and at what level.</summary>
    /// <remarks>
    /// The two signals are not interchangeable and neither is usable alone. A document that
    /// declares heading styles is taken at its word — <c>istd</c> is what Word itself renders
    /// from, and the shape heuristic invents headings there (1 detected against 7 declared in one
    /// corpus document). A document that declares none has nothing to be taken at its word about,
    /// and falls back to shape. The switch is per <em>document</em>, not per paragraph:
    /// per-paragraph fallback would re-add the invented headings alongside the declared ones,
    /// which is the worst of both (xberg-io/xberg#1553).
    /// </remarks>
    private static byte? HeadingKind(List<DocParagraph> paragraphs, int index, string text, bool styledHeadings)
    {
        if (styledHeadings) return paragraphs[index].HeadingLevel;
        string? next = index + 1 < paragraphs.Count ? paragraphs[index + 1].Content.Trim() : null;
        // The shape heuristic has no notion of depth; it only ever claimed h2.
        return LooksLikeHeading(text, next is { Length: > 0 } ? next : null) ? (byte)2 : null;
    }

    /// <summary>Close open list containers until only <paramref name="target"/> remain.</summary>
    private static void CloseLists(InternalDocument doc, List<bool> open, int target)
    {
        while (open.Count > target)
        {
            open.RemoveAt(open.Count - 1);
            doc.PushElement(InternalElement.TextElement(
                ElementKind.ListEnd, "", (ushort)Math.Min(open.Count, ushort.MaxValue)));
        }
    }

    // ── FIB / piece-table parsing (extraction/doc/mod.rs) ────────────────────────
    private static DocText ExtractTextWord97(byte[] wordDoc, byte[] table)
    {
        const int fibBaseSize = 32;
        int cswOffset = fibBaseSize;
        if (wordDoc.Length < cswOffset + 2) throw new InvalidDataException("FIB too short for csw");
        int csw = OleUtil.U16(wordDoc, cswOffset);
        int rgWOffset = cswOffset + 2;
        int cslwOffset = rgWOffset + csw * 2;
        if (wordDoc.Length < cslwOffset + 2) throw new InvalidDataException("FIB too short for cslw");
        int cslw = OleUtil.U16(wordDoc, cslwOffset);
        int rgLwOffset = cslwOffset + 2;

        int ccpTextOffset = rgLwOffset + FibLwIdxCcpText * 4;
        if (wordDoc.Length < ccpTextOffset + 4) throw new InvalidDataException("FIB too short for ccpText");
        int ccpText = (int)OleUtil.U32(wordDoc, ccpTextOffset);

        var ranges = SubdocRanges.FromFib(wordDoc, rgLwOffset, ccpText);

        int cbrgOffset = rgLwOffset + cslw * 4;
        if (wordDoc.Length < cbrgOffset + 2) throw new InvalidDataException("FIB too short for cbRgFcLcb");
        int rgFcLcbOffset = cbrgOffset + 2;

        int fcClxOffset = rgFcLcbOffset + FibFcLcbIdxClx * 8;
        int lcbClxOffset = fcClxOffset + 4;
        if (wordDoc.Length < lcbClxOffset + 4) throw new InvalidDataException("FIB too short for fcClx/lcbClx");

        int fcClx = (int)OleUtil.U32(wordDoc, fcClxOffset);
        int lcbClx = (int)OleUtil.U32(wordDoc, lcbClxOffset);

        if (fcClx == 0 || lcbClx == 0)
            return new DocText(ExtractTextContiguous(wordDoc, ccpText), new List<DocParagraph>());
        if (table.Length < fcClx + lcbClx)
            throw new InvalidDataException("CLX extends beyond table stream");

        // Parse CLX: skip Prc entries (0x01), find Pcdt (0x02).
        int pos = fcClx;
        int clxEnd = fcClx + lcbClx;
        while (pos < clxEnd)
        {
            Cancel.ThrowIfRequested();
            byte clxt = table[pos];
            if (clxt == 0x02)
            {
                pos += 1;
                if (pos + 4 > clxEnd) throw new InvalidDataException("Pcdt truncated at lcb");
                pos += 4; // lcb of PlcPcd
                return ExtractFromPieceTable(wordDoc, table, pos, clxEnd, ranges,
                    DocPapx.ListTables.Build(wordDoc, table, rgFcLcbOffset));
            }
            if (clxt == 0x01)
            {
                pos += 1;
                if (pos + 2 > clxEnd) break;
                int cbGrpprl = OleUtil.U16(table, pos);
                pos += 2 + cbGrpprl;
            }
            else break;
        }
        return new DocText(ExtractTextFallback(wordDoc), new List<DocParagraph>());
    }

    /// <summary>
    /// Walk the piece table (<c>PlcPcd</c>), bucketing each piece's characters into the
    /// subdocument CP range they fall in, and assemble the labelled sections.
    /// </summary>
    /// <remarks>
    /// Ports upstream's <c>extract_text_from_piece_table</c>. Any piece whose CP range started
    /// at or after <c>ccpText</c> — that is, every footnote, header/footer, comment and text-box
    /// piece — used to be skipped outright, so none of that content ever appeared
    /// (xberg-io/xberg#77).
    /// </remarks>
    private static DocText ExtractFromPieceTable(
        byte[] wordDoc, byte[] table, int plcStart, int plcEnd, SubdocRanges ranges,
        DocPapx.ListTables listTables)
    {
        int plcSize = plcEnd - plcStart;
        if (plcSize < 16) throw new InvalidDataException("PlcPcd too small");
        int n = (plcSize - 4) / 12;
        if (n == 0) return new DocText("", new List<DocParagraph>());

        var main = new StringBuilder(ranges.Main.Length);
        // Only the main document needs per-character FCs: paragraph properties are bound to body
        // text, and a subdocument paragraph carries no list numbering a reader would see.
        var mainFcEnds = new List<uint>(ranges.Main.Length);
        var footnote = new StringBuilder();
        var header = new StringBuilder();
        var annotation = new StringBuilder();
        var textbox = new StringBuilder();

        for (int i = 0; i < n; i++)
        {
            int cpStartOff = plcStart + i * 4;
            int cpEndOff = plcStart + (i + 1) * 4;
            int pcdOff = plcStart + (n + 1) * 4 + i * 8;
            if (cpEndOff + 4 > plcEnd || pcdOff + 8 > plcEnd) break;

            int cpStart = (int)OleUtil.U32(table, cpStartOff);
            int cpEnd = (int)OleUtil.U32(table, cpEndOff);
            if (cpStart >= ranges.TotalCp) break;

            uint fcRaw = OleUtil.U32(table, pcdOff + 2);
            int charCount = Math.Max(0, cpEnd - cpStart);
            var piece = DecodePieceChars(wordDoc, fcRaw, charCount);
            if (piece.Text.Length == 0) continue;

            AppendRangeOverlap(piece, cpStart, ranges.Main, main, mainFcEnds);
            AppendRangeOverlap(piece, cpStart, ranges.Footnote, footnote, null);
            AppendRangeOverlap(piece, cpStart, ranges.Header, header, null);
            AppendRangeOverlap(piece, cpStart, ranges.Annotation, annotation, null);
            AppendRangeOverlap(piece, cpStart, ranges.Textbox, textbox, null);
        }

        var content = new StringBuilder(NormalizeDocText(main.ToString()));
        var sections = new List<(string Label, string Text)>();
        foreach (var (label, section) in new[]
        {
            ("Footnotes", footnote),
            ("Headers and Footers", header),
            ("Comments", annotation),
            ("Text Boxes", textbox),
        })
        {
            string normalized = NormalizeDocText(section.ToString());
            if (normalized.Length == 0) continue;
            sections.Add((label, normalized));
            if (content.Length > 0) content.Append("\n\n");
            content.Append(label).Append("\n\n").Append(normalized);
        }
        // `content` stays a single normalization pass over the whole main text, unchanged and
        // byte-identical: `NormalizeDocText` carries a field stack across the string, so deriving
        // it from per-paragraph normalization would make "no field spans a paragraph mark" an
        // unstated assumption.
        return new DocText(
            content.ToString(), SplitMainParagraphs(main.ToString(), mainFcEnds, listTables), sections);
    }

    /// <summary>Word's paragraph mark. Splitting the raw main text on it gives the document's own
    /// paragraph granularity, which is finer than the blank-line chunking.</summary>
    private const char ParagraphMark = '\r';

    /// <summary>
    /// Split the raw main text into paragraphs and attach each one's list binding and style.
    /// </summary>
    /// <remarks>
    /// Operates on the <em>raw</em> text so character positions still line up with the recorded
    /// FCs; each paragraph's text is normalized individually afterwards. Elements used to be built
    /// from blank-line chunks, but Word's paragraph mark is a single CR, so any two consecutive
    /// non-empty paragraphs without a blank line between them arrived as one element — one
    /// corpus document returned its entire ten-paragraph body as a single element
    /// (xberg-io/xberg#1550).
    /// </remarks>
    private static List<DocParagraph> SplitMainParagraphs(
        string main, List<uint> mainFcEnds, DocPapx.ListTables listTables)
    {
        var paragraphs = new List<DocParagraph>();
        int start = 0;

        for (int i = 0; i < main.Length; i++)
        {
            Cancel.Every(i, 0x3FFF);
            if (main[i] != ParagraphMark) continue;
            PushParagraph(paragraphs, main, start, i, FcEndAt(mainFcEnds, i), listTables);
            start = i + 1;
        }

        if (start < main.Length)
        {
            // A final run with no paragraph mark still has properties keyed on the FC one past
            // its last character.
            PushParagraph(paragraphs, main, start, main.Length, FcEndAt(mainFcEnds, main.Length - 1), listTables);
        }

        return paragraphs;
    }

    private static uint? FcEndAt(List<uint> fcEnds, int index) =>
        index >= 0 && index < fcEnds.Count ? fcEnds[index] : null;

    /// <summary>Normalize one paragraph's raw text and record it when it survives.</summary>
    private static void PushParagraph(
        List<DocParagraph> outParagraphs, string main, int start, int end, uint? markFcEnd,
        DocPapx.ListTables listTables)
    {
        string content = NormalizeDocText(main[start..end]);
        if (content.Length == 0) return;

        // Word keys a paragraph's PAPX on the FC one past its paragraph mark, which is exactly
        // what the piece walk recorded for that character.
        outParagraphs.Add(new DocParagraph
        {
            Content = content,
            List = markFcEnd is { } fc ? listTables.MembershipForParagraphEnd(fc) : null,
            HeadingLevel = markFcEnd is { } fc2 ? listTables.HeadingLevelForParagraphEnd(fc2) : null,
        });
    }

    /// <summary>
    /// One decoded piece: its characters, and the byte offset (<c>FC</c>) one past each of them.
    /// </summary>
    /// <remarks>
    /// The FCs are what bind text to paragraph properties: PAPX is FC-addressed while text is
    /// CP-addressed, so the piece table's mapping is the only thing that relates the two.
    /// </remarks>
    private readonly record struct DecodedPiece(string Text, uint[] FcEnds);

    /// <summary>Decode one piece's characters, choosing CP1252 or UTF-16LE from its FC.</summary>
    private static DecodedPiece DecodePieceChars(byte[] wordDoc, uint fcRaw, int charCount)
    {
        // Compressed (CP1252) pieces address the stream at half the raw FC value; uncompressed
        // (UTF-16LE) pieces address it directly.
        bool isCompressed = (fcRaw & 0x4000_0000) != 0;
        int fc = (int)(fcRaw & 0x3FFF_FFFF);
        int byteOffset = isCompressed ? fc / 2 : fc;

        DecodedPiece DecodeCp1252(int start, int end)
        {
            if (start >= end) return new DecodedPiece("", Array.Empty<uint>());
            var sb = new StringBuilder(end - start);
            var fcEnds = new uint[end - start];
            for (int k = start; k < end; k++)
            {
                sb.Append(OleUtil.Cp1252ToChar(wordDoc[k]));
                fcEnds[k - start] = (uint)(k + 1);
            }
            return new DecodedPiece(sb.ToString(), fcEnds);
        }

        if (isCompressed)
        {
            int end = byteOffset + charCount;
            return DecodeCp1252(byteOffset, Math.Min(end, wordDoc.Length));
        }

        int utf16End = byteOffset + charCount * 2;
        int availableEnd = utf16End <= wordDoc.Length
            ? utf16End
            : byteOffset + Math.Max(0, (wordDoc.Length - byteOffset) / 2) * 2;

        DecodedPiece piece;
        if (byteOffset >= availableEnd)
        {
            piece = new DecodedPiece("", Array.Empty<uint>());
        }
        else
        {
            var sb = new StringBuilder((availableEnd - byteOffset) / 2);
            var fcEnds = new List<uint>((availableEnd - byteOffset) / 2);
            for (int k = byteOffset; k + 1 < availableEnd; k += 2)
            {
                sb.Append((char)(ushort)(wordDoc[k] | (wordDoc[k + 1] << 8)));
                fcEnds.Add((uint)(k + 2));
            }
            piece = new DecodedPiece(sb.ToString(), fcEnds.ToArray());
        }

        // Heuristic: a mostly-CJK decode means the compression bit was wrong → redo as CP1252.
        int suspicious = piece.Text.Count(c => c >= 0x4E00 && c <= 0x9FFF);
        if (piece.Text.Length > 4 && suspicious > piece.Text.Length / 4)
            return DecodeCp1252(byteOffset, Math.Min(byteOffset + charCount, wordDoc.Length));

        return piece;
    }

    /// <summary>
    /// Append the part of <paramref name="piece"/> that falls inside a CP range, carrying its
    /// per-character FCs alongside when the caller wants them.
    /// </summary>
    private static void AppendRangeOverlap(
        DecodedPiece piece, int cpStart, SubdocRange range, StringBuilder outText, List<uint>? outFcEnds)
    {
        if (range.Length == 0) return;
        int pieceEnd = cpStart + piece.Text.Length;
        int overlapStart = Math.Max(cpStart, range.Start);
        int overlapEnd = Math.Min(pieceEnd, range.End);
        if (overlapStart >= overlapEnd) return;

        int from = overlapStart - cpStart;
        int count = overlapEnd - overlapStart;
        outText.Append(piece.Text, from, count);
        if (outFcEnds is null) return;
        for (int i = from; i < from + count && i < piece.FcEnds.Length; i++) outFcEnds.Add(piece.FcEnds[i]);
    }

    /// <summary>What a <c>.doc</c>'s text extraction yields: the rendered content, and Word's own
    /// paragraphs when the document carries the properties that name them.</summary>
    /// <param name="Content">The whole document as text, main body then labelled sections.</param>
    /// <param name="Paragraphs">Word's own paragraphs, for the main body only.</param>
    /// <param name="Sections">
    /// The labelled subdocument sections — footnotes, headers, comments, text boxes — kept apart
    /// from <paramref name="Paragraphs"/> because paragraph properties address body text only.
    /// </param>
    private readonly record struct DocText(
        string Content, List<DocParagraph> Paragraphs, List<(string Label, string Text)> Sections)
    {
        public DocText(string content, List<DocParagraph> paragraphs)
            : this(content, paragraphs, new List<(string, string)>()) { }
    }

    /// <summary>A half-open CP-space range belonging to one subdocument.</summary>
    private readonly record struct SubdocRange(int Start, int End)
    {
        public int Length => Math.Max(0, End - Start);
    }

    /// <summary>
    /// The CP-space subdocument layout, derived from the FIB's <c>ccp*</c> fields.
    /// </summary>
    /// <remarks>
    /// Order matches the FIB's <c>FibRgLw97</c> field declaration order, which is also the
    /// document's physical CP-space layout ([MS-DOC] 2.4.2): main document, footnotes,
    /// headers/footers, comments, endnotes, text boxes, header text boxes. <c>ccpMcr</c> (the
    /// deprecated macro subdocument) is skipped: it does not occupy CP space. Endnotes and
    /// header text boxes are not extracted, but their spans must still be counted so later
    /// subdocuments resolve to the right offsets.
    /// </remarks>
    private readonly record struct SubdocRanges(
        SubdocRange Main, SubdocRange Footnote, SubdocRange Header,
        SubdocRange Annotation, SubdocRange Endnote, SubdocRange Textbox,
        SubdocRange HeaderTextbox)
    {
        public int TotalCp => HeaderTextbox.End;

        public static SubdocRanges FromFib(byte[] wordDoc, int rgLwOffset, int ccpText)
        {
            int Read(int index)
            {
                int off = rgLwOffset + index * 4;
                return wordDoc.Length >= off + 4 ? (int)OleUtil.U32(wordDoc, off) : 0;
            }

            var main = new SubdocRange(0, ccpText);
            var footnote = new SubdocRange(main.End, main.End + Read(FibLwIdxCcpFtn));
            var header = new SubdocRange(footnote.End, footnote.End + Read(FibLwIdxCcpHdd));
            var annotation = new SubdocRange(header.End, header.End + Read(FibLwIdxCcpAtn));
            var endnote = new SubdocRange(annotation.End, annotation.End + Read(FibLwIdxCcpEdn));
            var textbox = new SubdocRange(endnote.End, endnote.End + Read(FibLwIdxCcpTxbx));
            var headerTextbox = new SubdocRange(textbox.End, textbox.End + Read(FibLwIdxCcpHdrTxbx));
            return new SubdocRanges(main, footnote, header, annotation, endnote, textbox, headerTextbox);
        }
    }

    private static string ExtractTextContiguous(byte[] wordDoc, int ccpText)
    {
        if (wordDoc.Length < 0x20) return ExtractTextFallback(wordDoc);
        int fcMin = (int)OleUtil.U32(wordDoc, 0x18);
        int fcMac = (int)OleUtil.U32(wordDoc, 0x1C);
        if (fcMin == 0 || fcMin >= wordDoc.Length) return ExtractTextFallback(wordDoc);
        int dataLen = Math.Min(Math.Max(0, fcMac - fcMin), wordDoc.Length - fcMin);
        if (dataLen == 0) return ExtractTextFallback(wordDoc);

        int nullCount = 0;
        for (int i = fcMin; i < fcMin + dataLen; i++) if (wordDoc[i] == 0) nullCount++;
        bool isUnicode = dataLen >= ccpText * 2 || nullCount > dataLen / 4;

        var sb = new StringBuilder();
        if (isUnicode)
        {
            int taken = 0;
            for (int k = fcMin; k + 1 < fcMin + dataLen && taken < ccpText; k += 2, taken++)
            {
                ushort cu = (ushort)(wordDoc[k] | (wordDoc[k + 1] << 8));
                sb.Append((char)cu);
            }
        }
        else
        {
            int taken = 0;
            for (int k = fcMin; k < fcMin + dataLen && taken < ccpText; k++, taken++)
                sb.Append(OleUtil.Cp1252ToChar(wordDoc[k]));
        }
        string normalized = NormalizeDocText(sb.ToString());
        return normalized.Length == 0 ? ExtractTextFallback(wordDoc) : normalized;
    }

    private static string ExtractTextFallback(byte[] wordDoc)
    {
        var result = new StringBuilder();
        var run = new StringBuilder();
        for (int i = 256; i < wordDoc.Length; i++)
        {
            Cancel.Every(i, 0x3FFF);
            byte b = wordDoc[i];
            if (b == 0x0D || b == 0x0A || b == 0x09 || (b >= 0x20 && b <= 0xFE))
                run.Append(OleUtil.Cp1252ToChar(b));
            else if (run.Length > 0)
            {
                if (run.Length >= 3) { if (result.Length > 0) result.Append(' '); result.Append(run); }
                run.Clear();
            }
        }
        if (run.Length >= 3) { if (result.Length > 0) result.Append(' '); result.Append(run); }
        if (result.Length == 0) throw new InvalidDataException("No text content found in DOC file");
        return NormalizeDocText(result.ToString());
    }

    private static string ExtractTextWord6(byte[] wordDoc)
    {
        if (wordDoc.Length < 0x50) throw new InvalidDataException("Word 6/95 file too short");
        int ccpText = (int)OleUtil.U32(wordDoc, 0x4C);
        int fcMin = (int)OleUtil.U32(wordDoc, 0x18);
        if (fcMin + ccpText > wordDoc.Length) return ExtractTextFallback(wordDoc);
        var sb = new StringBuilder(ccpText);
        for (int k = fcMin; k < fcMin + ccpText; k++) sb.Append(OleUtil.Cp1252ToChar(wordDoc[k]));
        return NormalizeDocText(sb.ToString());
    }

    /// <summary>Word field BEGIN marker. Text from here to <see cref="FieldSeparator"/> is the
    /// field <em>instruction</em> (<c>HYPERLINK "…"</c>, <c>PAGEREF _Toc1 \h</c>,
    /// <c>TOC \o "1-3"</c>), i.e. markup.</summary>
    private const char FieldBegin = '\x13';

    /// <summary>Word field SEPARATOR marker. Text from here to <see cref="FieldEnd"/> is the field
    /// <em>result</em> — the only part a reader sees, and the only part that is document
    /// text.</summary>
    private const char FieldSeparator = '\x14';

    /// <summary>Word field END marker.</summary>
    private const char FieldEnd = '\x15';

    /// <summary>
    /// Word non-breaking hyphen (<c>0x1E</c> in the binary text stream), emitted as U+2011.
    /// </summary>
    /// <remarks>
    /// This is a <em>visible</em> character — the reader sees a hyphen; the only thing
    /// "non-breaking" suppresses is a line break at that position. Dropping it welds the two
    /// halves of a compound together (<c>twenty-one</c> → <c>twentyone</c>), corrupting the word
    /// rather than merely losing formatting. U+2011 rather than ASCII <c>-</c> so that the same
    /// document saved as <c>.doc</c> and as <c>.docx</c> extracts to the same text: the DOCX
    /// parser maps <c>w:noBreakHyphen</c> — the same character in the modern serialization of the
    /// same Word document model — to U+2011.
    /// </remarks>
    private const char NonBreakingHyphen = '\u2011';

    /// <summary>
    /// For each <see cref="FieldBegin"/> in <paramref name="text"/>, in order of occurrence,
    /// whether it has a matching <see cref="FieldEnd"/>.
    /// </summary>
    /// <remarks>
    /// Fields nest — a <c>TOC</c> result is full of <c>PAGEREF</c> fields — so matching is
    /// innermost-first via a stack. Begins left on the stack at end of input are unterminated and
    /// reported as <c>false</c>; the caller deliberately does <em>not</em> treat one as opening a
    /// suppression region, because doing so would swallow the entire remainder of a document whose
    /// stream happens to carry one stray <c>0x13</c>. Degrading to the historical behaviour (the
    /// instruction leaks) is far cheaper than losing the document tail.
    /// </remarks>
    private static bool[] ScanFieldBeginTermination(string text)
    {
        var terminated = new List<bool>();
        var open = new Stack<int>();

        foreach (char c in text)
        {
            if (c == FieldBegin)
            {
                terminated.Add(false);
                open.Push(terminated.Count - 1);
            }
            else if (c == FieldEnd && open.Count > 0)
            {
                // A stray END with nothing open is ignored rather than throwing: this is
                // user-supplied binary content.
                terminated[open.Pop()] = true;
            }
        }

        return terminated.ToArray();
    }

    /// <summary>
    /// Normalize extracted DOC text: strip field instructions, convert special characters and
    /// clean up whitespace.
    /// </summary>
    /// <remarks>
    /// Field handling (upstream GH#1460): the text between <see cref="FieldBegin"/> and
    /// <see cref="FieldSeparator"/> is the field instruction and is markup, not document text, so
    /// it is dropped; the result between <see cref="FieldSeparator"/> and <see cref="FieldEnd"/>
    /// is kept. A terminated field with no separator has no result and contributes nothing.
    /// </remarks>
    internal static string NormalizeDocText(string text)
    {
        var result = new StringBuilder(text.Length);

        bool[] beginTerminated = ScanFieldBeginTermination(text);
        int beginOrdinal = 0;
        // One entry per open field; true while that field is still in its instruction part.
        var fieldStack = new List<bool>();
        // Count of fieldStack entries still in their instruction part. Text is suppressed whenever
        // this is non-zero, which is what makes nesting work: a PAGEREF inside a TOC instruction
        // stays suppressed even after the inner field reaches its own separator.
        int instructionDepth = 0;

        foreach (char c in text)
        {
            if (c == FieldBegin)
            {
                bool terminated = beginOrdinal < beginTerminated.Length && beginTerminated[beginOrdinal];
                beginOrdinal++;
                if (terminated)
                {
                    fieldStack.Add(true);
                    instructionDepth++;
                }
                continue;
            }

            if (c == FieldSeparator)
            {
                if (fieldStack.Count > 0 && fieldStack[^1])
                {
                    fieldStack[^1] = false;
                    instructionDepth--;
                }
                continue;
            }

            if (c == FieldEnd)
            {
                if (fieldStack.Count > 0)
                {
                    bool inInstruction = fieldStack[^1];
                    fieldStack.RemoveAt(fieldStack.Count - 1);
                    if (inInstruction) instructionDepth--;
                }
                continue;
            }

            if (instructionDepth > 0) continue;

            switch (c)
            {
                case '\r': result.Append('\n'); break;
                case '\x07': result.Append('\t'); break;
                case '\x0B': result.Append('\n'); break;
                case '\x0C': result.Append('\n'); break;
                case '\x01' or '\x08': break;
                case '\x1E': result.Append(NonBreakingHyphen); break;
                // 0x1F is the *optional* (soft) hyphen: invisible unless the line happens to break
                // there, so discarding it is correct and must stay that way — emitting it would
                // insert a hyphen the reader never saw.
                default:
                    if (c < '\x20' && c != '\n' && c != '\t') break;
                    result.Append(c); break;
            }
        }

        var cleaned = new StringBuilder(result.Length);
        bool prevNl = false, prevPrevNl = false;
        foreach (char c in result.ToString())
        {
            if (c == '\n')
            {
                if (prevPrevNl && prevNl) continue;
                prevPrevNl = prevNl; prevNl = true;
            }
            else { prevPrevNl = false; prevNl = false; }
            cleaned.Append(c);
        }
        return cleaned.ToString().Trim();
    }

    private static JsonElement JsonStr(string s) =>
        JsonDocument.Parse(JsonSerializer.Serialize(s)).RootElement.Clone();
}
