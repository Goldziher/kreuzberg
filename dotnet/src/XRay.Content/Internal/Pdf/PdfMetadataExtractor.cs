// PDF metadata extraction — ports crates/xberg/src/pdf/oxide/metadata.rs
// (info dictionary strings, date parsing, author/keyword splitting, PDF-specific fields).
using System.Text;

namespace XRay.Content.Internal.Pdf;

public sealed class PdfMetaResult
{
    public string? Title, Subject, CreatedBy, CreatedAt, ModifiedAt, Producer, PdfVersion;
    public List<string>? Authors, Keywords;
    public bool IsEncrypted;
    public long? Width, Height;
    public uint PageCount;

    /// <summary>The catalog's /Version when it names a later version than the file header.</summary>
    public string? CatalogVersion;

    /// <summary>The catalog's /Lang, else dc:language from the XMP packet.</summary>
    public string? Language;

    /// <summary>Info-dictionary entries beyond the standard ones, as written.</summary>
    public SortedDictionary<string, string>? CustomInfo;

    /// <summary>XMP properties beyond the ones that feed the standard fields.</summary>
    public SortedDictionary<string, string>? XmpExtra;

    /// <summary>What the /Encrypt dictionary declares and how the document opened.</summary>
    public PdfEncryptionInfo? Encryption;
}

public static class PdfMetadataExtractor
{
    public static PdfMetaResult Extract(PdfDocument doc)
    {
        var r = new PdfMetaResult();
        if (doc.VersionMajor > 0) r.PdfVersion = $"{doc.VersionMajor}.{doc.VersionMinor}";
        // The catalog's /Version overrides the header when it names a later one (ISO 32000-1
        // §7.5.2). `PdfVersion` stays the header's, as upstream reports it; the override is kept
        // apart so neither reading is lost.
        if (doc.Catalog is { } cat && doc.Resolve(cat.Get("Version")).AsName() is { } catalogVersion
            && IsLaterVersion(catalogVersion, doc.VersionMajor, doc.VersionMinor))
            r.CatalogVersion = catalogVersion;
        r.IsEncrypted = doc.IsEncrypted;
        r.Encryption = doc.Encryption;
        r.Language = doc.Catalog is { } c2 && doc.Resolve(c2.Get("Lang")).AsStringBytes() is { } langBytes
            ? DecodePdfString(langBytes) : null;
        r.PageCount = (uint)doc.PageCount;

        if (doc.PageCount > 0)
        {
            var (llx, lly, urx, ury) = doc.GetPageMediaBox(0);
            r.Width = (long)Math.Round(Math.Abs(urx - llx), MidpointRounding.AwayFromZero);
            r.Height = (long)Math.Round(Math.Abs(ury - lly), MidpointRounding.AwayFromZero);
        }

        var info = doc.InfoDict;
        r.Producer = GetInfoString(info, "Producer");
        r.Title = GetInfoString(info, "Title");
        r.Subject = GetInfoString(info, "Subject");
        r.CreatedBy = GetInfoString(info, "Creator");

        var author = GetInfoString(info, "Author");
        if (author != null) { var a = ParseAuthors(author); if (a.Count > 0) r.Authors = a; }
        var kw = GetInfoString(info, "Keywords");
        if (kw != null) { var k = ParseKeywords(kw); if (k.Count > 0) r.Keywords = k; }

        var cd = GetInfoString(info, "CreationDate");
        if (cd != null) r.CreatedAt = ParsePdfDate(cd);
        var md = GetInfoString(info, "ModDate");
        if (md != null) r.ModifiedAt = ParsePdfDate(md);

        if (info is not null)
            foreach (var kv in info.Map)
            {
                if (StandardInfoKeys.Contains(kv.Key)) continue;
                string? text = kv.Value switch
                {
                    PdfString s => DecodePdfString(s.Bytes),
                    PdfName n => n.Value,
                    PdfNumber n => n.Value.ToString(System.Globalization.CultureInfo.InvariantCulture),
                    PdfBool b => b.Value ? "true" : "false",
                    _ => null,
                };
                if (!string.IsNullOrEmpty(text)) (r.CustomInfo ??= new(StringComparer.Ordinal))[kv.Key] = text;
            }

        var xmp = PdfXmp.Extract(doc);
        ApplyXmpFallbacks(r, xmp);
        if (xmp is { Extra.Count: > 0 })
        {
            r.XmpExtra = xmp.Extra;
            r.Language ??= NonEmpty(xmp.Extra.GetValueOrDefault("dc:language"));
        }
        return r;
    }

    private static readonly HashSet<string> StandardInfoKeys = new(StringComparer.Ordinal)
    {
        "Title", "Author", "Subject", "Keywords", "Creator", "Producer", "CreationDate", "ModDate", "Trapped",
    };

    private static bool IsLaterVersion(string version, int major, int minor)
    {
        var parts = version.Split('.');
        return parts.Length == 2 && int.TryParse(parts[0], out int a) && int.TryParse(parts[1], out int b)
            && (a > major || (a == major && b > minor));
    }

    /// <summary>
    /// Fill in from the XMP packet whatever the Info dictionary left unset.
    /// </summary>
    /// <remarks>
    /// The Info dictionary wins wherever it says anything, because a producer that writes both
    /// keeps Info current and XMP is frequently a stale copy from an earlier save. XMP dates are
    /// already ISO 8601 and need none of the <c>D:YYYYMMDD</c> unpicking.
    /// <para>
    /// Info <c>/Subject</c> maps to <c>dc:description</c>, not <c>dc:subject</c> — that is
    /// Adobe's own mapping. <c>dc:description</c> is one descriptive string; <c>dc:subject</c> is
    /// a keyword bag, and it feeds Keywords instead.
    /// </para>
    /// </remarks>
    private static void ApplyXmpFallbacks(PdfMetaResult r, XmpMetadata? xmp)
    {
        if (xmp is null) return;
        r.Title ??= NonEmpty(xmp.DcTitle);
        r.Subject ??= NonEmpty(xmp.DcDescription);
        r.CreatedBy ??= NonEmpty(xmp.XmpCreatorTool);
        if (r.Authors is null && xmp.DcCreator.Count > 0) r.Authors = new List<string>(xmp.DcCreator);
        if (r.Keywords is null && xmp.DcSubject.Count > 0) r.Keywords = new List<string>(xmp.DcSubject);
        r.CreatedAt ??= NonEmpty(xmp.XmpCreateDate);
        r.ModifiedAt ??= NonEmpty(xmp.XmpModifyDate);
    }

    /// <summary>An XMP field may legally be present but empty; that is the same as absent.</summary>
    private static string? NonEmpty(string? value) =>
        string.IsNullOrWhiteSpace(value) ? null : value;

    private static string? GetInfoString(PdfDict? info, string key)
    {
        if (info == null) return null;
        var v = info.Get(key);
        if (v is PdfString s) return DecodePdfString(s.Bytes);
        if (v is PdfName n) { var t = n.Value.Trim(); return t.Length == 0 ? null : t; }
        return null;
    }

    public static string? DecodePdfString(byte[] bytes)
    {
        if (bytes.Length == 0) return null;
        string decoded;
        if (bytes.Length >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF)
        {
            var sb = new StringBuilder();
            for (int i = 2; i + 1 < bytes.Length; i += 2)
                sb.Append((char)((bytes[i] << 8) | bytes[i + 1]));
            decoded = sb.ToString();
        }
        else if (bytes.Length >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF)
        {
            decoded = Encoding.UTF8.GetString(bytes, 3, bytes.Length - 3);
        }
        else
        {
            // Try strict UTF-8, else Latin-1 (PDFDocEncoding approximation).
            if (IsValidUtf8(bytes)) decoded = Encoding.UTF8.GetString(bytes);
            else { var sb = new StringBuilder(bytes.Length); foreach (var b in bytes) sb.Append((char)b); decoded = sb.ToString(); }
        }
        decoded = decoded.Trim();
        return decoded.Length == 0 ? null : decoded;
    }

    private static bool IsValidUtf8(byte[] bytes)
    {
        try { var d = new UTF8Encoding(false, true); d.GetString(bytes); return true; }
        catch (Exception) when (Cancel.NotCancelling) { return false; }
    }

    public static List<string> ParseAuthors(string authorStr)
    {
        authorStr = authorStr.Replace(" and ", ", ");
        var authors = new List<string>();
        foreach (var segment in authorStr.Split(';'))
            foreach (var a in segment.Split(','))
            {
                var t = a.Trim();
                if (t.Length > 0) authors.Add(t);
            }
        return authors;
    }

    public static List<string> ParseKeywords(string kw)
    {
        return kw.Replace(';', ',').Split(',')
            .Select(k => k.Trim()).Where(k => k.Length > 0).ToList();
    }

    public static string ParsePdfDate(string dateStr)
    {
        string c = dateStr.Trim();
        if (c.StartsWith("D:") && c.Length >= 10)
        {
            string year = c.Substring(2, 4), month = c.Substring(6, 2), day = c.Substring(8, 2);
            if (c.Length >= 16)
                return $"{year}-{month}-{day}T{c.Substring(10, 2)}:{c.Substring(12, 2)}:{c.Substring(14, 2)}Z";
            if (c.Length >= 14)
                return $"{year}-{month}-{day}T{c.Substring(10, 2)}:{c.Substring(12, 2)}:00Z";
            return $"{year}-{month}-{day}T00:00:00Z";
        }
        if (c.Length >= 8)
            return $"{c.Substring(0, 4)}-{c.Substring(4, 2)}-{c.Substring(6, 2)}T00:00:00Z";
        return dateStr;
    }
}
