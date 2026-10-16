using System.Text;
using System.Xml.Linq;

namespace XRay.Content.Internal.Ooxml;

// ── DOCX model ───────────────────────────────────────────────────────────────

public sealed class DocxRun
{
    public string Text = "";
    public bool Bold, Italic, Underline, Strike, Subscript, Superscript;
    public uint? FontSize;
    public string? FontColor;
    public bool Highlight;
    public string? HyperlinkUrl;
    public string? MathLatex;
    public bool MathDisplay;
}

public sealed class DocxParagraph
{
    public List<DocxRun> Runs = new();
    public string? StyleId;
    public long? NumId;
    public long? NumLevel;
}

public sealed class DocxCell
{
    public List<DocxParagraph> Paragraphs = new();
    public uint GridSpan = 1;
    public bool VMergeContinue;
}

public sealed class DocxRow
{
    public List<DocxCell> Cells = new();
    public bool IsHeader;
}

public sealed class DocxTable
{
    public List<DocxRow> Rows = new();
    public string? Caption;
}

public sealed class DocxDrawing
{
    public string? ImageRid;
    public string? Description;

    /// <summary>
    /// Visible text of a <c>w:txbxContent</c> subtree reached through this drawing — the
    /// DrawingML <c>wps:txbx</c> path or the VML <c>v:textbox</c> path. Paragraphs are joined
    /// with newlines and surfaced as a single paragraph, so a text box never contributes
    /// headings or list structure of its own.
    /// </summary>
    public string? TextBoxContent;
}

public enum DocElementKind { Paragraph, Table, Drawing, PageBreak }

public sealed class DocElement
{
    public DocElementKind Kind;
    public DocxParagraph? Paragraph;
    public DocxTable? Table;
    public DocxDrawing? Drawing;
}

public sealed class DocxNote
{
    public string Id = "";
    public List<DocxParagraph> Paragraphs = new();
}

public sealed class DocxDocument
{
    public List<DocElement> Elements = new();
    public List<DocxDrawing> Drawings = new(); // all drawings in document order (index = image_index)
    public List<List<DocxParagraph>> Headers = new();
    public List<List<DocxParagraph>> Footers = new();
    public List<DocxNote> Footnotes = new();
    public List<DocxNote> Endnotes = new();
    /// <summary>Reviewer comments from <c>word/comments.xml</c>, keyed by their own id.</summary>
    public List<DocxNote> Comments = new();
    public Dictionary<(long NumId, long Level), bool> NumberingOrdered = new(); // true = numbered
    public Dictionary<string, string> ImageRelationships = new(); // rId -> target (word/-relative)
}

/// <summary>
/// Pure-managed DOCX (WordprocessingML) parser — ports <c>extraction/docx/parser.rs</c> +
/// <c>styles.rs</c> + <c>table.rs</c> for the pieces that drive content/table/heading output.
/// Math (OMML→LaTeX) is not converted (math runs contribute no text).
/// </summary>
public static class DocxReader
{
    private const string W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    private const string R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

    private static readonly HashSet<string> ValidHighlights = new()
    {
        "yellow","green","cyan","magenta","blue","red","darkBlue","darkCyan","darkGreen",
        "darkMagenta","darkRed","darkYellow","darkGray","lightGray","black","none",
    };

    public static DocxDocument Parse(OoxmlPackage pkg)
    {
        var doc = new DocxDocument();
        var rels = ReadRelationships(pkg);
        var styles = ReadStyles(pkg);
        doc.NumberingOrdered = ReadNumbering(pkg);

        doc.ImageRelationships = rels;
        var mainXml = pkg.ReadXml("word/document.xml");
        var body = mainXml?.Root?.Elements().FirstOrDefault(e => e.Name.LocalName == "body");
        if (body is not null)
        {
            var pageBreaks = new PageBreakState();
            int bodyChildren = 0;
            foreach (var child in body.Elements())
            {
                Cancel.Every(bodyChildren++, 0x1F);
                ParseBodyChild(child, doc, rels, pageBreaks);
            }
        }

        _styles = styles; // used by ResolveHeadingLevel

        // Headers / footers
        foreach (var name in pkg.PartNames.Where(n => IsHeaderFooter(n, "header")).OrderBy(x => x, StringComparer.Ordinal))
            doc.Headers.Add(ReadHeaderFooterParagraphs(pkg, name, rels));
        foreach (var name in pkg.PartNames.Where(n => IsHeaderFooter(n, "footer")).OrderBy(x => x, StringComparer.Ordinal))
            doc.Footers.Add(ReadHeaderFooterParagraphs(pkg, name, rels));

        // Footnotes / endnotes
        doc.Footnotes = ReadNotes(pkg, "word/footnotes.xml", "footnote", rels);
        doc.Endnotes = ReadNotes(pkg, "word/endnotes.xml", "endnote", rels);
        // A comment has the same shape as a note — an id and a body — but lives in its own part
        // and is somebody's remark about the document rather than part of it.
        doc.Comments = ReadNotes(pkg, "word/comments.xml", "comment", rels);
        return doc;
    }

    [ThreadStatic] private static Dictionary<string, StyleDef>? _styles;

    // ── body walk ──────────────────────────────────────────────────────────────
    /// <summary>
    /// Page-break bookkeeping shared by <c>w:br</c> and <c>w:lastRenderedPageBreak</c>
    /// (Rust GH#1416, GH#1419).
    /// </summary>
    private sealed class PageBreakState
    {
        /// <summary>Whether rendered content has been emitted since the previous break.
        /// Starts <c>true</c> so a break with nothing before it — including the very first one in
        /// the document — is never treated as a spurious duplicate.</summary>
        public bool TextSinceBreak = true;

        /// <summary>Breaks seen inside a table, flushed once the table element is pushed.</summary>
        public uint PendingTableBreaks;
    }

    private static void ParseBodyChild(XElement el, DocxDocument doc, Dictionary<string, string> rels, PageBreakState pb)
    {
        switch (el.Name.LocalName)
        {
            case "p":
            {
                // Drawings + page breaks open (and are recorded) before the enclosing paragraph closes.
                uint deferred = EmitPreElements(el, doc, rels, pb, inTable: false);
                doc.Elements.Add(new DocElement { Kind = DocElementKind.Paragraph, Paragraph = ParseParagraph(el, rels) });
                // A break that fell after content already collected in this paragraph belongs
                // behind the paragraph's own element, not in front of it (Rust GH#1416).
                for (; deferred > 0; deferred--)
                    doc.Elements.Add(new DocElement { Kind = DocElementKind.PageBreak });
                break;
            }
            case "tbl":
                // Drawings inside cells open before the table closes → emitted before the Table.
                // A page break inside a table is deferred rather than dropped: a form feed cannot
                // be written into the middle of a table that renders as one markdown block, so it
                // is flushed once the table element has been pushed (GH#1419).
                EmitPreElements(el, doc, rels, pb, inTable: true);
                doc.Elements.Add(new DocElement { Kind = DocElementKind.Table, Table = ParseTable(el, rels) });
                for (; pb.PendingTableBreaks > 0; pb.PendingTableBreaks--)
                    doc.Elements.Add(new DocElement { Kind = DocElementKind.PageBreak });
                break;
        }
    }

    /// <summary>
    /// Walks a body child's subtree in document order, emitting the drawings and page breaks that
    /// have to be recorded before the enclosing element closes. Returns the number of page breaks
    /// that must instead be emitted straight <em>after</em> it — see <c>RecordBreak</c>.
    /// </summary>
    private static uint EmitPreElements(
        XElement container, DocxDocument doc, Dictionary<string, string> rels, PageBreakState pb, bool inTable)
    {
        // Mirrors Rust's `current_run.text` / `current_paragraph.runs`: whether the paragraph has
        // collected anything yet at the point a break is reached. A run is pushed into the
        // paragraph when its `</w:r>` closes, whether or not it carried text, so a drawing's run
        // counts once it has closed.
        uint closedRuns = 0;
        bool runHasText = false;
        uint deferredParagraph = 0;

        void RecordBreak()
        {
            // Inside a table the break is deferred to after the table element: a form feed cannot
            // be written into the middle of a table that renders as one markdown block (GH#1419).
            //
            // Outside one, the paragraph currently being walked has not been pushed yet, so a
            // break reached before any of its content correctly precedes it and can go out now.
            // A break reached *after* content in that same paragraph must not jump ahead of the
            // paragraph's own element, so it is deferred the same way (GH#1416).
            if (inTable) pb.PendingTableBreaks++;
            else if (runHasText || closedRuns > 0) deferredParagraph++;
            else doc.Elements.Add(new DocElement { Kind = DocElementKind.PageBreak });
            pb.TextSinceBreak = false;
        }

        // A plain `Descendants()` sweep cannot see where a run ends, and "the paragraph has a
        // closed run" is half of the test above; this walk is the same document order with the
        // end tags kept.
        void Walk(XElement node)
        {
            foreach (var e in node.Elements())
            {
                Visit(e);
                Walk(e);
                if (e.Name.LocalName == "r") { closedRuns++; runHasText = false; }
            }
        }

        void Visit(XElement e)
        {
            switch (e.Name.LocalName)
            {
                case "drawing":
                    var draw = ParseDrawing(e, rels);
                    doc.Drawings.Add(draw);
                    doc.Elements.Add(new DocElement { Kind = DocElementKind.Drawing, Drawing = draw });
                    pb.TextSinceBreak = true;
                    break;

                // Rendered content: anything that puts glyphs on the page counts as text emitted
                // since the last break, which is what makes a following render hint non-redundant.
                // It is also what fills the current run, which is the other half of the
                // has-this-paragraph-collected-anything test in `RecordBreak` — a formula is its
                // own run rather than run text, but it leaves the paragraph non-empty just the same.
                case "t" when e.Value.Length > 0:
                case "tab":
                case "noBreakHyphen":
                case "sym":
                case "oMath":
                case "oMathPara":
                case "footnoteReference":
                case "endnoteReference":
                    pb.TextSinceBreak = true;
                    runHasText = true;
                    break;
                // Legacy VML text box. `<mc:AlternateContent>` carries the same shape twice
                // (Choice=DrawingML, Fallback=VML), so the fallback copy is dropped — the
                // DrawingML copy inside `<w:drawing>` is the one that is kept, and taking both
                // would emit the text box twice. A `w:pict` only becomes an element when it
                // actually holds text; a picture-only one has nothing to contribute here.
                case "pict":
                {
                    if (e.Ancestors().Any(a => a.Name.LocalName == "Fallback")) break;
                    var boxes = e.Descendants().Where(x => x.Name.LocalName == "txbxContent").ToList();
                    if (boxes.Count == 0) break;
                    string boxText = CollectTxbxContentText(boxes[^1]);
                    if (boxText.Length == 0) break;
                    var pictDraw = new DocxDrawing { TextBoxContent = boxText };
                    doc.Drawings.Add(pictDraw);
                    doc.Elements.Add(new DocElement { Kind = DocElementKind.Drawing, Drawing = pictDraw });
                    pb.TextSinceBreak = true;
                    break;
                }
                // Word writes this hint at the start of the first run on a page *it* rendered —
                // including, redundantly, right after an authored `<w:br w:type="page"/>` that
                // already recorded the same transition (GH#1416). It is the only page-break signal
                // in documents Word paginated itself, so it cannot be dropped outright; it is
                // recorded only when real content has been emitted since the previous break, which
                // is exactly when it is not a redundant echo.
                case "lastRenderedPageBreak":
                    if (pb.TextSinceBreak) RecordBreak();
                    break;

                // The author's own explicit break — always recorded, never suppressed.
                case "br" when e.Attributes().FirstOrDefault(a => a.Name.LocalName == "type")?.Value == "page":
                    RecordBreak();
                    break;

                // Any other break type is a newline in the run's text, so it leaves the run
                // non-empty even though nothing was rendered.
                case "br":
                    runHasText = true;
                    break;
            }
        }

        Walk(container);
        return deferredParagraph;
    }

    private static DocxParagraph ParseParagraph(XElement p, Dictionary<string, string> rels)
    {
        var para = new DocxParagraph();
        var pPr = p.Elements().FirstOrDefault(e => e.Name.LocalName == "pPr");
        if (pPr is not null)
        {
            para.StyleId = Val(pPr.Elements().FirstOrDefault(e => e.Name.LocalName == "pStyle"));
            var numPr = pPr.Elements().FirstOrDefault(e => e.Name.LocalName == "numPr");
            if (numPr is not null)
            {
                para.NumLevel = ParseLong(Val(numPr.Elements().FirstOrDefault(e => e.Name.LocalName == "ilvl")));
                para.NumId = ParseLong(Val(numPr.Elements().FirstOrDefault(e => e.Name.LocalName == "numId")));
            }
        }
        // Walk paragraph content in order: runs, hyperlinks (which wrap runs).
        ParseRunContainer(p, para, null, rels);
        return para;
    }

    private static void ParseRunContainer(XElement container, DocxParagraph para, string? hyperlinkUrl, Dictionary<string, string> rels)
    {
        foreach (var child in container.Elements())
        {
            switch (child.Name.LocalName)
            {
                case "r":
                    var run = ParseRun(child, hyperlinkUrl);
                    if (run is not null) para.Runs.Add(run);
                    break;
                case "hyperlink":
                    string? url = null;
                    var rid = child.Attributes().FirstOrDefault(a => a.Name.LocalName == "id" && a.Name.NamespaceName == R)?.Value
                              ?? child.Attributes().FirstOrDefault(a => a.Name.LocalName == "id")?.Value;
                    if (rid is not null && rels.TryGetValue(rid, out var t)) url = t;
                    var anchor = child.Attributes().FirstOrDefault(a => a.Name.LocalName == "anchor")?.Value;
                    if (url is null && anchor is not null) url = "#" + anchor;
                    ParseRunContainer(child, para, url, rels);
                    break;
                case "oMathPara":
                {
                    string latex = OmmlMath.ConvertOMathPara(child);
                    if (latex.Length != 0)
                        para.Runs.Add(new DocxRun { MathLatex = latex, MathDisplay = true });
                    break;
                }
                case "oMath":
                {
                    string latex = OmmlMath.ConvertOMath(child);
                    if (latex.Length != 0)
                        para.Runs.Add(new DocxRun { MathLatex = latex, MathDisplay = false });
                    break;
                }
            }
        }
    }

    private static DocxRun? ParseRun(XElement r, string? hyperlinkUrl)
    {
        var run = new DocxRun { HyperlinkUrl = hyperlinkUrl };
        var rPr = r.Elements().FirstOrDefault(e => e.Name.LocalName == "rPr");
        if (rPr is not null)
        {
            foreach (var f in rPr.Elements())
            {
                switch (f.Name.LocalName)
                {
                    case "b": run.Bold = FormatEnabled(f); break;
                    case "i": run.Italic = FormatEnabled(f); break;
                    case "u": run.Underline = FormatEnabled(f); break;
                    case "strike":
                    case "dstrike": run.Strike = FormatEnabled(f); break;
                    case "vertAlign":
                        var va = Val(f);
                        if (va == "subscript") { run.Subscript = true; run.Superscript = false; }
                        else if (va == "superscript") { run.Superscript = true; run.Subscript = false; }
                        else { run.Subscript = false; run.Superscript = false; }
                        break;
                    case "sz":
                        if (uint.TryParse(Val(f), out var sz)) run.FontSize = sz;
                        break;
                    case "color":
                        var col = Val(f);
                        if (col is not null && col != "auto" && col.Length == 6 && col.All(Uri.IsHexDigit)) run.FontColor = col;
                        break;
                    case "highlight":
                        var hl = Val(f);
                        if (hl is not null && ValidHighlights.Contains(hl)) run.Highlight = true;
                        break;
                }
            }
        }
        // Text content: w:t (preserve), w:br (line break → \n), w:tab (→ \t), footnote/endnote refs.
        foreach (var child in r.Elements())
        {
            switch (child.Name.LocalName)
            {
                case "t": run.Text += child.Value; break;
                // An in-run <w:tab/> separates words; a <w:pPr><w:tabs> tab-stop definition lives
                // under the paragraph, not the run, so it stays invisible here (Rust GH#1377).
                case "tab": run.Text += "\t"; break;
                case "br":
                    var brType = child.Attributes().FirstOrDefault(a => a.Name.LocalName == "type")?.Value;
                    if (brType != "page") run.Text += "\n";
                    break;
                case "footnoteReference":
                case "endnoteReference":
                    var id = child.Attributes().FirstOrDefault(a => a.Name.LocalName == "id")?.Value;
                    if (id is not null && id != "0" && id != "1") run.Text += $"[^{id}]";
                    break;
                // A reviewer's comment is anchored to a point in the text, and that anchor is
                // part of what the paragraph says: without a marker there is nothing to say the
                // comment was attached here rather than anywhere else in it.
                case "commentReference":
                {
                    var commentId = child.Attributes().FirstOrDefault(a => a.Name.LocalName == "id")?.Value;
                    if (commentId is not null) run.Text += $"[cmt:{commentId}]";
                    break;
                }
            }
        }
        return run;
    }

    private static DocxDrawing ParseDrawing(XElement drawing, Dictionary<string, string> rels)
    {
        var draw = new DocxDrawing();
        var blip = drawing.Descendants().FirstOrDefault(e => e.Name.LocalName == "blip");
        var embed = blip?.Attributes().FirstOrDefault(a => a.Name.LocalName == "embed")?.Value
                    ?? blip?.Attributes().FirstOrDefault(a => a.Name.LocalName == "link")?.Value;
        draw.ImageRid = embed;
        // wp:docPr descr / title as description.
        var docPr = drawing.Descendants().FirstOrDefault(e => e.Name.LocalName == "docPr");
        if (docPr is not null)
        {
            // Word writes `descr` only when the author fills in the description field, but
            // always writes `name`. Without the fallback, an image the author named but never
            // described reaches the output carrying no alt text at all — and a document that is
            // nothing but images extracts to nothing.
            var descr = docPr.Attribute("descr")?.Value;
            if (string.IsNullOrEmpty(descr)) descr = docPr.Attribute("name")?.Value;
            draw.Description = string.IsNullOrEmpty(descr) ? null : descr;
        }
        foreach (var txbx in drawing.Descendants().Where(e => e.Name.LocalName == "txbxContent"))
        {
            string text = CollectTxbxContentText(txbx);
            if (text.Length != 0) draw.TextBoxContent = text;
        }
        return draw;
    }

    /// <summary>
    /// Collect the visible text of a <c>w:txbxContent</c> subtree: each <c>w:p</c> contributes one
    /// line, with <c>w:tab</c> and <c>w:br</c> mapped to a tab and a newline. Empty paragraphs are
    /// dropped and the rest joined with newlines.
    /// </summary>
    private static string CollectTxbxContentText(XElement txbx)
    {
        var paragraphs = new List<string>();
        var current = new StringBuilder();

        void Walk(XElement el)
        {
            foreach (var child in el.Elements())
            {
                switch (child.Name.LocalName)
                {
                    case "t": current.Append(child.Value); break;
                    case "tab": current.Append('\t'); break;
                    case "br": current.Append('\n'); break;
                    default: Walk(child); break;
                }
                if (child.Name.LocalName == "p")
                {
                    paragraphs.Add(current.ToString());
                    current.Clear();
                }
            }
        }

        Walk(txbx);
        if (current.Length > 0) paragraphs.Add(current.ToString());
        return string.Join("\n", paragraphs.Where(p => p.Length != 0));
    }

    // ── tables ───────────────────────────────────────────────────────────────
    private static DocxTable ParseTable(XElement tbl, Dictionary<string, string> rels)
    {
        var table = new DocxTable();
        var tblPr = tbl.Elements().FirstOrDefault(e => e.Name.LocalName == "tblPr");
        if (tblPr is not null)
            table.Caption = Val(tblPr.Elements().FirstOrDefault(e => e.Name.LocalName == "tblCaption"));

        foreach (var tr in tbl.Elements().Where(e => e.Name.LocalName == "tr"))
        {
            Cancel.ThrowIfRequested();
            var row = new DocxRow();
            var trPr = tr.Elements().FirstOrDefault(e => e.Name.LocalName == "trPr");
            if (trPr is not null)
                row.IsHeader = ToggleOn(trPr.Elements().FirstOrDefault(e => e.Name.LocalName == "tblHeader"));

            foreach (var tc in tr.Elements().Where(e => e.Name.LocalName == "tc"))
            {
                var cell = new DocxCell();
                var tcPr = tc.Elements().FirstOrDefault(e => e.Name.LocalName == "tcPr");
                if (tcPr is not null)
                {
                    var gs = Val(tcPr.Elements().FirstOrDefault(e => e.Name.LocalName == "gridSpan"));
                    if (uint.TryParse(gs, out var span) && span > 0) cell.GridSpan = span;
                    var vm = tcPr.Elements().FirstOrDefault(e => e.Name.LocalName == "vMerge");
                    if (vm is not null)
                    {
                        var vmVal = Val(vm);
                        cell.VMergeContinue = vmVal is null || vmVal != "restart";
                    }
                }
                // Flatten nested-table paragraphs into the parent cell (matches Rust),
                // preserving document order.
                foreach (var p in tc.Descendants().Where(e => e.Name.LocalName == "p"))
                    cell.Paragraphs.Add(ParseParagraph(p, rels));
                row.Cells.Add(cell);
            }
            table.Rows.Add(row);
        }
        return table;
    }

    // ── headers / footers / notes ──────────────────────────────────────────────
    private static bool IsHeaderFooter(string name, string kind) =>
        name.StartsWith($"word/{kind}", StringComparison.Ordinal) && name.EndsWith(".xml", StringComparison.Ordinal);

    private static List<DocxParagraph> ReadHeaderFooterParagraphs(OoxmlPackage pkg, string part, Dictionary<string, string> rels)
    {
        var list = new List<DocxParagraph>();
        var xml = pkg.ReadXml(part);
        var root = xml?.Root;
        if (root is null) return list;
        foreach (var p in root.Descendants().Where(e => e.Name.LocalName == "p"))
            list.Add(ParseParagraph(p, rels));
        return list;
    }

    private static List<DocxNote> ReadNotes(OoxmlPackage pkg, string part, string kind, Dictionary<string, string> rels)
    {
        var notes = new List<DocxNote>();
        var xml = pkg.ReadXml(part);
        var root = xml?.Root;
        if (root is null) return notes;
        foreach (var n in root.Elements().Where(e => e.Name.LocalName == kind))
        {
            var id = n.Attributes().FirstOrDefault(a => a.Name.LocalName == "id")?.Value ?? "";
            // A footnotes part reserves ids -1, 0 and 1 for the separator marks Word writes into
            // every document; a comments part numbers its comments from 0 and reserves nothing.
            if (kind != "comment" && id is "-1" or "0" or "1") continue;
            var note = new DocxNote { Id = id };
            foreach (var p in n.Elements().Where(e => e.Name.LocalName == "p"))
                note.Paragraphs.Add(ParseParagraph(p, rels));
            notes.Add(note);
        }
        return notes;
    }

    // ── relationships / styles / numbering ─────────────────────────────────────
    private static Dictionary<string, string> ReadRelationships(OoxmlPackage pkg)
    {
        var map = new Dictionary<string, string>(StringComparer.Ordinal);
        var xml = pkg.ReadXml("word/_rels/document.xml.rels");
        if (xml?.Root is null) return map;
        foreach (var r in xml.Root.Elements().Where(e => e.Name.LocalName == "Relationship"))
        {
            var type = r.Attribute("Type")?.Value ?? "";
            var id = r.Attribute("Id")?.Value;
            var target = r.Attribute("Target")?.Value;
            if (id is null || target is null) continue;
            if (type.Contains("hyperlink") || type.Contains("image")) map[id] = target;
        }
        return map;
    }

    private sealed class StyleDef { public string? Name, BasedOn; public int? OutlineLvl; }

    private static Dictionary<string, StyleDef> ReadStyles(OoxmlPackage pkg)
    {
        var map = new Dictionary<string, StyleDef>(StringComparer.Ordinal);
        var xml = pkg.ReadXml("word/styles.xml");
        if (xml?.Root is null) return map;
        foreach (var s in xml.Root.Elements().Where(e => e.Name.LocalName == "style"))
        {
            var id = s.Attributes().FirstOrDefault(a => a.Name.LocalName == "styleId")?.Value;
            if (id is null) continue;
            var def = new StyleDef
            {
                Name = Val(s.Elements().FirstOrDefault(e => e.Name.LocalName == "name")),
                BasedOn = Val(s.Elements().FirstOrDefault(e => e.Name.LocalName == "basedOn")),
            };
            var pPr = s.Elements().FirstOrDefault(e => e.Name.LocalName == "pPr");
            var ol = Val(pPr?.Elements().FirstOrDefault(e => e.Name.LocalName == "outlineLvl"));
            if (ol is not null && int.TryParse(ol, out var olv)) def.OutlineLvl = olv;
            map[id] = def;
        }
        return map;
    }

    public static byte? ResolveHeadingLevel(string? styleId)
    {
        if (styleId is null) return null;
        var styles = _styles;
        if (styles is not null)
        {
            string cur = styleId;
            for (int i = 0; i < 20; i++)
            {
                if (!styles.TryGetValue(cur, out var s)) break;
                if (s.OutlineLvl is { } ol) return (byte)Math.Min(ol + 1, 6);
                if (s.Name is "Title" or "title") return 1;
                if (s.BasedOn is not null) { cur = s.BasedOn; continue; }
                break;
            }
        }
        return HeadingLevelFromStyleName(styleId);
    }

    /// <summary>
    /// Heading level from a style id, for a document that ships no <c>styles.xml</c> or whose style
    /// omits <c>w:outlineLvl</c>.
    /// </summary>
    /// <remarks>
    /// The trailing digit of a style id like <c>Heading1</c> is already the level the author meant,
    /// so it is returned as-is. The style-catalog path reads <c>w:outlineLvl</c>, which ISO 29500
    /// defines as zero-based, so the <c>+ 1</c> there is correct and stays — this fallback used to
    /// add one as well, making every heading in such a document a level too deep. There is no
    /// reserved level for the title: ODT reads <c>text:outline-level</c> straight through with no
    /// offset, so the extractors do not share one.
    /// </remarks>
    private static byte? HeadingLevelFromStyleName(string id)
    {
        if (id == "Title") return 1;
        if (id.StartsWith("Heading", StringComparison.Ordinal) || id.StartsWith("heading", StringComparison.Ordinal))
        {
            var suffix = id["Heading".Length..];
            if (int.TryParse(suffix, out var n) && n >= 1 && n <= 6) return (byte)n;
            return null;
        }
        return null;
    }

    private static readonly HashSet<string> NumberedFormats = new()
    {
        "decimal","decimalZero","lowerLetter","upperLetter","lowerRoman","upperRoman",
    };

    private static Dictionary<(long, long), bool> ReadNumbering(OoxmlPackage pkg)
    {
        var result = new Dictionary<(long, long), bool>();
        var xml = pkg.ReadXml("word/numbering.xml");
        if (xml?.Root is null) return result;

        // abstractNumId -> (level -> ordered)
        var abstractFormats = new Dictionary<long, Dictionary<long, bool>>();
        foreach (var an in xml.Root.Elements().Where(e => e.Name.LocalName == "abstractNum"))
        {
            var aidStr = an.Attributes().FirstOrDefault(a => a.Name.LocalName == "abstractNumId")?.Value;
            if (!long.TryParse(aidStr, out var aid)) continue;
            var levels = new Dictionary<long, bool>();
            foreach (var lvl in an.Elements().Where(e => e.Name.LocalName == "lvl"))
            {
                var ilvlStr = lvl.Attributes().FirstOrDefault(a => a.Name.LocalName == "ilvl")?.Value;
                if (!long.TryParse(ilvlStr, out var ilvl)) continue;
                var fmt = Val(lvl.Elements().FirstOrDefault(e => e.Name.LocalName == "numFmt"));
                levels[ilvl] = fmt is not null && NumberedFormats.Contains(fmt);
            }
            abstractFormats[aid] = levels;
        }

        // numId -> abstractNumId
        var numToAbstract = new Dictionary<long, long>();
        foreach (var num in xml.Root.Elements().Where(e => e.Name.LocalName == "num"))
        {
            var nidStr = num.Attributes().FirstOrDefault(a => a.Name.LocalName == "numId")?.Value;
            if (!long.TryParse(nidStr, out var nid)) continue;
            var aidStr = Val(num.Elements().FirstOrDefault(e => e.Name.LocalName == "abstractNumId"));
            if (long.TryParse(aidStr, out var aid)) numToAbstract[nid] = aid;
        }

        foreach (var (nid, aid) in numToAbstract)
            if (abstractFormats.TryGetValue(aid, out var levels))
                foreach (var (lvl, ordered) in levels)
                    result[(nid, lvl)] = ordered;
        return result;
    }

    // ── helpers ──────────────────────────────────────────────────────────────
    private static string? Val(XElement? el) =>
        el?.Attributes().FirstOrDefault(a => a.Name.LocalName == "val")?.Value;

    private static bool FormatEnabled(XElement el)
    {
        var v = Val(el);
        if (v is null) return true;
        return v is not ("false" or "0" or "none");
    }

    private static bool ToggleOn(XElement? el)
    {
        if (el is null) return false;
        var v = Val(el);
        if (v is null) return true;
        return v is not ("0" or "false" or "off");
    }

    private static long? ParseLong(string? s) => long.TryParse(s, out var v) ? v : null;

}
