// Ported from crates/xberg/src/extractors/email.rs (`extract_attachment_children` and the
// attachment arm of `build_internal_document`).

using XRay.Content.Core;
using XRay.Content.Types;

namespace XRay.Content.Internal.Email;

/// <summary>
/// Runs a message's attachments back through the extraction pipeline and appends whatever text
/// they yield to the message's own document.
/// <para>
/// Listing an attachment's name and size says nothing about what it contains, and for a message
/// whose body is a covering note the attachment <em>is</em> the document — one fixture here
/// carries 60 KB of text behind a 432-character body. Each attachment that extracts to something
/// non-empty contributes a level-2 heading naming it, followed by its text.
/// </para>
/// </summary>
internal static class AttachmentInlining
{
    /// <summary>
    /// Depth bound shared with the archive path: an attached archive of attached messages must
    /// terminate. Mirrors Rust's <c>max_archive_depth</c>, which each level decrements.
    /// </summary>
    private const int MaxDepth = 3;

    [ThreadStatic] private static int _depth;

    /// <summary>
    /// Append each attachment's extracted text to <paramref name="builder"/>. Attachments that
    /// carry no data, whose type cannot be pinned down, or that fail to extract are skipped —
    /// each one costs only its own text, so there is nothing to abort.
    /// </summary>
    public static void Append(
        InternalDocumentBuilder builder,
        IReadOnlyList<EmailAttachment> attachments,
        ExtractionConfig config)
    {
        if (_depth >= MaxDepth || attachments.Count == 0) return;

        for (int idx = 0; idx < attachments.Count; idx++)
        {
            Cancel.ThrowIfRequested();
            var attachment = attachments[idx];

            // A message/rfc822 part is body structure rather than an attachment, and upstream
            // keeps it children-only rather than inlining it into the parent.
            if (string.Equals(attachment.MimeType, "message/rfc822", StringComparison.OrdinalIgnoreCase))
                continue;

            byte[]? bytes = attachment.Data;
            if (bytes is null || bytes.Length == 0) continue;

            string filename = attachment.Filename ?? attachment.Name ?? $"attachment_{idx}";

            string? mime = ResolveMime(filename, bytes, attachment.MimeType);
            if (mime is null) continue;

            // An image contributes no text: what this port recovers from one is EXIF, which
            // belongs to the attachment's own metadata and not to the message body. Inlining it
            // would append an empty document envelope — `{"body":[{"type":"image"}]}` under json,
            // nothing at all under plain — which is content in neither case.
            if (mime.StartsWith("image/", StringComparison.OrdinalIgnoreCase)) continue;

            _depth++;
            try
            {
                var result = Extractor.Shared.Extract(ExtractInput.FromBytes(bytes, mime, filename), config);
                if (result.Errors.Count > 0) continue;

                string content = (result.Results.FirstOrDefault()?.Content ?? "").Trim();
                if (content.Length == 0) continue;

                builder.PushHeading(2, filename, null, null);
                builder.PushParagraph(content, new(), null, null);
            }
            catch (Exception) when (Cancel.NotCancelling)
            {
                // An attachment that will not parse is not a failure of the message.
            }
            finally
            {
                _depth--;
            }
        }
    }

    /// <summary>
    /// The attachment's type: what its bytes are, then the name it was sent under, then what the
    /// sender declared. If nothing pins it down the attachment is left alone rather than guessed
    /// at.
    /// </summary>
    /// <remarks>
    /// Content leads here — unlike a file on disk, where the extension leads and content only
    /// overrules it. An attachment's name is frequently wrong about its contents: one fixture
    /// sends undecoded base64 named `image.jpg`, which is text and which upstream extracts as
    /// text. The one exception is a container signature: every OLE compound file looks alike, so
    /// content alone answers "Word document" for an attached `.msg` too, and there the name is
    /// the better evidence.
    /// </remarks>
    private static string? ResolveMime(string filename, byte[] bytes, string? declared)
    {
        string? detected = Mime.DetectMimeTypeFromBytes(bytes);
        string? byName = Mime.DetectMimeType(filename, checkExists: false);

        bool detectedIsUseful = detected is not null
            && detected != Mime.OctetStream
            && !Mime.IsAmbiguousContainer(detected);

        if (!detectedIsUseful && byName is not null && byName != Mime.OctetStream) return byName;
        if (detected is not null && detected != Mime.OctetStream) return detected;
        return declared is not null && declared != Mime.OctetStream ? declared : null;
    }
}
