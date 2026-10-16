// PDF font model for text extraction: simple fonts (Type1/TrueType/Type3) and
// composite Type0/CID fonts. Provides code iteration, code→Unicode, and glyph widths.
// Ports the text-relevant behaviour of pdf_oxide's FontInfo (crates/pdf/oxide).

namespace XRay.Content.Internal.Pdf;

public sealed class PdfFont
{
    public bool IsType0;
    public string BaseFont = "";
    public string Subtype = "";

    // Simple font: code (0-255) -> unicode string (from encoding + Differences).
    private string[]? _encoding;
    // Simple font widths.
    private Dictionary<int, double>? _widths;
    public int FirstChar;
    public double MissingWidth;
    // pdf_oxide default_width (fonts/font_dict.rs): 600 fixed-pitch / 500
    // proportional / 550 when the descriptor carries no /Flags. Used as the final
    // width fallback for simple fonts (after /Widths and Standard-14 AFM metrics).
    private double _simpleDefaultWidth = 550.0;
    // Standard-14 AFM metrics, resolved once from BaseFont (null if not a Standard-14).
    private Fonts.StandardFonts.Metrics? _afm;

    // ToUnicode (both simple and Type0).
    private PdfCMap? _toUnicode;

    // Type0: encoding CMap (bytes->code, code->CID), CID widths.
    private PdfCMap? _cmap;
    private Dictionary<int, double>? _cidWidths;
    public double DefaultWidth = 1000.0;

    public bool IsBold;
    public bool IsItalic;
    public bool IsMonospace;
    public double FontMatrixA = 0.001;
    public double Ascent = 0.95;
    public double Descent = -0.35;

    public static PdfFont Load(PdfDict fontDict, PdfDocument doc)
    {
        var f = new PdfFont();
        f.Subtype = doc.Resolve(fontDict.Get("Subtype")).AsName() ?? "";
        f.BaseFont = doc.Resolve(fontDict.Get("BaseFont")).AsName() ?? "";

        // ToUnicode
        if (doc.Resolve(fontDict.Get("ToUnicode")) is PdfStream tuStream)
        {
            try { f._toUnicode = PdfCMap.ParseToUnicode(doc.DecodeStream(tuStream)); }
            catch (Exception) when (Cancel.NotCancelling) { f._toUnicode = null; }
        }

        if (f.Subtype == "Type0")
        {
            f.IsType0 = true;
            LoadType0(f, fontDict, doc);
        }
        else
        {
            LoadSimple(f, fontDict, doc);
        }
        // Bold/italic hint from base font name.
        string bf = f.BaseFont.ToLowerInvariant();
        if (bf.Contains("bold")) f.IsBold = true;
        if (bf.Contains("italic") || bf.Contains("oblique")) f.IsItalic = true;
        if (bf.Contains("mono") || bf.Contains("courier") || bf.Contains("consol")) f.IsMonospace = true;
        return f;
    }

    private static void LoadSimple(PdfFont f, PdfDict fontDict, PdfDocument doc)
    {
        // Base encoding.
        string[] baseEnc;
        bool symbolic = false;
        var descriptor = doc.Resolve(fontDict.Get("FontDescriptor")).AsDict();
        if (descriptor != null)
        {
            var flagsObj = doc.Resolve(descriptor.Get("Flags"));
            int flags = (int)(flagsObj.AsLong() ?? 0);
            symbolic = (flags & 4) != 0 && (flags & 32) == 0;
            if ((flags & (1 << 18)) != 0) f.IsBold = true;
            if ((flags & (1 << 6)) != 0) f.IsItalic = true;
            if ((flags & 1) != 0) f.IsMonospace = true;
            // pdf_oxide default_width from FixedPitch flag (bit 1).
            if (flagsObj is PdfNumber) f._simpleDefaultWidth = (flags & 1) != 0 ? 600.0 : 500.0;
            f.MissingWidth = doc.Resolve(descriptor.Get("MissingWidth")).AsNumber() ?? 0;
            var asc = doc.Resolve(descriptor.Get("Ascent")).AsNumber();
            var desc = doc.Resolve(descriptor.Get("Descent")).AsNumber();
            if (asc.HasValue && asc.Value != 0) f.Ascent = asc.Value / 1000.0;
            if (desc.HasValue && desc.Value != 0) f.Descent = desc.Value / 1000.0;
        }

        var encObj = doc.Resolve(fontDict.Get("Encoding"));
        string? baseName = null;
        PdfArray? diffs = null;
        if (encObj is PdfName en) baseName = en.Value;
        else if (encObj.AsDict() is PdfDict ed)
        {
            baseName = doc.Resolve(ed.Get("BaseEncoding")).AsName();
            diffs = doc.Resolve(ed.Get("Differences")).AsArray();
        }

        // A Type 1 program carries its own encoding, which ISO 32000-1 §9.6.6.1 makes the
        // implicit base whenever the font dictionary names none. It is also the only place the
        // ligature slots of a TeX font are declared.
        var progEnc = (f.Subtype is "Type1" or "MMType1" && descriptor is not null)
            ? Fonts.Type1Encoding.Parse(LoadFontProgram(descriptor, doc) ?? [])
            : null;

        bool customEncoding = diffs != null;
        if (baseName is not null && PdfEncodings.ByName(baseName) is { } named)
        {
            baseEnc = named;
            // A named encoding plus an embedded program: the program's non-standard slots (a
            // space moved to 0xCA, say) win over the name, unless the program is really a subset
            // font's private glyph ordering — overlaying one of those turns every mapped code
            // into mojibake, so it is left alone.
            if (progEnc is not null && encObj is PdfName && !LooksLikeSubsetCipher(progEnc, named))
            {
                var merged = new string[256];
                Array.Copy(named, merged, 256);
                foreach (var (code, text) in progEnc) merged[code] = text;
                baseEnc = merged;
                customEncoding = true;
            }
        }
        else if (progEnc is not null)
        {
            // No usable named encoding: the program's encoding is the base. When an /Encoding
            // dictionary omits /BaseEncoding this is what §9.6.6.1 asks for, and with no
            // /Encoding at all upstream takes the program's map whole — codes it does not
            // declare stay unmapped.
            var built = new string[256];
            foreach (var (code, text) in progEnc) built[code] = text;
            baseEnc = built;
            customEncoding = true;
        }
        else
        {
            baseEnc = f.Subtype == "TrueType" && !symbolic ? PdfEncodings.WinAnsi : PdfEncodings.Standard;
        }
        // Type3 FontMatrix
        if (f.Subtype == "Type3" && doc.Resolve(fontDict.Get("FontMatrix")).AsArray() is PdfArray fm && fm.Items.Count >= 1)
        {
            f.FontMatrixA = doc.Resolve(fm.Items[0]).AsNumber() ?? 0.001;
            // pdf_oxide rescales default_width so callers multiplying by font_matrix_a
            // still yield the intended em fraction when the matrix isn't 1/1000.
            if (f.FontMatrixA != 0.001 && f.FontMatrixA != 0.0)
                f._simpleDefaultWidth = f._simpleDefaultWidth * 0.001 / f.FontMatrixA;
        }

        var enc = new string[256];
        Array.Copy(baseEnc, enc, 256);
        if (diffs != null)
        {
            int cur = 0;
            foreach (var it in diffs.Items)
            {
                var r = doc.Resolve(it);
                if (r is PdfNumber n) cur = (int)n.Value;
                else if (r is PdfName gn)
                {
                    if (cur >= 0 && cur < 256) enc[cur] = PdfEncodings.GlyphNameToUnicode(gn.Value);
                    cur++;
                }
            }
        }

        // Upstream expands ligature codepoints on every lookup through a *custom* encoding —
        // one reached through /Differences or through an embedded program — and never through a
        // plain named encoding or a ToUnicode CMap.
        if (customEncoding)
            for (int c = 0; c < 256; c++) enc[c] = ExpandLigature(enc[c]);

        // Symbolic standard fonts: per spec 9.6.6.1 (and pdf_oxide's priority order) a
        // symbolic font named *Symbol* / *Zapf* / *Dingbat* maps through its BUILT-IN
        // encoding, overriding /Encoding and /Differences. Symbolic = descriptor flag
        // bit 3 when present, else inferred from the base font name (standard-14
        // Symbol/ZapfDingbats often carry no descriptor).
        string bfLower = f.BaseFont.ToLowerInvariant();
        bool symbolicFont = descriptor != null
            ? (doc.Resolve(descriptor.Get("Flags")).AsLong() is long fl && (fl & 4) != 0)
            : bfLower.Contains("symbol") || bfLower.Contains("zapf") || bfLower.Contains("dingbat");
        string?[]? builtIn = null;
        if (symbolicFont && bfLower.Contains("symbol")) builtIn = PdfEncodings.Symbol;
        else if (symbolicFont && (bfLower.Contains("zapf") || bfLower.Contains("dingbat"))) builtIn = PdfEncodings.ZapfDingbats;
        if (builtIn != null)
            for (int c = 0; c < 256; c++)
                if (builtIn[c] is string s) enc[c] = s;

        f._encoding = enc;

        // Standard-14 AFM metrics (resolved once; used when /Widths lacks a code).
        f._afm = Fonts.StandardFonts.Resolve(f.BaseFont);

        // Widths
        f.FirstChar = (int)(doc.Resolve(fontDict.Get("FirstChar")).AsLong() ?? 0);
        var wArr = doc.Resolve(fontDict.Get("Widths")).AsArray();
        if (wArr != null)
        {
            f._widths = new Dictionary<int, double>();
            for (int i = 0; i < wArr.Items.Count; i++)
            {
                double w = doc.Resolve(wArr.Items[i]).AsNumber() ?? 0;
                f._widths[f.FirstChar + i] = w;
            }
        }
    }

    private static void LoadType0(PdfFont f, PdfDict fontDict, PdfDocument doc)
    {
        var encObj = doc.Resolve(fontDict.Get("Encoding"));
        if (encObj is PdfName en)
        {
            if (en.Value.StartsWith("Identity")) f._cmap = PdfCMap.Identity(2);
            else f._cmap = PdfCMap.Identity(2); // predefined CJK CMaps approximated as 2-byte identity
        }
        else if (encObj is PdfStream cmapStream)
        {
            try { f._cmap = PdfCMap.ParseCid(doc.DecodeStream(cmapStream)); }
            catch (Exception) when (Cancel.NotCancelling) { f._cmap = PdfCMap.Identity(2); }
        }
        else f._cmap = PdfCMap.Identity(2);

        var descFonts = doc.Resolve(fontDict.Get("DescendantFonts")).AsArray();
        PdfDict? cidFont = null;
        if (descFonts != null && descFonts.Items.Count > 0) cidFont = doc.Resolve(descFonts.Items[0]).AsDict();
        if (cidFont != null)
        {
            f.DefaultWidth = doc.Resolve(cidFont.Get("DW")).AsNumber() ?? 1000.0;
            var wArr = doc.Resolve(cidFont.Get("W")).AsArray();
            if (wArr != null) f._cidWidths = ParseCidWidths(wArr, doc);
            var descriptor = doc.Resolve(cidFont.Get("FontDescriptor")).AsDict();
            if (descriptor != null)
            {
                int flags = (int)(doc.Resolve(descriptor.Get("Flags")).AsLong() ?? 0);
                if ((flags & (1 << 18)) != 0) f.IsBold = true;
                if ((flags & (1 << 6)) != 0) f.IsItalic = true;
                var asc = doc.Resolve(descriptor.Get("Ascent")).AsNumber();
                var desc = doc.Resolve(descriptor.Get("Descent")).AsNumber();
                if (asc.HasValue && asc.Value != 0) f.Ascent = asc.Value / 1000.0;
                if (desc.HasValue && desc.Value != 0) f.Descent = desc.Value / 1000.0;
            }
        }
    }

    private static Dictionary<int, double> ParseCidWidths(PdfArray w, PdfDocument doc)
    {
        var map = new Dictionary<int, double>();
        int i = 0;
        while (i < w.Items.Count)
        {
            var first = doc.Resolve(w.Items[i]);
            if (first is not PdfNumber c) { i++; continue; }
            if (i + 1 >= w.Items.Count) break;
            var second = doc.Resolve(w.Items[i + 1]);
            if (second is PdfArray arr)
            {
                int cid = (int)c.Value;
                foreach (var it in arr.Items)
                    map[cid++] = doc.Resolve(it).AsNumber() ?? 0;
                i += 2;
            }
            else if (second is PdfNumber c2 && i + 2 < w.Items.Count)
            {
                double wv = doc.Resolve(w.Items[i + 2]).AsNumber() ?? 0;
                for (int cid = (int)c.Value; cid <= (int)c2.Value; cid++) map[cid] = wv;
                i += 3;
            }
            else i++;
        }
        return map;
    }

    /// <summary>Split a show-text byte string into character codes.</summary>
    public IEnumerable<int> DecodeCodes(byte[] bytes)
    {
        if (!IsType0)
        {
            foreach (var b in bytes) yield return b;
            yield break;
        }
        var cmap = _cmap ?? PdfCMap.Identity(2);
        int pos = 0;
        while (pos < bytes.Length)
        {
            int len = cmap.MatchCodeLength(bytes, pos);
            if (len <= 0) len = 1;
            if (pos + len > bytes.Length) len = bytes.Length - pos;
            int code = 0;
            for (int i = 0; i < len; i++) code = (code << 8) | bytes[pos + i];
            pos += len;
            yield return code;
        }
    }

    /// <summary>
    /// The decoded bytes of the descriptor's embedded font program, or <c>null</c> when there is
    /// none. <c>/FontFile2</c> (TrueType) and <c>/FontFile3</c> (CFF) come first because a
    /// descriptor carrying one of those is describing that program, whatever the /Subtype says.
    /// </summary>
    private static byte[]? LoadFontProgram(PdfDict descriptor, PdfDocument doc)
    {
        foreach (var key in (ReadOnlySpan<string>)["FontFile2", "FontFile3", "FontFile"])
        {
            if (descriptor.Get(key) is null) continue;
            if (doc.Resolve(descriptor.Get(key)) is not PdfStream st) continue;
            try { return doc.DecodeStream(st); }
            catch (Exception) when (Cancel.NotCancelling) { return null; }
        }
        return null;
    }

    /// <summary>
    /// Whether an embedded program's built-in encoding is a subset font's private glyph ordering
    /// rather than a real text encoding, judged by how often it agrees with the named encoding
    /// the producer declared: a real encoding agrees on most shared codes, a re-indexed cipher on
    /// almost none.
    /// </summary>
    private static bool LooksLikeSubsetCipher(Dictionary<int, string> progEnc, string[] named)
    {
        int agree = 0, overlap = 0;
        foreach (var (code, text) in progEnc)
        {
            if (code < 0 || code > 255) continue;
            string s = named[code];
            if (string.IsNullOrEmpty(s)) continue;
            overlap++;
            if (s == text) agree++;
        }
        return overlap > 0 && (float)agree / overlap < 0.5f;
    }

    /// <summary>
    /// A ligature codepoint spelled out, or the text unchanged.
    /// </summary>
    /// <remarks>
    /// This applies to <em>encoding</em>-derived mappings only, never to ToUnicode. A ToUnicode
    /// CMap is the font's own statement about what a code means and is taken at its word, but an
    /// encoding reaches Unicode through a glyph name, and the glyph named <c>fi</c> is the letters
    /// `f` and `i` set as one shape rather than a character anybody wrote. Upstream draws the line
    /// in the same place.
    /// </remarks>
    private static string ExpandLigature(string? text)
    {
        if (string.IsNullOrEmpty(text) || text.Length != 1) return text ?? "";
        return text[0] switch
        {
            '\uFB00' => "ff",
            '\uFB01' => "fi",
            '\uFB02' => "fl",
            '\uFB03' => "ffi",
            '\uFB04' => "ffl",
            '\uFB05' or '\uFB06' => "st",
            _ => text,
        };
    }

    // A ligature reaching us through ToUnicode is returned as the font wrote it; only the
    // encoding-derived mappings above are expanded. See ExpandLigature.
    public string CharToUnicode(int code)
    {
        if (_toUnicode != null)
        {
            var s = _toUnicode.LookupUnicode(code);
            if (s != null) return s;
        }
        if (!IsType0)
        {
            if (_encoding != null && code >= 0 && code < 256)
            {
                var s = _encoding[code];
                if (!string.IsNullOrEmpty(s)) return s;
            }
            if (code >= 32 && code < 127) return ((char)code).ToString();
            return "";
        }
        // Type0 without ToUnicode: last resort, treat CID as unicode if plausible.
        int? cid = _cmap?.LookupCid(code);
        if (cid.HasValue && cid.Value >= 32 && cid.Value < 0xFFFE) return ((char)cid.Value).ToString();
        return "";
    }

    /// <summary>Glyph advance width in glyph-space units (1/1000 em typically).</summary>
    public double GlyphWidth(int code)
    {
        if (!IsType0)
        {
            // pdf_oxide get_glyph_width order: /Widths (covered code) → Standard-14
            // AFM metrics → default_width. MissingWidth is intentionally NOT consulted
            // here (pdf_oxide uses the flags-derived default_width instead).
            if (_widths != null && _widths.TryGetValue(code, out var w)) return w;
            var afm = _afm?.Width(code);
            if (afm.HasValue) return afm.Value;
            return _simpleDefaultWidth;
        }
        int cid = _cmap?.LookupCid(code) ?? code;
        if (_cidWidths != null && _cidWidths.TryGetValue(cid, out var cw)) return cw;
        return DefaultWidth;
    }
}
