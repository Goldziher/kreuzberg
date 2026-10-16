// CMap parser (ISO 32000-1 §9.7.5 / §9.10): ToUnicode CMaps (bfchar/bfrange) and
// CID CMaps (cidchar/cidrange, codespacerange). Supports predefined Identity-H/V.
using System.Text;

namespace XRay.Content.Internal.Pdf;

public sealed class PdfCMap
{
    public sealed class CodeSpace { public int Low; public int High; public int Bytes; }
    public readonly List<CodeSpace> CodeSpaces = new();

    // ToUnicode: code -> string; ranges: (lo, hi, first-string)
    private readonly Dictionary<int, string> _uniSingle = new();
    private readonly List<(int lo, int hi, string dst, int dstBytes)> _uniRange = new();

    // CID: code -> cid; ranges
    private readonly Dictionary<int, int> _cidSingle = new();
    private readonly List<(int lo, int hi, int cid)> _cidRange = new();

    public bool IsIdentity { get; private set; }

    public static PdfCMap Identity(int bytes = 2)
    {
        var m = new PdfCMap { IsIdentity = true };
        m.CodeSpaces.Add(new CodeSpace { Low = 0, High = bytes == 2 ? 0xFFFF : 0xFF, Bytes = bytes });
        return m;
    }

    public static PdfCMap ParseToUnicode(byte[] data) => Parse(data, unicode: true);
    public static PdfCMap ParseCid(byte[] data) => Parse(data, unicode: false);

    private static PdfCMap Parse(byte[] data, bool unicode)
    {
        var m = new PdfCMap();
        var lex = new PdfLexer(data, 0, null);
        var stack = new List<PdfObject>();
        while (lex.Pos < lex.Length)
        {
            Cancel.ThrowIfRequested();
            lex.SkipWhitespace();
            if (lex.Pos >= lex.Length) break;
            byte b = lex.Buffer[lex.Pos];
            if (b == (byte)'<' || b == (byte)'(' || b == (byte)'[' || b == (byte)'/' ||
                (b >= (byte)'0' && b <= (byte)'9') || b == (byte)'-' || b == (byte)'.' || b == (byte)'+')
            {
                int before = lex.Pos;
                var obj = lex.ParseObject();
                if (lex.Pos == before) { lex.Pos++; continue; }
                stack.Add(obj);
                if (stack.Count > 4000) stack.RemoveRange(0, stack.Count - 100);
                continue;
            }
            string? tok = lex.ReadToken();
            if (tok == null) break;
            switch (tok)
            {
                case "begincodespacerange":
                    ReadCodeSpace(lex, m);
                    break;
                case "beginbfchar":
                    ReadBfChar(lex, m);
                    break;
                case "beginbfrange":
                    ReadBfRange(lex, m);
                    break;
                case "begincidchar":
                    ReadCidChar(lex, m);
                    break;
                case "begincidrange":
                    ReadCidRange(lex, m);
                    break;
                case "usecmap":
                    // Predefined base CMaps not chased; Identity handled by name at font level.
                    break;
                default:
                    if (stack.Count > 200) stack.Clear();
                    break;
            }
        }
        return m;
    }

    private static int BytesToInt(byte[] bytes)
    {
        int v = 0;
        foreach (var b in bytes) v = (v << 8) | b;
        return v;
    }

    private static void ReadCodeSpace(PdfLexer lex, PdfCMap m)
    {
        while (true)
        {
            lex.SkipWhitespace();
            if (PeekToken(lex, "endcodespacerange")) { lex.ReadToken(); break; }
            var lo = lex.ParseObject() as PdfString;
            var hi = lex.ParseObject() as PdfString;
            if (lo == null || hi == null) break;
            m.CodeSpaces.Add(new CodeSpace { Low = BytesToInt(lo.Bytes), High = BytesToInt(hi.Bytes), Bytes = lo.Bytes.Length });
        }
    }

    private static void ReadBfChar(PdfLexer lex, PdfCMap m)
    {
        while (true)
        {
            lex.SkipWhitespace();
            if (PeekToken(lex, "endbfchar")) { lex.ReadToken(); break; }
            var src = lex.ParseObject() as PdfString;
            var dst = lex.ParseObject();
            if (src == null) break;
            int code = BytesToInt(src.Bytes);
            m._uniSingle[code] = DstToString(dst);
        }
    }

    private static void ReadBfRange(PdfLexer lex, PdfCMap m)
    {
        while (true)
        {
            lex.SkipWhitespace();
            if (PeekToken(lex, "endbfrange")) { lex.ReadToken(); break; }
            var lo = lex.ParseObject() as PdfString;
            var hi = lex.ParseObject() as PdfString;
            var dst = lex.ParseObject();
            if (lo == null || hi == null) break;
            int lc = BytesToInt(lo.Bytes), hc = BytesToInt(hi.Bytes);
            if (dst is PdfArray arr)
            {
                for (int i = 0; i < arr.Items.Count && lc + i <= hc; i++)
                    m._uniSingle[lc + i] = DstToString(arr.Items[i]);
            }
            else if (dst is PdfString ds)
            {
                m._uniRange.Add((lc, hc, DstToString(ds), ds.Bytes.Length));
            }
        }
    }

    private static void ReadCidChar(PdfLexer lex, PdfCMap m)
    {
        while (true)
        {
            lex.SkipWhitespace();
            if (PeekToken(lex, "endcidchar")) { lex.ReadToken(); break; }
            var src = lex.ParseObject() as PdfString;
            var cid = lex.ParseObject();
            if (src == null) break;
            m._cidSingle[BytesToInt(src.Bytes)] = (int)(cid.AsLong() ?? 0);
        }
    }

    private static void ReadCidRange(PdfLexer lex, PdfCMap m)
    {
        while (true)
        {
            lex.SkipWhitespace();
            if (PeekToken(lex, "endcidrange")) { lex.ReadToken(); break; }
            var lo = lex.ParseObject() as PdfString;
            var hi = lex.ParseObject() as PdfString;
            var cid = lex.ParseObject();
            if (lo == null || hi == null) break;
            m._cidRange.Add((BytesToInt(lo.Bytes), BytesToInt(hi.Bytes), (int)(cid.AsLong() ?? 0)));
        }
    }

    private static bool PeekToken(PdfLexer lex, string tok)
    {
        int save = lex.Pos;
        lex.SkipWhitespace();
        if (lex.Pos >= lex.Length) { lex.Pos = save; return false; }
        byte b = lex.Buffer[lex.Pos];
        if (b == (byte)'e')
        {
            int p = lex.Pos;
            bool m = true;
            for (int i = 0; i < tok.Length; i++) if (p + i >= lex.Length || lex.Buffer[p + i] != (byte)tok[i]) { m = false; break; }
            lex.Pos = save;
            return m;
        }
        lex.Pos = save;
        return false;
    }

    private static string DstToString(PdfObject dst)
    {
        if (dst is PdfString s)
        {
            // Interpret as UTF-16BE.
            if (s.Bytes.Length % 2 == 0 && s.Bytes.Length >= 2)
            {
                var sb = new StringBuilder(s.Bytes.Length / 2);
                for (int i = 0; i + 1 < s.Bytes.Length; i += 2)
                    sb.Append((char)((s.Bytes[i] << 8) | s.Bytes[i + 1]));
                return sb.ToString();
            }
            if (s.Bytes.Length == 1) return ((char)s.Bytes[0]).ToString();
        }
        else if (dst is PdfName n)
        {
            return PdfEncodings.GlyphNameToUnicode(n.Value);
        }
        else if (dst is PdfNumber num)
        {
            return ((char)(int)num.Value).ToString();
        }
        return "";
    }

    /// <summary>Number of bytes for the code starting at position in the byte stream.</summary>
    public int MatchCodeLength(byte[] data, int pos)
    {
        if (IsIdentity) return 2;
        // Try each codespace by byte length; pick first that contains the prefix.
        // Prefer shortest match per spec ordering.
        foreach (var cs in CodeSpaces)
        {
            if (pos + cs.Bytes > data.Length) continue;
            int code = 0;
            for (int i = 0; i < cs.Bytes; i++) code = (code << 8) | data[pos + i];
            if (code >= cs.Low && code <= cs.High) return cs.Bytes;
        }
        // Fallback: shortest declared, else 1.
        int min = int.MaxValue;
        foreach (var cs in CodeSpaces) min = Math.Min(min, cs.Bytes);
        return min == int.MaxValue ? 1 : min;
    }

    public string? LookupUnicode(int code)
    {
        if (_uniSingle.TryGetValue(code, out var s)) return s;
        foreach (var (lo, hi, dst, dstBytes) in _uniRange)
        {
            if (code >= lo && code <= hi)
            {
                if (dst.Length == 0) return "";
                // Increment last UTF-16 unit by (code - lo).
                var chars = dst.ToCharArray();
                int delta = code - lo;
                int last = chars[^1] + delta;
                chars[^1] = (char)(last & 0xFFFF);
                return new string(chars);
            }
        }
        return null;
    }

    public int? LookupCid(int code)
    {
        if (IsIdentity) return code;
        if (_cidSingle.TryGetValue(code, out var c)) return c;
        foreach (var (lo, hi, cid) in _cidRange)
            if (code >= lo && code <= hi) return cid + (code - lo);
        return null;
    }
}
