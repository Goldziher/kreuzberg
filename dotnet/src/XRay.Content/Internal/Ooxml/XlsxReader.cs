using System.Globalization;
using System.Text;
using System.Xml.Linq;
using XRay.Content.Internal.Excel;

namespace XRay.Content.Internal.Ooxml;

/// <summary>One worksheet: name, dense cell grid (null when empty), and pre-rendered markdown.</summary>
public sealed class ExcelSheet
{
    public string Name = "";
    public string Markdown = "";
    public List<List<string>>? TableCells;
}

/// <summary>A parsed workbook: ordered sheets + a flat string metadata map.</summary>
public sealed class ExcelWorkbook
{
    public List<ExcelSheet> Sheets { get; } = new();
    public Dictionary<string, string> Metadata { get; } = new();
}

/// <summary>
/// Pure-managed XLSX/XLSM/XLTM/XLAM reader — ports the calamine-backed behaviour in
/// <c>extraction/excel.rs</c> (dense used-range grid, cell-to-string formatting,
/// markdown table generation) over an OOXML ZIP package. Binary formats (.xls/.xlsb) and
/// ODS are out of scope.
/// </summary>
public static class XlsxReader
{
    /// <summary>Reads a workbook's sheets and metadata.</summary>
    /// <param name="content">The OOXML package.</param>
    /// <param name="officeMetadata">Collect the package's document properties.</param>
    /// <param name="numberFormats">
    /// Render numeric cells through <c>styles.xml</c>, so a cell reads the way Excel shows it.
    /// False keeps the bare value calamine hands the Rust original — see "Deviation: formatted
    /// Excel cells" in <c>CLAUDE.md</c>.
    /// </param>
    public static ExcelWorkbook Read(ReadOnlySpan<byte> content, bool officeMetadata, bool numberFormats = true)
    {
        using var pkg = new OoxmlPackage(content);
        var wb = new ExcelWorkbook();

        var sheetNames = ReadSheetOrder(pkg);
        var shared = ReadSharedStrings(pkg);
        var formats = numberFormats ? ReadCellFormats(pkg) : ExcelCellFormats.Empty;

        foreach (var (name, target, _) in sheetNames)
        {
            Cancel.ThrowIfRequested();
            var sheet = ProcessSheet(pkg, name, target, shared, formats);
            wb.Sheets.Add(sheet);
            if (CollectSheetFormulas(pkg, target) is { } formulas) wb.Metadata[$"formulas_{name}"] = formulas;
        }

        if (ReadDefinedNames(pkg) is { } definedNames) wb.Metadata["defined_names"] = definedNames;

        ExtractMetadata(pkg, wb, sheetNames.Select(s => s.Name).ToList(), officeMetadata);

        var hidden = sheetNames.Where(s => s.Hidden).Select(s => s.Name).ToList();
        if (hidden.Count > 0) wb.Metadata["hidden_sheets"] = string.Join(", ", hidden);
        return wb;
    }

    // ── workbook.xml: ordered (sheet name → worksheet part path) ───────────────
    private static List<(string Name, string Target, bool Hidden)> ReadSheetOrder(OoxmlPackage pkg)
    {
        var result = new List<(string, string, bool)>();
        var wbXml = pkg.ReadXml("xl/workbook.xml");
        if (wbXml?.Root is null) return result;

        // rId → target (relative to xl/)
        var rels = new Dictionary<string, string>(StringComparer.Ordinal);
        var relsXml = pkg.ReadXml("xl/_rels/workbook.xml.rels");
        if (relsXml?.Root is not null)
            foreach (var r in relsXml.Root.Elements().Where(e => e.Name.LocalName == "Relationship"))
            {
                var id = r.Attribute("Id")?.Value;
                var tgt = r.Attribute("Target")?.Value;
                if (id is not null && tgt is not null) rels[id] = tgt;
            }

        var sheetsEl = wbXml.Root.Elements().FirstOrDefault(e => e.Name.LocalName == "sheets");
        if (sheetsEl is null) return result;
        foreach (var s in sheetsEl.Elements().Where(e => e.Name.LocalName == "sheet"))
        {
            var name = s.Attributes().FirstOrDefault(a => a.Name.LocalName == "name")?.Value ?? "";
            var rid = s.Attributes().FirstOrDefault(a => a.Name.LocalName == "id")?.Value; // r:id
            string target = "";
            if (rid is not null && rels.TryGetValue(rid, out var t))
                target = ResolveXlPath(t);
            // A sheet's own content says nothing about its visibility; only the workbook's
            // sheet list does, through `state`.
            string state = s.Attributes().FirstOrDefault(a => a.Name.LocalName == "state")?.Value ?? "visible";
            result.Add((name, target, !string.Equals(state, "visible", StringComparison.OrdinalIgnoreCase)));
        }
        return result;
    }

    private static string ResolveXlPath(string target)
    {
        if (target.StartsWith('/')) return target[1..];
        if (target.StartsWith("xl/", StringComparison.Ordinal)) return target;
        return "xl/" + target;
    }

    // ── sharedStrings.xml ──────────────────────────────────────────────────────
    private static List<string> ReadSharedStrings(OoxmlPackage pkg)
    {
        var list = new List<string>();
        var xml = pkg.ReadXml("xl/sharedStrings.xml");
        if (xml?.Root is null) return list;
        foreach (var si in xml.Root.Elements().Where(e => e.Name.LocalName == "si"))
        {
            Cancel.Every(list.Count, 0x3FF);
            list.Add(ConcatText(si));
        }
        return list;
    }

    /// <summary>Concatenate all descendant &lt;t&gt; text in document order.</summary>
    private static string ConcatText(XElement element)
    {
        var sb = new StringBuilder();
        foreach (var t in element.Descendants().Where(e => e.Name.LocalName == "t"))
            sb.Append(t.Value);
        return sb.ToString();
    }

    // ── worksheet → ExcelSheet ─────────────────────────────────────────────────
    /// <summary>Entries a single sheet contributes to workbook metadata before it is truncated.</summary>
    private const int MaxMetadataEntriesPerSheet = 200;

    /// <summary>
    /// A sheet's formulas as <c>cell=formula</c> entries.
    /// </summary>
    /// <remarks>
    /// The cell reference is relative to the top-left of the block of cells that carry formulas,
    /// not to the sheet: a lone formula anywhere on a sheet is reported as <c>A1</c>. That is
    /// what the reference implementation's formula range gives, and the entries are a summary of
    /// what the sheet computes rather than a map of where.
    /// </remarks>
    private static string? CollectSheetFormulas(OoxmlPackage pkg, string target)
    {
        if (target.Length == 0) return null;
        var xml = pkg.ReadXml(target);
        var sheetData = xml?.Root?.Elements().FirstOrDefault(e => e.Name.LocalName == "sheetData");
        if (sheetData is null) return null;

        var found = new List<(int Row, int Col, string Formula)>();
        int autoRow = 0;
        foreach (var row in sheetData.Elements().Where(e => e.Name.LocalName == "row"))
        {
            Cancel.ThrowIfRequested();
            int rowIdx = ParseRowIndex(row) ?? autoRow;
            autoRow = rowIdx + 1;
            int autoCol = 0;
            foreach (var c in row.Elements().Where(e => e.Name.LocalName == "c"))
            {
                int colIdx = ParseColIndex(c) ?? autoCol;
                autoCol = colIdx + 1;
                var f = c.Elements().FirstOrDefault(e => e.Name.LocalName == "f");
                if (f is null) continue;
                string formula = f.Value;
                if (formula.Length == 0) continue;
                found.Add((rowIdx, colIdx, formula));
            }
        }
        if (found.Count == 0) return null;

        int rowOrigin = found.Min(e => e.Row);
        int colOrigin = found.Min(e => e.Col);
        var entries = found
            .Take(MaxMetadataEntriesPerSheet)
            .Select(e => $"{CellReference(e.Row - rowOrigin, e.Col - colOrigin)}={e.Formula}");
        return string.Join("; ", entries);
    }

    /// <summary>The workbook's defined names as <c>name=formula</c> entries.</summary>
    private static string? ReadDefinedNames(OoxmlPackage pkg)
    {
        var wbXml = pkg.ReadXml("xl/workbook.xml");
        var block = wbXml?.Root?.Elements().FirstOrDefault(e => e.Name.LocalName == "definedNames");
        if (block is null) return null;

        var entries = block.Elements()
            .Where(e => e.Name.LocalName == "definedName")
            .Select(e => (Name: e.Attribute("name")?.Value ?? "", Formula: e.Value))
            .Where(e => e.Name.Length != 0)
            .Select(e => $"{e.Name}={e.Formula}")
            .ToList();
        return entries.Count == 0 ? null : string.Join("; ", entries);
    }

    /// <summary>A zero-based row and column as a spreadsheet cell reference, "A1".</summary>
    private static string CellReference(int row, int col) => ColumnLetters(col) + (row + 1).ToString(CultureInfo.InvariantCulture);

    private static string ColumnLetters(int col)
    {
        var letters = new StringBuilder();
        int n = col;
        while (true)
        {
            letters.Insert(0, (char)('A' + n % 26));
            n = n / 26 - 1;
            if (n < 0) break;
        }
        return letters.ToString();
    }

    private static ExcelSheet ProcessSheet(OoxmlPackage pkg, string name, string target, List<string> shared, ExcelCellFormats formats)
    {
        var cellsByPos = new Dictionary<(int Row, int Col), string>();
        int rowMin = int.MaxValue, rowMax = -1, colMin = int.MaxValue, colMax = -1;

        var xml = target.Length > 0 ? pkg.ReadXml(target) : null;
        var sheetData = xml?.Root?.Elements().FirstOrDefault(e => e.Name.LocalName == "sheetData");
        if (sheetData is not null)
        {
            int autoRow = 0;
            foreach (var row in sheetData.Elements().Where(e => e.Name.LocalName == "row"))
            {
                Cancel.ThrowIfRequested();
                int rowIdx = ParseRowIndex(row) ?? autoRow;
                autoRow = rowIdx + 1;
                int autoCol = 0;
                foreach (var c in row.Elements().Where(e => e.Name.LocalName == "c"))
                {
                    int colIdx = ParseColIndex(c) ?? autoCol;
                    autoCol = colIdx + 1;
                    if (TryCellValue(c, shared, formats, out var value))
                    {
                        cellsByPos[(rowIdx, colIdx)] = value;
                        if (rowIdx < rowMin) rowMin = rowIdx;
                        if (rowIdx > rowMax) rowMax = rowIdx;
                        if (colIdx < colMin) colMin = colIdx;
                        if (colIdx > colMax) colMax = colIdx;
                    }
                }
            }
        }

        if (rowMax < 0)
        {
            // Empty sheet.
            return new ExcelSheet { Name = name, Markdown = $"## {name}\n\n*Empty sheet*", TableCells = null };
        }

        // Dense grid over the used bounding box.
        int width = colMax - colMin + 1;
        var grid = new List<List<string>>(rowMax - rowMin + 1);
        for (int r = rowMin; r <= rowMax; r++)
        {
            Cancel.Every(r, 0xFF);
            var rowCells = new List<string>(width);
            for (int col = colMin; col <= colMax; col++)
                rowCells.Add(cellsByPos.TryGetValue((r, col), out var v) ? v : "");
            grid.Add(rowCells);
        }

        var markdown = GenerateMarkdown(name, grid);
        return new ExcelSheet { Name = name, Markdown = markdown, TableCells = grid };
    }

    private static int? ParseRowIndex(XElement row)
    {
        var r = row.Attribute("r")?.Value;
        if (r is not null && int.TryParse(r, out var v)) return v - 1;
        return null;
    }

    private static int? ParseColIndex(XElement cell)
    {
        var r = cell.Attribute("r")?.Value;
        if (r is null) return null;
        int col = 0; bool any = false;
        foreach (var ch in r)
        {
            if (ch is >= 'A' and <= 'Z') { col = col * 26 + (ch - 'A' + 1); any = true; }
            else if (ch is >= 'a' and <= 'z') { col = col * 26 + (ch - 'a' + 1); any = true; }
            else break;
        }
        return any ? col - 1 : null;
    }

    /// <summary>Format a cell's value like calamine's <c>format_cell_to_string</c>. Returns false for empty cells.</summary>
    private static bool TryCellValue(
        XElement c, List<string> shared, ExcelCellFormats formats, out string value)
    {
        value = "";
        string? type = c.Attribute("t")?.Value;

        if (type == "inlineStr")
        {
            var isEl = c.Elements().FirstOrDefault(e => e.Name.LocalName == "is");
            if (isEl is null) return false;
            value = ConcatText(isEl);
            return true;
        }

        var vEl = c.Elements().FirstOrDefault(e => e.Name.LocalName == "v");
        if (vEl is null) return false;
        string raw = vEl.Value;

        switch (type)
        {
            case "s":
                if (int.TryParse(raw, NumberStyles.Integer, CultureInfo.InvariantCulture, out var idx)
                    && idx >= 0 && idx < shared.Count)
                    value = shared[idx];
                else value = "";
                return true;
            case "str":
                value = raw;
                return true;
            case "b":
                value = raw.Trim() == "1" ? "true" : "false";
                return true;
            case "e":
                value = "#ERR: " + raw;
                return true;
            case "d":
                value = raw;
                return true;
            default:
                // Numeric (t absent or "n"): the cell's own format where it has one, and
                // otherwise shortest round-trippable, integers without ".0".
                if (double.TryParse(raw, NumberStyles.Float, CultureInfo.InvariantCulture, out var d))
                {
                    value = formats.Render(CellFormatIndex(c), d) ?? d.ToString("R", CultureInfo.InvariantCulture);
                }
                else
                {
                    value = raw;
                }
                return true;
        }
    }

    /// <summary>The cell's <c>s</c> attribute: its index into <c>cellXfs</c>, defaulting to the
    /// first entry the way a cell with no style does.</summary>
    private static int CellFormatIndex(XElement c) =>
        int.TryParse(c.Attribute("s")?.Value, NumberStyles.Integer, CultureInfo.InvariantCulture, out int s)
            ? s
            : 0;

    // ── styles.xml: numFmts + cellXfs ─────────────────────────────────────────
    /// <summary>
    /// The workbook's number formats and the cell formats that cite them. <c>cellXfs</c> is
    /// positional — a cell's <c>s</c> attribute indexes it — so the order is the file's.
    /// </summary>
    private static ExcelCellFormats ReadCellFormats(OoxmlPackage pkg)
    {
        var formats = new ExcelCellFormats();
        var xml = pkg.ReadXml("xl/styles.xml");
        if (xml?.Root is null) return formats;

        var numFmts = xml.Root.Elements().FirstOrDefault(e => e.Name.LocalName == "numFmts");
        if (numFmts is not null)
        {
            foreach (var f in numFmts.Elements().Where(e => e.Name.LocalName == "numFmt"))
            {
                if (int.TryParse(f.Attribute("numFmtId")?.Value, NumberStyles.Integer,
                        CultureInfo.InvariantCulture, out int id)
                    && f.Attribute("formatCode")?.Value is { } code)
                    formats.AddCode(id, code);
            }
        }

        var cellXfs = xml.Root.Elements().FirstOrDefault(e => e.Name.LocalName == "cellXfs");
        if (cellXfs is not null)
        {
            foreach (var xf in cellXfs.Elements().Where(e => e.Name.LocalName == "xf"))
            {
                formats.AddCellFormat(
                    int.TryParse(xf.Attribute("numFmtId")?.Value, NumberStyles.Integer,
                        CultureInfo.InvariantCulture, out int id)
                        ? id
                        : 0);
            }
        }

        return formats;
    }

    // ── markdown table (faithful to generate_markdown_and_cells) ───────────────
    internal static string GenerateMarkdown(string name, List<List<string>> cells)
    {
        var sb = new StringBuilder();
        sb.Append("## ").Append(name).Append("\n\n");
        if (cells.Count == 0) return sb.ToString();

        int headerLen = cells[0].Count;
        // Header row
        sb.Append("| ");
        for (int i = 0; i < headerLen; i++)
        {
            if (i > 0) sb.Append(" | ");
            AppendCell(sb, cells[0][i]);
        }
        sb.Append(" |\n");
        // Separator
        sb.Append("| ");
        for (int i = 0; i < headerLen; i++)
        {
            if (i > 0) sb.Append(" | ");
            sb.Append("---");
        }
        sb.Append(" |\n");
        // Body
        for (int r = 1; r < cells.Count; r++)
        {
            sb.Append("| ");
            for (int i = 0; i < headerLen; i++)
            {
                if (i > 0) sb.Append(" | ");
                AppendCell(sb, i < cells[r].Count ? cells[r][i] : "");
            }
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

    // ── metadata (extract_metadata + office metadata merge) ────────────────────
    private static void ExtractMetadata(OoxmlPackage pkg, ExcelWorkbook wb, List<string> sheetNames, bool officeMetadata)
    {
        int sheetCount = sheetNames.Count;
        wb.Metadata["sheet_count"] = sheetCount.ToString(CultureInfo.InvariantCulture);

        string namesStr;
        if (sheetCount <= 5)
            namesStr = string.Join(", ", sheetNames);
        else
            namesStr = string.Join(", ", sheetNames.Take(5)) + $", ... ({sheetCount} total)";
        wb.Metadata["sheet_names"] = namesStr;

        if (!officeMetadata) return;

        var core = OfficeMetadata.ExtractCore(pkg);
        if (core.Title is not null) wb.Metadata["title"] = core.Title;
        if (core.Creator is not null) { wb.Metadata["creator"] = core.Creator; wb.Metadata["created_by"] = core.Creator; }
        if (core.Subject is not null) wb.Metadata["subject"] = core.Subject;
        if (core.Keywords is not null) wb.Metadata["keywords"] = core.Keywords;
        if (core.Description is not null) wb.Metadata["description"] = core.Description;
        if (core.LastModifiedBy is not null) wb.Metadata["modified_by"] = core.LastModifiedBy;
        if (core.Created is not null) wb.Metadata["created_at"] = core.Created;
        if (core.Modified is not null) wb.Metadata["modified_at"] = core.Modified;
        if (core.Revision is not null) wb.Metadata["revision"] = core.Revision;
        if (core.Category is not null) wb.Metadata["category"] = core.Category;
        if (core.ContentStatus is not null) wb.Metadata["content_status"] = core.ContentStatus;
        if (core.Language is not null) wb.Metadata["language"] = core.Language;

        var app = OfficeMetadata.ExtractApp(pkg);
        var worksheetNames = app.TitlesForHeading("worksheet");
        if (worksheetNames.Count > 0) wb.Metadata["worksheet_names"] = string.Join(", ", worksheetNames);
        if (app.Company is not null) wb.Metadata["organization"] = app.Company;
        if (app.Application is not null) wb.Metadata["application"] = app.Application;
        if (app.AppVersion is not null) wb.Metadata["application_version"] = app.AppVersion;
        // #230: surface the raw DocSecurity integer plus its decoded ECMA-376 flags —
        // XlsxAppProperties never reaches the format metadata, so without this the workbook's
        // protection state is discarded entirely.
        if (app.DocSecurity is { } docSecurity)
        {
            wb.Metadata[OfficeMetadata.DocSecurityKey] = docSecurity.ToString(CultureInfo.InvariantCulture);
            foreach (var (key, value) in OfficeMetadata.DecodeDocSecurityFlags(docSecurity))
                wb.Metadata[key] = value ? "true" : "false";
        }

        foreach (var (k, v) in OfficeMetadata.ExtractCustom(pkg))
            wb.Metadata["custom_" + k] = v.ValueKind == System.Text.Json.JsonValueKind.String ? v.GetString() ?? "" : v.GetRawText();
    }
}
