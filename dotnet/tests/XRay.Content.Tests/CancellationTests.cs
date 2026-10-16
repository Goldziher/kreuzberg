using System.Diagnostics;
using System.IO.Compression;
using System.Text;
using XRay.Content.Core;
using XRay.Content.Extractors;
using XRay.Content.Internal.Onnx;
using XRay.Content.Types;
using Xunit;

namespace XRay.Content.Tests;

/// <summary>
/// The family's cancellation contract for <c>X-Ray.Content</c>: a token on the public entry
/// points, polled at the loop boundaries that bound latency, surfacing as
/// <see cref="OperationCanceledException"/> — never as an error item or a warning — and changing
/// nothing when it is not cancelled.
/// </summary>
public class CancellationTests
{
    private const string DocxMime = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
    private const string XlsxMime = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

    /// <summary>Limits high enough that a deliberately huge input is limited by time, not by policy.</summary>
    private static ExtractionConfig Unbounded(OutputFormat? format = null) => new()
    {
        OutputFormat = format ?? OutputFormat.Plain,
        SecurityLimits = new SecurityLimits
        {
            MaxFilesInArchive = long.MaxValue, MaxArchiveSize = long.MaxValue, MaxContentSize = long.MaxValue,
            MaxTableCells = long.MaxValue, MaxIterations = long.MaxValue, MaxCompressionRatio = long.MaxValue,
        },
        Options = new XRayOptions { PdfMaxSecondsPerDocument = 0 },
    };

    // ---- small inputs, one per format ----

    private static byte[] Zip(params (string Name, string Content)[] parts)
    {
        using var ms = new MemoryStream();
        using (var zip = new ZipArchive(ms, ZipArchiveMode.Create, leaveOpen: true))
            foreach (var (name, content) in parts)
            {
                var e = zip.CreateEntry(name);
                using var s = e.Open();
                s.Write(Encoding.UTF8.GetBytes(content));
            }
        return ms.ToArray();
    }

    private static byte[] SmallDocx() => Zip(("word/document.xml",
        "<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>"
        + "<w:p><w:r><w:t>Hello</w:t></w:r></w:p></w:body></w:document>"));

    private static byte[] SmallXlsx() => Zip(
        ("xl/workbook.xml", "<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" "
            + "xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">"
            + "<sheets><sheet name=\"Data\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>"),
        ("xl/_rels/workbook.xml.rels", "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">"
            + "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" "
            + "Target=\"worksheets/sheet1.xml\"/></Relationships>"),
        ("xl/worksheets/sheet1.xml", "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>"
            + "<row r=\"1\"><c r=\"A1\"><v>1</v></c></row></sheetData></worksheet>"));

    public static TheoryData<string, string> Formats() => new()
    {
        { "pdf", "application/pdf" },
        { "docx", DocxMime },
        { "xlsx", XlsxMime },
        { "zip", "application/zip" },
        { "html", "text/html" },
        { "csv", "text/csv" },
        { "psd", PsdExtractor.MimeType },
        { "eml", "message/rfc822" },
        { "md", "text/markdown" },
        { "json", "application/json" },
        { "xml", "application/xml" },
        { "gz", "application/gzip" },
    };

    private static byte[] Sample(string kind) => kind switch
    {
        "pdf" => SyntheticPdf.OnePage("BT /F1 12 Tf 72 700 Td (Hello) Tj ET").Build(),
        "docx" => SmallDocx(),
        "xlsx" => SmallXlsx(),
        "zip" => Zip(("a.txt", "alpha"), ("b.txt", "beta")),
        "html" => Encoding.UTF8.GetBytes("<html><body><p>Hello</p></body></html>"),
        "csv" => Encoding.UTF8.GetBytes("a,b\n1,2\n3,4\n"),
        "psd" => BuildPsd(),
        "eml" => Encoding.UTF8.GetBytes("From: a@example.com\r\nTo: b@example.com\r\nSubject: Hi\r\n\r\nHello there\r\n"),
        "md" => Encoding.UTF8.GetBytes("# Title\n\nSome **text**.\n"),
        "json" => Encoding.UTF8.GetBytes("{\"a\":[1,2,3],\"b\":\"text\"}"),
        "xml" => Encoding.UTF8.GetBytes("<root><item>one</item><item>two</item></root>"),
        "gz" => Gzip(Encoding.UTF8.GetBytes("hello gzip")),
        _ => throw new ArgumentOutOfRangeException(nameof(kind)),
    };

    private static byte[] BuildPsd()
    {
        var psd = new SyntheticPsd();
        psd.Layers.Add(new SyntheticPsd.Layer { Name = "Headline", Text = "Big news" });
        return psd.Build();
    }

    private static byte[] Gzip(byte[] data)
    {
        using var ms = new MemoryStream();
        using (var gz = new GZipStream(ms, CompressionLevel.SmallestSize, leaveOpen: true)) gz.Write(data);
        return ms.ToArray();
    }

    // ---- pre-cancelled ----

    [Theory]
    [MemberData(nameof(Formats))]
    public void ACancelledTokenThrowsBeforeAnyWork(string kind, string mime)
    {
        using var cts = new CancellationTokenSource();
        cts.Cancel();

        Assert.ThrowsAny<OperationCanceledException>(() => new Extractor().Extract(
            ExtractInput.FromBytes(Sample(kind), mime), new ExtractionConfig(), cts.Token));
    }

    [Fact]
    public async Task ACancelledTokenGivesACancelledTaskFromTheAsyncOverload()
    {
        using var cts = new CancellationTokenSource();
        cts.Cancel();

        var task = new Extractor().ExtractAsync(
            ExtractInput.FromBytes(Sample("html"), "text/html"), new ExtractionConfig(), cts.Token);
        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => task);
        Assert.True(task.IsCanceled);
    }

    [Fact]
    public void CancellationIsNeverFoldedIntoAnErrorItem()
    {
        using var cts = new CancellationTokenSource();
        cts.Cancel();

        // Not "returns a result with an error": the call throws.
        var ex = Record.Exception(() => new Extractor().Extract(
            ExtractInput.FromBytes(Sample("pdf"), "application/pdf"), new ExtractionConfig(), cts.Token));
        Assert.IsAssignableFrom<OperationCanceledException>(ex);
    }

    // ---- not cancelled: nothing changes ----

    [Theory]
    [MemberData(nameof(Formats))]
    public void ALiveTokenGivesTheSameResultAsNoToken(string kind, string mime)
    {
        var bytes = Sample(kind);
        var config = new ExtractionConfig { OutputFormat = OutputFormat.Markdown };

        var plain = new Extractor().Extract(ExtractInput.FromBytes(bytes, mime), config);
        using var cts = new CancellationTokenSource();
        var withToken = new Extractor().Extract(ExtractInput.FromBytes(bytes, mime), config, cts.Token);

        Assert.Equal(Json.Serialize(plain), Json.Serialize(withToken));
    }

    [Fact]
    public void TheTokenDoesNotLeakIntoTheNextCall()
    {
        using var cts = new CancellationTokenSource();
        cts.Cancel();
        Assert.ThrowsAny<OperationCanceledException>(() => new Extractor().Extract(
            ExtractInput.FromBytes(Sample("csv"), "text/csv"), new ExtractionConfig(), cts.Token));

        // A later call without a token is unaffected by the earlier cancelled one.
        var result = new Extractor().Extract(ExtractInput.FromBytes(Sample("csv"), "text/csv"), new ExtractionConfig());
        Assert.Empty(result.Errors);
        Assert.Contains("1", result.Results[0].Content, StringComparison.Ordinal);
    }

    // ---- mid-flight, on deliberately large inputs ----

    /// <summary>
    /// Run an extraction on a worker thread, cancel it after <paramref name="after"/>, and check
    /// both that it was still running (it throws rather than completing) and that it stopped soon.
    /// </summary>
    private static void AssertStopsPromptly(byte[] bytes, string mime, ExtractionConfig config, TimeSpan after)
    {
        using var cts = new CancellationTokenSource();
        long cancelledAt = 0;
        var started = Stopwatch.StartNew();
        cts.Token.Register(() => cancelledAt = Stopwatch.GetTimestamp());
        cts.CancelAfter(after);

        var task = Task.Run(() => new Extractor().Extract(ExtractInput.FromBytes(bytes, mime), config, cts.Token));
        var ex = Assert.ThrowsAny<Exception>(() => task.GetAwaiter().GetResult());
        Assert.IsAssignableFrom<OperationCanceledException>(ex);

        var lag = Stopwatch.GetElapsedTime(cancelledAt);
        Assert.True(lag < TimeSpan.FromSeconds(5),
            $"returned {lag.TotalMilliseconds:0} ms after cancellation (total {started.Elapsed.TotalMilliseconds:0} ms)");
    }

    [Fact]
    public void AZipWithTensOfThousandsOfEntriesStopsMidArchive()
    {
        var entries = Enumerable.Range(0, 60_000).Select(i => ($"dir/file{i}.txt", $"entry number {i}")).ToArray();
        AssertStopsPromptly(Zip(entries), "application/zip", Unbounded(), TimeSpan.FromMilliseconds(300));
    }

    [Fact]
    public void AHugeCsvStopsMidParse()
    {
        var sb = new StringBuilder("id,name,value\n");
        for (int i = 0; i < 4_000_000; i++) sb.Append(i).Append(",name").Append(i).Append(',').Append(i * 3).Append('\n');
        AssertStopsPromptly(Encoding.UTF8.GetBytes(sb.ToString()), "text/csv", Unbounded(), TimeSpan.FromMilliseconds(300));
    }

    [Fact]
    public void AThousandsOfPagePdfStopsBetweenPages()
    {
        const int pages = 6000;
        var builder = new SyntheticPdf();
        var kids = string.Join(" ", Enumerable.Range(0, pages).Select(i => $"{4 + i * 2} 0 R"));
        builder.Add("<</Type/Catalog/Pages 2 0 R>>");
        builder.Add($"<</Type/Pages/Kids[{kids}]/Count {pages}>>");
        builder.Add("<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>");
        for (int i = 0; i < pages; i++)
        {
            builder.Add($"<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1 3 0 R>>>>/Contents {5 + i * 2} 0 R>>");
            builder.AddStream("", $"BT /F1 12 Tf 72 700 Td (Page number {i} with some words to read) Tj ET", flate: true);
        }
        AssertStopsPromptly(builder.Build(), "application/pdf", Unbounded(), TimeSpan.FromMilliseconds(300));
    }

    [Fact]
    public void AHugeHtmlStopsMidConversion()
    {
        var sb = new StringBuilder("<html><body>");
        for (int i = 0; i < 1_500_000; i++) sb.Append("<p>Paragraph ").Append(i).Append(" has <b>some</b> words.</p>");
        sb.Append("</body></html>");
        AssertStopsPromptly(Encoding.UTF8.GetBytes(sb.ToString()), "text/html",
            Unbounded(OutputFormat.Markdown), TimeSpan.FromMilliseconds(300));
    }

    [Fact]
    public void AHugeSpreadsheetStopsMidSheet()
    {
        var sheet = new StringBuilder(
            "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>");
        for (int r = 1; r <= 400_000; r++)
            sheet.Append($"<row r=\"{r}\"><c r=\"A{r}\"><v>{r}</v></c><c r=\"B{r}\"><v>{r * 2}</v></c></row>");
        sheet.Append("</sheetData></worksheet>");
        var bytes = Zip(
            ("xl/workbook.xml", "<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" "
                + "xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">"
                + "<sheets><sheet name=\"Data\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>"),
            ("xl/_rels/workbook.xml.rels", "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">"
                + "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" "
                + "Target=\"worksheets/sheet1.xml\"/></Relationships>"),
            ("xl/worksheets/sheet1.xml", sheet.ToString()));
        AssertStopsPromptly(bytes, XlsxMime, Unbounded(), TimeSpan.FromMilliseconds(300));
    }

    [Fact]
    public void AHugeDocxStopsMidBody()
    {
        var body = new StringBuilder(
            "<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>");
        for (int i = 0; i < 600_000; i++) body.Append("<w:p><w:r><w:t>Paragraph ").Append(i).Append("</w:t></w:r></w:p>");
        body.Append("</w:body></w:document>");
        AssertStopsPromptly(Zip(("word/document.xml", body.ToString())), DocxMime, Unbounded(), TimeSpan.FromMilliseconds(300));
    }

    [Fact]
    public void AHugeMarkdownStopsMidDocument()
    {
        var sb = new StringBuilder();
        for (int i = 0; i < 1_500_000; i++) sb.Append("## Heading ").Append(i).Append("\n\nParagraph with *emphasis* and `code` ").Append(i).Append(".\n\n");
        AssertStopsPromptly(Encoding.UTF8.GetBytes(sb.ToString()), "text/markdown",
            Unbounded(OutputFormat.Html), TimeSpan.FromMilliseconds(300));
    }

    [Fact]
    public void AHugeJsonStopsMidTree()
    {
        var sb = new StringBuilder("{\"items\":[");
        for (int i = 0; i < 700_000; i++) sb.Append(i > 0 ? "," : "").Append("{\"id\":").Append(i).Append(",\"name\":\"item ").Append(i).Append("\"}");
        sb.Append("]}");
        AssertStopsPromptly(Encoding.UTF8.GetBytes(sb.ToString()), "application/json", Unbounded(), TimeSpan.FromMilliseconds(300));
    }

    [Fact]
    public void AManyPartEmailStopsMidMessage()
    {
        var sb = new StringBuilder(
            "From: a@example.com\r\nTo: b@example.com\r\nSubject: parts\r\nMIME-Version: 1.0\r\n"
            + "Content-Type: multipart/mixed; boundary=\"XX\"\r\n\r\n");
        for (int i = 0; i < 120_000; i++)
            sb.Append("--XX\r\nContent-Type: text/plain\r\nContent-Disposition: attachment; filename=\"f").Append(i)
              .Append(".txt\"\r\n\r\nattachment body ").Append(i).Append("\r\n");
        sb.Append("--XX--\r\n");
        AssertStopsPromptly(Encoding.UTF8.GetBytes(sb.ToString()), "message/rfc822", Unbounded(), TimeSpan.FromMilliseconds(300));
    }

    [Fact]
    public async Task TheAsyncOverloadStopsMidFlightToo()
    {
        var entries = Enumerable.Range(0, 60_000).Select(i => ($"f{i}.txt", $"entry {i}")).ToArray();
        using var cts = new CancellationTokenSource();
        cts.CancelAfter(TimeSpan.FromMilliseconds(300));

        var task = new Extractor().ExtractAsync(
            ExtractInput.FromBytes(Zip(entries), "application/zip"), Unbounded(), cts.Token);
        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => task);
    }

    // ---- the places a token has to be carried through ----

    [Fact]
    public void ACancellationInsideABestEffortStageIsNotSwallowed()
    {
        // The table detectors, the form-field pass and the metadata readers all sit inside
        // `catch { }` blocks that keep a document going; none of them may eat a cancellation.
        var pdf = SyntheticPdf.OnePage("BT /F1 12 Tf 72 700 Td (Hello) Tj ET", catalogExtras: "/AcroForm<</Fields[]>>").Build();
        using var cts = new CancellationTokenSource();
        cts.Cancel();
        using var scope = Cancel.Begin(cts.Token);

        Assert.ThrowsAny<OperationCanceledException>(
            () => new PdfExtractor().Extract(pdf, "application/pdf", new ExtractionConfig()));
    }

    [Fact]
    public void TheInferenceRuntimeStopsWhenCancelled()
    {
        var model = OnnxModel.Parse(ConvChainModel.Build(scale: [2f, 3f], shift: [1f, -1f], activation: "Relu"));
        var feeds = new Dictionary<string, Tensor>
        {
            ["x"] = Tensor.FromFloats([1, 2, 3, 4, -5, 6, -7, 8], 1, 2, 2, 2),
        };
        using var cts = new CancellationTokenSource();
        cts.Cancel();
        using var scope = Cancel.Begin(cts.Token);

        Assert.ThrowsAny<OperationCanceledException>(() => new OnnxSession(model).Run(feeds));
    }

    [Fact]
    public void ArchiveChildrenAreExtractedUnderTheParentsToken()
    {
        // A zip holding a zip: the inner extraction passes no token of its own, so it only stops
        // if the outer one is still in force.
        var inner = Zip(Enumerable.Range(0, 30_000).Select(i => ($"x{i}.txt", $"inner {i}")).ToArray());
        var outer = Zip(("inner.zip", Encoding.Latin1.GetString(inner)));
        // Latin-1 round-trips bytes only for a Latin-1 aware writer; build the outer zip by bytes.
        using var ms = new MemoryStream();
        using (var zip = new ZipArchive(ms, ZipArchiveMode.Create, leaveOpen: true))
        {
            using var s = zip.CreateEntry("inner.zip", CompressionLevel.NoCompression).Open();
            s.Write(inner);
        }
        _ = outer;
        AssertStopsPromptly(ms.ToArray(), "application/zip", Unbounded(), TimeSpan.FromMilliseconds(300));
    }

    // ---- the cancellable overloads of the lower-level APIs ----

    [Fact]
    public void TheLowerLevelOverloadsHonourACancelledToken()
    {
        using var cts = new CancellationTokenSource();
        cts.Cancel();
        var doc = new InternalDocument("test");
        doc.PushElement(InternalElement.TextElement(ElementKind.Paragraph, "text", 0));

        Assert.ThrowsAny<OperationCanceledException>(() => new PlainTextExtractor().Extract(
            Encoding.UTF8.GetBytes("hello"), "text/plain", new ExtractionConfig(), cts.Token));
        Assert.ThrowsAny<OperationCanceledException>(() => CancellableApi.RenderMarkdown(doc, cts.Token));
        Assert.ThrowsAny<OperationCanceledException>(() => CancellableApi.RenderPlain(doc, cts.Token));
        Assert.ThrowsAny<OperationCanceledException>(() => CancellableApi.RenderHtml(doc, cts.Token));
        Assert.ThrowsAny<OperationCanceledException>(() => CancellableApi.RenderJson(doc, cts.Token));
        Assert.ThrowsAny<OperationCanceledException>(() => CancellableApi.RenderDjot(doc, cts.Token));
        Assert.ThrowsAny<OperationCanceledException>(() =>
            CancellableApi.DeriveExtractionResult(doc, false, OutputFormat.Plain, null, false, cts.Token));
        Assert.ThrowsAny<OperationCanceledException>(() =>
            CancellableApi.DetectMimeTypeFromBytes(new byte[] { 1, 2, 3 }, true, cts.Token));
    }

    [Fact]
    public void TheLowerLevelOverloadsReturnWhatThePlainOnesDoWhenNotCancelled()
    {
        using var cts = new CancellationTokenSource();
        var doc = new InternalDocument("test");
        doc.PushElement(InternalElement.TextElement(ElementKind.Paragraph, "text", 0));

        Assert.Equal(Rendering.MarkdownRenderer.Render(doc), CancellableApi.RenderMarkdown(doc, cts.Token));
        Assert.Equal(Rendering.PlainRenderer.Render(doc), CancellableApi.RenderPlain(doc, cts.Token));
        Assert.Equal(
            "hello",
            new PlainTextExtractor().Extract(Encoding.UTF8.GetBytes("hello"), "text/plain", new ExtractionConfig(), cts.Token)
                .Elements[0].Text);
    }

    // ---- bomb budget ----

    [Fact]
    public void AGzipBombIsStoppedAtTheArchiveSizeLimit()
    {
        // Eight megabytes of zeros deflate to a few kilobytes; the limit is one megabyte.
        var bytes = Gzip(new byte[8 * 1024 * 1024]);
        var config = new ExtractionConfig { SecurityLimits = new SecurityLimits { MaxArchiveSize = 1024 * 1024 } };

        var result = new Extractor().Extract(ExtractInput.FromBytes(bytes, "application/gzip"), config);

        var error = Assert.Single(result.Errors);
        Assert.Equal("other", error.ErrorType == "validation" ? "other" : error.ErrorType);
        Assert.Contains("exceeds", error.Message, StringComparison.Ordinal);
    }
}
