// Ported from Rust `crates/xberg/src/extractors/excel.rs` + `extraction/excel.rs` (the ODS
// branch) and calamine 0.35's `ods.rs` (`read_table` / `read_row` / `get_range` /
// `get_datatype`). Self-contained: reads the ODF spreadsheet ZIP with System.IO.Compression
// and turns each table into a heading + Table, mirroring the XLSX extractor's document shape.

using System.Globalization;
using System.IO.Compression;
using System.Text;
using System.Text.Json;
using System.Xml.Linq;
using XRay.Content.Core;
using XRay.Content.Core.Ocr;
using XRay.Content.Types;
using XRay.Content.Internal.Odf;

namespace XRay.Content.Extractors;

/// <summary>OpenDocument Spreadsheet (.ods) extractor.</summary>
public sealed class OdsExtractor : IExtractor
{
    public IEnumerable<string> SupportedMimeTypes => new[] { "application/vnd.oasis.opendocument.spreadsheet" };

    // Higher than XlsxExtractor's 50 so the ODS-specific reader wins the shared MIME type.
    public int Priority => 60;

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        byte[] bytes = content.ToArray();
        var sheets = ReadSheets(bytes);
        TableBudget.ChargeSheets(SecurityBudget.FromConfig(config), sheets.Select(sh => sh.Cells));

        var props = ReadOfficeProperties(bytes);

        var doc = BuildInternalDocument(sheets);
        doc.MimeType = mimeType;

        var sheetNames = sheets.Select(s => s.Name).ToList();
        var excelMeta = new ExcelMetadata { SheetCount = (uint)sheets.Count, SheetNames = sheetNames };

        // Metadata dict (calamine `extract_metadata`): sheet_count + sheet_names.
        string namesStr;
        if (sheetNames.Count <= 5)
            namesStr = string.Join(", ", sheetNames);
        else
        {
            var sb = new StringBuilder();
            for (int i = 0; i < 5; i++) { if (i > 0) sb.Append(", "); sb.Append(sheetNames[i]); }
            sb.Append(", ... (").Append(sheetNames.Count).Append(" total)");
            namesStr = sb.ToString();
        }

        var additional = new Dictionary<string, JsonElement>(StringComparer.Ordinal)
        {
            ["sheet_count"] = JsonSerializer.SerializeToElement(sheets.Count.ToString(CultureInfo.InvariantCulture)),
            ["sheet_names"] = JsonSerializer.SerializeToElement(namesStr),
        };

        // ODF splits authorship the other way round from OOXML: `meta:initial-creator` is who
        // created the document and `dc:creator` is who last touched it, where OOXML's
        // `dc:creator` is the original author. Map by role, and fall back to `dc:creator` for
        // the author when a producer omits `meta:initial-creator`.
        string? author = !string.IsNullOrEmpty(props?.InitialCreator) ? props!.InitialCreator : props?.Creator;

        doc.Metadata = new Metadata
        {
            Format = FormatMetadata.Excel(excelMeta),
            Title = Blank(props?.Title),
            Subject = Blank(props?.Subject),
            Language = Blank(props?.Language),
            CreatedAt = Blank(props?.CreationDate),
            ModifiedAt = Blank(props?.Date),
            CreatedBy = Blank(author),
            ModifiedBy = Blank(props?.Creator),
            Authors = Blank(author) is { } a ? new List<string> { a } : null,
            Additional = additional,
        };
        OcrImageSource.AddPicturesFromZip(doc, bytes, config, "Pictures/");
        return doc;
    }

    private static string? Blank(string? v) => string.IsNullOrEmpty(v) ? null : v;

    /// <summary>
    /// An `.ods` is an ODF package, not an OOXML one: its document properties live in
    /// `meta.xml`, which the OOXML reader does not look at, so without this every ODS came
    /// back with no title, author or dates.
    /// </summary>
    private static OdtProperties? ReadOfficeProperties(byte[] bytes)
    {
        try
        {
            using var stream = new MemoryStream(bytes, writable: false);
            using var archive = new ZipArchive(stream, ZipArchiveMode.Read);
            return XRay.Content.Internal.Odf.OdfMetadata.Extract(archive);
        }
        catch (Exception) when (Cancel.NotCancelling)
        {
            // A spreadsheet whose meta.xml is absent or malformed still has its sheets.
            return null;
        }
    }

    // ── document building (mirrors XlsxExtractor.BuildInternalDocument) ─────────

    private sealed class OdsSheet
    {
        public string Name = "";
        public List<List<string>>? Cells;
        public string Markdown = "";
    }

    private static InternalDocument BuildInternalDocument(List<OdsSheet> sheets)
    {
        var builder = new InternalDocumentBuilder("excel");
        var pages = new List<PageContent>(sheets.Count);

        for (int i = 0; i < sheets.Count; i++)
        {
            var sheet = sheets[i];
            uint pageNumber = (uint)(i + 1);
            string? nameOpt = sheet.Name.Length == 0 ? null : sheet.Name;

            if (sheet.Cells is { Count: > 0 } cells)
            {
                if (sheet.Name.Length > 0)
                    builder.PushHeading(2, sheet.Name, null, null);
                builder.PushTableFromCells(cells, pageNumber, null);

                string pageContent = sheet.Name.Length == 0
                    ? sheet.Markdown
                    : $"## {EscapeSheetNameForHeading(sheet.Name)}\n\n{sheet.Markdown}";

                var table = new Table
                {
                    Cells = cells.Select(r => new List<string>(r)).ToList(),
                    Markdown = sheet.Markdown,
                    PageNumber = pageNumber,
                };
                pages.Add(new PageContent
                {
                    PageNumber = pageNumber,
                    Content = pageContent,
                    Tables = new List<Table> { table },
                    IsBlank = false,
                    SheetName = nameOpt,
                });
            }
            else
            {
                string pageContent = nameOpt is null ? "" : $"## {EscapeSheetNameForHeading(nameOpt)}\n\n";
                pages.Add(new PageContent
                {
                    PageNumber = pageNumber,
                    Content = pageContent,
                    IsBlank = true,
                    SheetName = nameOpt,
                });
            }
        }

        var doc = builder.Build();
        doc.PrebuiltPages = pages;
        return doc;
    }

    private static readonly char[] InlineMetachars = { '\\', '`', '*', '_', '[', ']', '<', '>', '!' };

    private static string EscapeSheetNameForHeading(string name)
    {
        name = name.TrimStart();
        var sb = new StringBuilder(name.Length + 8);
        for (int i = 0; i < name.Length; i++)
        {
            char ch = name[i];
            bool needsEscape = (i == 0 && ch is '#' or '>' or '-' or '*' or '+' or '~') || Array.IndexOf(InlineMetachars, ch) >= 0;
            if (needsEscape) sb.Append('\\');
            sb.Append(ch);
        }
        return sb.ToString();
    }

    private static string GenerateMarkdown(string name, List<List<string>> cells)
    {
        var sb = new StringBuilder();
        sb.Append("## ").Append(name).Append("\n\n");
        if (cells.Count == 0) return sb.ToString();

        int headerLen = cells[0].Count;
        sb.Append("| ");
        for (int i = 0; i < headerLen; i++) { if (i > 0) sb.Append(" | "); AppendCell(sb, cells[0][i]); }
        sb.Append(" |\n");
        sb.Append("| ");
        for (int i = 0; i < headerLen; i++) { if (i > 0) sb.Append(" | "); sb.Append("---"); }
        sb.Append(" |\n");
        for (int r = 1; r < cells.Count; r++)
        {
            sb.Append("| ");
            for (int i = 0; i < headerLen; i++) { if (i > 0) sb.Append(" | "); AppendCell(sb, i < cells[r].Count ? cells[r][i] : ""); }
            sb.Append(" |\n");
        }
        return sb.ToString();
    }

    private static void AppendCell(StringBuilder sb, string cell)
    {
        if (cell.Contains('|') || cell.Contains('\\'))
        {
            foreach (var ch in cell)
            {
                if (ch == '|') sb.Append("\\|");
                else if (ch == '\\') sb.Append("\\\\");
                else sb.Append(ch);
            }
        }
        else sb.Append(cell);
    }

    // ── ODS content.xml parsing (calamine ods.rs) ──────────────────────────────

    /// <summary>A cell value; <see cref="IsEmpty"/> marks calamine's <c>Data::Empty</c> default.</summary>
    private readonly record struct OdsCell(bool IsEmpty, string Value)
    {
        public static readonly OdsCell Empty = new(true, "");
    }

    private static List<OdsSheet> ReadSheets(byte[] bytes)
    {
        using var stream = new MemoryStream(bytes, writable: false);
        using var archive = new ZipArchive(stream, ZipArchiveMode.Read);
        var entry = archive.GetEntry("content.xml")
            ?? throw new InvalidDataException("ODS archive missing content.xml");

        XDocument content;
        using (var s = entry.Open())
            content = XDocument.Load(Cancel.Poll(s), LoadOptions.PreserveWhitespace);

        var sheets = new List<OdsSheet>();
        foreach (var table in content.Descendants().Where(e => e.Name.LocalName == "table"
                                                               && e.Name.NamespaceName.Contains("table")))
        {
            string name = table.Attributes().FirstOrDefault(a => a.Name.LocalName == "name")?.Value ?? "";
            var grid = ReadTable(table);
            if (grid is { Count: > 0 })
                sheets.Add(new OdsSheet { Name = name, Cells = grid, Markdown = GenerateMarkdown(name, grid) });
            else
                sheets.Add(new OdsSheet { Name = name, Cells = null, Markdown = $"## {name}\n\n*Empty sheet*" });
        }
        return sheets;
    }

    /// <summary>Read a table:table into the dense string grid calamine's <c>Range</c> would yield.</summary>
    private static List<List<string>>? ReadTable(XElement table)
    {
        var storedRows = new List<List<OdsCell>>();
        var rowRepeats = new List<int>();

        foreach (var row in table.Elements().Where(e => e.Name.LocalName == "table-row"))
        {
            int repeats = GetIntAttr(row, "number-rows-repeated") ?? 1;
            storedRows.Add(ReadRow(row));
            rowRepeats.Add(repeats);
        }

        return BuildRange(storedRows, rowRepeats);
    }

    /// <summary>Ports calamine `read_row`: interior empty cells become <c>Empty</c>, trailing
    /// empty cells are dropped, valued cells are repeated per number-columns-repeated.</summary>
    private static List<OdsCell> ReadRow(XElement row)
    {
        var cells = new List<OdsCell>();
        int emptyColRepeats = 0;

        foreach (var c in row.Elements().Where(e => e.Name.LocalName is "table-cell" or "covered-table-cell"))
        {
            int repeats = GetIntAttr(c, "number-columns-repeated") ?? 1;
            var value = GetDatatype(c);

            for (int i = 0; i < emptyColRepeats; i++) cells.Add(OdsCell.Empty);
            emptyColRepeats = 0;

            if (value.IsEmpty)
                emptyColRepeats = repeats;
            else
                for (int i = 0; i < repeats; i++) cells.Add(value);
        }
        // Trailing emptyColRepeats deliberately dropped (matches calamine).
        return cells;
    }

    /// <summary>Ports calamine `get_datatype` (formulas ignored — see PORT_NOTES).</summary>
    private static OdsCell GetDatatype(XElement c)
    {
        bool isString = false, isValueSet = false;
        OdsCell val = OdsCell.Empty;

        foreach (var a in c.Attributes())
        {
            if (a.Name.NamespaceName.Length == 0) continue;
            switch (a.Name.LocalName)
            {
                case "value" when !isValueSet:
                    if (double.TryParse(a.Value, NumberStyles.Float, CultureInfo.InvariantCulture, out var d))
                        val = new OdsCell(false, FormatFloat(d));
                    else
                        val = new OdsCell(false, a.Value);
                    isValueSet = true;
                    break;
                case "string-value" when !isValueSet:
                    val = new OdsCell(false, a.Value);
                    isValueSet = true;
                    break;
                case "date-value" when !isValueSet:
                    val = new OdsCell(false, a.Value);
                    isValueSet = true;
                    break;
                case "time-value" when !isValueSet:
                    val = new OdsCell(false, "DURATION: " + a.Value);
                    isValueSet = true;
                    break;
                case "boolean-value" when !isValueSet:
                    val = new OdsCell(false, a.Value is "TRUE" or "true" ? "true" : "false");
                    isValueSet = true;
                    break;
                case "value-type" when !isValueSet:
                    isString = a.Value == "string";
                    break;
            }
        }

        if (!isValueSet && isString)
            return new OdsCell(false, ReadStringContent(c));
        return val;
    }

    /// <summary>Read a string cell's text content: paragraphs joined with '\n', text:s → spaces.</summary>
    private static string ReadStringContent(XElement cell)
    {
        var sb = new StringBuilder();
        bool firstParagraph = true;
        WalkText(cell, sb, ref firstParagraph);
        return sb.ToString();
    }

    private static void WalkText(XElement node, StringBuilder sb, ref bool firstParagraph)
    {
        foreach (var n in node.Nodes())
        {
            switch (n)
            {
                case XText t:
                    sb.Append(t.Value);
                    break;
                case XElement e when e.Name.LocalName == "annotation":
                    break; // skip office:annotation
                case XElement e when e.Name.LocalName == "p":
                    if (firstParagraph) firstParagraph = false; else sb.Append('\n');
                    WalkText(e, sb, ref firstParagraph);
                    break;
                case XElement e when e.Name.LocalName == "s":
                    int count = GetIntAttr(e, "c") ?? 1;
                    sb.Append(' ', count);
                    break;
                case XElement e:
                    WalkText(e, sb, ref firstParagraph);
                    break;
            }
        }
    }

    /// <summary>Ports calamine `get_range`: trims to the populated bounding box and expands
    /// row/empty-row repeats, producing the dense grid of formatted strings.</summary>
    private static List<List<string>>? BuildRange(List<List<OdsCell>> storedRows, List<int> rowRepeats)
    {
        int? rowMin = null, rowMax = 0;
        int colMin = int.MaxValue, colMax = 0;

        for (int i = 0; i < storedRows.Count; i++)
        {
            var row = storedRows[i];
            int first = row.FindIndex(c => !c.IsEmpty);
            if (first < 0) continue;
            rowMin ??= i;
            rowMax = i;
            if (first < colMin) colMin = first;
            int last = row.FindLastIndex(c => !c.IsEmpty);
            if (last > colMax) colMax = last;
        }

        if (rowMin is null) return null;

        int rowWidth = colMax + 1 - colMin;
        var result = new List<List<string>>();
        int emptyRowRepeats = 0;

        for (int i = rowMin.Value; i <= rowMax; i++)
        {
            var row = storedRows[i];
            int rr = rowRepeats[i];

            if (row.All(c => c.IsEmpty))
            {
                emptyRowRepeats += rr;
                continue;
            }

            for (int k = 0; k < emptyRowRepeats; k++)
                result.Add(EmptyStringRow(rowWidth));
            emptyRowRepeats = 0;

            for (int rep = 0; rep < rr; rep++)
            {
                var outRow = new List<string>(rowWidth);
                for (int col = colMin; col <= colMax; col++)
                    outRow.Add(col < row.Count ? FormatCell(row[col]) : "");
                result.Add(outRow);
            }
        }

        return result;
    }

    private static List<string> EmptyStringRow(int width)
    {
        var r = new List<string>(width);
        for (int i = 0; i < width; i++) r.Add("");
        return r;
    }

    private static string FormatCell(OdsCell c) => c.IsEmpty ? "" : c.Value;

    // Rust `format!("{}", f)`: integers without a decimal point, otherwise shortest round-trip.
    private static string FormatFloat(double d) => d.ToString("R", CultureInfo.InvariantCulture);

    private static int? GetIntAttr(XElement e, string localName)
    {
        var attr = e.Attributes().FirstOrDefault(a => a.Name.LocalName == localName && a.Name.NamespaceName.Length > 0)
                   ?? e.Attributes().FirstOrDefault(a => a.Name.LocalName == localName);
        return attr is not null && int.TryParse(attr.Value, NumberStyles.Integer, CultureInfo.InvariantCulture, out var v) ? v : null;
    }
}
