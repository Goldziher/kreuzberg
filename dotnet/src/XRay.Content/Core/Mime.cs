using System.Text;
using System.Text.Json;

namespace XRay.Content.Core;

/// <summary>
/// MIME detection ported from Rust `core/mime.rs`.
///
/// The extension→MIME table is ported verbatim. Content sniffing reproduces the common
/// signatures directly (the Rust code delegates to the `infer` crate + `mime_guess`; a
/// byte-exact port of those crates is deferred — see PORT_NOTES). The ZIP→Office/iWork/HWPX
/// subsequence scan and the UTF-8 text heuristics are ported faithfully.
/// </summary>
public static class Mime
{
    public const string OctetStream = "application/octet-stream";

    private static readonly Dictionary<string, string> ExtToMime = BuildExtTable();

    /// <summary>Extension-only detection (never reads content). Returns null when unknown.</summary>
    public static string? DetectMimeType(string path, bool checkExists, bool sourceCode = true)
    {
        if (checkExists && !File.Exists(path))
            throw new FileNotFoundException($"File does not exist: {path}", path);

        string ext = Path.GetExtension(path);
        if (ext.StartsWith('.')) ext = ext.Substring(1);
        ext = ext.ToLowerInvariant();
        if (ext.Length > 0 && ExtToMime.TryGetValue(ext, out var mime))
            return mime;

        // Only after the format table has had its say: `.json`, `.yaml`, `.md` and `.html` are
        // languages tree-sitter knows and formats xberg handles, and the format wins.
        if (sourceCode && Internal.Code.CodeLanguages.FromPath(path) is not null)
            return CodeMimeType;

        return null;
    }

    /// <summary>The MIME every source file resolves to, whatever language it turns out to be.</summary>
    public const string CodeMimeType = "text/x-source-code";

    /// <summary>GeoJSON — JSON syntax, routed to the structured extractor.</summary>
    public const string GeoJsonMimeType = "application/geo+json";

    /// <summary>KML — XML syntax, routed to the XML extractor.</summary>
    public const string KmlMimeType = "application/vnd.google-earth.kml+xml";

    /// <summary>A packaged OpenDocument drawing.</summary>
    public const string OdgMimeType = "application/vnd.oasis.opendocument.graphics";

    /// <summary>
    /// The flat single-file XML variant of <see cref="OdgMimeType"/> (<c>.fodg</c>): the whole
    /// package's <c>content.xml</c> inlined as one document, with no ZIP layer.
    /// </summary>
    public const string OdgFlatMimeType = "application/vnd.oasis.opendocument.graphics-flat-xml";

    /// <summary>The ODF namespace bound to the <c>office:</c> prefix, used to confirm that a flat
    /// ODF document's root really is <c>office:document</c> before trusting its
    /// <c>office:mimetype</c>.</summary>
    private const string OdfOfficeNamespace = "urn:oasis:names:tc:opendocument:xmlns:office:1.0";

    /// <summary>The MS-CFB (compound binary file) signature, shared by legacy .doc/.xls/.ppt.</summary>
    private static bool IsOle2(ReadOnlySpan<byte> b) =>
        b.Length >= 8 && b[0] == 0xD0 && b[1] == 0xCF && b[2] == 0x11 && b[3] == 0xE0 &&
        b[4] == 0xA1 && b[5] == 0xB1 && b[6] == 0x1A && b[7] == 0xE1;

    /// <summary>
    /// Identify a legacy MS-CFB compound document (.doc/.xls/.ppt) from its root storage CLSID,
    /// returning null when the container declares one this port does not recognise.
    /// </summary>
    /// <remarks>
    /// Ports upstream's <c>detect_ole2_package</c>. The container magic is shared by every legacy
    /// Office binary format, so without this a <c>.xls</c> or <c>.ppt</c> whose extension is
    /// missing or wrong was answered as <c>application/msword</c>. A compound file cannot be
    /// typed from a truncated prefix — locating the root directory entry means following the FAT
    /// sector chain, and a chain built from a partial read references sectors the buffer does not
    /// contain — so this is only attempted against the whole file (xberg-io/xberg#1590).
    /// </remarks>
    private static string? DetectOle2Package(ReadOnlySpan<byte> content)
    {
        if (!IsOle2(content)) return null;
        try
        {
            return Internal.Cfb.CompoundFile.Open(content).RootClsid.ToString() switch
            {
                "00020810-0000-0000-c000-000000000046" or "00020820-0000-0000-c000-000000000046"
                    => "application/vnd.ms-excel",
                "00020906-0000-0000-c000-000000000046" => "application/msword",
                "64818d10-4f9b-11cf-86ea-00aa00b929e8" => "application/vnd.ms-powerpoint",
                _ => null,
            };
        }
        catch (Exception e) when (e is InvalidDataException or ArgumentException or IndexOutOfRangeException)
        {
            return null;
        }
    }

    /// <summary>Content-based detection. Returns null when the type cannot be determined.</summary>
    public static string? DetectMimeTypeFromBytes(ReadOnlySpan<byte> content, bool sourceCode = true)
    {
        string? magic = SniffMagic(content);
        if (magic is not null)
        {
            if (IsOle2(content) && DetectOle2Package(content) is { } ole2) return ole2;
            if (magic == "application/zip")
            {
                string? office = DetectOfficeFormatFromZip(content);
                if (office is not null) return office;
            }
            if (SupportedMimeTypes.Contains(magic) || magic.StartsWith("image/"))
            {
                // The magic sniff reads the `<?xml` declaration and stops at generic XML, so the
                // vocabulary check has to run before that result is returned. A caller may pass a
                // truncated header, so decode lossily: a split multi-byte character must not
                // suppress the check.
                if (IsGenericXmlMime(magic))
                {
                    string prolog = System.Text.Encoding.UTF8.GetString(
                        content[..Math.Min(content.Length, 8192)]);
                    string? vocabulary = XmlVocabulary(prolog.TrimStart());
                    if (vocabulary is not null) return vocabulary;
                }
                return magic;
            }
            // else fall through to PST/text heuristics
        }

        // PST
        if (content.Length >= 4 && content[0] == 0x21 && content[1] == 0x42 && content[2] == 0x44 && content[3] == 0x4E)
            return "application/vnd.ms-outlook-pst";

        // UTF-8 text heuristics
        string? text = TryDecodeUtf8(content);
        if (text is not null)
        {
            string trimmed = text.TrimStart();
            if ((trimmed.StartsWith('{') || trimmed.StartsWith('[')) && IsValidJson(text))
                return "application/json";
            // The HTML test must precede the generic `<` fallback. Behind it, the fallback claims
            // every tag first and the HTML test never runs — upstream's issue #235, which this
            // port had inherited. It went unnoticed while the extension always won; once content
            // can overrule the extension, every HTML file routes to the XML extractor.
            // The WHATWG sniffing table, which upstream reaches through `infer`. It sits inside
            // the UTF-8 branch, as upstream's own HTML test does: a file that does not decode is
            // not a document whose opening bytes can be read as markup, whatever they spell. A
            // corrupted download with an ISP error page stapled in front of a real PDF is exactly
            // that, and hoisting this ahead of the decode handed those files to the HTML
            // extractor, which emitted the PDF's raw bytes as a paragraph.
            if (SniffWhatwgHtml(content)) return "text/html";
            if (!trimmed.StartsWith("<?xml", StringComparison.Ordinal) && LooksLikeHtml(SkipLeadingComments(trimmed)))
                return "text/html";
            if (trimmed.StartsWith("<?xml", StringComparison.Ordinal) || trimmed.StartsWith('<'))
                return XmlVocabulary(trimmed) ?? "application/xml";
            if (trimmed.StartsWith("%PDF", StringComparison.Ordinal))
                return "application/pdf";
            // A shebang is the last thing checked: every signature above is more specific than
            // "some script", and a file that matched one of them is not source whatever its
            // first line says.
            if (sourceCode && Internal.Code.CodeLanguages.FromContent(trimmed) is not null)
                return CodeMimeType;
            return "text/plain";
        }

        return null;
    }

    /// <summary>
    /// Resolve the type of a file whose extension is already known, letting content overrule the
    /// extension when the two disagree.
    /// <para>
    /// An extension is a claim, not evidence, and the corpus is full of files where it is simply
    /// wrong — DocTags streams named <c>.doctags.txt</c>, markup saved as <c>.txt</c>. Where the
    /// content carries a recognisable signature, it decides.
    /// </para>
    /// </summary>
    /// <remarks>
    /// Three cases keep the extension despite disagreement, because the content signature is
    /// strictly less informative than the extension rather than in conflict with it: plain text
    /// says nothing at all; generic XML cannot tell FictionBook from DocBook; generic JSON cannot
    /// tell a notebook from line-delimited JSON.
    /// </remarks>
    public static string ResolveWithContent(string? extensionMime, ReadOnlySpan<byte> content, bool sourceCode = true)
    {
        if (extensionMime is null)
            return DetectMimeTypeFromBytes(content, sourceCode) ?? OctetStream;

        // Upstream reads a 4 KiB header rather than the whole file, which is visible behaviour:
        // the JSON check parses what it was given, so a large JSON body in a mis-named file does
        // not validate and does not override.
        var header = content.Length > MagicHeaderBytes ? content[..MagicHeaderBytes] : content;
        if (header.IsEmpty) return extensionMime;

        // A compound file is the exception to reading only the header: its magic names the
        // container, not the format, and the root directory entry that does name the format is
        // reached by following the FAT sector chain — through sectors a 4 KiB prefix does not
        // contain (xberg-io/xberg#1590). Resolve it over the whole buffer instead.
        string? fromMagic = IsOle2(content)
            ? DetectOle2Package(content)
            : DetectMimeTypeFromBytes(header, sourceCode);
        if (fromMagic is null || fromMagic == extensionMime) return extensionMime;

        if (fromMagic == "text/plain") return extensionMime;
        // A more specific vocabulary only outranks the generic syntax it is written in when this
        // port can actually extract it: `.atom` and `.gltf` are XML and JSON vocabularies nothing
        // here handles, so a file with one of those extensions is better served as the XML or
        // JSON the content says it is than as a type no extractor claims (upstream
        // `fix(mime): reject unsupported vocabulary MIME`).
        if (IsGenericXmlMime(fromMagic) && IsSpecificXmlMime(extensionMime)
            && SupportedMimeTypes.Contains(extensionMime)) return extensionMime;
        if (fromMagic == "application/json" && IsSpecificJsonMime(extensionMime)
            && SupportedMimeTypes.Contains(extensionMime)) return extensionMime;
        if (IsAmbiguousContainer(fromMagic)) return extensionMime;

        return SupportedMimeTypes.Contains(fromMagic) || fromMagic.StartsWith("image/", StringComparison.Ordinal)
            ? fromMagic
            : extensionMime;
    }

    /// <summary>How much of a file upstream reads when checking content against the extension.</summary>
    private const int MagicHeaderBytes = 4096;

    private const string DocbookMimeType = "application/docbook+xml";
    private const string JatsMimeType = "application/x-jats+xml";

    /// <summary>
    /// The XML vocabulary <paramref name="trimmed"/> declares, if it declares one.
    /// </summary>
    /// <remarks>
    /// Real DocBook and JATS documents use the <c>.xml</c> extension, so the extension map alone
    /// routes them to the generic XML extractor and their structure and equations are lost. The
    /// test is structural rather than a search of the text: a public identifier counts only
    /// inside the DOCTYPE declaration, and a namespace only when the root element declares it, so
    /// a stylesheet, schema or catalog that merely names DocBook keeps its generic routing.
    /// </remarks>
    private static string? XmlVocabulary(string trimmed)
    {
        string? doctype = DeclarationOf(trimmed, "<!DOCTYPE");
        if (doctype is not null)
        {
            if (doctype.Contains("//OASIS//DTD DocBook", StringComparison.Ordinal)) return DocbookMimeType;
            if (doctype.Contains("//NLM//DTD JATS", StringComparison.Ordinal)
                || doctype.Contains("//NLM//DTD Journal", StringComparison.Ordinal)) return JatsMimeType;
        }
        string? root = RootStartTag(trimmed);
        if (root is null) return null;
        if (RootIsInNamespace(root, "http://docbook.org/ns/docbook")) return DocbookMimeType;
        // A flat ODF document carries the packaged MIME type inside itself, as the root
        // element's `office:mimetype`, rather than as a separate file in a ZIP — so content
        // detection reads that attribute rather than sniffing a container.
        if (RootIsInNamespace(root, OdfOfficeNamespace)
            && RootLocalNameIs(root, "document")
            && RootAttributeValue(root, "office:mimetype") == OdgMimeType)
            return OdgFlatMimeType;
        return null;
    }

    /// <summary>The local name of the root element, ignoring any namespace prefix.</summary>
    private static bool RootLocalNameIs(string root, string localName)
    {
        string name = root.TrimStart('<').Split(' ', '\t', '\n', '\r', '>', '/')[0];
        int colon = name.IndexOf(':');
        return (colon >= 0 ? name[(colon + 1)..] : name) == localName;
    }

    /// <summary>The value of a quoted attribute on the root start tag, or null when absent.</summary>
    private static string? RootAttributeValue(string root, string attribute)
    {
        int start = root.IndexOf(attribute + "=", StringComparison.Ordinal);
        if (start < 0) return null;
        string value = root[(start + attribute.Length + 1)..];
        if (value.Length == 0) return null;
        char quote = value[0];
        if (quote != '"' && quote != '\'') return null;
        int end = value.IndexOf(quote, 1);
        return end > 0 ? value[1..end] : null;
    }

    /// <summary>
    /// Whether the root element itself belongs to <paramref name="ns"/>. A declaration alone
    /// proves nothing — an XSL stylesheet that transforms DocBook binds the namespace on its own
    /// root — so the element belongs to it only when the binding it carries is the one its name
    /// uses.
    /// </summary>
    private static bool RootIsInNamespace(string root, string ns)
    {
        string name = root.TrimStart('<').Split(' ', '\t', '\n', '\r', '>', '/')[0];
        int colon = name.IndexOf(':');
        string binding = colon >= 0 ? $"xmlns:{name[..colon]}=" : "xmlns=";
        int start = root.IndexOf(binding, StringComparison.Ordinal);
        if (start < 0) return false;
        string value = root[(start + binding.Length)..];
        if (value.Length == 0) return false;
        char quote = value[0];
        if (quote != '"' && quote != '\'') return false;
        int end = value.IndexOf(quote, 1);
        return end > 0 && value[1..end] == ns;
    }

    /// <summary>
    /// The declaration beginning with <paramref name="opener"/>, delimiters included. An internal
    /// subset may hold a <c>&gt;</c> inside its brackets, so the scan tracks bracket depth rather
    /// than searching for a <c>]</c> anywhere in the document — a <c>]</c> in the body would
    /// otherwise stretch the declaration over the whole file.
    /// </summary>
    private static string? DeclarationOf(string trimmed, string opener)
    {
        int start = trimmed.IndexOf(opener, StringComparison.Ordinal);
        if (start < 0) return null;
        string rest = trimmed[start..];
        int end = DoctypeEnd(rest[opener.Length..]);
        return end < 0 ? null : rest[..(opener.Length + end)];
    }

    /// <summary>
    /// The offset of the <c>&gt;</c> that closes a <c>&lt;!DOCTYPE</c> declaration whose tail
    /// starts at <paramref name="tail"/>, or -1 when it never closes.
    /// </summary>
    private static int DoctypeEnd(string tail)
    {
        int bracketDepth = 0;
        for (int i = 0; i < tail.Length; i++)
        {
            char c = tail[i];
            if (c == '[') bracketDepth++;
            else if (c == ']') { if (bracketDepth > 0) bracketDepth--; }
            else if (c == '>' && bracketDepth == 0) return i;
        }
        return -1;
    }

    /// <summary>The document's root start tag, skipping declarations and processing instructions.</summary>
    private static string? RootStartTag(string trimmed)
    {
        string rest = trimmed;
        while (true)
        {
            int open = rest.IndexOf('<');
            if (open < 0) return null;
            rest = rest[open..];
            if (rest.StartsWith("<?", StringComparison.Ordinal) || rest.StartsWith("<!", StringComparison.Ordinal))
            {
                int skip;
                if (rest.StartsWith("<!DOCTYPE", StringComparison.Ordinal))
                {
                    string? decl = DeclarationOf(rest, "<!DOCTYPE");
                    if (decl is null) return null;
                    skip = decl.Length;
                }
                else
                {
                    skip = rest.IndexOf('>');
                    if (skip < 0) return null;
                }
                if (skip + 1 > rest.Length) return null;
                rest = rest[(skip + 1)..];
                continue;
            }
            int close = rest.IndexOf('>');
            return close < 0 ? null : rest[..(close + 1)];
        }
    }

    private static bool IsGenericXmlMime(string mime) => mime is "application/xml" or "text/xml";

    private static bool IsSpecificXmlMime(string mime) =>
        mime != "application/xml"
        && (mime.EndsWith("+xml", StringComparison.Ordinal) || mime.Contains("xml+", StringComparison.Ordinal));

    /// <summary>
    /// Signatures that identify a container without identifying the format inside it, and so can
    /// never overrule an extension that names one.
    /// <para>
    /// An OLE compound file is the container for .doc, .xls, .ppt, .msg and .hwp alike; telling
    /// them apart needs the root storage's CLSID, which this port does not read, so the sniffer
    /// answers <c>application/msword</c> as a placeholder rather than a finding. A ZIP that is
    /// not recognised as one of the Office or ODF layouts is likewise just a ZIP — an .epub is
    /// one, and so is a .jar.
    /// </para>
    /// </summary>
    public static bool IsAmbiguousContainer(string mime) =>
        mime is "application/msword" or "application/zip";

    private static bool IsSpecificJsonMime(string mime) =>
        mime != "application/json"
        && (mime.EndsWith("+json", StringComparison.Ordinal)
            || mime is "application/x-ndjson" or "application/jsonl" or "application/x-jsonlines");

    /// <summary>
    /// Elements common enough at the start of a bare HTML fragment to identify one. A whole
    /// document announces itself with a doctype or an <c>&lt;html&gt;</c> tag; a fragment has
    /// only its first element to go on.
    /// </summary>
    private static readonly HashSet<string> HtmlFragmentElements = new(StringComparer.Ordinal)
    {
        "a", "b", "blockquote", "body", "br", "button", "div", "em", "figcaption", "figure",
        "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6", "head", "header", "hr", "i",
        "iframe", "img", "input", "label", "li", "main", "meta", "nav", "ol", "option", "p",
        "pre", "script", "select", "span", "strong", "style", "table", "tbody", "td", "textarea",
        "tfoot", "th", "thead", "tr", "ul",
    };

    /// <summary>
    /// Skip any leading comments, so the markup test sees the document's first real tag.
    /// </summary>
    /// <remarks>
    /// The WHATWG sniffing table upstream reaches through <c>infer</c> lists <c>&lt;!--</c> among
    /// the HTML openings outright — a document that starts with a comment is HTML. Applying that
    /// only after the content is known to be text keeps a binary file with an HTML preamble (a
    /// captive-portal page prepended to a PDF, which the corpus has) routed by its real
    /// signature, while a page whose first line is a <c>Last-Modified</c> note above the DOCTYPE
    /// is recognised rather than handed to the XML extractor as a tag outline.
    /// </remarks>
    private static string SkipLeadingComments(string trimmed)
    {
        while (trimmed.StartsWith("<!--", StringComparison.Ordinal))
        {
            int end = trimmed.IndexOf("-->", 4, StringComparison.Ordinal);
            if (end < 0) return trimmed;
            trimmed = trimmed[(end + 3)..].TrimStart();
        }
        return trimmed;
    }

    /// <summary>Whether markup opens as HTML rather than as some other XML vocabulary.</summary>
    private static bool LooksLikeHtml(string trimmed)
    {
        string lowered = (trimmed.Length > 16 ? trimmed[..16] : trimmed).ToLowerInvariant();
        if (lowered.StartsWith("<!doctype html", StringComparison.Ordinal)
            || lowered.StartsWith("<html", StringComparison.Ordinal))
            return true;

        if (!trimmed.StartsWith('<')) return false;
        string afterBracket = trimmed[1..];

        int nameLength = 0;
        while (nameLength < afterBracket.Length && char.IsAsciiLetterOrDigit(afterBracket[nameLength]))
            nameLength++;

        // The tag has to actually end here, so `<tr:foo>` — a namespace prefix that happens to
        // collide with an HTML element name — stays XML.
        if (nameLength >= afterBracket.Length) return false;
        if (afterBracket[nameLength] is not ('>' or ' ' or '/' or '\t' or '\n' or '\r')) return false;

        return HtmlFragmentElements.Contains(afterBracket[..nameLength].ToLowerInvariant());
    }

    /// <summary>
    /// The WHATWG mime-sniffing table's HTML openings, as <c>infer</c>'s <c>is_html</c>
    /// implements them: one of these tag names, case-insensitive, after leading whitespace, and
    /// terminated by a space or <c>&gt;</c> so that <c>&lt;HTML</c> on its own does not count.
    /// </summary>
    private static bool SniffWhatwgHtml(ReadOnlySpan<byte> content)
    {
        int i = 0;
        while (i < content.Length && content[i] is 0x09 or 0x0A or 0x0C or 0x0D or 0x20) i++;
        var buf = content[i..];
        foreach (string value in WhatwgHtmlOpenings)
        {
            if (buf.Length <= value.Length) continue;
            bool same = true;
            for (int k = 0; k < value.Length && same; k++)
            {
                char c = (char)buf[k];
                if (c is >= 'a' and <= 'z') c = (char)(c - 32);
                same = c == value[k];
            }
            if (same && (buf[value.Length] == 0x20 || buf[value.Length] == 0x3E)) return true;
        }
        return false;
    }

    private static readonly string[] WhatwgHtmlOpenings =
    {
        "<!DOCTYPE HTML", "<HTML", "<HEAD", "<SCRIPT", "<IFRAME", "<H1", "<DIV", "<FONT",
        "<TABLE", "<A", "<STYLE", "<TITLE", "<B", "<BODY", "<BR", "<P", "<!--",
    };

    private static string? SniffMagic(ReadOnlySpan<byte> b)
    {
        if (b.Length >= 3 && b[0] == 0xFF && b[1] == 0xD8 && b[2] == 0xFF) return "image/jpeg";
        if (b.Length >= 4 && b[0] == 0x89 && b[1] == 0x50 && b[2] == 0x4E && b[3] == 0x47) return "image/png";
        if (b.Length >= 3 && b[0] == 0x47 && b[1] == 0x49 && b[2] == 0x46) return "image/gif";
        if (b.Length >= 12 && b[8] == 0x57 && b[9] == 0x45 && b[10] == 0x42 && b[11] == 0x50) return "image/webp";
        if (b.Length >= 2 && b[0] == 0x42 && b[1] == 0x4D) return "image/bmp";
        if (b.Length >= 4 && ((b[0] == 0x49 && b[1] == 0x49 && b[2] == 0x2A && b[3] == 0x00) ||
                              (b[0] == 0x4D && b[1] == 0x4D && b[2] == 0x00 && b[3] == 0x2A))) return "image/tiff";
        if (b.Length >= 4 && b[0] == 0x25 && b[1] == 0x50 && b[2] == 0x44 && b[3] == 0x46) return "application/pdf";
        // Photoshop `8BPS`, PSD and PSB alike, as `infer` reports it. Upstream sniffs it too but has
        // no extractor for it; here it routes to PsdExtractor, an X-Ray addition.
        if (b.Length >= 4 && b[0] == 0x38 && b[1] == 0x42 && b[2] == 0x50 && b[3] == 0x53)
            return "image/vnd.adobe.photoshop";
        // OLE2 / CFB compound file (doc/xls/ppt/msg/hwp). The container magic names only the
        // container, so `DetectMimeTypeFromBytes` follows up with the root storage's CLSID;
        // `application/msword` is what a caller sees when that read cannot settle it.
        if (IsOle2(b)) return "application/msword";
        if (b.Length >= 6 && b[0] == 0x37 && b[1] == 0x7A && b[2] == 0xBC && b[3] == 0xAF && b[4] == 0x27 && b[5] == 0x1C)
            return "application/x-7z-compressed";
        if (b.Length >= 2 && b[0] == 0x1F && b[1] == 0x8B) return "application/gzip";
        // ZIP (PK\x03\x04) — catch-all; caller runs office subsequence scan.
        if (b.Length >= 4 && b[0] == 0x50 && b[1] == 0x4B &&
            (b[2] == 0x03 || b[2] == 0x05 || b[2] == 0x07)) return "application/zip";
        return null;
    }

    private static string? DetectOfficeFormatFromZip(ReadOnlySpan<byte> content)
    {
        (byte[] Marker, string Mime)[] markers =
        {
            (Encoding.ASCII.GetBytes("Contents/content.hpf"), "application/haansofthwpx"),
            (Encoding.ASCII.GetBytes("Index/Document.iwa"), "application/x-iwork-pages-sffpages"),
            (Encoding.ASCII.GetBytes("Index/CalculationEngine.iwa"), "application/x-iwork-numbers-sffnumbers"),
            (Encoding.ASCII.GetBytes("Index/Presentation.iwa"), "application/x-iwork-keynote-sffkey"),
            (Encoding.ASCII.GetBytes("word/document.xml"),
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
            (Encoding.ASCII.GetBytes("xl/workbook.xml"),
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
            (Encoding.ASCII.GetBytes("ppt/presentation.xml"),
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"),
        };
        foreach (var (marker, mime) in markers)
        {
            if (IndexOf(content, marker) >= 0) return mime;
        }
        return null;
    }

    private static int IndexOf(ReadOnlySpan<byte> haystack, ReadOnlySpan<byte> needle)
    {
        if (needle.Length == 0 || haystack.Length < needle.Length) return -1;
        for (int i = 0; i <= haystack.Length - needle.Length; i++)
        {
            Cancel.Every(i, 0xFFFF);
            if (haystack.Slice(i, needle.Length).SequenceEqual(needle)) return i;
        }
        return -1;
    }

    private static string? TryDecodeUtf8(ReadOnlySpan<byte> content)
    {
        try
        {
            var enc = new UTF8Encoding(encoderShouldEmitUTF8Identifier: false, throwOnInvalidBytes: true);
            return enc.GetString(content);
        }
        catch (DecoderFallbackException)
        {
            return null;
        }
    }

    private static bool IsValidJson(string text)
    {
        try
        {
            using var _ = JsonDocument.Parse(text);
            return true;
        }
        catch (JsonException)
        {
            return false;
        }
    }

    public static IReadOnlyList<string> GetExtensionsForMime(string mimeType) =>
        ExtToMime.Where(kv => kv.Value == mimeType).Select(kv => kv.Key).OrderBy(x => x, StringComparer.Ordinal).ToList();

    public static IReadOnlyList<(string Extension, string MimeType)> ListSupportedFormats() =>
        ExtToMime.Select(kv => (kv.Key, kv.Value)).OrderBy(x => x.Key, StringComparer.Ordinal).ToList();

    private static readonly HashSet<string> SupportedMimeTypes = new(ExtToMime.Values)
    {
        "text/troff", "text/x-mdoc", "text/x-pod", "text/x-dokuwiki", "text/x-gfm",
        "text/x-markdown-extra", "text/x-multimarkdown", "application/csl+json", "text/x-source-code",
    };

    private static Dictionary<string, string> BuildExtTable()
    {
        var m = new Dictionary<string, string>(StringComparer.Ordinal);
        void Add(string mime, params string[] exts) { foreach (var e in exts) m[e] = mime; }

        Add("text/plain", "txt");
        // Extension only, as upstream has it. The corpus's DocTags streams are named
        // `*.doctags.txt` and so resolve as plain text, which is why none of them reaches the
        // DocTags extractor and why adding a content sniff here would move them.
        Add(XRay.Content.Internal.DocTags.DocTagsMime.MimeType, "doctags");
        Add("text/markdown", "md", "markdown");
        Add("text/x-commonmark", "commonmark");
        Add("text/x-quarto", "qmd");
        Add("text/x-r-markdown", "rmd");
        Add("text/mdx", "mdx");
        Add("text/x-djot", "djot", "dj");
        Add("application/pdf", "pdf");
        Add("text/html", "html", "htm");
        Add("application/xhtml+xml", "xhtml", "xht");
        Add("application/vnd.openxmlformats-officedocument.wordprocessingml.document", "docx");
        Add("application/vnd.ms-word.document.macroEnabled.12", "docm");
        Add("application/vnd.openxmlformats-officedocument.wordprocessingml.template", "dotx");
        Add("application/vnd.ms-word.template.macroEnabled.12", "dotm");
        Add("application/msword", "doc", "dot");
        Add("application/vnd.oasis.opendocument.text", "odt");
        Add("application/vnd.openxmlformats-officedocument.presentationml.presentation", "pptx");
        Add("application/vnd.openxmlformats-officedocument.presentationml.slideshow", "ppsx");
        Add("application/vnd.ms-powerpoint.presentation.macroEnabled.12", "pptm");
        Add("application/vnd.openxmlformats-officedocument.presentationml.template", "potx");
        Add("application/vnd.ms-powerpoint.template.macroEnabled.12", "potm");
        Add("application/vnd.ms-powerpoint", "ppt", "pot", "pps");
        Add("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "xlsx");
        Add("application/vnd.openxmlformats-officedocument.spreadsheetml.template", "xltx");
        Add("application/vnd.ms-excel", "xls", "xlt", "xla");
        Add("application/vnd.ms-excel.sheet.macroEnabled.12", "xlsm");
        Add("application/vnd.ms-excel.sheet.binary.macroEnabled.12", "xlsb");
        Add("application/vnd.ms-excel.addin.macroEnabled.12", "xlam");
        Add("application/vnd.ms-excel.template.macroEnabled.12", "xltm");
        Add("application/vnd.oasis.opendocument.spreadsheet", "ods");
        Add("application/vnd.oasis.opendocument.presentation", "odp");
        Add("application/x-dbf", "dbf");
        Add("application/x-hwp", "hwp");
        Add("application/vnd.wordperfect", "wpd", "wp", "wp5", "wp6");
        Add("application/haansofthwpx", "hwpx");
        Add("image/bmp", "bmp");
        Add("image/gif", "gif");
        Add("image/jpeg", "jpg", "jpeg");
        Add("image/png", "png");
        Add("image/tiff", "tiff", "tif");
        Add("image/webp", "webp");
        Add("image/jp2", "jp2", "jpg2");
        Add("image/j2c", "j2c", "j2k", "jpc");
        Add("image/jpx", "jpx");
        Add("image/jpm", "jpm");
        Add("image/mj2", "mj2");
        Add("image/x-jbig2", "jbig2", "jb2");
        Add("image/heic", "heic");
        Add("image/heic-sequence", "heics");
        Add("image/heif", "heif", "hif");
        Add("image/heif-sequence", "heifs");
        Add("image/avif", "avif");
        Add("image/avcs", "avcs");
        Add("image/x-portable-anymap", "pnm");
        Add("image/x-portable-bitmap", "pbm");
        Add("image/x-portable-graymap", "pgm");
        Add("image/x-portable-pixmap", "ppm");
        // Not in upstream's table (its `mime_guess` fallback maps `.psd` to the same type, and
        // nothing extracts it); added for PsdExtractor, an X-Ray addition. `.psb` is the large
        // document variant of the same format.
        Add("image/vnd.adobe.photoshop", "psd", "psb");
        Add("text/csv", "csv");
        Add("text/tab-separated-values", "tsv");
        Add("application/json", "json");
        // Geospatial siblings of JSON and XML: each keeps its own MIME so a consumer can tell
        // what it is, and routes to the extractor for the syntax it is written in.
        Add(GeoJsonMimeType, "geojson");
        Add("application/x-ndjson", "jsonl", "ndjson");
        Add("application/x-yaml", "yaml", "yml");
        Add("application/toml", "toml");
        Add("application/xml", "xml");
        Add(KmlMimeType, "kml");
        Add(OdgFlatMimeType, "fodg");
        Add("image/svg+xml", "svg");
        Add("message/rfc822", "eml");
        Add("application/vnd.ms-outlook", "msg");
        Add("application/vnd.ms-outlook-pst", "pst");
        Add("application/zip", "zip");
        Add("application/x-tar", "tar");
        Add("application/gzip", "gz", "tgz");
        Add("application/x-7z-compressed", "7z");
        Add("text/x-rst", "rst");
        Add("text/asciidoc", "adoc", "asciidoc");
        Add("text/vtt", "vtt");
        Add("text/x-org", "org");
        Add("application/epub+zip", "epub");
        Add("application/rtf", "rtf");
        Add("application/x-bibtex", "bib");
        Add("application/x-research-info-systems", "ris");
        Add("application/x-pubmed", "nbib");
        Add("application/x-endnote+xml", "enw");
        Add("application/x-fictionbook+xml", "fb2");
        Add("application/xml+opml", "opml");
        Add("application/docbook+xml", "dbk", "docbook", "docbook4", "docbook5");
        Add("application/x-jats+xml", "jats", "nxml");
        Add("application/x-ipynb+json", "ipynb");
        Add("application/x-latex", "tex", "latex");
        Add("application/x-typst", "typst", "typ");
        Add("application/x-iwork-pages-sffpages", "pages");
        Add("application/x-iwork-numbers-sffnumbers", "numbers");
        Add("application/x-iwork-keynote-sffkey", "key");
        Add("audio/mpeg", "mp3", "mpga");
        Add("audio/mp4", "m4a");
        Add("audio/wav", "wav");
        Add("audio/webm", "webm");
        Add("video/mp4", "mp4", "mpeg");
        return m;
    }
}
