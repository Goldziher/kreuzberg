using System.Globalization;
using System.Text;
using XRay.Content.Internal.Cfb;
using XRay.Content.Internal.Excel;

namespace XRay.Content.Internal.Biff;

/// <summary>
/// Minimal BIFF8/BIFF5 (.xls) workbook reader — the OLE branch that calamine handles in the Rust
/// <c>extraction/excel.rs</c>. Reads the "Workbook"/"Book" stream from the CFB container, parses the
/// shared string table and per-sheet cell records (LabelSst / Label / RK / MulRk / Number / BoolErr
/// / Formula+String), and produces a rectangular string grid per sheet bounded by the used-cell
/// range — matching calamine's <c>worksheet_range</c> semantics that the extractor consumes.
/// </summary>
internal static class BiffReader
{
    internal sealed class Sheet
    {
        public string Name = "";
        public long StreamPos;
        public List<List<string>>? Cells;

        /// <summary>
        /// Decoded formula text, on a grid bounded by the cells that carry formulas rather than by
        /// the used range — the two grids do not share an origin.
        /// </summary>
        public List<List<string>>? Formulas;
    }

    private const int BOF = 0x0809;
    private const int EOF = 0x000A;
    private const int BOUNDSHEET8 = 0x0085;
    private const int SST = 0x00FC;
    private const int CONTINUE = 0x003C;
    private const int DIMENSIONS = 0x0200;
    private const int LABELSST = 0x00FD;
    private const int LABEL = 0x0204;
    private const int RSTRING = 0x00D6;
    private const int RK = 0x027E;
    private const int MULRK = 0x00BD;
    private const int NUMBER = 0x0203;
    private const int BLANK = 0x0201;
    private const int MULBLANK = 0x00BE;
    private const int BOOLERR = 0x0205;
    private const int FORMULA = 0x0006;
    private const int STRING_REC = 0x0207;
    private const int NAME = 0x0018;
    private const int FORMAT = 0x041E;
    private const int XF = 0x00E0;
    private const int EXTERNSHEET = 0x0017;

    /// <summary>Reads every sheet's cell grid.</summary>
    /// <param name="comp">The workbook's CFB container.</param>
    /// <param name="numberFormats">
    /// Render numeric cells through the workbook's FORMAT/XF records, so a cell reads the way
    /// Excel shows it. False keeps the bare value calamine hands the Rust original — see
    /// "Deviation: formatted Excel cells" in <c>CLAUDE.md</c>.
    /// </param>
    public static List<Sheet> ReadSheets(CompoundFile comp, bool numberFormats = true)
    {
        byte[] wb = comp.TryReadStream("/Workbook") ?? comp.TryReadStream("/Book")
            ?? throw new InvalidDataException("No Workbook/Book stream in XLS");

        var (sst, sheets, ctx, formats) = ParseGlobals(wb);
        if (!numberFormats) formats = ExcelCellFormats.Empty;
        foreach (var sheet in sheets)
        {
            var (cells, formulas) = ParseSheet(wb, (int)sheet.StreamPos, sst, ctx, formats);
            sheet.Cells = cells;
            sheet.Formulas = formulas;
        }
        return sheets;
    }

    // ── globals substream: BoundSheet + SST ─────────────────────────────────────
    private static (List<string> Sst, List<Sheet> Sheets, BiffFormulaContext Ctx, ExcelCellFormats Formats)
        ParseGlobals(byte[] wb)
    {
        var sheets = new List<Sheet>();
        var sst = new List<string>();
        var ctx = new BiffFormulaContext();
        var formats = new ExcelCellFormats();
        int pos = 0;
        while (pos + 4 <= wb.Length)
        {
            Cancel.ThrowIfRequested();
            int type = U16(wb, pos);
            int len = U16(wb, pos + 2);
            int dataStart = pos + 4;
            if (dataStart + len > wb.Length) break;

            if (type == BOF)
            {
                ctx.Biff = BiffVersionOf(wb, dataStart, len);
                pos = dataStart + len;
            }
            else if (type == BOUNDSHEET8)
            {
                long lbPlyPos = U32(wb, dataStart);
                var s = new Sheet { StreamPos = lbPlyPos, Name = ReadShortXlString(wb, dataStart + 6) };
                sheets.Add(s);
                ctx.SheetNames.Add(s.Name);
                pos = dataStart + len;
            }
            else if (type == NAME)
            {
                // Lbl: only the name itself is referenced by formulas, not its own expression.
                ctx.DefinedNames.Add(ReadDefinedName(wb, dataStart, len, ctx.Biff));
                pos = dataStart + len;
            }
            else if (type == EXTERNSHEET)
            {
                ParseExternSheet(wb, dataStart, len, ctx);
                pos = dataStart + len;
            }
            else if (type == FORMAT)
            {
                // Format [MS-XLS 2.4.126]: the id cell formats cite, then the code itself —
                // an XLUnicodeString in BIFF8 and a byte-counted one before it.
                int ifmt = U16(wb, dataStart);
                formats.AddCode(ifmt, ctx.IsPreBiff8
                    ? ReadByteCountedString(wb, dataStart + 2)
                    : ReadXlString(wb, dataStart + 2));
                pos = dataStart + len;
            }
            else if (type == XF)
            {
                // XF [MS-XLS 2.4.353]: font index, then the number format id. Style and cell
                // records share one list, which is what a cell's ixfe indexes into.
                formats.AddCellFormat(U16(wb, dataStart + 2));
                pos = dataStart + len;
            }
            else if (type == SST)
            {
                // Collect SST data + following CONTINUE records into one logical buffer.
                var segments = new List<(int Start, int Len)> { (dataStart, len) };
                int scan = dataStart + len;
                while (scan + 4 <= wb.Length && U16(wb, scan) == CONTINUE)
                {
                    int clen = U16(wb, scan + 2);
                    if (scan + 4 + clen > wb.Length) break;
                    segments.Add((scan + 4, clen));
                    scan += 4 + clen;
                }
                ParseSst(wb, segments, sst);
                pos = scan;
            }
            else if (type == EOF)
            {
                pos = dataStart + len;
                break;
            }
            else
            {
                pos = dataStart + len;
            }
        }

        // Before BIFF8 the formula tokens carry sheet indices directly, so give them an
        // identity XTI table to look through.
        if (ctx.IsPreBiff8 && ctx.XtiItabFirst.Count == 0)
        {
            for (int i = 0; i < sheets.Count; i++) ctx.XtiItabFirst.Add((short)i);
        }
        return (sst, sheets, ctx, formats);
    }

    /// <summary>BOF [MS-XLS 2.4.21]: the version word, with the document type as a tie-breaker.</summary>
    private static BiffVersion BiffVersionOf(byte[] wb, int dataStart, int len)
    {
        int version = len >= 2 ? U16(wb, dataStart) : 0;
        int dt = len >= 4 ? U16(wb, dataStart + 2) : 0;
        return version switch
        {
            0x0200 or 0x0002 or 0x0007 => BiffVersion.Biff2,
            0x0300 => BiffVersion.Biff3,
            0x0400 => BiffVersion.Biff4,
            0x0500 => BiffVersion.Biff5,
            0x0600 => BiffVersion.Biff8,
            0 => dt == 0x1000 ? BiffVersion.Biff5 : BiffVersion.Biff8,
            _ => BiffVersion.Biff8,
        };
    }

    /// <summary>Lbl [MS-XLS 2.4.150]: a defined name, whose text starts at a fixed offset.</summary>
    private static string ReadDefinedName(byte[] wb, int dataStart, int len, BiffVersion biff)
    {
        if (len < 15) return "";
        int cch = wb[dataStart + 3];
        int at = dataStart + 14;
        var sb = new StringBuilder(cch);
        if (biff == BiffVersion.Biff8)
        {
            // A flags byte, then cch bytes of text — UTF-16 when the low bit is set.
            bool highByte = (wb[at] & 0x01) != 0;
            int available = Math.Max(0, Math.Min(cch, dataStart + len - (at + 1)));
            int p = at + 1;
            if (highByte)
            {
                for (int i = 0; i + 1 < available; i += 2) sb.Append((char)(ushort)(wb[p + i] | (wb[p + i + 1] << 8)));
            }
            else
            {
                for (int i = 0; i < available; i++) sb.Append((char)wb[p + i]);
            }
        }
        else
        {
            int available = Math.Max(0, Math.Min(cch, dataStart + len - at));
            for (int i = 0; i < available; i++) sb.Append((char)wb[at + i]);
        }
        return sb.ToString();
    }

    /// <summary>ExternSheet [MS-XLS 2.4.106]: the XTI table that BIFF8 3d references index into.</summary>
    private static void ParseExternSheet(byte[] wb, int dataStart, int len, BiffFormulaContext ctx)
    {
        if (ctx.Biff != BiffVersion.Biff8 || len < 2) return;
        int cxti = U16(wb, dataStart);
        for (int i = 0; i < cxti; i++)
        {
            int at = dataStart + 2 + i * 6;
            if (at + 6 > dataStart + len) break;
            ctx.XtiItabFirst.Add((short)U16(wb, at + 2));
        }
    }

    private static void ParseSst(byte[] wb, List<(int Start, int Len)> segments, List<string> sst)
    {
        // Flatten segments into one buffer, tracking boundary offsets for the per-CONTINUE grbit.
        var buf = new List<byte>();
        var boundaries = new HashSet<int>();
        foreach (var (start, len) in segments)
        {
            if (buf.Count > 0) boundaries.Add(buf.Count);
            for (int i = 0; i < len; i++) buf.Add(wb[start + i]);
        }
        byte[] d = buf.ToArray();
        int pos = 0;
        if (d.Length < 8) return;
        pos += 4; // cstTotal
        long cstUnique = (uint)(d[4] | (d[5] << 8) | (d[6] << 16) | (d[7] << 24));
        pos += 4;

        for (long si = 0; si < cstUnique && pos + 3 <= d.Length; si++)
        {
            int cch = d[pos] | (d[pos + 1] << 8); pos += 2;
            byte flags = d[pos]; pos += 1;
            bool highByte = (flags & 0x01) != 0;
            int cRun = 0, cbExt = 0;
            if ((flags & 0x08) != 0) { cRun = d[pos] | (d[pos + 1] << 8); pos += 2; }
            if ((flags & 0x04) != 0) { cbExt = (int)((uint)(d[pos] | (d[pos + 1] << 8) | (d[pos + 2] << 16) | (d[pos + 3] << 24))); pos += 4; }

            var sb = new StringBuilder(cch);
            int read = 0;
            while (read < cch && pos < d.Length)
            {
                if (boundaries.Contains(pos))
                {
                    // A continue boundary in the middle of char data carries a fresh grbit byte.
                    flags = d[pos]; pos += 1;
                    highByte = (flags & 0x01) != 0;
                }
                if (highByte)
                {
                    if (pos + 1 >= d.Length) break;
                    sb.Append((char)(ushort)(d[pos] | (d[pos + 1] << 8)));
                    pos += 2;
                }
                else
                {
                    sb.Append((char)d[pos]);
                    pos += 1;
                }
                read++;
            }
            // Skip rich-text runs (4 bytes each) and extended (phonetic) data.
            pos += cRun * 4;
            pos += cbExt;
            sst.Add(sb.ToString());
        }
    }

    // ── worksheet substream: cells ──────────────────────────────────────────────
    private static (List<List<string>>? Cells, List<List<string>>? Formulas) ParseSheet(
        byte[] wb, int start, List<string> sst, BiffFormulaContext ctx, ExcelCellFormats formats)
    {
        var cells = new List<(int Row, int Col, string Val)>();
        var formulas = new List<(int Row, int Col, string Val)>();
        int pos = start;
        // Expect a BOF at the sheet start.
        bool started = false;
        int depth = 0;
        while (pos + 4 <= wb.Length)
        {
            Cancel.ThrowIfRequested();
            int type = U16(wb, pos);
            int len = U16(wb, pos + 2);
            int d = pos + 4;
            if (d + len > wb.Length) break;

            if (type == BOF)
            {
                depth++;
                started = true;
                pos = d + len;
                continue;
            }
            if (type == EOF)
            {
                depth--;
                pos = d + len;
                if (started && depth <= 0) break;
                continue;
            }

            switch (type)
            {
                case LABELSST:
                {
                    int row = U16(wb, d), col = U16(wb, d + 2);
                    int isst = (int)U32(wb, d + 6);
                    string v = isst >= 0 && isst < sst.Count ? sst[isst] : "";
                    AddCell(cells, row, col, v);
                    break;
                }
                case LABEL:
                case RSTRING:
                {
                    int row = U16(wb, d), col = U16(wb, d + 2);
                    string v = ReadXlString(wb, d + 6);
                    AddCell(cells, row, col, v);
                    break;
                }
                case RK:
                {
                    int row = U16(wb, d), col = U16(wb, d + 2);
                    double v = RkToDouble(U32(wb, d + 6));
                    AddCell(cells, row, col, Render(formats, U16(wb, d + 4), v));
                    break;
                }
                case MULRK:
                {
                    int row = U16(wb, d), colFirst = U16(wb, d + 2);
                    int n = (len - 6) / 6;
                    for (int i = 0; i < n; i++)
                    {
                        // Each RkRec is { ixfe:u16, rk:u32 }; the RK value sits after the ixfe.
                        int rec = d + 4 + i * 6;
                        double v = RkToDouble(U32(wb, rec + 2));
                        AddCell(cells, row, colFirst + i, Render(formats, U16(wb, rec), v));
                    }
                    break;
                }
                case NUMBER:
                {
                    int row = U16(wb, d), col = U16(wb, d + 2);
                    double v = BitConverter.Int64BitsToDouble((long)U64(wb, d + 6));
                    AddCell(cells, row, col, Render(formats, U16(wb, d + 4), v));
                    break;
                }
                case BOOLERR:
                {
                    int row = U16(wb, d), col = U16(wb, d + 2);
                    byte val = wb[d + 6];
                    byte isErr = wb[d + 7];
                    string v = isErr != 0 ? $"#ERR: {val}" : (val != 0 ? "true" : "false");
                    AddCell(cells, row, col, v);
                    break;
                }
                case FORMULA:
                {
                    int row = U16(wb, d), col = U16(wb, d + 2);
                    // The expression follows the 20-byte header. A shared or array formula decodes
                    // to empty text but still occupies its cell, which fixes where the grid starts.
                    formulas.Add((row, col, BiffFormula.Parse(wb, d + 20, d + len, ctx, row, col)));
                    // Cached result: 8 bytes at d+6.
                    if (wb[d + 12] == 0xFF && wb[d + 13] == 0xFF)
                    {
                        byte kind = wb[d + 6];
                        if (kind == 0)
                        {
                            // String result follows in a STRING record.
                            int np = d + len;
                            if (np + 4 <= wb.Length && U16(wb, np) == STRING_REC)
                            {
                                string v = ReadXlString(wb, np + 4);
                                AddCell(cells, row, col, v);
                            }
                        }
                        else if (kind == 1)
                            AddCell(cells, row, col, wb[d + 8] != 0 ? "true" : "false");
                        else if (kind == 2)
                            AddCell(cells, row, col, $"#ERR: {wb[d + 8]}");
                        // kind == 3 → empty string, skip.
                    }
                    else
                    {
                        double v = BitConverter.Int64BitsToDouble((long)U64(wb, d + 6));
                        AddCell(cells, row, col, Render(formats, U16(wb, d + 4), v));
                    }
                    break;
                }
            }
            pos = d + len;
        }

        return (BuildGrid(cells), BuildGrid(formulas));
    }

    /// <summary>
    /// A numeric cell's text: what its format says, and the bare value where the format says
    /// nothing this renders.
    /// </summary>
    private static string Render(ExcelCellFormats formats, int ixfe, double value) =>
        formats.Render(ixfe, value) ?? FormatNumber(value);

    /// <summary>Read a byte-counted, single-byte string, which is how BIFF5 writes a Format
    /// record's code.</summary>
    private static string ReadByteCountedString(byte[] b, int off)
    {
        if (off >= b.Length) return "";
        int cch = b[off];
        var sb = new StringBuilder(cch);
        for (int i = 0; i < cch && off + 1 + i < b.Length; i++) sb.Append((char)b[off + 1 + i]);
        return sb.ToString();
    }

    private static void AddCell(List<(int, int, string)> cells, int row, int col, string val)
    {
        if (val.Length == 0) return; // Empty cells don't extend the used range.
        cells.Add((row, col, val));
    }

    private static List<List<string>>? BuildGrid(List<(int Row, int Col, string Val)> cells)
    {
        if (cells.Count == 0) return null;
        int minRow = int.MaxValue, maxRow = int.MinValue, minCol = int.MaxValue, maxCol = int.MinValue;
        foreach (var (r, c, _) in cells)
        {
            if (r < minRow) minRow = r;
            if (r > maxRow) maxRow = r;
            if (c < minCol) minCol = c;
            if (c > maxCol) maxCol = c;
        }
        int rows = maxRow - minRow + 1;
        int cols = maxCol - minCol + 1;
        var grid = new List<List<string>>(rows);
        for (int r = 0; r < rows; r++)
        {
            var row = new List<string>(cols);
            for (int c = 0; c < cols; c++) row.Add("");
            grid.Add(row);
        }
        foreach (var (r, c, v) in cells)
            grid[r - minRow][c - minCol] = v;
        return grid;
    }

    // ── value helpers ────────────────────────────────────────────────────────────
    internal static double RkToDouble(uint rk)
    {
        bool fx100 = (rk & 0x01) != 0;
        bool fInt = (rk & 0x02) != 0;
        double value;
        if (fInt)
        {
            int signed = (int)rk >> 2; // arithmetic shift keeps sign
            value = signed;
        }
        else
        {
            ulong bits = (ulong)(rk & 0xFFFF_FFFC) << 32;
            value = BitConverter.Int64BitsToDouble((long)bits);
        }
        return fx100 ? value / 100.0 : value;
    }

    /// <summary>Format a numeric cell the way Rust's <c>format!("{}", f64)</c> does: shortest
    /// round-trippable representation, whole numbers without a trailing decimal.</summary>
    internal static string FormatNumber(double v)
    {
        if (double.IsNaN(v)) return "NaN";
        if (double.IsPositiveInfinity(v)) return "inf";
        if (double.IsNegativeInfinity(v)) return "-inf";
        // .NET "R"/default (Core 3+) gives shortest round-trip; whole values render without ".0".
        return v.ToString("R", CultureInfo.InvariantCulture);
    }

    /// <summary>Read an XLUnicodeString (u16 char-count + flags byte + chars).</summary>
    private static string ReadXlString(byte[] b, int off)
    {
        if (off + 3 > b.Length) return "";
        int cch = U16(b, off);
        byte flags = b[off + 2];
        bool highByte = (flags & 0x01) != 0;
        int p = off + 3;
        var sb = new StringBuilder(cch);
        for (int i = 0; i < cch; i++)
        {
            if (highByte) { if (p + 1 >= b.Length) break; sb.Append((char)(ushort)(b[p] | (b[p + 1] << 8))); p += 2; }
            else { if (p >= b.Length) break; sb.Append((char)b[p]); p += 1; }
        }
        return sb.ToString();
    }

    /// <summary>Read a ShortXLUnicodeString (u8 char-count + flags byte + chars), used by BoundSheet8.</summary>
    private static string ReadShortXlString(byte[] b, int off)
    {
        if (off + 2 > b.Length) return "";
        int cch = b[off];
        byte flags = b[off + 1];
        bool highByte = (flags & 0x01) != 0;
        int p = off + 2;
        var sb = new StringBuilder(cch);
        for (int i = 0; i < cch; i++)
        {
            if (highByte) { if (p + 1 >= b.Length) break; sb.Append((char)(ushort)(b[p] | (b[p + 1] << 8))); p += 2; }
            else { if (p >= b.Length) break; sb.Append((char)b[p]); p += 1; }
        }
        return sb.ToString();
    }

    private static int U16(byte[] b, int o) => o + 1 < b.Length ? b[o] | (b[o + 1] << 8) : 0;
    private static uint U32(byte[] b, int o) =>
        o + 3 < b.Length ? (uint)(b[o] | (b[o + 1] << 8) | (b[o + 2] << 16) | (b[o + 3] << 24)) : 0u;
    private static ulong U64(byte[] b, int o) => U32(b, o) | ((ulong)U32(b, o + 4) << 32);
}
