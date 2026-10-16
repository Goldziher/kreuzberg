using System.Text;
using XRay.Content.Core;
using XRay.Content.Extractors;
using XRay.Content.Types;
using Xunit;

namespace XRay.Content.Tests;

/// <summary>
/// Embedded files: the <c>/EmbeddedFiles</c> name tree, file-attachment annotations, portfolios
/// and associated files, extracted through the normal pipeline under the archive limits.
/// </summary>
public class PdfEmbeddedFileTests
{
    private const string PageText = "BT /F1 14 Tf 72 700 Td (Container page) Tj ET";

    /// <summary>
    /// A PDF holding <paramref name="files"/> (name, payload, flate, mime) in its name tree.
    /// Objects 1-5 are the page set; each file adds a stream and a filespec.
    /// </summary>
    private static byte[] BuildWithAttachments(
        IEnumerable<(string Name, byte[] Data, bool Flate, string Mime)> files,
        string catalogExtras = "", Func<int, string>? annotations = null)
    {
        var list = files.ToList();
        var builder = new SyntheticPdf();
        // The catalog is object 1; the name tree root is object 6, then stream/filespec pairs.
        builder.Add($"<</Type/Catalog/Pages 2 0 R/Names<</EmbeddedFiles 6 0 R>>{catalogExtras}>>");
        builder.Add("<</Type/Pages/Kids[3 0 R]/Count 1>>");
        string annots = annotations is null ? "" : $"/Annots[{annotations(7 + list.Count * 2)}]";
        builder.Add($"<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1 4 0 R>>>>/Contents 5 0 R{annots}>>");
        builder.Add("<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>");
        builder.AddStream("", PageText);

        var names = new StringBuilder();
        for (int i = 0; i < list.Count; i++)
            names.Append($"({list[i].Name}) {8 + i * 2} 0 R ");
        builder.Add($"<</Names[{names}]>>");                                   // 6
        for (int i = 0; i < list.Count; i++)
        {
            var (name, data, flate, mime) = list[i];
            // Object 7 + 2i is the stream, 8 + 2i the filespec — but the name tree above points
            // at 8 + 2i, so the stream has to come first in numbering.
            builder.AddStream("/Type/EmbeddedFile" + (string.IsNullOrEmpty(mime) ? "" : $"/Subtype/{mime.Replace("/", "#2F")}"), data, flate);
            builder.Add($"<</Type/Filespec/F({name})/UF({name})/EF<</F {7 + i * 2} 0 R>>>>");
        }
        if (annotations is not null) { /* annotation objects are added by the caller's builder variant */ }
        return builder.Build();
    }

    private static ExtractedDocument Extract(byte[] bytes, PdfOptions? options = null, SecurityLimits? limits = null)
    {
        var result = new Extractor().Extract(
            ExtractInput.FromBytes(bytes, "application/pdf"),
            new ExtractionConfig { Pdf = options, SecurityLimits = limits });
        Assert.Empty(result.Errors);
        return Assert.Single(result.Results);
    }

    private static byte[] Utf8(string s) => Encoding.UTF8.GetBytes(s);

    [Fact]
    public void NameTreeFilesAreListedAndExtractedIntoChildren()
    {
        var doc = Extract(BuildWithAttachments(new[]
        {
            ("a.txt", Utf8("Alpha attachment text"), false, "text/plain"),
            ("b.csv", Utf8("x,y\n1,2\n"), true, "text/csv"),
        }));

        Assert.Equal(new[] { "a.txt", "b.csv" }, doc.Attachments!.Select(a => a.Name));
        var a = doc.Attachments![0];
        Assert.Equal("text/plain", a.MimeType);
        Assert.Equal(21, a.Size);
        Assert.Equal("name_tree", a.Source);
        Assert.Null(a.SkippedReason);
        Assert.Null(a.Data);

        Assert.Equal(new[] { "a.txt", "b.csv" }, doc.Children!.Select(c => c.Path));
        Assert.Contains("Alpha attachment text", doc.Children![0].Result.Content, StringComparison.Ordinal);
        Assert.Equal("text/csv", doc.Children![1].MimeType);
    }

    [Fact]
    public void BytesAreKeptOnlyWhenAskedFor()
    {
        var bytes = BuildWithAttachments(new[] { ("a.txt", Utf8("payload"), true, "text/plain") });

        Assert.Null(Extract(bytes).Attachments![0].Data);
        Assert.Equal(Utf8("payload"), Extract(bytes, new PdfOptions { AttachEmbeddedFileBytes = true }).Attachments![0].Data);
    }

    [Fact]
    public void ExtractionCanBeSwitchedOff()
    {
        var bytes = BuildWithAttachments(new[] { ("a.txt", Utf8("payload"), false, "text/plain") });
        var doc = Extract(bytes, new PdfOptions { ExtractEmbeddedFiles = false });
        Assert.Null(doc.Attachments);
        Assert.Null(doc.Children);
    }

    [Fact]
    public void ATypeIsTakenFromTheBytesWhenTheFilespecSaysNothingUseful()
    {
        var html = Utf8("<html><body><h1>Embedded heading</h1></body></html>");
        var doc = Extract(BuildWithAttachments(new[] { ("page.dat", html, false, "application/octet-stream") }));

        var child = Assert.Single(doc.Children!);
        Assert.Equal("text/html", child.MimeType);
        Assert.Contains("Embedded heading", child.Result.Content, StringComparison.Ordinal);
    }

    [Fact]
    public void ADecompressionBombIsSkippedWithAReasonNotInflated()
    {
        // Ten megabytes of zeros deflate to a few kilobytes: far past the 100:1 ratio limit.
        var bomb = new byte[10 * 1024 * 1024];
        var doc = Extract(BuildWithAttachments(new[]
        {
            ("bomb.txt", bomb, true, "text/plain"),
            ("ok.txt", Utf8("fine"), false, "text/plain"),
        }));

        var skipped = doc.Attachments!.Single(a => a.Name == "bomb.txt");
        Assert.Contains("ratio", skipped.SkippedReason, StringComparison.Ordinal);
        Assert.Equal(0, skipped.Size);
        Assert.Contains(doc.ProcessingWarnings, w => w.Source == "pdf_embedded_files" && w.Message.Contains("bomb.txt"));

        // The other file is unaffected.
        Assert.Equal(new[] { "ok.txt" }, doc.Children!.Select(c => c.Path));
    }

    [Fact]
    public void ThePerFileSizeCapIsEnforced()
    {
        var doc = Extract(
            BuildWithAttachments(new[] { ("big.txt", Utf8(new string('x', 5000)), false, "text/plain") }),
            new PdfOptions { MaxEmbeddedFileBytes = 1000 });

        var a = Assert.Single(doc.Attachments!);
        Assert.Contains("size exceeds", a.SkippedReason, StringComparison.Ordinal);
        Assert.Null(doc.Children);
    }

    [Fact]
    public void TheArchiveWideBudgetBoundsTheTotal()
    {
        var limits = new SecurityLimits { MaxArchiveSize = 1500 };
        var doc = Extract(
            BuildWithAttachments(new[]
            {
                ("one.txt", Utf8(new string('a', 1000)), false, "text/plain"),
                ("two.txt", Utf8(new string('b', 1000)), false, "text/plain"),
            }),
            limits: limits);

        Assert.Null(doc.Attachments![0].SkippedReason);
        Assert.NotNull(doc.Attachments![1].SkippedReason);
    }

    [Fact]
    public void ACyclicNameTreeTerminates()
    {
        var builder = new SyntheticPdf();
        builder.Add("<</Type/Catalog/Pages 2 0 R/Names<</EmbeddedFiles 6 0 R>>>>");
        builder.Add("<</Type/Pages/Kids[3 0 R]/Count 1>>");
        builder.Add("<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1 4 0 R>>>>/Contents 5 0 R>>");
        builder.Add("<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>");
        builder.AddStream("", PageText);
        builder.Add("<</Kids[6 0 R 7 0 R]>>");
        builder.Add("<</Kids[6 0 R]>>");

        var doc = Extract(builder.Build());
        Assert.Null(doc.Attachments);
    }

    [Fact]
    public void ANestedPdfIsExtractedAndRecursionIsBounded()
    {
        byte[] Wrap(byte[] inner) => BuildWithAttachments(new[] { ("inner.pdf", inner, true, "application/pdf") });

        byte[] leaf = BuildWithAttachments(new[] { ("leaf.txt", Utf8("deepest text"), false, "text/plain") });
        byte[] nested = Wrap(Wrap(Wrap(Wrap(leaf))));   // five levels of PDF

        var doc = Extract(nested);

        // Three archive levels are followed; the fourth is left as an attachment only.
        var level1 = Assert.Single(doc.Children!).Result;
        var level2 = Assert.Single(level1.Children!).Result;
        var level3 = Assert.Single(level2.Children!).Result;
        Assert.Null(level3.Children);
        Assert.Single(level3.Attachments!);
    }

    [Fact]
    public void FileAttachmentAnnotationsAndAssociatedFilesAreFoundToo()
    {
        var builder = new SyntheticPdf();
        // 6: stream, 7: filespec for the annotation; 8: stream, 9: filespec for /AF.
        builder.Add("<</Type/Catalog/Pages 2 0 R/AF[9 0 R]>>");
        builder.Add("<</Type/Pages/Kids[3 0 R]/Count 1>>");
        builder.Add("<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1 4 0 R>>>>/Contents 5 0 R/Annots[10 0 R]>>");
        builder.Add("<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>");
        builder.AddStream("", PageText);
        builder.AddStream("/Type/EmbeddedFile", "note from the paperclip");           // 6
        builder.Add("<</Type/Filespec/F(clip.txt)/EF<</F 6 0 R>>>>");                 // 7
        builder.AddStream("/Type/EmbeddedFile", "<xml>invoice</xml>");                // 8
        builder.Add("<</Type/Filespec/F(invoice.xml)/AFRelationship/Alternative/EF<</F 8 0 R>>>>"); // 9
        builder.Add("<</Type/Annot/Subtype/FileAttachment/Rect[0 0 10 10]/Contents(clip)/FS 7 0 R>>"); // 10

        var doc = Extract(builder.Build());

        var clip = doc.Attachments!.Single(a => a.Name == "clip.txt");
        Assert.Equal("file_attachment_annotation", clip.Source);
        Assert.Equal(1u, clip.PageNumber);
        var invoice = doc.Attachments!.Single(a => a.Name == "invoice.xml");
        Assert.Equal("associated_file", invoice.Source);
        Assert.Equal("Alternative", invoice.Relationship);
        Assert.Equal("text/plain", clip.MimeType);
    }

    [Fact]
    public void ACollectionMarksNameTreeFilesAsPortfolioMembers()
    {
        var doc = Extract(BuildWithAttachments(
            new[] { ("a.txt", Utf8("one"), false, "text/plain") },
            catalogExtras: "/Collection<</Type/Collection/View/D>>"));
        Assert.Equal("portfolio", Assert.Single(doc.Attachments!).Source);
    }

    // ---- real documents (test_documents/pdf/features) ----

    private static string? Fixture(string name)
    {
        var roots = new List<string> { CorpusPresenceTests.CorpusRoot };
        if (Environment.GetEnvironmentVariable("XRAY_TEST_DOCUMENTS") is { Length: > 0 } env) roots.Add(env);
        return roots.Select(r => Path.Combine(r, "pdf", "features", name)).FirstOrDefault(File.Exists);
    }

    [Fact]
    public void TheAttachmentsFixtureYieldsEveryFileAndExtractsItsText()
    {
        if (Fixture("attachments.pdf") is not { } path) return;

        var doc = Extract(File.ReadAllBytes(path), new PdfOptions { AttachEmbeddedFileBytes = true });

        Assert.Equal(
            new[] { "inner.pdf", "notes.txt", "page.html", "table.csv", "margin-note.txt" },
            doc.Attachments!.Select(a => a.Name));
        var notes = doc.Attachments!.Single(a => a.Name == "notes.txt");
        Assert.Equal("Meeting notes", notes.Description);
        Assert.Equal("text/plain", notes.MimeType);
        Assert.Equal("D:20240101000000Z", notes.Created);
        Assert.Equal("Attachment notes: the budget was approved on Monday.\n", Encoding.UTF8.GetString(notes.Data!));
        Assert.Equal("Data", doc.Attachments!.Single(a => a.Name == "table.csv").Relationship);
        Assert.Equal(2u, doc.Attachments!.Single(a => a.Name == "margin-note.txt").PageNumber);

        var children = doc.Children!.ToDictionary(c => c.Path);
        Assert.Contains("Quarterly report", children["inner.pdf"].Result.Content, StringComparison.Ordinal);
        Assert.Contains("budget was approved", children["notes.txt"].Result.Content, StringComparison.Ordinal);
        Assert.Contains("Embedded page", children["page.html"].Result.Content, StringComparison.Ordinal);
        Assert.Contains("north", children["table.csv"].Result.Content, StringComparison.Ordinal);
        Assert.Contains("paperclip", children["margin-note.txt"].Result.Content, StringComparison.Ordinal);
    }

    [Fact]
    public void ThePortfolioFixtureListsItsMembers()
    {
        if (Fixture("portfolio.pdf") is not { } path) return;

        var doc = Extract(File.ReadAllBytes(path));
        Assert.All(doc.Attachments!, a => Assert.Equal("portfolio", a.Source));
        Assert.Equal(new[] { "cover-letter.txt", "figures.csv" }, doc.Attachments!.Select(a => a.Name));
        Assert.Contains("Cover letter text", doc.Children![0].Result.Content, StringComparison.Ordinal);
    }
}
