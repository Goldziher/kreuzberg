using XRay.Content.Rendering;
using XRay.Content.Types;

namespace XRay.Content.Core;

/// <summary>
/// Cancellable entry points for the library's lower-level public APIs, as extension methods so
/// the existing signatures — and every implementation of <see cref="IExtractor"/> — stay as
/// they are. Each installs the token for the extent of the call (see <see cref="Cancel"/>),
/// throws <see cref="OperationCanceledException"/> as soon as it is cancelled, and otherwise
/// returns exactly what the plain overload does.
/// </summary>
public static class CancellableApi
{
    /// <summary>
    /// Run an extractor directly, bypassing MIME detection and the pipeline's later stages.
    /// <see cref="Extractor.Extract(ExtractInput, ExtractionConfig, CancellationToken)"/> is the
    /// usual entry point; this one is for a caller that already holds the bytes and the type.
    /// </summary>
    public static InternalDocument Extract(
        this IExtractor extractor, byte[] content, string mimeType, ExtractionConfig config,
        CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        using var scope = Cancel.Begin(cancellationToken);
        return extractor.Extract(content, mimeType, config);
    }

    /// <summary><see cref="Derive.DeriveExtractionResult"/>, cancellable.</summary>
    public static ExtractedDocument DeriveExtractionResult(
        InternalDocument doc, bool includeDocumentStructure, OutputFormat outputFormat,
        HtmlOutputConfig? htmlOutput, bool renderPagesInOutputFormat, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        using var scope = Cancel.Begin(cancellationToken);
        return Derive.DeriveExtractionResult(
            doc, includeDocumentStructure, outputFormat, htmlOutput, renderPagesInOutputFormat);
    }

    /// <summary>Render a document as plain text, cancellable.</summary>
    public static string RenderPlain(InternalDocument doc, CancellationToken cancellationToken) =>
        Render(cancellationToken, () => PlainRenderer.Render(doc));

    /// <summary>Render a document as Markdown, cancellable.</summary>
    public static string RenderMarkdown(InternalDocument doc, CancellationToken cancellationToken) =>
        Render(cancellationToken, () => MarkdownRenderer.Render(doc));

    /// <summary>Render a document as HTML, cancellable.</summary>
    public static string RenderHtml(InternalDocument doc, CancellationToken cancellationToken) =>
        Render(cancellationToken, () => HtmlRenderer.Render(doc));

    /// <summary>Render a document as Djot, cancellable.</summary>
    public static string RenderDjot(InternalDocument doc, CancellationToken cancellationToken) =>
        Render(cancellationToken, () => DjotRenderer.Render(doc));

    /// <summary>Render a document as the JSON section tree, cancellable.</summary>
    public static string RenderJson(InternalDocument doc, CancellationToken cancellationToken) =>
        Render(cancellationToken, () => JsonRenderer.Render(doc));

    /// <summary><see cref="QrPostProcessor.Process"/>, cancellable.</summary>
    public static void ProcessQrCodes(
        ExtractedDocument result, ExtractionConfig config, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        using var scope = Cancel.Begin(cancellationToken);
        QrPostProcessor.Process(result, config);
    }

    /// <summary><see cref="QrDetection.Detect"/>, cancellable.</summary>
    public static List<QrCode> DetectQrCodes(
        ReadOnlySpan<byte> imageBytes, string? formatHint, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        using var scope = Cancel.Begin(cancellationToken);
        return QrDetection.Detect(imageBytes, formatHint);
    }

    /// <summary><see cref="Mime.DetectMimeTypeFromBytes"/>, cancellable: sniffing a large container scans it.</summary>
    public static string? DetectMimeTypeFromBytes(
        ReadOnlySpan<byte> content, bool sourceCode, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        using var scope = Cancel.Begin(cancellationToken);
        return Mime.DetectMimeTypeFromBytes(content, sourceCode);
    }

    private static string Render(CancellationToken cancellationToken, Func<string> render)
    {
        cancellationToken.ThrowIfCancellationRequested();
        using var scope = Cancel.Begin(cancellationToken);
        return render();
    }
}
