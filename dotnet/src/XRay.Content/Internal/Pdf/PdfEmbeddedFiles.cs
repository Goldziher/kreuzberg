// Embedded files, ported from crates/xberg/src/pdf/embedded_files.rs and extended to the other
// two places a PDF keeps attachments.
//
//   * the /Names /EmbeddedFiles name tree (what every viewer's attachments pane lists);
//   * file-attachment annotations (/Subtype /FileAttachment, the filespec in /FS);
//   * PDF portfolios (/Collection): the members are ordinary entries of the same name tree, so
//     they are found by the first rule and only labelled differently;
//   * the catalog's /AF array of associated files (PDF/A-3, ZUGFeRD and Factur-X invoices), when
//     the name tree does not already list the same stream.
//
// Each file's stream is decoded under a ceiling derived from the configured limits, so a few
// kilobytes that inflate to gigabytes stop at the ceiling instead of exhausting memory.
using XRay.Content.Core;
using XRay.Content.Types;

namespace XRay.Content.Internal.Pdf;

internal sealed class PdfEmbeddedFile
{
    public string Name = "";
    public byte[] Data = Array.Empty<byte>();
    public long CompressedSize;
    public string? MimeType;
    public string? Description;
    public string? Relationship;
    public string? Created;
    public string? Modified;
    public string Source = "name_tree";
    public uint? PageNumber;
    /// <summary>Set when the file's content is not usable (limit exceeded, undecodable).</summary>
    public string? SkippedReason;
}

internal static class PdfEmbeddedFiles
{
    /// <summary>
    /// A cyclic or very wide name tree cannot force unbounded work: depth and total nodes are
    /// capped, as in the reference.
    /// </summary>
    private const int MaxNameTreeDepth = 50;
    private const int MaxNameTreeNodes = 500;
    private const int MaxFiles = 10_000;

    public static List<PdfEmbeddedFile> Collect(
        PdfDocument pdf, ExtractionConfig config, List<ProcessingWarning> warnings)
    {
        var limits = config.SecurityLimits ?? new SecurityLimits();
        var options = config.Pdf ?? new PdfOptions();
        var files = new List<PdfEmbeddedFile>();
        var seenStreams = new HashSet<int>();
        long totalBytes = 0;

        bool isPortfolio = pdf.Catalog is { } cat && pdf.Resolve(cat.Get("Collection")).AsDict() is not null;

        void Add(PdfObject? filespecRef, string treeName, string source, uint? page)
        {
            if (files.Count >= Math.Min(MaxFiles, limits.MaxFilesInArchive)) return;
            Cancel.ThrowIfRequested();
            var file = ReadFileSpec(pdf, filespecRef, treeName, source, page, limits, options, ref totalBytes, seenStreams, warnings);
            if (file is not null) files.Add(file);
        }

        // 1. /Names /EmbeddedFiles (and, through it, portfolio members).
        if (pdf.Catalog is { } catalog
            && pdf.Resolve(catalog.Get("Names")).AsDict() is { } names
            && names.Get("EmbeddedFiles") is { } root)
        {
            int nodes = 0;
            var visited = new HashSet<PdfRef>();
            WalkNameTree(pdf, root, 0, ref nodes, visited,
                (name, spec) => Add(spec, name, isPortfolio ? "portfolio" : "name_tree", null));
        }

        // 2. File-attachment annotations.
        int pageCount = pdf.PageCount;
        for (int i = 0; i < pageCount; i++)
        {
            Cancel.Every(i, 0x0F);
            foreach (var annot in PdfAnnotations.PageAnnotationDicts(pdf, i))
            {
                if (pdf.Resolve(annot.Get("Subtype")).AsName() != "FileAttachment") continue;
                if (annot.Get("FS") is not { } fs) continue;
                string label = PdfFormFields.DecodeTextString(
                    pdf.Resolve(annot.Get("Contents")).AsStringBytes() ?? Array.Empty<byte>());
                Add(fs, label, "file_attachment_annotation", (uint)(i + 1));
            }
        }

        // 3. Associated files.
        if (pdf.Catalog is { } cat2 && pdf.Resolve(cat2.Get("AF")).AsArray() is { } af)
            foreach (var item in af.Items) Add(item, "", "associated_file", null);

        return files;
    }

    private static void WalkNameTree(
        PdfDocument pdf, PdfObject node, int depth, ref int nodes, HashSet<PdfRef> visited,
        Action<string, PdfObject> onPair)
    {
        if (depth > MaxNameTreeDepth || nodes >= MaxNameTreeNodes) return;
        nodes++;
        if (node is PdfRef r && !visited.Add(r)) return;
        var dict = pdf.Resolve(node).AsDict();
        if (dict is null) return;

        if (pdf.Resolve(dict.Get("Names")).AsArray() is { } pairs)
            for (int i = 0; i + 1 < pairs.Items.Count; i += 2)
            {
                string name = pdf.Resolve(pairs.Items[i]) is PdfString s ? PdfFormFields.DecodeTextString(s.Bytes) : "";
                onPair(name, pairs.Items[i + 1]);
            }

        if (pdf.Resolve(dict.Get("Kids")).AsArray() is { } kids)
            foreach (var kid in kids.Items)
            {
                if (nodes >= MaxNameTreeNodes) break;
                WalkNameTree(pdf, kid, depth + 1, ref nodes, visited, onPair);
            }
    }

    private static PdfEmbeddedFile? ReadFileSpec(
        PdfDocument pdf, PdfObject? specObject, string treeName, string source, uint? page,
        SecurityLimits limits, PdfOptions options, ref long totalBytes, HashSet<int> seenStreams,
        List<ProcessingWarning> warnings)
    {
        var spec = pdf.Resolve(specObject).AsDict();
        if (spec is null) return null;

        string name = TextOf(pdf, spec.Get("UF")) ?? TextOf(pdf, spec.Get("F")) ?? treeName;
        var ef = pdf.Resolve(spec.Get("EF")).AsDict();
        if (ef is null) return null;

        var streamObject = ef.Get("F") ?? ef.Get("UF");
        int? streamNumber = (streamObject as PdfRef)?.Number;
        if (streamNumber is { } n && !seenStreams.Add(n)) return null; // already listed
        if (pdf.Resolve(streamObject) is not PdfStream stream) return null;

        var file = new PdfEmbeddedFile
        {
            Name = name.Length == 0 ? "attachment" : name,
            CompressedSize = stream.RawData.LongLength,
            Source = source,
            PageNumber = page,
            Description = TextOf(pdf, spec.Get("Desc")),
            Relationship = pdf.Resolve(spec.Get("AFRelationship")).AsName(),
        };

        string? subtype = pdf.Resolve(stream.Dict.Get("Subtype")).AsName();
        // A MIME type is a PDF name, so its slash is written #2F; the lexer has already resolved that.
        if (!string.IsNullOrEmpty(subtype)) file.MimeType = subtype;

        var parameters = pdf.Resolve(stream.Dict.Get("Params")).AsDict();
        file.Created = TextOf(pdf, parameters?.Get("CreationDate"));
        file.Modified = TextOf(pdf, parameters?.Get("ModDate"));

        // The decode ceiling: the per-file cap, what is left of the archive-wide budget, and the
        // compression ratio, whichever is tightest. Which one tripped decides the message.
        long ratioCeiling = stream.RawData.Length > 0
            ? SaturatingMultiply(stream.RawData.LongLength, Math.Max(limits.MaxCompressionRatio, 1))
            : long.MaxValue;
        long sizeCeiling = Math.Min(options.MaxEmbeddedFileBytes ?? long.MaxValue, limits.MaxArchiveSize - totalBytes);
        // Raw bytes that are not filtered at all are never a ratio problem.
        bool filtered = stream.Dict.Has("Filter") || stream.Dict.Has("F");
        long ceiling = filtered ? Math.Min(ratioCeiling, sizeCeiling) : sizeCeiling;

        byte[] data;
        try
        {
            data = PdfFilters.Decode(stream, pdf, ceiling);
        }
        catch (SecurityException)
        {
            file.SkippedReason = filtered && ratioCeiling <= sizeCeiling
                ? $"decompression ratio exceeds {limits.MaxCompressionRatio}x (compressed {file.CompressedSize} B)"
                : $"size exceeds {Math.Min(options.MaxEmbeddedFileBytes ?? long.MaxValue, limits.MaxArchiveSize - totalBytes)} bytes";
            warnings.Add(new ProcessingWarning
            {
                Source = "pdf_embedded_files",
                Message = $"Skipped embedded file '{file.Name}': {file.SkippedReason}",
            });
            return file;
        }
        catch (Exception e) when (Cancel.NotCancelling)
        {
            file.SkippedReason = $"could not be decoded: {e.Message}";
            warnings.Add(new ProcessingWarning
            {
                Source = "pdf_embedded_files",
                Message = $"Failed to read embedded file '{file.Name}': {e.Message}",
            });
            return file;
        }

        totalBytes += data.LongLength;
        file.Data = data;
        if (file.MimeType is null || file.MimeType == Mime.OctetStream)
            file.MimeType = Mime.DetectMimeType(file.Name, checkExists: false);
        return file;
    }

    private static long SaturatingMultiply(long a, long b)
    {
        try { return checked(a * b); }
        catch (OverflowException) { return long.MaxValue; }
    }

    private static string? TextOf(PdfDocument pdf, PdfObject? o) =>
        pdf.Resolve(o).AsStringBytes() is { } bytes && bytes.Length > 0
            ? PdfFormFields.DecodeTextString(bytes)
            : null;
}
