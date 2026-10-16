using System.Text.Json.Serialization;

namespace XRay.Content.Core;

/// <summary>
/// PDF-specific extraction settings, <see cref="ExtractionConfig.Pdf"/>. Upstream's
/// <c>PdfConfig</c> trimmed to what the native C# reader acts on, plus the port's own switches.
/// </summary>
public sealed class PdfOptions
{
    /// <summary>
    /// Password to open an encrypted document with, tried first. It is tried as the user password
    /// and then as the owner password, so either one works; a document that only restricts
    /// copying or printing needs none.
    /// </summary>
    /// <remarks>Never serialized, so a logged or persisted configuration does not leak it.</remarks>
    [JsonIgnore]
    public string? Password { get; set; }

    /// <summary>An owner password, tried after <see cref="Password"/> (as the user and as the owner password).</summary>
    [JsonIgnore]
    public string? OwnerPassword { get; set; }

    /// <summary>
    /// Further passwords to try, in order — upstream's <c>passwords</c>, for a caller that does not
    /// know which of several a document uses. The empty password is always tried last.
    /// </summary>
    [JsonIgnore]
    public List<string>? Passwords { get; set; }

    /// <summary>
    /// Extract page annotations (comments, highlights with the text they cover, stamps, links)
    /// into <see cref="Types.ExtractedDocument.Annotations"/>, append them to the rendered content
    /// as upstream does, and surface Link URIs in <see cref="Types.ExtractedDocument.Uris"/>.
    /// Off by default, as upstream's <c>extract_annotations</c> is.
    /// </summary>
    [JsonPropertyName("extract_annotations")]
    public bool ExtractAnnotations { get; set; }

    /// <summary>
    /// Extract the documents embedded in the PDF — attachments in the <c>/EmbeddedFiles</c> name
    /// tree, file-attachment annotations and PDF portfolio members — through the normal pipeline
    /// into <see cref="Types.ExtractedDocument.Children"/>. On by default, as upstream does it
    /// unconditionally. Bounded by the archive recursion depth, <see cref="MaxEmbeddedFileBytes"/>
    /// and <see cref="SecurityLimits.MaxCompressionRatio"/>.
    /// </summary>
    [JsonPropertyName("extract_embedded_files")]
    public bool ExtractEmbeddedFiles { get; set; } = true;

    /// <summary>
    /// Largest decompressed size of one embedded file that is extracted (null = no per-file cap,
    /// apart from <see cref="SecurityLimits.MaxArchiveSize"/> for all of them together).
    /// </summary>
    [JsonPropertyName("max_embedded_file_bytes")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public long? MaxEmbeddedFileBytes { get; set; }

    /// <summary>
    /// Keep each embedded file's bytes on <see cref="Types.ExtractedAttachment.Data"/>, for a
    /// caller that stores the attachments itself. Off by default: a PDF can carry hundreds of
    /// megabytes of them.
    /// </summary>
    [JsonPropertyName("attach_embedded_file_bytes")]
    public bool AttachEmbeddedFileBytes { get; set; }

    /// <summary>The passwords to try, in order, without the implicit empty one.</summary>
    internal List<string> CandidatePasswords()
    {
        var list = new List<string>();
        if (!string.IsNullOrEmpty(Password)) list.Add(Password);
        if (!string.IsNullOrEmpty(OwnerPassword)) list.Add(OwnerPassword);
        if (Passwords is not null)
            foreach (var p in Passwords)
                if (!string.IsNullOrEmpty(p)) list.Add(p);
        return list;
    }
}

/// <summary>
/// The PDF is encrypted and no supplied password (nor the empty one) opened it — or a supplied
/// password was wrong. Deliberately not an <see cref="InvalidDataException"/>, which is what a corrupt file fails with.
/// </summary>
/// <remarks>
/// <see cref="Extractor"/> reports it as an error item with <see cref="ExtractionErrorItem.ErrorType"/>
/// <c>"password_required"</c> (code 1010) when no password was supplied and <c>"invalid_password"</c>
/// (code 1011) when one was, so a caller that goes through the result envelope never has to parse
/// the message.
/// </remarks>
public sealed class PdfPasswordRequiredException : Exception
{
    /// <summary>True when the caller supplied at least one password and none of them worked.</summary>
    public bool PasswordWasSupplied { get; }

    /// <summary>What the document declares about its encryption, for a caller that wants to say more.</summary>
    public Internal.Pdf.PdfEncryptionInfo? Encryption { get; }

    public PdfPasswordRequiredException(bool passwordWasSupplied, Internal.Pdf.PdfEncryptionInfo? encryption = null)
        : base(passwordWasSupplied
            ? "PDF is encrypted and none of the supplied passwords opened it"
            : "PDF is password-protected")
    {
        PasswordWasSupplied = passwordWasSupplied;
        Encryption = encryption;
    }
}
