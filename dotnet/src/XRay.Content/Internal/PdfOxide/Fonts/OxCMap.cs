// Port of pdf_oxide 0.3.77 `src/fonts/cmap.rs`
// (CMap, RangeEntry, CMapKey, the global CMap cache, LazyCMap, parse_tounicode_cmap,
//  parse_escape_sequence, decode_utf16_surrogate_pair, parse_wmode_directive,
//  parse_codespacerange_line_width, parse_bfchar_line, parse_bfrange_line,
//  parse_notdefrange_line, parse_cid_to_unicode).
//
// A /ToUnicode CMap stream (ISO 32000-1 §9.7.5, §9.10.3) maps character codes to Unicode.
// Without it, text drawn with a subset/custom encoding cannot be recovered at all.
using System.Text;
using System.Text.RegularExpressions;

namespace XRay.Content.Internal.PdfOxide.Fonts;

/// <summary>A character map from character codes to Unicode strings.</summary>
/// <remarks>
/// Two-tier storage: <c>_chars</c> holds individual mappings (O(1)), <c>_ranges</c> holds
/// contiguous runs collapsed out of <c>_chars</c> and is binary-searched (O(log n)). Codes are
/// <see cref="uint"/> because CID fonts use multi-byte codes.
/// </remarks>
internal sealed partial class OxCMap
{
    private readonly struct RangeEntry(uint start, uint end, uint target)
    {
        internal readonly uint Start = start;
        internal readonly uint End = end;
        internal readonly uint Target = target;
    }

    private readonly Dictionary<uint, string> _chars = new();
    private readonly List<RangeEntry> _ranges = new();

    // Upstream declares notdef ranges as a third lookup tier but never appends to the list:
    // `beginnotdefrange` entries are expanded straight into `_chars`. Kept so `Get`, `Count` and
    // `IsEmpty` match the Rust definitions exactly.
    private readonly List<RangeEntry> _notdefRanges = new();

    /// <summary>
    /// Maximum character code width in bytes, from <c>begincodespacerange</c>: 1 for simple fonts,
    /// 2 for CJK composite fonts and Identity-H.
    /// </summary>
    /// <remarks>
    /// The content-stream reader needs this to decide how many bytes to consume per character.
    /// Without it, any CJK ToUnicode CMap that does not use a well-known encoding name would be
    /// read one byte at a time, splitting every 2-byte CID into two wrong codes.
    /// </remarks>
    internal byte CodeWidth { get; private set; } = 1;

    /// <summary>Writing mode from <c>/WMode &lt;int&gt; def</c>: 0 horizontal (default), 1 vertical.</summary>
    /// <remarks>
    /// This is the authoritative signal for *embedded* CMap streams, which may carry /WMode 1 even
    /// when their /CMapName does not advertise a `-V` suffix. Predefined CMaps whose names end in
    /// `-V` are detected separately from the encoding name.
    /// </remarks>
    internal byte WMode { get; private set; }

    internal bool IsEmpty => _chars.Count == 0 && _ranges.Count == 0 && _notdefRanges.Count == 0;

    internal int Count => _chars.Count + _ranges.Count + _notdefRanges.Count;

    // Exposed for tests that assert the range-compression pass actually fired.
    internal int CharCount => _chars.Count;
    internal int RangeCount => _ranges.Count;

    /// <summary>Unicode string for a character code, or null when unmapped.</summary>
    /// <remarks>
    /// <c>_chars</c> is checked first because it holds the document-order-correct value for any code
    /// a later bfchar redefined (§9.10.3); <c>_ranges</c> only ever holds runs that were still
    /// contiguous in the final <c>_chars</c> state.
    /// </remarks>
    internal string? Get(uint code)
    {
        if (_chars.TryGetValue(code, out string? s)) return s;

        int lo = 0, hi = _ranges.Count - 1;
        while (lo <= hi)
        {
            int mid = lo + ((hi - lo) >> 1);
            RangeEntry r = _ranges[mid];
            if (r.End < code) lo = mid + 1;
            else if (r.Start > code) hi = mid - 1;
            else
            {
                uint cp = unchecked(r.Target + (code - r.Start));
                return TryCharFromU32(cp, out string? ch) ? ch : null;
            }
        }

        foreach (RangeEntry range in _notdefRanges)
        {
            if (range.Start <= code && code <= range.End && _chars.TryGetValue(range.Target, out string? fallback))
                return fallback;
        }

        return null;
    }

    private void Insert(uint code, string unicode) => _chars[code] = unicode;

    /// <summary>
    /// Collapse long contiguous runs out of <c>_chars</c> into <c>_ranges</c>.
    /// </summary>
    /// <remarks>
    /// A `&lt;0000&gt; &lt;FFFF&gt;` bfrange otherwise materialises tens of thousands of strings that
    /// stay resident for the life of the cached CMap. Operating on the *final* <c>_chars</c> state
    /// makes this semantics-preserving: a redefined code already holds its winning value, and a
    /// redefinition breaks contiguity so it stays in <c>_chars</c>. Multi-char (ligature) values and
    /// notdef-range targets are never collapsed.
    /// </remarks>
    private void CompressSequentialRanges()
    {
        const int MinRun = 256;

        HashSet<uint> notdefTargets = new();
        foreach (RangeEntry r in _notdefRanges) notdefTargets.Add(r.Target);

        List<(uint Code, uint Cp)> singles = new();
        foreach (KeyValuePair<uint, string> kv in _chars)
        {
            if (notdefTargets.Contains(kv.Key)) continue;
            if (TrySingleScalar(kv.Value, out uint cp)) singles.Add((kv.Key, cp));
        }
        if (singles.Count < MinRun) return;
        singles.Sort((a, b) => a.Code.CompareTo(b.Code));

        int i = 0;
        while (i < singles.Count)
        {
            int j = i;
            while (j + 1 < singles.Count
                   && singles[j + 1].Code == singles[j].Code + 1
                   && singles[j + 1].Cp == singles[j].Cp + 1)
            {
                j++;
            }
            if (j - i + 1 >= MinRun)
            {
                _ranges.Add(new RangeEntry(singles[i].Code, singles[j].Code, singles[i].Cp));
                for (int k = i; k <= j; k++) _chars.Remove(singles[k].Code);
            }
            i = j + 1;
        }
        _ranges.Sort((a, b) => a.Start.CompareTo(b.Start));
    }

    /// <summary>True when <paramref name="s"/> is exactly one Unicode scalar, yielding its value.</summary>
    private static bool TrySingleScalar(string s, out uint cp)
    {
        cp = 0;
        if (s.Length == 1 && !char.IsSurrogate(s[0])) { cp = s[0]; return true; }
        if (s.Length == 2 && char.IsHighSurrogate(s[0]) && char.IsLowSurrogate(s[1]))
        {
            cp = (uint)char.ConvertToUtf32(s[0], s[1]);
            return true;
        }
        return false;
    }

    /// <summary>Rust `char::from_u32`: rejects lone surrogates and anything above U+10FFFF.</summary>
    private static bool TryCharFromU32(uint cp, out string? s)
    {
        if (cp > 0x10FFFF || (cp >= 0xD800 && cp <= 0xDFFF)) { s = null; return false; }
        s = char.ConvertFromUtf32((int)cp);
        return true;
    }

    /// <summary>
    /// Rust `u32::from_str_radix(s, 16)`: rejects an empty string, any non-hex digit, and overflow.
    /// </summary>
    private static bool TryParseHex(string s, out uint value)
    {
        value = 0;
        if (s.Length == 0 || s.Length > 8) return false;
        uint v = 0;
        foreach (char c in s)
        {
            int d;
            if (c >= '0' && c <= '9') d = c - '0';
            else if (c >= 'a' && c <= 'f') d = c - 'a' + 10;
            else if (c >= 'A' && c <= 'F') d = c - 'A' + 10;
            else return false;
            v = (v << 4) | (uint)d;
        }
        value = v;
        return true;
    }

    private static string StripWhitespace(string s)
    {
        int i = 0;
        while (i < s.Length && !char.IsWhiteSpace(s[i])) i++;
        if (i == s.Length) return s;
        StringBuilder sb = new(s.Length);
        sb.Append(s, 0, i);
        for (; i < s.Length; i++)
        {
            if (!char.IsWhiteSpace(s[i])) sb.Append(s[i]);
        }
        return sb.ToString();
    }

    /// <summary>Rust `str::lines()`: split on '\n', dropping one trailing '\r'.</summary>
    private static IEnumerable<string> Lines(string s, int start, int end)
    {
        int i = start;
        while (i < end)
        {
            int nl = s.IndexOf('\n', i, end - i);
            int stop = nl < 0 ? end : nl;
            int trimmed = (stop > i && s[stop - 1] == '\r') ? stop - 1 : stop;
            yield return s.Substring(i, trimmed - i);
            if (nl < 0) yield break;
            i = nl + 1;
        }
    }

    /// <summary>
    /// Symbolic destination names some producers emit instead of hex (`&lt;space&gt;`, `&lt;tab&gt;`).
    /// </summary>
    private static string? ParseEscapeSequence(string token)
    {
        token = token.Trim();
        if (token.Length >= 2 && token[0] == '<' && token[^1] == '>')
            token = token.Substring(1, token.Length - 2);

        return token.ToLowerInvariant().Trim() switch
        {
            "space" => " ",
            "tab" => "\t",
            "newline" => "\n",
            "carriage return" => "\r",
            _ => null,
        };
    }

    /// <summary>
    /// Decode a code point &gt; U+FFFF written as an 8-hex-digit UTF-16 surrogate pair
    /// (e.g. D835DF0C -> U+1D70C), falling back to treating the value as a direct code point.
    /// </summary>
    private static string? DecodeUtf16SurrogatePair(uint value)
    {
        ushort high = (ushort)(value >> 16);
        ushort low = (ushort)(value & 0xFFFF);

        if (high >= 0xD800 && high <= 0xDBFF && low >= 0xDC00 && low <= 0xDFFF)
        {
            uint codepoint = 0x10000u + ((uint)(high & 0x3FF) << 10) + (uint)(low & 0x3FF);
            return TryCharFromU32(codepoint, out string? ch) ? ch : null;
        }

        return TryCharFromU32(value, out string? direct) ? direct : null;
    }

    [GeneratedRegex(@"/WMode\s+([0-9]+)\s+def")]
    private static partial Regex WModeRegex();

    [GeneratedRegex(@"<([^>]*)>\s*<([^>]*)>")]
    private static partial Regex PairRegex();

    [GeneratedRegex(@"<([^>]*)>\s*<([^>]*)>\s*<([^>]*)>")]
    private static partial Regex SeqRegex();

    [GeneratedRegex(@"<([^>]*)>\s*<([^>]*)>\s*\[((?:\s*<[^>]+>\s*)+)\]")]
    private static partial Regex ArrayRegex();

    [GeneratedRegex(@"<([^>]*)>")]
    private static partial Regex HexTokenRegex();

    /// <summary>Parse a ToUnicode CMap stream. Never fails; an unparseable stream yields an empty map.</summary>
    internal static OxCMap ParseToUnicodeCMap(byte[] data)
    {
        OxCMap cmap = new();
        string content = Encoding.UTF8.GetString(data);

        // `/WMode N def` sits at the top level of the stream, outside any begin…end block, so a
        // lexical scan is enough and avoids a second tokenizer pass.
        byte? parsedWMode = ParseWModeDirective(content);
        if (parsedWMode.HasValue) cmap.WMode = parsedWMode.Value;

        foreach ((int s, int e) in ExtractSections(content, "begincodespacerange", "endcodespacerange"))
        {
            foreach (string line in Lines(content, s, e))
            {
                Cancel.ThrowIfRequested();
                byte width = ParseCodespaceRangeLineWidth(line);
                if (width > cmap.CodeWidth) cmap.CodeWidth = width;
            }
        }

        // bfchar and bfrange sections are walked in document order so later entries overwrite
        // earlier ones for the same code (§9.10.3) — the last-wins semantics pdf.js, MuPDF and
        // Poppler all implement.
        foreach ((bool isRange, int s, int e) in BfSectionsInDocumentOrder(content))
        {
            foreach (string line in Lines(content, s, e))
            {
                Cancel.ThrowIfRequested();
                if (isRange)
                {
                    List<(uint Src, string Dst)>? mappings = ParseBfRangeLine(line);
                    if (mappings == null) continue;
                    foreach ((uint src, string dst) in mappings) cmap.Insert(src, dst);
                }
                else
                {
                    foreach ((uint src, string dst) in ParseBfCharLine(line)) cmap.Insert(src, dst);
                }
            }
        }

        foreach ((int s, int e) in ExtractSections(content, "beginnotdefrange", "endnotdefrange"))
        {
            foreach (string line in Lines(content, s, e))
            {
                Cancel.ThrowIfRequested();
                List<(uint Src, string Dst)>? mappings = ParseNotdefRangeLine(line);
                if (mappings == null) continue;
                foreach ((uint src, string dst) in mappings)
                {
                    // notdef is a fallback for *unmapped* codes only; normal mappings win.
                    if (!cmap._chars.ContainsKey(src)) cmap.Insert(src, dst);
                }
            }
        }

        cmap.CompressSequentialRanges();
        return cmap;
    }

    /// <summary>CID-to-Unicode CMaps use the same grammar as ToUnicode CMaps.</summary>
    internal static OxCMap ParseCidToUnicode(byte[] data) => ParseToUnicodeCMap(data);

    /// <summary>
    /// Yield <c>beginbfchar</c> / <c>beginbfrange</c> section bodies as (isRange, start, end) in
    /// stream order.
    /// </summary>
    private static IEnumerable<(bool IsRange, int Start, int End)> BfSectionsInDocumentOrder(string content)
    {
        int remaining = 0;
        while (true)
        {
            int pos = content.IndexOf("beginbf", remaining, StringComparison.Ordinal);
            if (pos < 0) yield break;
            int after = pos + "beginbf".Length;

            if (content.AsSpan(after).StartsWith("char", StringComparison.Ordinal))
            {
                int body = after + 4;
                int end = content.IndexOf("endbfchar", body, StringComparison.Ordinal);
                if (end >= 0)
                {
                    remaining = end + "endbfchar".Length;
                    yield return (false, body, end);
                    continue;
                }
            }
            else if (content.AsSpan(after).StartsWith("range", StringComparison.Ordinal))
            {
                int body = after + 5;
                int end = content.IndexOf("endbfrange", body, StringComparison.Ordinal);
                if (end >= 0)
                {
                    remaining = end + "endbfrange".Length;
                    yield return (true, body, end);
                    continue;
                }
            }

            // Unrecognised "beginbf…" token or missing end marker; skip past it.
            remaining = after;
        }
    }

    /// <summary>Section bodies between <paramref name="begin"/> and <paramref name="end"/> markers.</summary>
    private static List<(int Start, int End)> ExtractSections(string content, string begin, string end)
    {
        List<(int, int)> sections = new();
        int remaining = 0;
        while (true)
        {
            int beginPos = content.IndexOf(begin, remaining, StringComparison.Ordinal);
            if (beginPos < 0) break;
            int afterBegin = beginPos + begin.Length;
            int endPos = content.IndexOf(end, afterBegin, StringComparison.Ordinal);
            if (endPos < 0) break;
            sections.Add((afterBegin, endPos));
            remaining = endPos + end.Length;
        }
        return sections;
    }

    /// <summary>
    /// Parse <c>/WMode N def</c>: 0 horizontal, 1 vertical, null when absent or non-spec.
    /// </summary>
    /// <remarks>
    /// PostScript comments run from '%' to end of line, so a commented-out `% /WMode 1 def` must not
    /// flip the writing mode. Newlines are preserved so a legitimate directive on a later line is
    /// still matched. Values other than 0/1 are undefined by §9.7.5.4 and fall back to horizontal.
    /// </remarks>
    internal static byte? ParseWModeDirective(string content)
    {
        StringBuilder cleaned = new(content.Length);
        bool first = true;
        foreach (string line in Lines(content, 0, content.Length))
        {
            Cancel.ThrowIfRequested();
            if (!first) cleaned.Append('\n');
            first = false;
            int idx = line.IndexOf('%');
            cleaned.Append(idx >= 0 ? line.Substring(0, idx) : line);
        }

        Match m = WModeRegex().Match(cleaned.ToString());
        if (!m.Success) return null;
        if (!uint.TryParse(m.Groups[1].Value, out uint value)) return null;
        return value switch { 0 => (byte)0, 1 => (byte)1, _ => null };
    }

    /// <summary>
    /// Maximum code byte-width on a <c>begincodespacerange</c> line: 2 hex digits is a 1-byte code,
    /// 4 or more is a 2-byte code.
    /// </summary>
    private static byte ParseCodespaceRangeLineWidth(string line)
    {
        byte maxWidth = 1;
        foreach (Match m in PairRegex().Matches(line))
        {
            string loHex = StripWhitespace(m.Groups[1].Value.Trim());
            string hiHex = StripWhitespace(m.Groups[2].Value.Trim());
            if (loHex.Length >= 4 || hiHex.Length >= 4) maxWidth = 2;
        }
        return maxWidth;
    }

    /// <summary>
    /// Parse a bfchar line, returning every <c>&lt;src&gt; &lt;dst&gt;</c> pair on it. A pair that
    /// fails to parse is dropped without affecting the rest of the line.
    /// </summary>
    private static List<(uint Src, string Dst)> ParseBfCharLine(string line)
    {
        List<(uint, string)> results = new();

        foreach (Match m in PairRegex().Matches(line))
        {
            string srcStr = StripWhitespace(m.Groups[1].Value.Trim());
            if (!TryParseHex(srcStr, out uint src)) continue;

            string dstStr = m.Groups[2].Value.Trim();
            string? escape = ParseEscapeSequence("<" + dstStr + ">");
            string dst;
            if (escape != null)
            {
                dst = escape;
            }
            else
            {
                string dstHex = StripWhitespace(dstStr);
                if (dstHex.Length <= 6)
                {
                    // <=4 digits is a BMP code point; 5-6 digits is a direct supplementary one
                    // (e.g. 020BB7 = U+20BB7).
                    if (!TryParseHex(dstHex, out uint dstCode)) continue;
                    if (!TryCharFromU32(dstCode, out string? ch)) continue;
                    dst = ch!;
                }
                else if (dstHex.Length == 8)
                {
                    if (!TryParseHex(dstHex, out uint dstCode)) continue;
                    string? decoded = DecodeUtf16SurrogatePair(dstCode);
                    // Not a surrogate pair — fall back to two BMP characters (a ligature).
                    dst = decoded ?? ConcatBmpChunks(dstHex);
                    if (dst.Length == 0) continue;
                }
                else
                {
                    dst = ConcatBmpChunks(dstHex);
                    if (dst.Length == 0) continue;
                }
            }

            results.Add((src, dst));
        }

        return results;
    }

    /// <summary>Split a hex string into 4-digit chunks, each one BMP code point (ligature form).</summary>
    private static string ConcatBmpChunks(string dstHex)
    {
        StringBuilder sb = new();
        for (int i = 0; i < dstHex.Length; i += 4)
        {
            int end = Math.Min(i + 4, dstHex.Length);
            if (TryParseHex(dstHex.Substring(i, end - i), out uint code)
                && TryCharFromU32(code, out string? ch))
            {
                sb.Append(ch);
            }
        }
        return sb.ToString();
    }

    private enum DstOutcome { Ok, Skip, Abort }

    /// <summary>
    /// Decode one bfrange array destination. <see cref="DstOutcome.Skip"/> drops just this entry;
    /// <see cref="DstOutcome.Abort"/> discards every mapping on the line, matching upstream's `?`
    /// propagation out of the whole line parser.
    /// </summary>
    private static DstOutcome DecodeArrayDestination(string dstHex, out string dst)
    {
        dst = "";
        if (dstHex.Length <= 4)
        {
            if (!TryParseHex(dstHex, out uint code)) return DstOutcome.Abort;
            if (!TryCharFromU32(code, out string? ch)) return DstOutcome.Abort;
            dst = ch!;
            return DstOutcome.Ok;
        }
        if (dstHex.Length <= 6)
        {
            if (!TryParseHex(dstHex, out uint code)) return DstOutcome.Abort;
            if (!TryCharFromU32(code, out string? ch)) return DstOutcome.Skip;
            dst = ch!;
            return DstOutcome.Ok;
        }
        if (dstHex.Length == 8)
        {
            if (!TryParseHex(dstHex, out uint code)) return DstOutcome.Abort;
            string? decoded = DecodeUtf16SurrogatePair(code);
            if (decoded != null) { dst = decoded; return DstOutcome.Ok; }
            string pair = ConcatBmpChunks(dstHex);
            if (pair.Length == 0) return DstOutcome.Skip;
            dst = pair;
            return DstOutcome.Ok;
        }
        string lig = ConcatBmpChunks(dstHex);
        if (lig.Length == 0) return DstOutcome.Skip;
        dst = lig;
        return DstOutcome.Ok;
    }

    /// <summary>
    /// Parse a bfrange line in either the array form
    /// (<c>&lt;lo&gt; &lt;hi&gt; [&lt;d0&gt; &lt;d1&gt; …]</c>) or the sequential form
    /// (<c>&lt;lo&gt; &lt;hi&gt; &lt;dst&gt;</c>). Returns null when neither form matches.
    /// </summary>
    private static List<(uint Src, string Dst)>? ParseBfRangeLine(string line)
    {
        // Array form is tried first: `<005F> <0061> [<00660066> <00660069> <00660066006C>]`
        // maps 0x5F, 0x60, 0x61 to "ff", "fi", "ffl".
        Match arr = ArrayRegex().Match(line);
        if (arr.Success)
        {
            if (!TryParseHex(StripWhitespace(arr.Groups[1].Value.Trim()), out uint start)) return null;
            if (!TryParseHex(StripWhitespace(arr.Groups[2].Value.Trim()), out uint end)) return null;

            List<string> dstHexes = new();
            foreach (Match h in HexTokenRegex().Matches(arr.Groups[3].Value))
            {
                string s = StripWhitespace(h.Groups[1].Value.Trim());
                if (s.Length != 0) dstHexes.Add(s);
            }

            // §9.10.3 requires exactly (hi - lo + 1) entries; upstream is deliberately lenient and
            // uses whatever is present rather than rejecting the range.
            uint rangeSize = unchecked(end - start + 1);

            List<(uint, string)> result = new();
            for (int i = 0; i < dstHexes.Count && (uint)i < rangeSize; i++)
            {
                DstOutcome outcome = DecodeArrayDestination(dstHexes[i], out string dst);
                if (outcome == DstOutcome.Abort) return null;
                if (outcome == DstOutcome.Skip) continue;
                result.Add((unchecked(start + (uint)i), dst));
            }
            return result;
        }

        Match seq = SeqRegex().Match(line);
        if (seq.Success)
        {
            if (!TryParseHex(StripWhitespace(seq.Groups[1].Value.Trim()), out uint start)) return null;
            if (!TryParseHex(StripWhitespace(seq.Groups[2].Value.Trim()), out uint end)) return null;
            if (!TryParseHex(StripWhitespace(seq.Groups[3].Value.Trim()), out uint dstStart)) return null;

            uint rangeSize = Math.Min(end > start ? end - start : 0u, 10000u);

            // A surrogate-pair destination must be decoded to its code point *before* incrementing:
            // bumping the raw u32 would run the low surrogate past 0xDFFF into 0xE000.
            uint baseCp;
            bool haveBase;
            if (dstStart > 0xFFFF)
            {
                string? decoded = DecodeUtf16SurrogatePair(dstStart);
                if (decoded != null)
                {
                    haveBase = TrySingleScalar(decoded, out baseCp);
                }
                else
                {
                    baseCp = dstStart;
                    haveBase = true;
                }
            }
            else
            {
                baseCp = dstStart;
                haveBase = true;
            }

            List<(uint, string)> result = new();
            if (haveBase)
            {
                for (uint i = 0; i <= rangeSize; i++)
                {
                    uint src = unchecked(start + i);
                    uint cp = unchecked(baseCp + i);
                    if (TryCharFromU32(cp, out string? ch)) result.Add((src, ch!));
                }
            }
            return result;
        }

        return null;
    }

    /// <summary>
    /// Parse a notdefrange line <c>&lt;lo&gt; &lt;hi&gt; &lt;dst&gt;</c>: every code in the range
    /// maps to the same replacement character. Sequential form only — no array form.
    /// </summary>
    private static List<(uint Src, string Dst)>? ParseNotdefRangeLine(string line)
    {
        Match seq = SeqRegex().Match(line);
        if (!seq.Success) return null;

        if (!TryParseHex(StripWhitespace(seq.Groups[1].Value.Trim()), out uint start)) return null;
        if (!TryParseHex(StripWhitespace(seq.Groups[2].Value.Trim()), out uint end)) return null;

        string dstStr = seq.Groups[3].Value.Trim();
        string? escape = ParseEscapeSequence("<" + dstStr + ">");
        string dst;
        if (escape != null)
        {
            dst = escape;
        }
        else
        {
            if (!TryParseHex(StripWhitespace(dstStr), out uint dstCode)) return null;
            string? decoded = dstCode > 0xFFFF ? DecodeUtf16SurrogatePair(dstCode) : null;
            if (decoded == null)
            {
                if (!TryCharFromU32(dstCode, out string? ch)) return null;
                decoded = ch!;
            }
            dst = decoded;
        }

        List<(uint, string)> result = new();
        uint rangeSize = Math.Min(end > start ? end - start : 0u, 10000u);
        for (uint i = 0; i <= rangeSize; i++) result.Add((unchecked(start + i), dst));
        return result;
    }
}

/// <summary>
/// Lazily-parsed ToUnicode CMap: defers parsing until the first lookup and shares the parsed result
/// across fonts through a process-wide cache keyed by the stream's content hash.
/// </summary>
/// <remarks>
/// Documents commonly repeat the same ToUnicode stream across many font objects; parsing it once
/// and sharing it is what keeps font loading cheap on font-heavy PDFs.
/// </remarks>
internal sealed class OxLazyCMap
{
    /// <summary>Bounded so long-lived hosts processing many PDFs do not accumulate CMaps forever.</summary>
    private const int MaxCMapCacheEntries = 1024;

    private static readonly Dictionary<ulong, OxCMap> Cache = new();
    private static readonly LinkedList<ulong> InsertionOrder = new();
    private static readonly Lock CacheLock = new();

    private readonly byte[] _rawStream;
    private readonly ulong _cacheKey;
    private OxCMap? _parsed;

    internal OxLazyCMap(byte[] rawStream)
    {
        _rawStream = rawStream;
        _cacheKey = ComputeStreamHash(rawStream);
    }

    internal byte[] RawData => _rawStream;

    /// <summary>Code width (1 or 2) from the codespace range; 1 when the CMap is missing.</summary>
    internal byte CodeWidth() => Get()?.CodeWidth ?? 1;

    /// <summary>Writing mode; 0 (horizontal) when the CMap is missing or declares no /WMode.</summary>
    internal byte WMode() => Get()?.WMode ?? 0;

    /// <summary>The parsed CMap, parsing and caching it on first access.</summary>
    internal OxCMap? Get()
    {
        if (_parsed != null) return _parsed;

        lock (CacheLock)
        {
            if (Cache.TryGetValue(_cacheKey, out OxCMap? cached))
            {
                Promote(_cacheKey);
                _parsed = cached;
                return cached;
            }
        }

        OxCMap parsed = OxCMap.ParseToUnicodeCMap(_rawStream);
        _parsed = parsed;

        lock (CacheLock)
        {
            if (Cache.ContainsKey(_cacheKey))
            {
                Cache[_cacheKey] = parsed;
                Promote(_cacheKey);
            }
            else
            {
                while (Cache.Count >= MaxCMapCacheEntries && InsertionOrder.First != null)
                {
                    ulong oldKey = InsertionOrder.First.Value;
                    InsertionOrder.RemoveFirst();
                    Cache.Remove(oldKey);
                }
                Cache[_cacheKey] = parsed;
                InsertionOrder.AddLast(_cacheKey);
            }
        }

        return parsed;
    }

    private static void Promote(ulong key)
    {
        LinkedListNode<ulong>? node = InsertionOrder.Find(key);
        if (node != null) InsertionOrder.Remove(node);
        InsertionOrder.AddLast(key);
    }

    /// <summary>Reclaim cache memory in long-lived processes that handle many different PDFs.</summary>
    internal static void ClearCMapCache()
    {
        lock (CacheLock)
        {
            Cache.Clear();
            InsertionOrder.Clear();
        }
    }

    internal static int CMapCacheSize()
    {
        lock (CacheLock) return Cache.Count;
    }

    /// <summary>FNV-1a over the raw stream bytes; identical streams share one parsed CMap.</summary>
    private static ulong ComputeStreamHash(byte[] data)
    {
        ulong hash = 14695981039346656037UL;
        foreach (byte b in data)
        {
            hash ^= b;
            hash *= 1099511628211UL;
        }
        // Mix the length in so same-content-different-length inputs cannot alias.
        hash ^= (ulong)data.Length;
        return hash * 1099511628211UL;
    }
}
