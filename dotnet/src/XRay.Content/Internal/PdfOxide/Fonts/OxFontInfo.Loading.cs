// Ported from pdf_oxide `fonts/font_dict.rs`:
//   from_dict (581-1499), parse_cidsysteminfo (1501-1552),
//   parse_descendant_fonts (1567-1925), extract_truetype_cmap_from_descriptor (1926-1977),
//   read_raw_ascent_descent_from_descriptor (1983-2013),
//   extract_embedded_font_from_descriptor (2015-2094), wrap_cff_in_opentype (2101-2237),
//   resolve_encoding_writing_mode (2270-2313), parse_dw2 (2315-2355),
//   parse_cid_vertical_metrics (2357-2510), parse_cid_widths (2512-2629)
//   and parse_encoding (2661-2958).
//
// Objects arrive through XRay.Content's own PDF object layer (`Ox.*` over
// PdfDocument/PdfObject), not pdf_oxide's. Where the Rust requires an entry to be an
// indirect reference (`as_reference()`) that is kept: a direct /FontDescriptor or
// /ToUnicode is ignored upstream too, and honouring one here would change which
// fonts get flags and metrics.
using System;
using System.Collections.Generic;
using System.Text;
using XRay.Content.Internal.Pdf;

namespace XRay.Content.Internal.PdfOxide.Fonts;

internal sealed partial class OxFontInfo
{
    /// <summary>
    /// Parse a font dictionary. Returns null when the object is not a dictionary; every other
    /// malformation degrades to defaults rather than failing the font, because a broken
    /// descriptor still leaves usable text.
    /// </summary>
    public static OxFontInfo? FromDict(PdfObject? dictObj, PdfDocument? doc)
    {
        // The modules this reaches — glyph names, CMaps, TrueType, font programs, the
        // encoding tables — were ported separately and find each other through process-wide
        // seams that something has to fill in. Installing here rather than in a static
        // constructor avoids a type-initialization cycle: a seam's own construction can
        // touch this class, and the CLR would let that re-entrant call skip the install.
        OxFontWiring.Install();

        var fontDict = Ox.Dict(doc, dictObj);
        if (fontDict is null) return null;

        try
        {
            return FromDictCore(fontDict, doc);
        }
        catch (Exception) when (Cancel.NotCancelling)
        {
            // A font that throws here would take the whole page's text with it.
            return new OxFontInfo
            {
                BaseFont = fontDict.Get("BaseFont").AsName() ?? "Unknown",
                Subtype = fontDict.Get("Subtype").AsName() ?? "Unknown",
            };
        }
    }

    private static OxFontInfo FromDictCore(PdfDict fontDict, PdfDocument? doc)
    {
        string baseFont = fontDict.Get("BaseFont").AsName() ?? "Unknown";
        string subtype = fontDict.Get("Subtype").AsName() ?? "Unknown";

        // Standard Type 1 FontMatrix is [0.001 0 0 0.001 0 0], so widths are 1/1000 em.
        // A Type 3 font may use an identity FontMatrix, putting widths in text space already.
        float fontMatrixA = 0.001f;
        if (subtype == "Type3")
        {
            var fm = fontDict.Get("FontMatrix").AsArray();
            double? a = fm is not null && fm.Items.Count > 0 ? fm.Items[0].AsNumber() : null;
            if (a is not null)
            {
                float av = (float)a.Value;
                // A degenerate FontMatrix[0] — zero, near-zero or non-finite — is malformed
                // (§9.2.4 / §9.6.5) and would make the default-width rescale below divide by
                // ~0 and collapse every advance to 0.
                if (float.IsFinite(av) && MathF.Abs(av) > 1e-6f) fontMatrixA = av;
            }
        }

        // The FontDescriptor is parsed first: its /Flags decide how /Encoding is treated.
        int? fontWeight = null;
        int? flags = null;
        float? stemV = null;
        byte[]? embeddedFontData = null;
        bool isTrueTypeFont = false;
        float? rawAscent = null;
        float? rawDescent = null;
        // Whether the document embeds its own outlines at all. Upstream this gates the CJK
        // predefined-CIDFont substitution, which is a renderer concern and not ported; the
        // fact is still tracked here because it is the one signal that tells a font with no
        // program apart from one whose program failed to decode.
        bool hasFontProgram = false;

        var descriptorRef = fontDict.Get("FontDescriptor");
        PdfDict? descriptorDict = descriptorRef is PdfRef ? Ox.Resolve(doc, descriptorRef).AsDict() : null;
        if (descriptorDict is not null)
        {
            fontWeight = (int?)descriptorDict.Get("FontWeight").AsLong();
            flags = (int?)descriptorDict.Get("Flags").AsLong();
            stemV = (float?)descriptorDict.Get("StemV").AsNumber();
            rawAscent = (float?)descriptorDict.Get("Ascent").AsNumber();
            rawDescent = (float?)descriptorDict.Get("Descent").AsNumber();

            // Key presence is recorded separately from extraction success: a
            // present-but-undecodable font program means the document intended to be
            // self-contained, which downstream gates must tell apart from "no program at all".
            hasFontProgram = descriptorDict.Has("FontFile2") || descriptorDict.Has("FontFile3")
                || descriptorDict.Has("FontFile");

            if (descriptorDict.Has("FontFile2"))
            {
                embeddedFontData = LoadStreamViaRef(doc, descriptorDict.Get("FontFile2"));
                isTrueTypeFont = true; // only TrueType programs carry a cmap
            }
            else if (descriptorDict.Has("FontFile3"))
            {
                byte[]? data = LoadStreamViaRef(doc, descriptorDict.Get("FontFile3"));
                if (data is not null && data.Length > 4 && data[0] == 1)
                    data = WrapCffInOpenType(data); // raw CFF needs an OpenType container
                embeddedFontData = data;
            }
            else if (descriptorDict.Has("FontFile"))
            {
                embeddedFontData = LoadStreamViaRef(doc, descriptorDict.Get("FontFile"));
            }
        }

        bool IsSymbolicFont(int? flagsOpt)
        {
            if (flagsOpt is not null)
            {
                const int SymbolicBit = 1 << 2; // Bit 3
                return (flagsOpt.Value & SymbolicBit) != 0;
            }
            string nameLower = baseFont.ToLowerInvariant();
            return nameLower.Contains("symbol", StringComparison.Ordinal)
                || nameLower.Contains("zapf", StringComparison.Ordinal)
                || nameLower.Contains("dingbat", StringComparison.Ordinal);
        }

        // The font program's own built-in encoding, needed as the base encoding for
        // /Differences (§9.6.6.1) as well as for the merge below.
        IReadOnlyDictionary<byte, char>? fontProgramEnc = null;
        if (embeddedFontData is not null)
        {
            var programs = OxFontSeams.FontPrograms;
            if (programs is not null)
            {
                fontProgramEnc = subtype is "Type1" or "MMType1"
                    ? programs.ParseType1Encoding(embeddedFontData)
                    : programs.ParseCffEncoding(embeddedFontData);
            }
        }

        // Writing mode from the encoding object. Resolved here because OxEncoding collapses
        // Identity-H and Identity-V into one variant, losing the name wmode is read from.
        byte encodingWmode = 0;
        OxEncoding encoding;
        Dictionary<byte, string> multiCharMap;
        Dictionary<byte, string> diffGlyphNames;

        var encObjRaw = fontDict.Get("Encoding");
        if (encObjRaw is not null)
        {
            PdfObject? resolvedEnc = encObjRaw is PdfRef ? Ox.Resolve(doc, encObjRaw) : encObjRaw;

            // Read the `-V` name / embedded `/WMode 1 def` before parse_encoding flattens it.
            encodingWmode = ResolveEncodingWritingMode(resolvedEnc, doc).Wmode;

            // §9.6.6.1 says a symbolic font's /Encoding is ignored, but LaTeX/LibreOffice
            // routinely set Symbolic on non-symbolic fonts, and every real viewer parses an
            // explicit /Encoding anyway. The flag only decides what happens with no /Encoding.
            var parsed = ParseEncoding(resolvedEnc, doc, fontProgramEnc);
            encoding = parsed.Encoding;
            multiCharMap = parsed.MultiCharMap;
            diffGlyphNames = parsed.DiffGlyphNames;

            // A named encoding plus an embedded program: overlay the program's few
            // non-standard slots (e.g. space at 0xCA) onto the named base.
            if (encoding.IsStandard && fontProgramEnc is not null)
            {
                string stdName = encoding.Name ?? "StandardEncoding";

                // Discriminate a real encoding from a subset *cipher* — the font's own glyph
                // ordering, which bears no relation to the declared base. Overlaying a cipher
                // rewrites every mapped code into mojibake, so the named encoding is kept.
                if (!OxFontTables.BuiltinEncodingLooksLikeCipher(fontProgramEnc, stdName))
                {
                    var customMap = new Dictionary<byte, char>();
                    for (int code = 0; code <= 255; code++)
                    {
                        string? unicodeStr = OxFontTables.StandardEncodingLookup(stdName, (byte)code);
                        if (unicodeStr is not null && unicodeStr.Length > 0) customMap[(byte)code] = unicodeStr[0];
                    }
                    foreach (var kv in fontProgramEnc)
                    {
                        customMap[kv.Key] = kv.Value;
                        if (OxFontTables.IsLigatureChar(kv.Value))
                        {
                            string? expanded = OxFontTables.ExpandLigatureChar(kv.Value);
                            if (expanded is not null) multiCharMap[kv.Key] = expanded;
                        }
                    }
                    encoding = OxEncoding.Custom(customMap);
                }
            }
        }
        else if (fontProgramEnc is not null)
        {
            var map = new Dictionary<byte, char>();
            multiCharMap = new Dictionary<byte, string>();
            foreach (var kv in fontProgramEnc)
            {
                map[kv.Key] = kv.Value;
                if (OxFontTables.IsLigatureChar(kv.Value))
                {
                    string? expanded = OxFontTables.ExpandLigatureChar(kv.Value);
                    if (expanded is not null) multiCharMap[kv.Key] = expanded;
                }
            }
            encoding = OxEncoding.Custom(map);
            diffGlyphNames = new Dictionary<byte, string>();
        }
        else if (IsSymbolicFont(flags))
        {
            // Symbol / ZapfDingbats resolve through their built-in encodings at decode time.
            encoding = OxEncoding.Standard("SymbolicBuiltIn");
            multiCharMap = new Dictionary<byte, string>();
            diffGlyphNames = new Dictionary<byte, string>();
        }
        else
        {
            encoding = OxEncoding.Standard("StandardEncoding");
            multiCharMap = new Dictionary<byte, string>();
            diffGlyphNames = new Dictionary<byte, string>();
        }

        // ToUnicode is stored raw and parsed on first lookup; eager validation would parse
        // every CMap twice.
        IOxCMap? toUnicode = null;
        byte[]? cmapBytes = LoadStreamViaRef(doc, fontDict.Get("ToUnicode"));
        if (cmapBytes is not null) toUnicode = OxFontSeams.CMaps?.CreateLazy(cmapBytes);

        // /Widths (§9.7.4) for simple fonts; Type0 widths come from the CIDFont's /W below.
        float[]? widths = null;
        uint? firstChar = null;
        uint? lastChar = null;
        if (subtype != "Type0")
        {
            var widthsArr = Ox.Arr(doc, fontDict.Get("Widths"));
            if (widthsArr is not null)
            {
                var list = new List<float>(widthsArr.Items.Count);
                foreach (var item in widthsArr.Items)
                {
                    double? v = item.AsNumber();
                    if (v is not null) list.Add((float)v.Value);
                }
                widths = list.ToArray();
            }
            firstChar = (uint?)(long?)fontDict.Get("FirstChar").AsLong();
            lastChar = (uint?)(long?)fontDict.Get("LastChar").AsLong();
        }

        // Typical values are 500-600 proportional, ~600 monospace; with no /Flags at all,
        // a middle-ground 550.
        float defaultWidth;
        if (flags is not null)
        {
            const int FixedPitchBit = 1 << 0; // Bit 1
            defaultWidth = (flags.Value & FixedPitchBit) != 0 ? 600.0f : 500.0f;
        }
        else
        {
            defaultWidth = 550.0f;
        }

        // That heuristic assumes glyph space is 1/1000 em. A Type 3 font can pick any
        // FontMatrix, so rescale to keep callers that multiply by font_matrix_a correct.
        if (subtype == "Type3" && fontMatrixA != 0.001f)
            defaultWidth = defaultWidth * 0.001f / fontMatrixA;

        OxCIDToGIDMap? cidToGidMap = null;
        OxCIDSystemInfo? cidSystemInfo = null;
        string? cidFontType = null;
        Dictionary<ushort, float>? cidWidths = null;
        float cidDefaultWidth = 1000.0f;
        bool hasExplicitDw = false;
        IOxTrueTypeCMap? descendantTtCmap = null;
        Dictionary<ushort, OxVerticalMetrics>? cidVerticalMetrics = null;
        OxVerticalMetrics cidDefaultVerticalMetrics = OxVerticalMetrics.SpecDefault;

        if (subtype == "Type0")
        {
            var descendant = ParseDescendantFonts(fontDict, doc);
            if (descendant is not null)
            {
                cidToGidMap = descendant.CidToGidMap;
                cidSystemInfo = descendant.CidSystemInfo;
                cidFontType = descendant.CidFontType;
                cidWidths = descendant.CidWidths;
                cidDefaultWidth = descendant.CidDefaultWidth;
                hasExplicitDw = descendant.HasExplicitDw;
                descendantTtCmap = descendant.TrueTypeCMap;
                cidVerticalMetrics = descendant.VerticalMetrics;
                cidDefaultVerticalMetrics = descendant.DefaultVerticalMetrics;

                // The embedded program of a Type0 font lives on the CIDFont descendant.
                if (descendant.EmbeddedFontData is not null && embeddedFontData is null)
                    embeddedFontData = descendant.EmbeddedFontData;
                hasFontProgram |= descendant.HasFontProgram;

                // The Type0 wrapper usually has no /FontDescriptor, so fall back to the
                // descendant's metrics rather than the 0.95/-0.35 default (§9.7.4, Table 117).
                rawAscent ??= descendant.RawAscent;
                rawDescent ??= descendant.RawDescent;
            }
            else
            {
                cidToGidMap = OxCIDToGIDMap.Identity;
            }
        }

        // Parse the CFF GID mapping only for simple fonts: Type0/CID fonts route through
        // CIDToGIDMap, not the CFF encoding. §9.6.6 makes the PDF /Encoding the byte→name
        // source and the CFF charset the name→GID resolver — subsetter-emitted CFF Encoding
        // tables are often near-empty and would drop most bytes to .notdef.
        Dictionary<byte, ushort>? cffGidMap = null;
        if (subtype != "Type0" && embeddedFontData is not null)
        {
            var parsed = OxFontSeams.FontPrograms?.ParseCffGidMappingWithPdfEncoding(
                embeddedFontData, encoding, diffGlyphNames);
            if (parsed is not null)
            {
                cffGidMap = new Dictionary<byte, ushort>(parsed.Count);
                foreach (var kv in parsed) cffGidMap[kv.Key] = kv.Value;
            }
        }

        // /Ascent and /Descent are in 1/1000 em; normalize to a fraction of em, falling back
        // to the Standard-14 AFM values and then to Poppler's 0.95 / -0.35.
        var (defaultAscent, defaultDescent) = OxFontTables.StandardFontMetrics(baseFont) ?? (0.95f, -0.35f);
        float ascent = rawAscent is not null ? rawAscent.Value / 1000.0f : defaultAscent;
        float descent = defaultDescent;
        if (rawDescent is not null)
        {
            // Descent should be ≤ 0. Some PDFs store it as a positive magnitude; Poppler
            // normalizes by negating, and matching that keeps line metrics comparable.
            float d = rawDescent.Value / 1000.0f;
            descent = d > 0.0f ? -d : d;
        }

        // §9.10.2 makes ToUnicode an extraction-time mapping only; the active writing mode
        // comes from the /Encoding CMap (§9.7.5). Reading ToUnicode's /WMode here would flip
        // a horizontal document to vertical whenever a producer left a stale directive in the
        // ToUnicode prologue — a real tooling failure mode.
        byte wmode = encodingWmode;

        var font = new OxFontInfo
        {
            BaseFont = baseFont,
            Subtype = subtype,
            Encoding = encoding,
            ToUnicode = toUnicode,
            FontWeight = fontWeight,
            Flags = flags,
            StemV = stemV,
            Ascent = ascent,
            Descent = descent,
            EmbeddedFontData = embeddedFontData,
            IsTrueTypeFont = isTrueTypeFont,
            CidToGidMap = cidToGidMap,
            CidSystemInfo = cidSystemInfo,
            CidFontType = cidFontType,
            FontMatrixA = fontMatrixA,
            Widths = widths,
            FirstChar = firstChar,
            LastChar = lastChar,
            DefaultWidth = defaultWidth,
            CidWidths = cidWidths,
            CidDefaultWidth = cidDefaultWidth,
            HasExplicitDw = hasExplicitDw,
            CffGidMap = cffGidMap,
            MultiCharMap = multiCharMap,
            DiffGlyphNames = diffGlyphNames,
            Wmode = wmode,
            CidVerticalMetrics = cidVerticalMetrics,
            CidDefaultVerticalMetrics = cidDefaultVerticalMetrics,
        };

        // Pre-populate the cmap memo with the descendant's, leaving lazy extraction from
        // embedded_font_data for everything else.
        if (descendantTtCmap is not null) font.SetTrueTypeCmap(descendantTtCmap);

        return font;
    }

    /// <summary>Decoded bytes of a stream entry, but only when it is an indirect reference.</summary>
    private static byte[]? LoadStreamViaRef(PdfDocument? doc, PdfObject? entry)
    {
        if (entry is not PdfRef) return null;
        return Ox.StreamData(doc, entry);
    }

    /// <summary>
    /// Writing mode implied by a Type0 font's /Encoding, with the name kept for diagnostics.
    /// An embedded CMap stream's explicit /WMode overrides what the name suggests (§9.7.5.4).
    /// </summary>
    private static (string? Name, byte Wmode) ResolveEncodingWritingMode(PdfObject? encObj, PdfDocument? doc)
    {
        string? atom = encObj.AsName();
        if (atom is not null) return (atom, OxFontTables.WmodeFromPredefinedCMapName(atom));

        var dict = encObj.AsDict();
        string? name = dict is null ? null : dict.Get("CMapName").AsName();

        // Decode failures are swallowed: parse_encoding logs them, and for wmode we simply
        // fall back to the name-based signal.
        byte? streamWmode = null;
        byte[]? bytes = Ox.StreamData(doc, encObj);
        if (bytes is not null)
        {
            string content = System.Text.Encoding.UTF8.GetString(bytes);
            streamWmode = OxFontSeams.CMaps?.ParseWModeDirective(content);
        }

        byte nameWmode = name is null ? (byte)0 : OxFontTables.WmodeFromPredefinedCMapName(name);
        return (name, streamWmode ?? nameWmode);
    }

    // -----------------------------------------------------------------------
    // /Encoding
    // -----------------------------------------------------------------------

    private readonly struct ParsedEncoding
    {
        public readonly OxEncoding Encoding;
        public readonly Dictionary<byte, string> MultiCharMap;
        public readonly Dictionary<byte, string> DiffGlyphNames;

        public ParsedEncoding(OxEncoding encoding, Dictionary<byte, string> multi, Dictionary<byte, string> names)
        { Encoding = encoding; MultiCharMap = multi; DiffGlyphNames = names; }
    }

    /// <summary>
    /// Parse an /Encoding object: a predefined name, a CMap stream, or a dictionary with
    /// /BaseEncoding and /Differences (§9.6.6.2).
    ///
    /// The third result keeps `code → /Differences glyph name`: the name, not the resolved
    /// char, is what the punctuation-recovery interceptions in CharToUnicode consult.
    /// </summary>
    private static ParsedEncoding ParseEncoding(
        PdfObject? encObj, PdfDocument? doc, IReadOnlyDictionary<byte, char>? fontProgramEncoding)
    {
        var empty = new Dictionary<byte, string>();

        string? name = encObj.AsName();
        if (name is not null)
        {
            return name switch
            {
                "WinAnsiEncoding" => new ParsedEncoding(OxEncoding.Standard("WinAnsiEncoding"), empty, new Dictionary<byte, string>()),
                "MacRomanEncoding" => new ParsedEncoding(OxEncoding.Standard("MacRomanEncoding"), empty, new Dictionary<byte, string>()),
                "MacExpertEncoding" => new ParsedEncoding(OxEncoding.Standard("MacExpertEncoding"), empty, new Dictionary<byte, string>()),
                "Identity-H" or "Identity-V" => new ParsedEncoding(OxEncoding.Identity, empty, new Dictionary<byte, string>()),
                _ => new ParsedEncoding(OxEncoding.Standard(name), empty, new Dictionary<byte, string>()),
            };
        }

        var dict = encObj.AsDict();
        if (dict is null)
            return new ParsedEncoding(OxEncoding.Standard("StandardEncoding"), empty, new Dictionary<byte, string>());

        // A Type0 font can reference a CMap stream through /Encoding (§9.7.5.2). Adobe's
        // collection CMaps define an identity charcode→CID mapping, so CIDs can then be
        // resolved through the predefined CID→Unicode tables; arbitrary CMap programs cannot
        // be executed, so those keep the default behaviour.
        string? cmapName = dict.Get("CMapName").AsName();
        if (cmapName is not null)
        {
            bool isAdobeCollection = cmapName.StartsWith("Adobe-", StringComparison.Ordinal)
                && (cmapName.Contains("Japan", StringComparison.Ordinal)
                    || cmapName.Contains("GB", StringComparison.Ordinal)
                    || cmapName.Contains("CNS", StringComparison.Ordinal)
                    || cmapName.Contains("Korea", StringComparison.Ordinal));
            if (isAdobeCollection || cmapName is "Identity-H" or "Identity-V")
                return new ParsedEncoding(OxEncoding.Identity, new Dictionary<byte, string>(), new Dictionary<byte, string>());

            return new ParsedEncoding(OxEncoding.Standard(cmapName), new Dictionary<byte, string>(), new Dictionary<byte, string>());
        }

        var multiCharMap = new Dictionary<byte, string>();
        var diffGlyphNames = new Dictionary<byte, string>();
        Dictionary<byte, char> encodingMap;

        var baseEncObj = dict.Get("BaseEncoding");
        if (baseEncObj is not null)
        {
            PdfObject? resolvedBase = baseEncObj is PdfRef ? Ox.Resolve(doc, baseEncObj) : baseEncObj;
            string? baseName = resolvedBase.AsName();
            encodingMap = new Dictionary<byte, char>();
            if (baseName is not null)
            {
                for (int code = 0; code <= 255; code++)
                {
                    string? unicodeStr = OxFontTables.StandardEncodingLookup(baseName, (byte)code);
                    if (unicodeStr is not null && unicodeStr.Length > 0) encodingMap[(byte)code] = unicodeStr[0];
                }
            }
        }
        else if (fontProgramEncoding is not null)
        {
            // §9.6.6.1: with no /BaseEncoding, the font's built-in encoding is the base.
            encodingMap = new Dictionary<byte, char>(fontProgramEncoding.Count);
            foreach (var kv in fontProgramEncoding) encodingMap[kv.Key] = kv.Value;
        }
        else
        {
            encodingMap = new Dictionary<byte, char>();
            for (int code = 0; code <= 255; code++)
            {
                string? unicodeStr = OxFontTables.StandardEncodingLookup("StandardEncoding", (byte)code);
                if (unicodeStr is not null && unicodeStr.Length > 0) encodingMap[(byte)code] = unicodeStr[0];
            }
        }

        var differencesObj = dict.Get("Differences");
        if (differencesObj is not null)
        {
            PdfObject? resolvedDiff = differencesObj is PdfRef ? Ox.Resolve(doc, differencesObj) : differencesObj;
            var diffArray = resolvedDiff.AsArray();
            if (diffArray is not null)
            {
                long currentCode = 0;
                foreach (var item in diffArray.Items)
                {
                    PdfObject? actual = item is PdfRef ? Ox.Resolve(doc, item) : item;

                    if (actual is PdfNumber { IsInteger: true } num)
                    {
                        currentCode = num.AsLong;
                        continue;
                    }

                    string? glyphName = actual.AsName();
                    if (glyphName is null) continue; // anything else in /Differences is invalid

                    // The glyph name is retained whatever it resolves to (§9.6.6.1, Table 114),
                    // so the interceptions in CharToUnicode can consult it.
                    if (currentCode >= 0 && currentCode <= 255) diffGlyphNames[(byte)currentCode] = glyphName;

                    var glyphNames = OxFontSeams.GlyphNames;
                    char? unicodeChar = glyphNames?.GlyphNameToUnicode(glyphName);
                    if (unicodeChar is not null)
                    {
                        if (currentCode >= 0 && currentCode <= 255) encodingMap[(byte)currentCode] = unicodeChar.Value;
                    }
                    else
                    {
                        // Compound glyph name (f_f → "ff", f_f_i → "ffi").
                        string? unicodeString = glyphNames?.GlyphNameToUnicodeString(glyphName);
                        if (unicodeString is not null && currentCode >= 0 && currentCode <= 255)
                            multiCharMap[(byte)currentCode] = unicodeString;
                    }
                    currentCode++;
                }
            }
        }

        if (encodingMap.Count > 0 || multiCharMap.Count > 0)
            return new ParsedEncoding(OxEncoding.Custom(encodingMap), multiCharMap, diffGlyphNames);

        return new ParsedEncoding(OxEncoding.Standard("StandardEncoding"), new Dictionary<byte, string>(), diffGlyphNames);
    }

    // -----------------------------------------------------------------------
    // DescendantFonts (Type0)
    // -----------------------------------------------------------------------

    private sealed class DescendantFontInfo
    {
        public OxCIDToGIDMap? CidToGidMap;
        public OxCIDSystemInfo? CidSystemInfo;
        public string? CidFontType;
        public Dictionary<ushort, float>? CidWidths;
        public float CidDefaultWidth = 1000.0f;
        public bool HasExplicitDw;
        public IOxTrueTypeCMap? TrueTypeCMap;
        /// <summary>/FontFile{,2,3} key present, whether or not the bytes could be extracted.</summary>
        public bool HasFontProgram;
        public byte[]? EmbeddedFontData;
        public float? RawAscent;
        public float? RawDescent;
        public Dictionary<ushort, OxVerticalMetrics>? VerticalMetrics;
        public OxVerticalMetrics DefaultVerticalMetrics = OxVerticalMetrics.SpecDefault;
    }

    /// <summary>
    /// Parse the CIDFont descendant of a Type0 font (§9.7.1). Returns null when the entry is
    /// missing or unusable, which makes the caller fall back to Identity.
    /// </summary>
    private static DescendantFontInfo? ParseDescendantFonts(PdfDict fontDict, PdfDocument? doc)
    {
        var array = Ox.Arr(doc, fontDict.Get("DescendantFonts"));
        if (array is null || array.Items.Count == 0) return null;

        // §9.7.6 mandates an indirect reference here, but Persian/Farsi PDFs from older
        // XeTeX/pdfTeX writers inline the CIDFont dictionary. Rejecting the inline form falls
        // back to Identity-H, which emits CIDs as Latin-Extended-B garbage, so it is accepted.
        var first = array.Items[0];
        var cidFontDict = first is PdfRef ? Ox.Resolve(doc, first).AsDict() : first.AsDict();
        if (cidFontDict is null) return null;

        string? cidFontType = cidFontDict.Get("Subtype").AsName();
        if (cidFontType is null) return null;
        if (cidFontType != "CIDFontType0" && cidFontType != "CIDFontType2") return null;

        var info = new DescendantFontInfo { CidFontType = cidFontType };

        info.CidSystemInfo = ParseCidSystemInfo(cidFontDict, doc);

        // CIDToGIDMap applies to CIDFontType2 (TrueType) only; CIDFontType0 is CFF-keyed.
        if (cidFontType == "CIDFontType2")
        {
            var cidToGidObj = cidFontDict.Get("CIDToGIDMap");
            if (cidToGidObj is null)
            {
                info.CidToGidMap = OxCIDToGIDMap.Identity;
            }
            else
            {
                string? mapName = cidToGidObj.AsName();
                if (mapName is not null)
                {
                    // "Identity" is the only valid name; anything else is malformed and
                    // Identity is the safe reading.
                    info.CidToGidMap = OxCIDToGIDMap.Identity;
                }
                else
                {
                    byte[]? streamData = LoadStreamViaRef(doc, cidToGidObj);
                    if (streamData is null || streamData.Length == 0 || streamData.Length % 2 != 0)
                    {
                        info.CidToGidMap = OxCIDToGIDMap.Identity;
                    }
                    else
                    {
                        int numEntries = streamData.Length / 2;
                        var gids = new ushort[numEntries];
                        for (int i = 0; i < numEntries; i++)
                            gids[i] = (ushort)((streamData[i * 2] << 8) | streamData[i * 2 + 1]);
                        info.CidToGidMap = OxCIDToGIDMap.Explicit(gids);
                    }
                }
            }
        }

        // /DW (§9.7.4.3), default 1000 when absent.
        double? dw = Ox.Resolve(doc, cidFontDict.Get("DW")).AsNumber();
        info.HasExplicitDw = dw is not null;
        info.CidDefaultWidth = dw is not null ? (float)dw.Value : 1000.0f;

        info.CidWidths = ParseCidWidths(Ox.Arr(doc, cidFontDict.Get("W")));

        // /W2 and /DW2 (§9.7.4.3). Most fonts are horizontal-only, and then the per-CID map
        // is never allocated.
        var w2Array = Ox.Arr(doc, cidFontDict.Get("W2"));
        info.VerticalMetrics = ParseCidVerticalMetrics(w2Array);
        info.DefaultVerticalMetrics = ParseDw2(Ox.Arr(doc, cidFontDict.Get("DW2")));

        // A Type0 parent often has no embedded data — it is on the CIDFont.
        if (cidFontType == "CIDFontType2")
            info.TrueTypeCMap = ExtractTrueTypeCmapFromDescriptor(cidFontDict, doc);

        var (hasProgram, embedded) = ExtractEmbeddedFontFromDescriptor(cidFontDict, doc);
        info.HasFontProgram = hasProgram;
        info.EmbeddedFontData = embedded;

        var (descAscent, descDescent) = ReadRawAscentDescentFromDescriptor(cidFontDict, doc);
        info.RawAscent = descAscent;
        info.RawDescent = descDescent;

        return info;
    }

    /// <summary>CIDSystemInfo (§9.7.3): Registry, Ordering and Supplement of the collection.</summary>
    private static OxCIDSystemInfo? ParseCidSystemInfo(PdfDict cidFontDict, PdfDocument? doc)
    {
        var sysinfoDict = Ox.Dict(doc, cidFontDict.Get("CIDSystemInfo"));
        if (sysinfoDict is null) return null;

        var info = new OxCIDSystemInfo();
        byte[]? registry = sysinfoDict.Get("Registry").AsStringBytes();
        byte[]? ordering = sysinfoDict.Get("Ordering").AsStringBytes();
        if (registry is not null) info.Registry = System.Text.Encoding.UTF8.GetString(registry);
        if (ordering is not null) info.Ordering = System.Text.Encoding.UTF8.GetString(ordering);
        info.Supplement = (int)(sysinfoDict.Get("Supplement").AsLong() ?? 0);
        return info;
    }

    /// <summary>Extract a TrueType cmap from a dictionary's /FontDescriptor /FontFile2.</summary>
    private static IOxTrueTypeCMap? ExtractTrueTypeCmapFromDescriptor(PdfDict fontDict, PdfDocument? doc)
    {
        var descObj = fontDict.Get("FontDescriptor");
        var descDict = descObj is PdfRef ? Ox.Resolve(doc, descObj).AsDict() : descObj.AsDict();
        if (descDict is null) return null;

        byte[]? fontData = LoadStreamViaRef(doc, descDict.Get("FontFile2"));
        if (fontData is null || fontData.Length == 0) return null;

        var cmap = OxFontSeams.TrueType?.CMapFromFontData(fontData);
        return cmap is not null && !cmap.IsEmpty ? cmap : null;
    }

    /// <summary>Raw /Ascent and /Descent (1/1000 em) off a dictionary's /FontDescriptor.</summary>
    private static (float? Ascent, float? Descent) ReadRawAscentDescentFromDescriptor(PdfDict fontDict, PdfDocument? doc)
    {
        var descObj = fontDict.Get("FontDescriptor");
        var descDict = descObj is PdfRef ? Ox.Resolve(doc, descObj).AsDict() : descObj.AsDict();
        if (descDict is null) return (null, null);
        return ((float?)descDict.Get("Ascent").AsNumber(), (float?)descDict.Get("Descent").AsNumber());
    }

    /// <summary>
    /// Embedded font program off a dictionary's /FontDescriptor, trying FontFile2, FontFile3
    /// then FontFile. Key presence is reported separately from extraction success.
    /// </summary>
    private static (bool HasFontProgram, byte[]? Data) ExtractEmbeddedFontFromDescriptor(PdfDict fontDict, PdfDocument? doc)
    {
        var descObj = fontDict.Get("FontDescriptor");
        var descDict = descObj is PdfRef ? Ox.Resolve(doc, descObj).AsDict() : descObj.AsDict();
        if (descDict is null) return (false, null);

        string[] fontFileKeys = { "FontFile2", "FontFile3", "FontFile" };
        bool hasFontProgram = false;
        foreach (string key in fontFileKeys) hasFontProgram |= descDict.Has(key);

        foreach (string key in fontFileKeys)
        {
            var entry = descDict.Get(key);
            if (entry is null) continue;
            byte[]? fontData = LoadStreamViaRef(doc, entry);
            if (fontData is null || fontData.Length == 0) continue;

            if (key == "FontFile3" && fontData.Length > 4 && fontData[0] == 1)
                fontData = WrapCffInOpenType(fontData); // CFF version 1 needs a container
            return (hasFontProgram, fontData);
        }
        return (hasFontProgram, null);
    }

    // -----------------------------------------------------------------------
    // /W, /W2, /DW2
    // -----------------------------------------------------------------------

    /// <summary>
    /// Parse a CIDFont's /W array (§9.7.4.3). Two forms, freely intermixed:
    /// <c>c [w1 w2 …]</c> assigns successive CIDs, <c>cfirst clast w</c> a whole range.
    /// </summary>
    private static Dictionary<ushort, float>? ParseCidWidths(PdfArray? wArray)
    {
        if (wArray is null || wArray.Items.Count == 0) return null;

        var widths = new Dictionary<ushort, float>();
        var items = wArray.Items;
        int i = 0;

        while (i < items.Count)
        {
            if (items[i] is not PdfNumber { IsInteger: true } startNum) { i++; continue; }
            ushort cidStart = unchecked((ushort)startNum.AsLong);
            i++;
            if (i >= items.Count) break;

            if (items[i] is PdfArray widthArray)
            {
                for (int j = 0; j < widthArray.Items.Count; j++)
                {
                    double? w = widthArray.Items[j] is PdfNumber wn ? wn.Value : null;
                    if (w is null) continue;
                    ushort cid = SaturatingAddU16(cidStart, j);
                    widths[cid] = (float)w.Value;
                }
                i++;
            }
            else if (items[i] is PdfNumber { IsInteger: true } endNum)
            {
                ushort cidEnd = unchecked((ushort)endNum.AsLong);
                i++;
                if (i >= items.Count) break; // range with no width

                if (items[i] is not PdfNumber widthNum) { i++; continue; }
                float width = (float)widthNum.Value;
                i++;

                for (int cid = cidStart; cid <= cidEnd; cid++) widths[(ushort)cid] = width;
            }
            else
            {
                i++;
            }
        }

        return widths.Count == 0 ? null : widths;
    }

    /// <summary>
    /// Parse /W2 (§9.7.4.3). Form A is <c>c [ w1y v_x v_y … ]</c> — successive triples for
    /// CIDs c, c+1, …; form B is <c>c_first c_last w1y v_x v_y</c>.
    /// </summary>
    private static Dictionary<ushort, OxVerticalMetrics>? ParseCidVerticalMetrics(PdfArray? w2Array)
    {
        if (w2Array is null || w2Array.Items.Count == 0) return null;

        var metrics = new Dictionary<ushort, OxVerticalMetrics>();
        var items = w2Array.Items;
        int i = 0;

        while (i < items.Count)
        {
            if (items[i] is not PdfNumber { IsInteger: true } startNum) { i++; continue; }
            ushort cidStart = unchecked((ushort)startNum.AsLong);
            i++;
            if (i >= items.Count) break;

            if (items[i] is PdfArray triples)
            {
                // A triple is atomic: a non-numeric element drops the WHOLE triple, keeping
                // the CID alignment of the rest of the inner array. Advancing by one instead
                // would silently shift every subsequent CID by a slot.
                int j = 0;
                uint emitted = 0;
                while (j + 2 < triples.Items.Count)
                {
                    double? w1y = triples.Items[j] is PdfNumber a ? a.Value : null;
                    double? vx = triples.Items[j + 1] is PdfNumber b ? b.Value : null;
                    double? vy = triples.Items[j + 2] is PdfNumber c ? c.Value : null;

                    // Overflow is detected before writing: saturating would collapse every
                    // overflowing slot onto the same CID, so the walk stops instead.
                    long cid = (long)cidStart + emitted;
                    if (cid > ushort.MaxValue) break;

                    if (w1y is not null && vx is not null && vy is not null)
                        metrics[(ushort)cid] = new OxVerticalMetrics((float)w1y.Value, (float)vx.Value, (float)vy.Value);

                    emitted++;
                    j += 3;
                }
                i++;
            }
            else if (items[i] is PdfNumber { IsInteger: true } endNum)
            {
                ushort cidEnd = unchecked((ushort)endNum.AsLong);
                i++;
                if (i + 2 >= items.Count) break; // truncated range

                double? w1y = items[i] is PdfNumber a ? a.Value : null;
                if (w1y is null) { i += 3; continue; }
                double? vx = items[i + 1] is PdfNumber b ? b.Value : null;
                if (vx is null) { i += 3; continue; }
                double? vy = items[i + 2] is PdfNumber c ? c.Value : null;
                if (vy is null) { i += 3; continue; }
                i += 3;

                var metric = new OxVerticalMetrics((float)w1y.Value, (float)vx.Value, (float)vy.Value);
                for (int cid = cidStart; cid <= cidEnd; cid++) metrics[(ushort)cid] = metric;
            }
            else
            {
                i++;
            }
        }

        return metrics.Count == 0 ? null : metrics;
    }

    /// <summary>
    /// Parse /DW2 = <c>[v_y_default w1y_default]</c> (§9.7.4.3). The default v_x is always
    /// 500 — the spec gives no way to override it — and a missing or malformed array falls
    /// back to the spec defaults [880 -1000].
    /// </summary>
    private static OxVerticalMetrics ParseDw2(PdfArray? dw2Array)
    {
        if (dw2Array is null || dw2Array.Items.Count < 2) return OxVerticalMetrics.SpecDefault;
        if (dw2Array.Items[0] is not PdfNumber vy) return OxVerticalMetrics.SpecDefault;
        if (dw2Array.Items[1] is not PdfNumber w1y) return OxVerticalMetrics.SpecDefault;
        return new OxVerticalMetrics((float)w1y.Value, 500.0f, (float)vy.Value);
    }

    private static ushort SaturatingAddU16(ushort a, int b)
    {
        long sum = (long)a + b;
        return sum > ushort.MaxValue ? ushort.MaxValue : (ushort)sum;
    }

    // -----------------------------------------------------------------------
    // Raw CFF → OpenType
    // -----------------------------------------------------------------------

    /// <summary>
    /// Wrap raw CFF data in a minimal OpenType container (`head`, `hhea`, `maxp`, `CFF `) so
    /// a TrueType/OpenType parser will accept it — FontFile3 ships the bare CFF table.
    /// </summary>
    private static byte[] WrapCffInOpenType(byte[] cffData)
    {
        const ushort numTables = 4; // CFF + head + hhea + maxp
        const ushort searchRange = 32; // largest power of 2 ≤ numTables*16 = 64
        const ushort entrySelector = 2;
        const ushort rangeShift = (numTables * 16) - searchRange;

        // Minimal head table (54 bytes) — the OpenType-required fields.
        byte[] headTable =
        {
            0x00, 0x01, 0x00, 0x00, // majorVersion=1, minorVersion=0
            0x00, 0x01, 0x00, 0x00, // fontRevision=1.0
            0x00, 0x00, 0x00, 0x00, // checksumAdjustment
            0x5F, 0x0F, 0x3C, 0xF5, // magicNumber
            0x00, 0x0B,             // flags
            0x03, 0xE8,             // unitsPerEm = 1000
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // created
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // modified
            0xFF, 0x38,             // xMin = -200
            0xFF, 0x38,             // yMin = -200
            0x03, 0xE8,             // xMax = 1000
            0x03, 0xE8,             // yMax = 1000
            0x00, 0x00,             // macStyle
            0x00, 0x08,             // lowestRecPPEM = 8
            0x00, 0x02,             // fontDirectionHint
            0x00, 0x01,             // indexToLocFormat = 1 (long)
            0x00, 0x00,             // glyphDataFormat
        };

        byte[] hheaTable =
        {
            0x00, 0x01, 0x00, 0x00, // majorVersion=1, minorVersion=0
            0x03, 0x20,             // ascender = 800
            0xFF, 0x38,             // descender = -200
            0x00, 0x00,             // lineGap = 0
            0x04, 0x00,             // advanceWidthMax = 1024
            0x00, 0x00,             // minLeftSideBearing
            0x00, 0x00,             // minRightSideBearing
            0x04, 0x00,             // xMaxExtent = 1024
            0x00, 0x01,             // caretSlopeRise = 1
            0x00, 0x00,             // caretSlopeRun = 0
            0x00, 0x00,             // caretOffset = 0
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00,             // metricDataFormat = 0
            0x01, 0x00,             // numberOfHMetrics = 256
        };

        byte[] maxpTable =
        {
            0x00, 0x00, 0x50, 0x00, // version = 0.5 (CFF)
            0x01, 0x00,             // numGlyphs = 256
        };

        uint headerSize = 12 + (uint)numTables * 16;
        uint headOffset = (headerSize + 3) & ~3u;
        uint headLen = (uint)headTable.Length;
        uint hheaOffset = (headOffset + headLen + 3) & ~3u;
        uint hheaLen = (uint)hheaTable.Length;
        uint maxpOffset = (hheaOffset + hheaLen + 3) & ~3u;
        uint maxpLen = (uint)maxpTable.Length;
        uint cffOffset = (maxpOffset + maxpLen + 3) & ~3u;
        uint cffLen = (uint)cffData.Length;

        var outBytes = new List<byte>((int)(cffOffset + cffLen));

        void PutU16(ushort v) { outBytes.Add((byte)(v >> 8)); outBytes.Add((byte)v); }
        void PutU32(uint v)
        {
            outBytes.Add((byte)(v >> 24)); outBytes.Add((byte)(v >> 16));
            outBytes.Add((byte)(v >> 8)); outBytes.Add((byte)v);
        }
        void PutTag(string tag) { foreach (char c in tag) outBytes.Add((byte)c); }
        void PadTo(uint offset) { while (outBytes.Count < offset) outBytes.Add(0); }

        PutTag("OTTO");
        PutU16(numTables);
        PutU16(searchRange);
        PutU16(entrySelector);
        PutU16(rangeShift);

        // Table records, alphabetical by tag.
        PutTag("CFF "); PutU32(TableChecksum(cffData)); PutU32(cffOffset); PutU32(cffLen);
        PutTag("head"); PutU32(TableChecksum(headTable)); PutU32(headOffset); PutU32(headLen);
        PutTag("hhea"); PutU32(TableChecksum(hheaTable)); PutU32(hheaOffset); PutU32(hheaLen);
        PutTag("maxp"); PutU32(TableChecksum(maxpTable)); PutU32(maxpOffset); PutU32(maxpLen);

        PadTo(headOffset); outBytes.AddRange(headTable);
        PadTo(hheaOffset); outBytes.AddRange(hheaTable);
        PadTo(maxpOffset); outBytes.AddRange(maxpTable);
        PadTo(cffOffset); outBytes.AddRange(cffData);

        return outBytes.ToArray();
    }

    private static uint TableChecksum(byte[] data)
    {
        uint sum = 0;
        for (int i = 0; i < data.Length; i += 4)
        {
            uint word = 0;
            for (int b = 0; b < 4; b++)
            {
                word <<= 8;
                if (i + b < data.Length) word |= data[i + b];
            }
            unchecked { sum += word; }
        }
        return sum;
    }
}
