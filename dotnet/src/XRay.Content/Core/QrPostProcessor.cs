using System.Text;
using XRay.Content.Types;

namespace XRay.Content.Core;

/// <summary>
/// Decode the QR codes in a finished document's images and surface what they said, ported from
/// Rust <c>plugins/processor/builtin/qr.rs</c>.
/// </summary>
/// <remarks>
/// Payloads land in three places, each for a reason. On <c>ExtractedImage.QrCodes</c>, which is
/// where a caller looks for per-image detail. In the document text, because nothing in the
/// renderers reads that field — a payload that stayed there alone would never reach the content
/// a consumer actually processes. And, for URL-shaped payloads, in the document's URI list, so a
/// QR link is indistinguishable from a hyperlink found anywhere else.
/// </remarks>
public static class QrPostProcessor
{
    private const string WarningSource = "qr-codes";

    /// <summary>
    /// Schemes a payload must start with to count as a link. QR codes also carry plain text and
    /// <c>WIFI:</c> / <c>BEGIN:VCARD</c> blobs; those stay text rather than polluting the URI list.
    /// </summary>
    private static readonly string[] UriSchemes = { "http://", "https://", "mailto:" };

    /// <summary>Matches <c>InternalDocument</c>'s own per-document cap.</summary>
    private const int MaxUris = 100_000;

    public static void Process(ExtractedDocument result, ExtractionConfig config)
    {
        if (config.QrCodes != true) return;
        if (result.Images is not { Count: > 0 } images) return;

        var payloads = new List<string>();
        var lossyImages = new List<int>();

        for (int index = 0; index < images.Count; index++)
        {
            Cancel.ThrowIfRequested();
            var codes = QrDetection.Detect(images[index].Data, images[index].Format);
            foreach (var code in codes)
            {
                // A grid whose bytes are not valid UTF-8 is kept with the undecodable bytes
                // replaced — a partial result beats none — but the caller is told.
                if (code.Payload.Contains('�')) lossyImages.Add(index);
                payloads.Add(code.Payload);
            }
            images[index].QrCodes = codes.Cast<object>().ToList();
        }

        foreach (int index in lossyImages)
            result.ProcessingWarnings.Add(new ProcessingWarning
            {
                Source = WarningSource,
                Message = $"A QR code in image {index} decoded to bytes that are not valid UTF-8; the "
                          + "undecodable bytes were replaced with U+FFFD and the payload is included as-is",
            });

        if (payloads.Count == 0) return;

        AppendPayloadSection(result, config, payloads);
        CollectPayloadUris(result, payloads);
    }

    /// <summary>
    /// Append the payloads to the document text.
    /// </summary>
    /// <remarks>
    /// JSON, DocTags and a custom format are emitted verbatim by a renderer that owns their
    /// syntax, so a free-text section cannot be spliced into them — DocTags in particular is a
    /// tag stream with no free-text position, and an untagged section would stop it
    /// round-tripping. Those get a warning rather than a corrupted document.
    /// </remarks>
    private static void AppendPayloadSection(
        ExtractedDocument result, ExtractionConfig config, List<string> payloads)
    {
        bool rendersAsText = config.OutputFormat.Which is OutputFormat.Kind.Plain
            or OutputFormat.Kind.Markdown or OutputFormat.Kind.Djot or OutputFormat.Kind.Html
            or OutputFormat.Kind.Structured;

        if (rendersAsText)
        {
            string section = config.OutputFormat.Which == OutputFormat.Kind.Html
                ? HtmlSection(payloads)
                : MarkdownSection(payloads);
            result.Content = AppendSection(result.Content, section);
            return;
        }

        result.ProcessingWarnings.Add(new ProcessingWarning
        {
            Source = WarningSource,
            Message = $"{payloads.Count} decoded QR payload(s) could not be merged into the requested "
                      + "output format because it is produced verbatim by a renderer; the payloads are "
                      + "available on the per-image `qr_codes` field but are absent from the returned "
                      + "content, chunks and embeddings",
        });
    }

    /// <summary>Route URL-shaped payloads into the document's existing URI list.</summary>
    private static void CollectPayloadUris(ExtractedDocument result, List<string> payloads)
    {
        if (!payloads.Any(IsUriPayload)) return;

        result.Uris ??= new List<ExtractedUri>();
        int dropped = 0;

        foreach (string payload in payloads.Where(IsUriPayload))
        {
            var kind = ClassifyUri(payload);
            if (result.Uris.Any(u => u.Url == payload && u.Kind == kind)) continue;
            if (result.Uris.Count >= MaxUris) { dropped++; continue; }
            result.Uris.Add(new ExtractedUri { Url = payload, Label = null, Page = null, Kind = kind });
        }

        if (dropped > 0)
            result.ProcessingWarnings.Add(new ProcessingWarning
            {
                Source = "uris",
                Message = $"{dropped} QR payload URI(s) were dropped at the per-document limit of "
                          + $"{MaxUris} and are missing from the result",
            });
    }

    private static UriKind ClassifyUri(string url) =>
        url.StartsWith("mailto:", StringComparison.Ordinal) ? UriKind.Email
        : url.StartsWith('#') ? UriKind.Anchor
        : UriKind.Hyperlink;

    private static bool IsUriPayload(string payload) =>
        UriSchemes.Any(scheme => payload.Length > scheme.Length
                                 && payload.AsSpan(0, scheme.Length).Equals(scheme, StringComparison.OrdinalIgnoreCase));

    private static string MarkdownSection(List<string> payloads)
    {
        var section = new StringBuilder("## QR Codes\n\n");
        foreach (string payload in payloads) section.Append("- ").Append(payload).Append('\n');
        return section.ToString();
    }

    private static string HtmlSection(List<string> payloads)
    {
        var section = new StringBuilder("<h2>QR Codes</h2>\n<ul>\n");
        foreach (string payload in payloads)
            section.Append("<li>").Append(EscapeHtml(payload)).Append("</li>\n");
        section.Append("</ul>\n");
        return section.ToString();
    }

    /// <summary>The three characters that would otherwise break out of HTML text.</summary>
    private static string EscapeHtml(string text)
    {
        var escaped = new StringBuilder(text.Length);
        foreach (char c in text)
            escaped.Append(c switch { '&' => "&amp;", '<' => "&lt;", '>' => "&gt;", _ => c.ToString() });
        return escaped.ToString();
    }

    /// <summary>
    /// Append a section separated by exactly one blank line, and none at all when the target is
    /// empty — a document whose only text is a QR payload must not start with whitespace.
    /// </summary>
    private static string AppendSection(string target, string section)
    {
        var sb = new StringBuilder(target);
        if (sb.Length > 0)
        {
            if (sb[^1] != '\n') sb.Append('\n');
            if (sb.Length < 2 || sb[^2] != '\n') sb.Append('\n');
        }
        sb.Append(section);
        return sb.ToString();
    }
}
