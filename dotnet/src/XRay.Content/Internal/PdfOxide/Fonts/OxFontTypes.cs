// Ported from pdf_oxide `fonts/font_dict.rs` — the supporting types and free
// functions of the font-dictionary layer:
//   Std14Flags (lines 20-33), Encoding (219-232), CIDToGIDMap (234-281),
//   CIDSystemInfo (283-302), VerticalMetrics (304-355),
//   wmode_from_predefined_cmap_name (341-355),
//   punctuation_unicode_for_glyph_name / is_non_sensible_symbol (5284-5316),
//   is_ligature_char / expand_ligature_char (5354-5445),
//   symbol_encoding_lookup (5446-5620), zapf_dingbats_encoding_lookup (5622-5741),
//   pdfdoc_encoding_lookup (5764-5806), builtin_encoding_looks_like_cipher (5832-5846),
//   standard_encoding_lookup entry point (5848-6144), shift_jis_to_unicode (5121-5138),
//   normalize_cjk_radical_forms (5149-5170), decode_cjk_raw_charcode (6146-6228),
//   lookup_predefined_cmap + CID_MAX_* (6230-6341), standard_font_metrics (6342-6365),
//   the Standard-14 width tables of std14_width (3088-3605),
// plus `fonts/provenance.rs` (MappingProvenance) and the two FontWeight helpers
// from `layout/text_block.rs` (from_pdf_value / is_bold, lines 603-627).
//
// Everything here is f32, as pdf_oxide is: the widths and thresholds downstream
// of this layer were calibrated in single precision.
using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text;

namespace XRay.Content.Internal.PdfOxide.Fonts;

// ---------------------------------------------------------------------------
// Seams to the sibling ports. Each interface is the narrow slice of another
// pdf_oxide module that the font dictionary reads; the concrete types are
// adapted onto them and registered on OxFontSeams. Nothing here allocates or
// locks — the extractor is single-threaded per page.
// ---------------------------------------------------------------------------

/// <summary>
/// Glyph name → Unicode. Ports `fonts/font_dict.rs::glyph_name_to_unicode` /
/// `glyph_name_to_unicode_string`, `fonts/character_mapper.rs::glyph_name_to_unicode`
/// and direct `ADOBE_GLYPH_LIST` lookups.
/// </summary>
internal interface IOxGlyphNames
{
    /// <summary>`glyph_name_to_unicode` — AGL, TeX-math names and uniXXXX/uXXXXX synthesis.</summary>
    char? GlyphNameToUnicode(string glyphName);

    /// <summary>`glyph_name_to_unicode_string` — compound names (`f_f` → "ff").</summary>
    string? GlyphNameToUnicodeString(string glyphName);

    /// <summary>
    /// `character_mapper::glyph_name_to_unicode` — the string-returning variant used by
    /// §9.10.2 Priority 3c on embedded post/charset names.
    /// </summary>
    string? MapGlyphNameToUnicodeString(string glyphName);

    /// <summary>Raw Adobe Glyph List lookup (`ADOBE_GLYPH_LIST.get`), no synthesis.</summary>
    char? AdobeGlyphListLookup(string glyphName);
}

/// <summary>A parsed-on-demand ToUnicode CMap — ports `fonts/cmap.rs::LazyCMap`.</summary>
internal interface IOxCMap
{
    /// <summary><c>LazyCMap::get()</c> succeeded, i.e. the stream parsed.</summary>
    bool IsParsed { get; }

    /// <summary><c>CMap::len()</c>; 0 when the stream did not parse.</summary>
    int Count { get; }

    /// <summary><c>CMap::get(&amp;code)</c>.</summary>
    string? Lookup(uint code);

    /// <summary><c>LazyCMap::wmode()</c>.</summary>
    byte Wmode { get; }
}

internal interface IOxCMapProvider
{
    /// <summary><c>LazyCMap::new(raw_stream)</c> — never parses eagerly.</summary>
    IOxCMap CreateLazy(byte[] rawStream);

    /// <summary><c>cmap::parse_wmode_directive_public</c> — the `/WMode N def` scan.</summary>
    byte? ParseWModeDirective(string cmapText);
}

/// <summary>Ports `fonts/truetype_cmap.rs::TrueTypeCMap`.</summary>
internal interface IOxTrueTypeCMap
{
    bool IsEmpty { get; }
    int Count { get; }
    char? GetUnicode(ushort gid);
    ushort? CodeToGid(ushort code);
}

internal interface IOxTrueTypeProvider
{
    /// <summary><c>TrueTypeCMap::from_font_data</c>; null when the font has no usable cmap.</summary>
    IOxTrueTypeCMap? CMapFromFontData(byte[] fontData);

    /// <summary>
    /// The embedded program's own glyph names indexed by GID (TrueType `post` Format 2 or
    /// CFF charset). Entries are null where the font has no name for that GID or the name
    /// is empty/`.notdef`; the whole result is null when the program carries no usable
    /// names at all (post Format 3, stripped subset).
    /// </summary>
    IReadOnlyList<string?>? GlyphNames(byte[] fontData);
}

/// <summary>Built-in encodings of embedded font programs (`fonts/type1_encoding.rs`, `fonts/cff_encoding.rs`).</summary>
internal interface IOxFontProgramEncodings
{
    /// <summary><c>type1_encoding::parse_type1_encoding</c>.</summary>
    IReadOnlyDictionary<byte, char>? ParseType1Encoding(byte[] fontData);

    /// <summary><c>cff_encoding::parse_cff_encoding</c>.</summary>
    IReadOnlyDictionary<byte, char>? ParseCffEncoding(byte[] fontData);

    /// <summary><c>cff_encoding::parse_cff_gid_mapping_with_pdf_encoding</c> — byte code → CFF GID.</summary>
    IReadOnlyDictionary<byte, ushort>? ParseCffGidMappingWithPdfEncoding(
        byte[] fontData, OxEncoding pdfEncoding, IReadOnlyDictionary<byte, string> differences);
}

/// <summary>The standard encoding tables (`standard_encoding_lookup`, §Annex D).</summary>
internal interface IOxEncodingTables
{
    /// <summary>
    /// Unicode string for <paramref name="code"/> in the named encoding
    /// (WinAnsiEncoding / MacRomanEncoding / StandardEncoding / PDFDocEncoding / …),
    /// or null when the code is undefined there.
    /// </summary>
    string? StandardEncodingLookup(string encoding, byte code);
}

/// <summary>CID → Unicode for Adobe's predefined character collections (`fonts/cid_mappings`).</summary>
internal interface IOxPredefinedCidUnicode
{
    uint? LookupAdobeGb1(ushort cid);
    uint? LookupAdobeJapan1(ushort cid);
    uint? LookupAdobeCns1(ushort cid);
    uint? LookupAdobeKorea1(ushort cid);
    uint? LookupAdobeArabic(ushort cid);
}

/// <summary>
/// Registry for the seams above. Unset seams degrade the affected §9.10.2 tier to a
/// miss rather than throwing, so a font still loads when a sibling module is absent.
/// </summary>
internal static class OxFontSeams
{
    public static IOxGlyphNames? GlyphNames { get; set; }
    public static IOxCMapProvider? CMaps { get; set; }
    public static IOxTrueTypeProvider? TrueType { get; set; }
    public static IOxFontProgramEncodings? FontPrograms { get; set; }
    public static IOxEncodingTables? EncodingTables { get; set; }
    public static IOxPredefinedCidUnicode? PredefinedCidUnicode { get; set; }
}

// ---------------------------------------------------------------------------
// Font-dictionary value types
// ---------------------------------------------------------------------------

/// <summary>Name-derived Standard-14 classification of a font, resolved once and memoized.</summary>
internal readonly struct OxStd14Flags
{
    public readonly bool IsTimes;
    public readonly bool IsCourier;
    public readonly bool IsBold;
    public readonly bool IsBoldItalic;
    public readonly bool IsHelvetica;
    public readonly bool IsItalic;

    public OxStd14Flags(bool isTimes, bool isCourier, bool isBold, bool isBoldItalic, bool isHelvetica, bool isItalic)
    {
        IsTimes = isTimes; IsCourier = isCourier; IsBold = isBold;
        IsBoldItalic = isBoldItalic; IsHelvetica = isHelvetica; IsItalic = isItalic;
    }
}

/// <summary>Font encoding: a named standard encoding, an explicit code→char map, or Identity.</summary>
internal sealed class OxEncoding
{
    internal enum Kind { Standard, Custom, Identity }

    public readonly Kind EncodingKind;
    /// <summary>Set for <see cref="Kind.Standard"/>.</summary>
    public readonly string? Name;
    /// <summary>Set for <see cref="Kind.Custom"/>.</summary>
    public readonly Dictionary<byte, char>? Map;

    private OxEncoding(Kind kind, string? name, Dictionary<byte, char>? map)
    { EncodingKind = kind; Name = name; Map = map; }

    public static OxEncoding Standard(string name) => new(Kind.Standard, name, null);
    public static OxEncoding Custom(Dictionary<byte, char> map) => new(Kind.Custom, null, map);
    /// <summary>Identity carries no state, so one instance serves every font.</summary>
    public static readonly OxEncoding Identity = new(Kind.Identity, null, null);

    public bool IsIdentity => EncodingKind == Kind.Identity;
    public bool IsStandard => EncodingKind == Kind.Standard;
    public bool IsCustom => EncodingKind == Kind.Custom;
}

/// <summary>
/// CID → GID mapping for Type 2 CIDFonts (ISO 32000-1 §9.7.4.2). Identity is the default
/// and by far the common case; the explicit form is a big-endian uint16 stream.
/// </summary>
internal sealed class OxCIDToGIDMap
{
    internal enum Kind { Identity, Explicit }

    public readonly Kind MapKind;
    public readonly ushort[]? GidArray;

    private OxCIDToGIDMap(Kind kind, ushort[]? gids) { MapKind = kind; GidArray = gids; }

    public static readonly OxCIDToGIDMap Identity = new(Kind.Identity, null);
    public static OxCIDToGIDMap Explicit(ushort[] gids) => new(Kind.Explicit, gids);

    public ushort GetGid(ushort cid)
    {
        if (MapKind == Kind.Identity || GidArray is null) return cid;
        // Out of range falls back to identity rather than .notdef: a truncated
        // CIDToGIDMap stream is common and dropping the glyph loses the text.
        return cid < GidArray.Length ? GidArray[cid] : cid;
    }
}

/// <summary>CIDFont character collection identifier (§9.7.3).</summary>
internal sealed class OxCIDSystemInfo
{
    public string Registry = "Unknown";
    public string Ordering = "Unknown";
    public int Supplement;
}

/// <summary>
/// Per-CID vertical-writing metrics from `/W2` (§9.7.4.3), in 1000ths of em.
/// </summary>
internal readonly struct OxVerticalMetrics : IEquatable<OxVerticalMetrics>
{
    /// <summary>Vertical advance along y; negative because vertical text advances downward.</summary>
    public readonly float W1y;
    /// <summary>x of the vector from the horizontal origin to the vertical origin.</summary>
    public readonly float Vx;
    /// <summary>y of that vector.</summary>
    public readonly float Vy;

    public OxVerticalMetrics(float w1y, float vx, float vy) { W1y = w1y; Vx = vx; Vy = vy; }

    /// <summary>Spec default per §9.7.4.3: origin (500, 880), displacement one full em down.</summary>
    public static readonly OxVerticalMetrics SpecDefault = new(-1000.0f, 500.0f, 880.0f);

    public bool Equals(OxVerticalMetrics other) => W1y == other.W1y && Vx == other.Vx && Vy == other.Vy;
    public override bool Equals(object? obj) => obj is OxVerticalMetrics o && Equals(o);
    public override int GetHashCode() => HashCode.Combine(W1y, Vx, Vy);
}

/// <summary>
/// Which §9.10.2 tier produced a character's Unicode value. A fact about how the value was
/// derived, not a judgment about whether the text is correct — <see cref="Fallback"/> means
/// the value was fabricated by the extractor, not read from the file.
/// </summary>
internal enum OxMappingProvenance
{
    ActualText,
    ToUnicode,
    EncodingName,
    PredefinedCMap,
    EmbeddedCmap,
    Fallback,
}

internal static class OxMappingProvenanceOps
{
    /// <summary>0 = most authoritative, 5 = least; `max` over a span yields its weakest tier.</summary>
    public static byte Rank(OxMappingProvenance p) => p switch
    {
        OxMappingProvenance.ActualText => 0,
        OxMappingProvenance.ToUnicode => 1,
        OxMappingProvenance.EncodingName => 2,
        OxMappingProvenance.PredefinedCMap => 3,
        OxMappingProvenance.EmbeddedCmap => 4,
        _ => 5,
    };

    public static bool IsFromFile(OxMappingProvenance p) => p != OxMappingProvenance.Fallback;

    /// <summary>Stable lowercase label, shared with every other binding.</summary>
    public static string AsStr(OxMappingProvenance p) => p switch
    {
        OxMappingProvenance.ActualText => "actual_text",
        OxMappingProvenance.ToUnicode => "to_unicode",
        OxMappingProvenance.EncodingName => "encoding",
        OxMappingProvenance.PredefinedCMap => "predefined_cmap",
        OxMappingProvenance.EmbeddedCmap => "embedded_cmap",
        _ => "fallback",
    };

    public static OxMappingProvenance Weaker(OxMappingProvenance a, OxMappingProvenance b) =>
        Rank(a) >= Rank(b) ? a : b;
}

/// <summary>The two <c>FontWeight</c> helpers the font dictionary needs (`layout/text_block.rs`).</summary>
internal static class OxFontWeightValues
{
    /// <summary>Rounds a /FontWeight value (Table 122) to the nearest standard weight.</summary>
    public static OxFontWeight FromPdfValue(int value) => value switch
    {
        <= 150 => OxFontWeight.Thin,
        <= 250 => OxFontWeight.ExtraLight,
        <= 350 => OxFontWeight.Light,
        <= 450 => OxFontWeight.Normal,
        <= 550 => OxFontWeight.Medium,
        <= 650 => OxFontWeight.SemiBold,
        <= 750 => OxFontWeight.Bold,
        <= 850 => OxFontWeight.ExtraBold,
        _ => OxFontWeight.Black,
    };

    public static bool IsBold(OxFontWeight w) => (int)w >= 600;
}

// ---------------------------------------------------------------------------
// Free functions of font_dict.rs
// ---------------------------------------------------------------------------

internal static class OxFontTables
{
    /// <summary>
    /// Writing mode implied by a predefined CMap name (§9.7.5.2, Table 118): a `-V` suffix
    /// or the bare legacy `V` declares vertical writing, everything else horizontal.
    /// </summary>
    public static byte WmodeFromPredefinedCMapName(string name) =>
        name == "V" || name.EndsWith("-V", StringComparison.Ordinal) ? (byte)1 : (byte)0;

    /// <summary>
    /// PDFDocEncoding (Annex D.2): ASCII below 128, a block of typographic characters at
    /// 128-159 that differs from Latin-1, and Latin-1 above.
    /// </summary>
    public static char? PdfDocEncodingLookup(byte code) => code switch
    {
        <= 0x7F => (char)code,
        0x80 => '•',  // bullet
        0x81 => '†',  // dagger
        0x82 => '‡',  // daggerdbl
        0x83 => '…',  // ellipsis
        0x84 => '—',  // emdash
        0x85 => '–',  // endash
        0x86 => 'ƒ',  // florin
        0x87 => '⁄',  // fraction
        0x88 => '‹',  // guilsinglleft
        0x89 => '›',  // guilsinglright
        0x8A => '−',  // minus (different from hyphen!)
        0x8B => '‰',  // perthousand
        0x8C => '„',  // quotedblbase
        0x8D => '“',  // quotedblleft
        0x8E => '”',  // quotedblright
        0x8F => '‘',  // quoteleft
        0x90 => '’',  // quoteright
        0x91 => '‚',  // quotesinglbase
        0x92 => '™',  // trademark
        0x93 => 'ﬁ',  // fi ligature
        0x94 => 'ﬂ',  // fl ligature
        0x95 => 'Ł',  // Lslash
        0x96 => 'Œ',  // OE
        0x97 => 'Š',  // Scaron
        0x98 => 'Ÿ',  // Ydieresis
        0x99 => 'Ž',  // Zcaron
        0x9A => 'ı',  // dotlessi
        0x9B => 'ł',  // lslash
        0x9C => 'œ',  // oe
        0x9D => 'š',  // scaron
        0x9E => 'ž',  // zcaron
        0x9F => null,      // undefined
        _ => (char)code,   // 0xA0-0xFF: ISO Latin-1
    };

    /// <summary>
    /// Named-encoding lookup. The tables themselves live in the encoding-tables port; the
    /// two arms that font_dict.rs answers on its own — PDFDocEncoding and the
    /// unknown-encoding ASCII identity — are kept here so a font still decodes Latin text
    /// when that seam is unregistered.
    /// </summary>
    public static string? StandardEncodingLookup(string encoding, byte code)
    {
        var tables = OxFontSeams.EncodingTables;
        if (tables is not null) return tables.StandardEncodingLookup(encoding, code);
        if (encoding == "PDFDocEncoding") return PdfDocEncodingLookup(code)?.ToString();
        return code >= 32 && code <= 127 ? ((char)code).ToString() : null;
    }

    /// <summary>Adobe Symbol font encoding (Annex D.4).</summary>
    public static char? SymbolEncodingLookup(byte code)
    {
        return code switch
        {
            // Greek lowercase letters
            0x61 => '\u03B1',  // alpha
            0x62 => '\u03B2',  // beta
            0x63 => '\u03C7',  // chi
            0x64 => '\u03B4',  // delta
            0x65 => '\u03B5',  // epsilon
            0x66 => '\u03C6',  // phi
            0x67 => '\u03B3',  // gamma
            0x68 => '\u03B7',  // eta
            0x69 => '\u03B9',  // iota
            0x6A => '\u03D5',  // phi1 (variant)
            0x6B => '\u03BA',  // kappa
            0x6C => '\u03BB',  // lambda
            0x6D => '\u03BC',  // mu
            0x6E => '\u03BD',  // nu
            0x6F => '\u03BF',  // omicron
            0x70 => '\u03C0',  // pi
            0x71 => '\u03B8',  // theta
            0x72 => '\u03C1',  // rho ← THE IMPORTANT ONE for Pearson's ρ!
            0x73 => '\u03C3',  // sigma
            0x74 => '\u03C4',  // tau
            0x75 => '\u03C5',  // upsilon
            0x76 => '\u03D6',  // omega1 (variant pi)
            0x77 => '\u03C9',  // omega
            0x78 => '\u03BE',  // xi
            0x79 => '\u03C8',  // psi
            0x7A => '\u03B6',  // zeta

            // Greek uppercase letters
            0x41 => '\u0391',  // Alpha
            0x42 => '\u0392',  // Beta
            0x43 => '\u03A7',  // Chi
            0x44 => '\u0394',  // Delta
            0x45 => '\u0395',  // Epsilon
            0x46 => '\u03A6',  // Phi
            0x47 => '\u0393',  // Gamma
            0x48 => '\u0397',  // Eta
            0x49 => '\u0399',  // Iota
            0x4B => '\u039A',  // Kappa
            0x4C => '\u039B',  // Lambda
            0x4D => '\u039C',  // Mu
            0x4E => '\u039D',  // Nu
            0x4F => '\u039F',  // Omicron
            0x50 => '\u03A0',  // Pi
            0x51 => '\u0398',  // Theta
            0x52 => '\u03A1',  // Rho
            0x53 => '\u03A3',  // Sigma
            0x54 => '\u03A4',  // Tau
            0x55 => '\u03A5',  // Upsilon
            0x57 => '\u03A9',  // Omega
            0x58 => '\u039E',  // Xi
            0x59 => '\u03A8',  // Psi
            0x5A => '\u0396',  // Zeta

            // Mathematical operators
            0xB1 => '\u00B1',  // plusminus
            0xB4 => '\u00F7',  // divide
            0xB5 => '\u221E',  // infinity
            0xB6 => '\u2202',  // partialdiff
            0xB7 => '\u2022',  // bullet
            0xB9 => '\u2260',  // notequal
            0xBA => '\u2261',  // equivalence
            0xBB => '\u2248',  // approxequal
            0xBC => '\u2026',  // ellipsis
            0xBE => '\u22A5',  // perpendicular
            0xBF => '\u2299',  // circleplus

            0xD0 => '\u00B0',  // degree
            0xD1 => '\u2207',  // gradient (nabla)
            0xD2 => '\u00AC',  // logicalnot
            0xD3 => '\u2227',  // logicaland
            0xD4 => '\u2228',  // logicalor
            0xD5 => '\u220F',  // product ← Product symbol!
            0xD6 => '\u221A',  // radical ← Square root!
            0xD7 => '\u22C5',  // dotmath
            0xD8 => '\u2295',  // circleplus
            0xD9 => '\u2297',  // circletimes

            0xDA => '\u2208',  // element
            0xDB => '\u2209',  // notelement
            0xDC => '\u2220',  // angle
            0xDD => '\u2207',  // gradient
            0xDE => '\u00AE',  // registered
            0xDF => '\u00A9',  // copyright
            0xE0 => '\u2122',  // trademark

            0xE1 => '\u2211',  // summation ← Summation symbol!
            0xE2 => '\u2282',  // propersubset
            0xE3 => '\u2283',  // propersuperset
            0xE4 => '\u2286',  // reflexsubset
            0xE5 => '\u2287',  // reflexsuperset
            0xE6 => '\u222A',  // union
            0xE7 => '\u2229',  // intersection
            0xE8 => '\u2200',  // universal
            0xE9 => '\u2203',  // existential
            0xEA => '\u00AC',  // logicalnot

            0xF1 => '\u3008',  // angleleft
            0xF2 => '\u222B',  // integral ← Integral symbol!
            0xF3 => '\u2320',  // integraltp
            0xF4 => '\u2321',  // integralbt
            0xF5 => '\u2293',  // square intersection
            0xF6 => '\u2294',  // square union
            0xF7 => '\u3009',  // angleright

            // Basic punctuation and symbols (overlap with ASCII)
            0x20 => ' ',  // space
            0x21 => '!',  // exclam
            0x22 => '\u2200',  // universal (sometimes mapped here)
            0x23 => '#',  // numbersign
            0x24 => '\u2203',  // existential (sometimes mapped here)
            0x25 => '%',  // percent
            0x26 => '&',  // ampersand
            0x27 => '\u220B',  // suchthat
            0x28 => '(',  // parenleft
            0x29 => ')',  // parenright
            0x2A => '\u2217',  // asteriskmath
            0x2B => '+',  // plus
            0x2C => ',',  // comma
            0x2D => '\u2212',  // minus
            0x2E => '.',  // period
            0x2F => '/',  // slash

            // Digits 0-9 (0x30-0x39) map to themselves
            >= 0x30 and <= 0x39 => (char)code,

            0x3A => ':',  // colon
            0x3B => ';',  // semicolon
            0x3C => '<',  // less
            0x3D => '=',  // equal
            0x3E => '>',  // greater
            0x3F => '?',  // question

            0x40 => '\u2245',  // congruent

            // Brackets and arrows
            0x5B => '[',  // bracketleft
            0x5C => '\u2234',  // therefore
            0x5D => ']',  // bracketright
            0x5E => '\u22A5',  // perpendicular
            0x5F => '_',  // underscore

            0x7B => '{',  // braceleft
            0x7C => '|',  // bar
            0x7D => '}',  // braceright
            0x7E => '\u223C',  // similar

            // Math operators previously missing from the Adobe Symbol set (Annex D.5).
            0xA3 => '\u2264',  // ≤ lessequal    (octal 243)
            0xA5 => '\u221E',  // ∞ infinity     (octal 245)
            0xB3 => '\u2265',  // ≥ greaterequal (octal 263)

            _ => null,
        };
    }

    /// <summary>Adobe ZapfDingbats font encoding (Annex D.5).</summary>
    public static char? ZapfDingbatsEncodingLookup(byte code)
    {
        return code switch
        {
            0x20 => ' ',  // space
            0x21 => '\u2701',  // scissors
            0x22 => '\u2702',  // scissors (filled)
            0x23 => '\u2703',  // scissors (outline)
            0x24 => '\u2704',  // scissors (small)
            0x25 => '\u260E',  // telephone
            0x26 => '\u2706',  // telephone (filled)
            0x27 => '\u2707',  // tape drive
            0x28 => '\u2708',  // airplane
            0x29 => '\u2709',  // envelope
            0x2A => '\u261B',  // hand pointing right
            0x2B => '\u261E',  // hand pointing right (filled)
            0x2C => '\u270C',  // victory hand
            0x2D => '\u270D',  // writing hand
            0x2E => '\u270E',  // pencil
            0x2F => '\u270F',  // pencil (filled)

            0x30 => '\u2710',  // pen nib
            0x31 => '\u2711',  // pen nib (filled)
            0x32 => '\u2712',  // pen nib (outline)
            0x33 => '\u2713',  // checkmark
            0x34 => '\u2714',  // checkmark (bold)
            0x35 => '\u2715',  // multiplication X
            0x36 => '\u2716',  // multiplication X (heavy)
            0x37 => '\u2717',  // ballot X
            0x38 => '\u2718',  // ballot X (heavy)
            0x39 => '\u2719',  // outlined Greek cross
            0x3A => '\u271A',  // heavy Greek cross
            0x3B => '\u271B',  // open center cross
            0x3C => '\u271C',  // heavy open center cross
            0x3D => '\u271D',  // Latin cross
            0x3E => '\u271E',  // Latin cross (shadowed)
            0x3F => '\u271F',  // Latin cross (outline)

            // Common symbols
            0x40 => '\u2720',  // Maltese cross
            0x41 => '\u2721',  // Star of David
            0x42 => '\u2722',  // four teardrop-spoked asterisk
            0x43 => '\u2723',  // four balloon-spoked asterisk
            0x44 => '\u2724',  // heavy four balloon-spoked asterisk
            0x45 => '\u2725',  // four club-spoked asterisk
            0x46 => '\u2726',  // black four pointed star
            0x47 => '\u2727',  // white four pointed star
            0x48 => '\u2605',  // black star
            0x49 => '\u2729',  // outlined black star
            0x4A => '\u272A',  // circled white star
            0x4B => '\u272B',  // circled black star
            0x4C => '\u272C',  // shadowed white star
            0x4D => '\u272D',  // heavy asterisk
            0x4E => '\u272E',  // eight spoke asterisk
            0x4F => '\u272F',  // eight pointed black star

            // More ornaments
            0x50 => '\u2730',  // eight pointed pinwheel star
            0x51 => '\u2731',  // heavy eight pointed pinwheel star
            0x52 => '\u2732',  // eight pointed star
            0x53 => '\u2733',  // eight pointed star (outlined)
            0x54 => '\u2734',  // eight pointed star (heavy)
            0x55 => '\u2735',  // six pointed black star
            0x56 => '\u2736',  // six pointed star
            0x57 => '\u2737',  // eight pointed star (black)
            0x58 => '\u2738',  // heavy eight pointed star
            0x59 => '\u2739',  // twelve pointed black star
            0x5A => '\u273A',  // sixteen pointed star
            0x5B => '\u273B',  // teardrop-spoked asterisk
            0x5C => '\u273C',  // open center teardrop-spoked asterisk
            0x5D => '\u273D',  // heavy teardrop-spoked asterisk
            0x5E => '\u273E',  // six petalled black and white florette
            0x5F => '\u273F',  // black florette

            // Geometric shapes
            0x60 => '\u2740',  // white florette
            0x61 => '\u2741',  // eight petalled outlined black florette
            0x62 => '\u2742',  // circled open center eight pointed star
            0x63 => '\u2743',  // heavy teardrop-spoked pinwheel asterisk
            0x64 => '\u2744',  // snowflake
            0x65 => '\u2745',  // tight trifoliate snowflake
            0x66 => '\u2746',  // heavy chevron snowflake
            0x67 => '\u2747',  // sparkle
            0x68 => '\u2748',  // heavy sparkle
            0x69 => '\u2749',  // balloon-spoked asterisk
            0x6A => '\u274A',  // eight teardrop-spoked propeller asterisk
            0x6B => '\u274B',  // heavy eight teardrop-spoked propeller asterisk

            // Arrows
            0x6C => '\u25CF',  // black circle
            0x6D => '\u25CB',  // white circle
            0x6E => '\u274D',  // shadowed white circle
            0x6F => '\u25A0',  // black square
            0x70 => '\u25A1',  // white square
            0x71 => '\u25A2',  // white square with rounded corners
            0x72 => '\u25A3',  // white square containing black small square
            0x73 => '\u25A4',  // square with horizontal fill
            0x74 => '\u25A5',  // square with vertical fill
            0x75 => '\u25A6',  // square with orthogonal crosshatch fill
            0x76 => '\u25A7',  // square with upper left to lower right fill
            0x77 => '\u25A8',  // square with upper right to lower left fill
            0x78 => '\u25A9',  // square with diagonal crosshatch fill
            0x79 => '\u25AA',  // black small square
            0x7A => '\u25AB',  // white small square

            // Circled digits (Annex D.6, octal 254–323), previously dropped. Codes
            // are the spec's octal CODE in hex; each range is contiguous in Unicode.
            >= 0xAC and <= 0xB5 => (char)(0x2460 + (code - 0xAC)),  // ① ⑩  a120 a129
            >= 0xB6 and <= 0xBF => (char)(0x2776 + (code - 0xB6)),  // ❶ ❿  a130 a139
            >= 0xC0 and <= 0xC9 => (char)(0x2780 + (code - 0xC0)),  // ➀ ➉  a140 a149
            >= 0xCA and <= 0xD3 => (char)(0x278A + (code - 0xCA)),  // ➊ ➓  a150 a159

            // Arrows (Annex D.6, octal 324–376): four singletons, then two runs.
            0xD4 => '\u2794',  // ➔ a160  heavy wide-headed rightwards arrow
            0xD5 => '\u2192',  // → a161  rightwards arrow
            0xD6 => '\u2194',  // ↔ a163  left right arrow
            0xD7 => '\u2195',  // ↕ a164  up down arrow
            >= 0xD8 and <= 0xEF => (char)(0x2798 + (code - 0xD8)),  // ➘ ➯  a196…a182
            >= 0xF1 and <= 0xFE => (char)(0x27B1 + (code - 0xF1)),  // ➱ ➾  a201…a191

            _ => null,
        };
    }

    /// <summary>Unicode typographic ligatures U+FB00–U+FB06.</summary>
    public static bool IsLigatureChar(char c) => c is 'ﬀ' or 'ﬁ' or 'ﬂ' or 'ﬃ'
        or 'ﬄ' or 'ﬅ' or 'ﬆ';

    /// <summary>Ligature → component letters. Not spec behaviour, but it keeps extracted text searchable.</summary>
    public static string? ExpandLigatureChar(char c) => c switch
    {
        'ﬀ' => "ff",
        'ﬁ' => "fi",
        'ﬂ' => "fl",
        'ﬃ' => "ffi",
        'ﬄ' => "ffl",
        'ﬅ' => "st",  // long s + t
        'ﬆ' => "st",
        _ => null,
    };

    /// <summary>The four punctuation glyph names the Item 1 interceptions can recover from.</summary>
    public static string? PunctuationUnicodeForGlyphName(string name) => name switch
    {
        "period" => ".",
        "comma" => ",",
        "hyphen" => "-",
        "minus" => "−",
        _ => null,
    };

    /// <summary>
    /// True when <paramref name="s"/> is a single symbol/arrow/math glyph that clearly is not
    /// the punctuation a `period`/`comma`/`hyphen`/`minus` name denotes. Gates the Item 1
    /// interceptions so a correctly-decoded period never enters them.
    /// </summary>
    public static bool IsNonSensibleSymbol(string s)
    {
        if (s.Length != 1) return false;
        char c = s[0];
        if (char.IsLetter(c) || (c >= '0' && c <= '9') || IsAsciiPunctuation(c)) return false;
        int cp = c;
        return (cp >= 0x00A1 && cp <= 0x00BF) || (cp >= 0x2190 && cp <= 0x2BFF);
    }

    private static bool IsAsciiPunctuation(char c) =>
        (c >= '!' && c <= '/') || (c >= ':' && c <= '@') || (c >= '[' && c <= '`') || (c >= '{' && c <= '~');

    /// <summary>
    /// Carry CJK Radicals Supplement / Kangxi Radicals codepoints to their unified ideograph.
    /// A font cmap that maps a shared glyph to the *radical* codepoint surfaces e.g. 欠→⽋;
    /// NFKC on just those two (contiguous) blocks fixes it without touching real text.
    /// </summary>
    public static string NormalizeCjkRadicalForms(string s)
    {
        bool any = false;
        foreach (char c in s) { if (c >= 0x2E80 && c <= 0x2FDF) { any = true; break; } }
        if (!any) return s;

        var sb = new StringBuilder(s.Length);
        foreach (char c in s)
        {
            if (c >= 0x2E80 && c <= 0x2FDF) sb.Append(c.ToString().Normalize(NormalizationForm.FormKC));
            else sb.Append(c);
        }
        return sb.ToString();
    }

    /// <summary>Decode a Shift-JIS code (one or two bytes) to a single character.</summary>
    public static char? ShiftJisToUnicode(ushort code)
    {
        byte[] bytes = code <= 0xFF
            ? new[] { (byte)code }
            : new[] { (byte)(code >> 8), (byte)(code & 0xFF) };
        string? decoded = DecodeStrict(932, bytes);
        if (decoded is null || decoded.Length != 1) return null;
        return decoded[0];
    }

    /// <summary>
    /// Decode a raw CJK multi-byte character code with a legacy encoding.
    ///
    /// Fonts using named CJK CMaps (GBK-EUC-H, ETen-B5-H, KSC-EUC-H, …) put a raw
    /// multi-byte value in the content stream, not an Adobe CID: the CID tables reject
    /// those values, and the caller's `char::from_u32` fallback would map them into
    /// unrelated Hangul. Returns null for Identity/UCS2 CMaps, so trying it first is safe.
    /// </summary>
    public static string? DecodeCjkRawCharCode(uint charCode, string encName, OxCIDSystemInfo? cidSystemInfo)
    {
        string ordering = cidSystemInfo?.Ordering ?? "";

        // The bare predefined CMaps "H"/"V" are (overwhelmingly) Adobe-Japan1-H/V and carry
        // JIS X 0208 in GL form (both bytes 0x21-0x7E). Lift GL→EUC by OR-ing 0x8080 so the
        // EUC-JP decoder sees them; without this, non-embedded Japanese comes out as Latin.
        if ((encName == "H" || encName == "V") && (ordering == "Japan1" || ordering.Length == 0))
        {
            uint hi = (charCode >> 8) & 0xFF;
            uint lo = charCode & 0xFF;
            if (hi >= 0x21 && hi <= 0x7E && lo >= 0x21 && lo <= 0x7E)
            {
                string? decoded = DecodeStrict(51932, new[] { (byte)(hi | 0x80), (byte)(lo | 0x80) });
                if (decoded is not null)
                {
                    string r = decoded.Replace("�", "");
                    if (r.Length > 0) return r;
                }
            }
            if (charCode <= 0x7E)
            {
                string? c = CharFromU32(charCode);
                if (c is not null) return c;
            }
        }

        int? codePage = null;
        if (encName.Contains("GBK", StringComparison.Ordinal)
            || encName.Contains("GB-", StringComparison.Ordinal)
            || encName.Contains("GBpc", StringComparison.Ordinal)
            || (encName.Contains("EUC", StringComparison.Ordinal)
                && (ordering == "GB1" || encName.StartsWith("GB", StringComparison.Ordinal))))
        {
            codePage = 936; // GBK
        }
        else if (encName.Contains("B5", StringComparison.Ordinal)
            || encName.Contains("CNS", StringComparison.Ordinal)
            || (encName.Contains("EUC", StringComparison.Ordinal) && ordering == "CNS1"))
        {
            codePage = 950; // Big5
        }
        else if (encName.Contains("EUC", StringComparison.Ordinal) && ordering == "Japan1")
        {
            codePage = 51932; // EUC-JP
        }
        else if ((encName.Contains("KSC", StringComparison.Ordinal)
            || encName.Contains("KSCms", StringComparison.Ordinal)) && ordering == "Korea1")
        {
            codePage = 51949; // EUC-KR
        }

        if (codePage is null) return null;

        byte[] bytes = { (byte)((charCode >> 8) & 0xFF), (byte)(charCode & 0xFF) };
        string? text = DecodeStrict(codePage.Value, bytes);
        if (text is null) return null;
        string result = text.Replace("�", "");
        return result.Length == 0 ? null : result;
    }

    /// <summary>
    /// Decode with a legacy code page, treating any malformed sequence as a failure —
    /// the equivalent of encoding_rs's <c>had_errors</c>, which the callers branch on.
    /// </summary>
    private static string? DecodeStrict(int codePage, byte[] bytes)
    {
        try
        {
            var enc = Encoding.GetEncoding(codePage, EncoderFallback.ExceptionFallback, DecoderFallback.ExceptionFallback);
            return enc.GetString(bytes);
        }
        catch (Exception) when (Cancel.NotCancelling)
        {
            return null;
        }
    }

    // Maximum valid CID per Adobe collection. CIDs beyond these have no defined Unicode
    // mapping, so they are rejected before the table lookup rather than wrapping around.
    //   Adobe-GB1-5 (TN #5079), Adobe-Japan1-7 (#5078), Adobe-CNS1-7 (#5080),
    //   Adobe-Korea1-2 (#5093).
    private const ushort CidMaxGb1 = 30_283;
    private const ushort CidMaxJapan1 = 23_059;
    private const ushort CidMaxCns1 = 20_316;
    private const ushort CidMaxKorea1 = 18_351;

    /// <summary>
    /// Unicode code point for a CID in a predefined Unicode-based CMap (§9.7.5.2). Falls back
    /// to matching on the /CIDSystemInfo Ordering alone, because producers name identity
    /// encoding CMaps freely ("Adobe-Japan1-2") while the collection stays authoritative.
    /// </summary>
    public static uint? LookupPredefinedCMap(string cmapName, OxCIDSystemInfo? cidSystemInfo, ushort cid)
    {
        if (cidSystemInfo is null) return null;
        var tables = OxFontSeams.PredefinedCidUnicode;
        if (tables is null) return null;

        ushort maxCid;
        switch (cidSystemInfo.Ordering)
        {
            case "GB1": maxCid = CidMaxGb1; break;
            case "Japan1": maxCid = CidMaxJapan1; break;
            case "CNS1": maxCid = CidMaxCns1; break;
            case "Korea1": maxCid = CidMaxKorea1; break;
            // Adobe-Arabic-1 / Adobe-Persian-1 reject unmapped CIDs themselves.
            case "Arabic":
            case "Persian": maxCid = ushort.MaxValue; break;
            default: return null;
        }
        if (cid > maxCid) return null;

        string ordering = cidSystemInfo.Ordering;
        if (cmapName == "UniGB-UCS2-H" && ordering == "GB1") return tables.LookupAdobeGb1(cid);
        if (cmapName == "UniJIS-UCS2-H" && ordering == "Japan1") return tables.LookupAdobeJapan1(cid);
        if (cmapName == "UniCNS-UCS2-H" && ordering == "CNS1") return tables.LookupAdobeCns1(cid);
        if (cmapName == "UniKS-UCS2-H" && ordering == "Korea1") return tables.LookupAdobeKorea1(cid);
        return ordering switch
        {
            "GB1" => tables.LookupAdobeGb1(cid),
            "Japan1" => tables.LookupAdobeJapan1(cid),
            "CNS1" => tables.LookupAdobeCns1(cid),
            "Korea1" => tables.LookupAdobeKorea1(cid),
            "Arabic" or "Persian" => tables.LookupAdobeArabic(cid),
            _ => null,
        };
    }

    /// <summary>
    /// Whether an embedded program's built-in encoding is a re-indexed subset *cipher* rather
    /// than a handful of non-standard slots to overlay on the producer-declared named base.
    /// A real encoding agrees with the named base on most shared codes; a cipher on almost
    /// none, and overlaying one rewrites every mapped code into mojibake. Empty overlap is
    /// treated as not-a-cipher — no evidence either way.
    /// </summary>
    public static bool BuiltinEncodingLooksLikeCipher(IReadOnlyDictionary<byte, char> progEnc, string stdName)
    {
        uint agree = 0, overlap = 0;
        foreach (var kv in progEnc)
        {
            string? us = StandardEncodingLookup(stdName, kv.Key);
            if (us is null || us.Length == 0) continue;
            overlap++;
            if (us[0] == kv.Value) agree++;
        }
        return overlap > 0 && (float)agree / overlap < 0.5f;
    }

    /// <summary>Ascent/descent (fractions of em) for the 14 standard fonts, from the Adobe AFMs.</summary>
    public static (float Ascent, float Descent)? StandardFontMetrics(string baseFont)
    {
        int pos = baseFont.IndexOf('+');
        string name = pos >= 0 ? baseFont.Substring(pos + 1) : baseFont;
        return name switch
        {
            "Courier" or "Courier-Bold" or "Courier-Oblique" or "Courier-BoldOblique" => (0.629f, -0.157f),
            "Helvetica" or "Helvetica-Bold" or "Helvetica-Oblique" or "Helvetica-BoldOblique" => (0.718f, -0.207f),
            "Times-Roman" => (0.683f, -0.217f),
            "Times-Bold" => (0.676f, -0.205f),
            "Times-Italic" => (0.683f, -0.205f),
            "Times-BoldItalic" => (0.683f, -0.205f),
            "Symbol" => (1.010f, -0.293f),
            "ZapfDingbats" => (0.820f, -0.143f),
            _ => null,
        };
    }

    /// <summary>
    /// Rust's <c>char::from_u32</c>: rejects surrogates and values above U+10FFFF, which the
    /// Type0 CID-as-Unicode fallbacks rely on to refuse nonsense code points.
    /// </summary>
    public static string? CharFromU32(uint cp)
    {
        if (cp > 0x10FFFF || (cp >= 0xD800 && cp <= 0xDFFF)) return null;
        return char.ConvertFromUtf32((int)cp);
    }

    /// <summary>Rust's <c>char::is_control</c> — Unicode category Cc.</summary>
    public static bool IsControlCodePoint(uint cp) => cp <= 0x1F || (cp >= 0x7F && cp <= 0x9F);
}

/// <summary>
/// Standard-14 width tables (Adobe Core 14 AFM metrics), used when a font ships without a
/// /Widths array or the code falls outside [FirstChar, LastChar].
/// </summary>
internal static class OxStd14
{
    /// <summary>Width in 1000ths of em, or null when the code is outside the table.</summary>
    public static float? Width(OxStd14Flags std14, bool isTimes, bool isBold, byte code)
    {
        if (isTimes)
        {
            if (std14.IsBoldItalic)
            {
                return code switch
                {
                    32 => 250.0f,
                    33 => 389.0f,
                    34 => 555.0f,
                    35 => 500.0f,
                    36 => 500.0f,
                    37 => 833.0f,
                    38 => 778.0f,
                    39 => 333.0f,
                    40 => 333.0f,
                    41 => 333.0f,
                    42 => 500.0f,
                    43 => 570.0f,
                    44 => 250.0f,
                    45 => 333.0f,
                    46 => 250.0f,
                    47 => 278.0f,
                    >= 48 and <= 57 => 500.0f,
                    58 => 333.0f,
                    59 => 333.0f,
                    60 => 570.0f,
                    61 => 570.0f,
                    62 => 570.0f,
                    63 => 500.0f,
                    64 => 832.0f,
                    65 => 667.0f,
                    66 => 667.0f,
                    67 => 667.0f,
                    68 => 722.0f,
                    69 => 667.0f,
                    70 => 667.0f,
                    71 => 722.0f,
                    72 => 778.0f,
                    73 => 389.0f,
                    74 => 500.0f,
                    75 => 667.0f,
                    76 => 611.0f,
                    77 => 889.0f,
                    78 => 722.0f,
                    79 => 722.0f,
                    80 => 611.0f,
                    81 => 722.0f,
                    82 => 667.0f,
                    83 => 556.0f,
                    84 => 611.0f,
                    85 => 722.0f,
                    86 => 667.0f,
                    87 => 889.0f,
                    88 => 667.0f,
                    89 => 611.0f,
                    90 => 611.0f,
                    91 => 333.0f,
                    92 => 278.0f,
                    93 => 333.0f,
                    94 => 570.0f,
                    95 => 500.0f,
                    97 => 500.0f,
                    98 => 500.0f,
                    99 => 444.0f,
                    100 => 500.0f,
                    101 => 444.0f,
                    102 => 333.0f,
                    103 => 500.0f,
                    104 => 556.0f,
                    105 => 278.0f,
                    106 => 278.0f,
                    107 => 500.0f,
                    108 => 278.0f,
                    109 => 778.0f,
                    110 => 556.0f,
                    111 => 500.0f,
                    112 => 500.0f,
                    113 => 500.0f,
                    114 => 389.0f,
                    115 => 389.0f,
                    116 => 278.0f,
                    117 => 556.0f,
                    118 => 444.0f,
                    119 => 667.0f,
                    120 => 500.0f,
                    121 => 444.0f,
                    122 => 389.0f,
                    _ => null,
                };
            }
            if (isBold)
            {
                return code switch
                {
                    32 => 250.0f,
                    33 => 333.0f,
                    34 => 555.0f,
                    35 => 500.0f,
                    36 => 500.0f,
                    37 => 1000.0f,
                    38 => 833.0f,
                    39 => 333.0f,
                    40 => 333.0f,
                    41 => 333.0f,
                    42 => 500.0f,
                    43 => 570.0f,
                    44 => 250.0f,
                    45 => 333.0f,
                    46 => 250.0f,
                    47 => 278.0f,
                    >= 48 and <= 57 => 500.0f,
                    58 => 333.0f,
                    59 => 333.0f,
                    60 => 570.0f,
                    61 => 570.0f,
                    62 => 570.0f,
                    63 => 500.0f,
                    64 => 930.0f,
                    65 => 722.0f,
                    66 => 667.0f,
                    67 => 722.0f,
                    68 => 722.0f,
                    69 => 667.0f,
                    70 => 611.0f,
                    71 => 778.0f,
                    72 => 778.0f,
                    73 => 389.0f,
                    74 => 500.0f,
                    75 => 778.0f,
                    76 => 667.0f,
                    77 => 944.0f,
                    78 => 722.0f,
                    79 => 778.0f,
                    80 => 611.0f,
                    81 => 778.0f,
                    82 => 722.0f,
                    83 => 556.0f,
                    84 => 667.0f,
                    85 => 722.0f,
                    86 => 722.0f,
                    87 => 1000.0f,
                    88 => 722.0f,
                    89 => 722.0f,
                    90 => 667.0f,
                    91 => 333.0f,
                    92 => 278.0f,
                    93 => 333.0f,
                    94 => 581.0f,
                    95 => 500.0f,
                    97 => 500.0f,
                    98 => 556.0f,
                    99 => 444.0f,
                    100 => 556.0f,
                    101 => 444.0f,
                    102 => 333.0f,
                    103 => 500.0f,
                    104 => 556.0f,
                    105 => 278.0f,
                    106 => 333.0f,
                    107 => 556.0f,
                    108 => 278.0f,
                    109 => 833.0f,
                    110 => 556.0f,
                    111 => 500.0f,
                    112 => 556.0f,
                    113 => 556.0f,
                    114 => 444.0f,
                    115 => 389.0f,
                    116 => 333.0f,
                    117 => 556.0f,
                    118 => 500.0f,
                    119 => 722.0f,
                    120 => 500.0f,
                    121 => 500.0f,
                    122 => 444.0f,
                    _ => null,
                };
            }
            if (std14.IsItalic)
            {
                return code switch
                {
                    32 => 250.0f,
                    33 => 333.0f,
                    34 => 420.0f,
                    35 => 500.0f,
                    36 => 500.0f,
                    37 => 833.0f,
                    38 => 778.0f,
                    39 => 333.0f,
                    40 => 333.0f,
                    41 => 333.0f,
                    42 => 500.0f,
                    43 => 675.0f,
                    44 => 250.0f,
                    45 => 333.0f,
                    46 => 250.0f,
                    47 => 278.0f,
                    >= 48 and <= 57 => 500.0f,
                    58 => 333.0f,
                    59 => 333.0f,
                    60 => 675.0f,
                    61 => 675.0f,
                    62 => 675.0f,
                    63 => 500.0f,
                    64 => 920.0f,
                    65 => 611.0f,
                    66 => 611.0f,
                    67 => 667.0f,
                    68 => 722.0f,
                    69 => 611.0f,
                    70 => 611.0f,
                    71 => 722.0f,
                    72 => 722.0f,
                    73 => 333.0f,
                    74 => 444.0f,
                    75 => 667.0f,
                    76 => 556.0f,
                    77 => 833.0f,
                    78 => 667.0f,
                    79 => 722.0f,
                    80 => 611.0f,
                    81 => 722.0f,
                    82 => 611.0f,
                    83 => 500.0f,
                    84 => 556.0f,
                    85 => 722.0f,
                    86 => 611.0f,
                    87 => 833.0f,
                    88 => 611.0f,
                    89 => 556.0f,
                    90 => 556.0f,
                    91 => 389.0f,
                    92 => 278.0f,
                    93 => 389.0f,
                    94 => 422.0f,
                    95 => 500.0f,
                    97 => 500.0f,
                    98 => 500.0f,
                    99 => 444.0f,
                    100 => 500.0f,
                    101 => 444.0f,
                    102 => 278.0f,
                    103 => 500.0f,
                    104 => 500.0f,
                    105 => 278.0f,
                    106 => 278.0f,
                    107 => 444.0f,
                    108 => 278.0f,
                    109 => 722.0f,
                    110 => 500.0f,
                    111 => 500.0f,
                    112 => 500.0f,
                    113 => 500.0f,
                    114 => 389.0f,
                    115 => 389.0f,
                    116 => 278.0f,
                    117 => 500.0f,
                    118 => 444.0f,
                    119 => 667.0f,
                    120 => 444.0f,
                    121 => 444.0f,
                    122 => 389.0f,
                    _ => null,
                };
            }
            return code switch
            {
                32 => 250.0f,
                33 => 333.0f,
                34 => 408.0f,
                35 => 500.0f,
                36 => 500.0f,
                37 => 833.0f,
                38 => 778.0f,
                39 => 333.0f,
                40 => 333.0f,
                41 => 333.0f,
                42 => 500.0f,
                43 => 564.0f,
                44 => 250.0f,
                45 => 333.0f,
                46 => 250.0f,
                47 => 278.0f,
                48 => 500.0f,
                49 => 500.0f,
                50 => 500.0f,
                51 => 500.0f,
                52 => 500.0f,
                53 => 500.0f,
                54 => 500.0f,
                55 => 500.0f,
                56 => 500.0f,
                57 => 500.0f,
                58 => 278.0f,
                59 => 278.0f,
                60 => 564.0f,
                61 => 564.0f,
                62 => 564.0f,
                63 => 444.0f,
                64 => 921.0f,
                65 => 722.0f,
                66 => 667.0f,
                67 => 667.0f,
                68 => 722.0f,
                69 => 611.0f,
                70 => 556.0f,
                71 => 722.0f,
                72 => 722.0f,
                73 => 333.0f,
                74 => 389.0f,
                75 => 722.0f,
                76 => 611.0f,
                77 => 889.0f,
                78 => 722.0f,
                79 => 722.0f,
                80 => 556.0f,
                81 => 722.0f,
                82 => 667.0f,
                83 => 556.0f,
                84 => 611.0f,
                85 => 722.0f,
                86 => 722.0f,
                87 => 944.0f,
                88 => 722.0f,
                89 => 722.0f,
                90 => 611.0f,
                91 => 333.0f,
                92 => 278.0f,
                93 => 333.0f,
                97 => 444.0f,
                98 => 500.0f,
                99 => 444.0f,
                100 => 500.0f,
                101 => 444.0f,
                102 => 333.0f,
                103 => 500.0f,
                104 => 500.0f,
                105 => 278.0f,
                106 => 278.0f,
                107 => 500.0f,
                108 => 278.0f,
                109 => 778.0f,
                110 => 500.0f,
                111 => 500.0f,
                112 => 500.0f,
                113 => 500.0f,
                114 => 333.0f,
                115 => 389.0f,
                116 => 278.0f,
                117 => 500.0f,
                118 => 500.0f,
                119 => 722.0f,
                120 => 500.0f,
                121 => 500.0f,
                122 => 444.0f,
                _ => null,
            };
        }

        if (std14.IsHelvetica)
        {
            if (isBold)
            {
                return code switch
                {
                    32 => 278.0f,
                    33 => 333.0f,
                    34 => 474.0f,
                    44 => 278.0f,
                    45 => 333.0f,
                    46 => 278.0f,
                    47 => 278.0f,
                    >= 48 and <= 57 => 556.0f,
                    58 => 333.0f,
                    59 => 333.0f,
                    65 => 722.0f,
                    66 => 722.0f,
                    67 => 722.0f,
                    68 => 722.0f,
                    69 => 667.0f,
                    70 => 611.0f,
                    71 => 778.0f,
                    72 => 722.0f,
                    73 => 278.0f,
                    74 => 556.0f,
                    75 => 722.0f,
                    76 => 611.0f,
                    77 => 833.0f,
                    78 => 722.0f,
                    79 => 778.0f,
                    80 => 667.0f,
                    81 => 778.0f,
                    82 => 722.0f,
                    83 => 667.0f,
                    84 => 611.0f,
                    85 => 722.0f,
                    86 => 667.0f,
                    87 => 944.0f,
                    88 => 667.0f,
                    89 => 667.0f,
                    90 => 611.0f,
                    97 => 556.0f,
                    98 => 611.0f,
                    99 => 556.0f,
                    100 => 611.0f,
                    101 => 556.0f,
                    102 => 333.0f,
                    103 => 611.0f,
                    104 => 611.0f,
                    105 => 278.0f,
                    106 => 278.0f,
                    107 => 556.0f,
                    108 => 278.0f,
                    109 => 889.0f,
                    110 => 611.0f,
                    111 => 611.0f,
                    112 => 611.0f,
                    113 => 611.0f,
                    114 => 389.0f,
                    115 => 556.0f,
                    116 => 333.0f,
                    117 => 611.0f,
                    118 => 556.0f,
                    119 => 778.0f,
                    120 => 556.0f,
                    121 => 556.0f,
                    122 => 500.0f,
                    _ => null,
                };
            }
            return code switch
            {
                32 => 278.0f,
                33 => 278.0f,
                34 => 355.0f,
                44 => 278.0f,
                45 => 333.0f,
                46 => 278.0f,
                47 => 278.0f,
                >= 48 and <= 57 => 556.0f,  // digits
                58 => 278.0f,
                59 => 278.0f,
                65 => 667.0f,
                66 => 667.0f,
                67 => 722.0f,
                68 => 722.0f,
                69 => 667.0f,
                70 => 611.0f,
                71 => 778.0f,
                72 => 722.0f,
                73 => 278.0f,
                74 => 500.0f,
                75 => 667.0f,
                76 => 556.0f,
                77 => 833.0f,
                78 => 722.0f,
                79 => 778.0f,
                80 => 667.0f,
                81 => 778.0f,
                82 => 722.0f,
                83 => 667.0f,
                84 => 611.0f,
                85 => 722.0f,
                86 => 667.0f,
                87 => 944.0f,
                88 => 667.0f,
                89 => 667.0f,
                90 => 611.0f,
                97 => 556.0f,
                98 => 556.0f,
                99 => 500.0f,
                100 => 556.0f,
                101 => 556.0f,
                102 => 278.0f,
                103 => 556.0f,
                104 => 556.0f,
                105 => 222.0f,
                106 => 222.0f,
                107 => 500.0f,
                108 => 222.0f,
                109 => 833.0f,
                110 => 556.0f,
                111 => 556.0f,
                112 => 556.0f,
                113 => 556.0f,
                114 => 333.0f,
                115 => 500.0f,
                116 => 278.0f,
                117 => 556.0f,
                118 => 500.0f,
                119 => 722.0f,
                120 => 500.0f,
                121 => 500.0f,
                122 => 444.0f,
                _ => null,
            };
        }
        return null;
    }
}
