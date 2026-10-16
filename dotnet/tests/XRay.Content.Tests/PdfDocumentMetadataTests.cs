using System.Text;
using XRay.Content.Core;
using XRay.Content.Internal.Pdf;
using XRay.Content.Types;
using Xunit;

namespace XRay.Content.Tests;

/// <summary>
/// Document-level metadata beyond the Info strings: the outline, language, custom Info entries,
/// extra XMP properties, and the catalog version.
/// </summary>
public class PdfDocumentMetadataTests
{
    private static ExtractedDocument Extract(byte[] bytes)
    {
        var result = new Extractor().Extract(ExtractInput.FromBytes(bytes, "application/pdf"), new ExtractionConfig());
        Assert.Empty(result.Errors);
        return Assert.Single(result.Results);
    }

    private static PdfMetadata Pdf(ExtractedDocument d) => Assert.IsType<PdfMetadata>(d.Metadata.Format!.Payload);

    [Fact]
    public void TheOutlineIsListedWithLevelsAndPages()
    {
        var builder = new SyntheticPdf();
        builder.Add("<</Type/Catalog/Pages 2 0 R/Outlines 6 0 R>>");
        builder.Add("<</Type/Pages/Kids[3 0 R]/Count 1>>");
        builder.Add("<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Contents 5 0 R>>");
        builder.Add("<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>");
        builder.AddStream("", "BT ET");
        builder.Add("<</Type/Outlines/First 7 0 R/Last 7 0 R/Count 2>>");
        builder.Add("<</Title(Chapter)/Parent 6 0 R/First 8 0 R/Last 8 0 R/Dest[3 0 R/Fit]>>");
        builder.Add("<</Title(Section)/Parent 7 0 R/Dest[3 0 R/Fit]>>");

        var doc = Extract(builder.Build());
        var outline = Pdf(doc).Outline!;

        Assert.Equal(new[] { ("Chapter", 0, 1), ("Section", 1, 1) }, outline.Select(o => (o.Title, o.Level, o.Page ?? 0)));
        // Both items point at page 1, and the result de-duplicates URIs by address.
        var uri = Assert.Single(doc.Uris!);
        Assert.Equal("#page=1", uri.Url);
        Assert.Equal("Chapter", uri.Label);
    }

    [Fact]
    public void ADocumentWithoutAnOutlineHasNone()
    {
        var doc = Extract(SyntheticPdf.OnePage("BT ET").Build());
        Assert.Null(Pdf(doc).Outline);
        Assert.Null(doc.Uris);
    }

    [Fact]
    public void LanguageCustomInfoXmpExtrasAndCatalogVersionAreSurfaced()
    {
        const string xmp = "<?xpacket begin='x'?><x:xmpmeta xmlns:x='adobe:ns:meta/'><rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>"
            + "<rdf:Description xmlns:dc='http://purl.org/dc/elements/1.1/' xmlns:pdfaid='http://www.aiim.org/pdfa/ns/id/'>"
            + "<dc:rights><rdf:Alt><rdf:li xml:lang='x-default'>(c) Example</rdf:li></rdf:Alt></dc:rights>"
            + "<pdfaid:part>2</pdfaid:part><pdfaid:conformance>U</pdfaid:conformance>"
            + "</rdf:Description></rdf:RDF></x:xmpmeta><?xpacket end='w'?>";

        var builder = SyntheticPdf.OnePage("BT ET", catalogExtras: "/Lang(fr-FR)/Version/1.7/Metadata 6 0 R");
        builder.AddStream("/Type/Metadata/Subtype/XML", xmp);                                    // 6
        builder.Info = builder.Add("<</Title(T)/Department(Finance)/Trapped/False/CustomNumber 42>>");
        builder.Version("1.4");

        var doc = Extract(builder.Build());

        Assert.Equal("fr-FR", doc.Metadata.Language);
        Assert.Equal("1.4", Pdf(doc).PdfVersion);
        Assert.Equal("1.7", doc.Metadata.Additional["pdf_catalog_version"].GetString());
        var info = doc.Metadata.Additional["pdf_info"];
        Assert.Equal("Finance", info.GetProperty("Department").GetString());
        Assert.Equal("42", info.GetProperty("CustomNumber").GetString());
        Assert.False(info.TryGetProperty("Trapped", out _));
        var extra = doc.Metadata.Additional["xmp"];
        Assert.Equal("(c) Example", extra.GetProperty("dc:rights").GetString());
        Assert.Equal("2", extra.GetProperty("pdfaid:part").GetString());
        Assert.Equal("U", extra.GetProperty("pdfaid:conformance").GetString());
    }

    [Fact]
    public void ADocumentWithNothingExtraAddsNoMetadataKeys()
    {
        var doc = Extract(SyntheticPdf.OnePage("BT ET").Build());
        Assert.False(doc.Metadata.Additional.ContainsKey("pdf_info"));
        Assert.False(doc.Metadata.Additional.ContainsKey("xmp"));
        Assert.False(doc.Metadata.Additional.ContainsKey("pdf_catalog_version"));
        Assert.Null(doc.Metadata.Language);
    }

    private static string? Fixture(string name)
    {
        var roots = new List<string> { CorpusPresenceTests.CorpusRoot };
        if (Environment.GetEnvironmentVariable("XRAY_TEST_DOCUMENTS") is { Length: > 0 } env) roots.Add(env);
        return roots.Select(r => Path.Combine(r, "pdf", "features", name)).FirstOrDefault(File.Exists);
    }

    [Fact]
    public void TheOutlineAndMetadataFixtureSurfacesEverything()
    {
        if (Fixture("outline-metadata.pdf") is not { } path) return;

        var result = new Extractor().Extract(ExtractInput.FromUri(path), new ExtractionConfig());
        var doc = Assert.Single(result.Results);

        Assert.Equal("Feature fixture", doc.Metadata.Title);
        Assert.Equal("Feature test", doc.Metadata.Subject);
        Assert.Equal(new[] { "Info Author" }, doc.Metadata.Authors);
        Assert.Equal(new[] { "pdf", "outline", "metadata" }, doc.Metadata.Keywords);
        Assert.Equal("generate.py", doc.Metadata.CreatedBy);
        Assert.Equal("2024-01-15T12:00:00Z", doc.Metadata.CreatedAt);
        Assert.Equal("2024-01-16T13:00:00Z", doc.Metadata.ModifiedAt);
        Assert.Equal("en-GB", doc.Metadata.Language);
        Assert.Equal("pikepdf", Pdf(doc).Producer);

        Assert.Equal(
            new[] { ("Quarterly report", 0, 1), ("Revenue", 1, 1), ("Costs", 1, 1), ("Outlook", 0, 2) },
            Pdf(doc).Outline!.Select(o => (o.Title, o.Level, o.Page ?? 0)));
        Assert.Equal(new[] { "i", "A-1" }, doc.Metadata.Additional["page_labels"].EnumerateArray().Select(e => e.GetString()));
        Assert.Equal("Finance", doc.Metadata.Additional["pdf_info"].GetProperty("Department").GetString());
        var xmp = doc.Metadata.Additional["xmp"];
        Assert.Equal("(c) Example Corp", xmp.GetProperty("dc:rights").GetString());
        Assert.Equal("B", xmp.GetProperty("pdfaid:conformance").GetString());
    }
}
