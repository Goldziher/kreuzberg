using System.Text;
using XRay.Content.Core;
using XRay.Content.Extractors;
using XRay.Content.Internal.Pdf;
using XRay.Content.Types;
using Xunit;

namespace XRay.Content.Tests;

/// <summary>
/// Page annotations (ported from <c>pdf/native/annotations.rs</c>): subtypes, comments, authors,
/// dates, colours, link targets, the text a markup annotation covers, and where all of it shows
/// up — <c>Annotations</c>, <c>Uris</c> and the rendered content.
/// </summary>
public class PdfAnnotationTests
{
    private const string PageText = "BT /F1 14 Tf 72 700 Td (Hello World again) Tj ET";

    private static ExtractedDocument Extract(byte[] bytes, OutputFormat? format = null, bool annotations = true)
    {
        var result = new Extractor().Extract(
            ExtractInput.FromBytes(bytes, "application/pdf"),
            new ExtractionConfig
            {
                OutputFormat = format ?? OutputFormat.Plain,
                Pdf = new PdfOptions { ExtractAnnotations = annotations },
            });
        Assert.Empty(result.Errors);
        return Assert.Single(result.Results);
    }

    /// <summary>One page of <see cref="PageText"/> with the given annotation dictionaries, 6.. onwards.</summary>
    private static byte[] Build(params string[] annotationDictionaries)
    {
        // The five fixed objects come first, so the annotations are objects 6 onwards.
        var builder = new SyntheticPdf();
        builder.Add("<</Type/Catalog/Pages 2 0 R>>");
        builder.Add("<</Type/Pages/Kids[3 0 R]/Count 1>>");
        var refs = string.Join(" ", Enumerable.Range(6, annotationDictionaries.Length).Select(n => $"{n} 0 R"));
        builder.Add($"<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1 4 0 R>>>>/Contents 5 0 R/Annots[{refs}]>>");
        builder.Add("<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>");
        builder.AddStream("", PageText);
        foreach (var dict in annotationDictionaries) builder.Add(dict);
        return builder.Build();
    }

    [Fact]
    public void ATextNoteCarriesItsCommentAuthorDateColourAndSubject()
    {
        var doc = Extract(Build(
            "<</Type/Annot/Subtype/Text/Rect[100 600 120 620]/Contents(Check this)/T(Alice)/M(D:20240115120000Z)"
            + "/C[1 1 0]/Subj(Review)/F 4>>"));

        var a = Assert.Single(doc.Annotations!);
        Assert.Equal(PdfAnnotationType.Text, a.AnnotationType);
        Assert.Equal("Text", a.Subtype);
        Assert.Equal("Check this", a.Content);
        Assert.Equal("Alice", a.Author);
        Assert.Equal("D:20240115120000Z", a.Modified);
        Assert.Equal("#ffff00", a.Color);
        Assert.Equal("Review", a.Subject);
        Assert.Equal(1u, a.PageNumber);
        Assert.Equal(100, a.BoundingBox!.X0);
        Assert.Equal(620, a.BoundingBox.Y1);
        Assert.Equal(612, a.PageWidth);
        Assert.Equal(792, a.PageHeight);
        Assert.True(a.IsVisible);
    }

    [Fact]
    public void AFreeTextBoxWithOnlyARichTextStringStillYieldsItsWords()
    {
        var doc = Extract(Build(
            "<</Type/Annot/Subtype/FreeText/Rect[100 600 300 640]/RC(<body><p>Rich <b>words</b> here</p></body>)>>"));

        var a = Assert.Single(doc.Annotations!);
        Assert.Equal(PdfAnnotationType.Text, a.AnnotationType);
        Assert.Equal("FreeText", a.Subtype);
        Assert.Equal("Rich words here", a.Content);
    }

    [Fact]
    public void AHighlightCarriesItsQuadBoxesAndTheTextUnderThem()
    {
        var doc = Extract(Build(
            "<</Type/Annot/Subtype/Highlight/Rect[107 698 145 714]/QuadPoints[107 714 145 714 107 698 145 698]"
            + "/C[1 0.9 0]/Contents(Nice)/T(Carol)>>"));

        var a = Assert.Single(doc.Annotations!);
        Assert.Equal(PdfAnnotationType.Highlight, a.AnnotationType);
        Assert.Equal("World", a.MarkedText);
        Assert.Equal("Nice", a.Content);
        var quad = Assert.Single(a.QuadPoints!);
        Assert.Equal(107, quad.X0);
        Assert.Equal(714, quad.Y1);
    }

    [Theory]
    [InlineData("Underline", PdfAnnotationType.Underline)]
    [InlineData("StrikeOut", PdfAnnotationType.StrikeOut)]
    [InlineData("Squiggly", PdfAnnotationType.Squiggly)]
    public void EveryTextMarkupTypeRecoversItsCoveredText(string subtype, PdfAnnotationType expected)
    {
        var doc = Extract(Build(
            $"<</Type/Annot/Subtype/{subtype}/Rect[72 698 107 714]/QuadPoints[72 714 107 714 72 698 107 698]>>"));

        var a = Assert.Single(doc.Annotations!);
        Assert.Equal(expected, a.AnnotationType);
        Assert.Equal("Hello", a.MarkedText);
    }

    [Fact]
    public void AMarkupBoxOverNoTextHasNoMarkedText()
    {
        var doc = Extract(Build(
            "<</Type/Annot/Subtype/Highlight/Rect[300 100 400 120]/QuadPoints[300 120 400 120 300 100 400 100]>>"));
        Assert.Null(Assert.Single(doc.Annotations!).MarkedText);
    }

    [Fact]
    public void LinksBecomeUrisWithTheirKindAndPage()
    {
        var doc = Extract(Build(
            "<</Type/Annot/Subtype/Link/Rect[0 0 10 10]/A<</S/URI/URI(https://example.com/a?b=1)>>>>",
            "<</Type/Annot/Subtype/Link/Rect[0 20 10 30]/A<</S/URI/URI(mailto:me@example.com)>>>>",
            "<</Type/Annot/Subtype/Link/Rect[0 40 10 50]/Dest[3 0 R/Fit]>>"));

        Assert.Equal(
            new[] { "https://example.com/a?b=1", "mailto:me@example.com", null },
            doc.Annotations!.Select(a => a.Content));
        Assert.Equal("#page=1", doc.Annotations![2].Destination);

        Assert.Equal(
            new[]
            {
                ("https://example.com/a?b=1", XRay.Content.Types.UriKind.Hyperlink),
                ("mailto:me@example.com", XRay.Content.Types.UriKind.Email),
                ("#page=1", XRay.Content.Types.UriKind.Anchor),
            },
            doc.Uris!.Select(u => (u.Url, u.Kind)));
        Assert.All(doc.Uris!, u => Assert.Equal(1u, u.Page));
    }

    [Fact]
    public void WidgetsAndPopupsAreNotAnnotations()
    {
        var doc = Extract(Build(
            "<</Type/Annot/Subtype/Widget/Rect[0 0 10 10]/FT/Tx/T(f)/V(x)>>",
            "<</Type/Annot/Subtype/Popup/Rect[0 0 10 10]>>",
            "<</Type/Annot/Subtype/Stamp/Rect[0 0 10 10]/Contents(Draft)>>"));

        var a = Assert.Single(doc.Annotations!);
        Assert.Equal(PdfAnnotationType.Stamp, a.AnnotationType);
        Assert.Equal("Draft", a.Content);
    }

    [Fact]
    public void FlagsTellAHiddenAnnotationFromAVisibleOne()
    {
        var doc = Extract(Build(
            "<</Type/Annot/Subtype/Text/Rect[0 0 10 10]/Contents(shown)/F 4>>",
            "<</Type/Annot/Subtype/Text/Rect[0 0 10 10]/Contents(hidden)/F 2>>",
            "<</Type/Annot/Subtype/Text/Rect[0 0 10 10]/Contents(noview)/F 32>>",
            "<</Type/Annot/Subtype/Text/Rect[0 0 10 10]/Contents(invisible)/F 1>>"));

        Assert.Equal(
            new[] { "shown" },
            doc.Annotations!.Where(a => a.IsVisible).Select(a => a.Content));
        Assert.Equal(4, doc.Annotations!.Count);
    }

    [Theory]
    [InlineData("Ink", PdfAnnotationType.Ink)]
    [InlineData("Square", PdfAnnotationType.Square)]
    [InlineData("Circle", PdfAnnotationType.Circle)]
    [InlineData("Line", PdfAnnotationType.Line)]
    [InlineData("Caret", PdfAnnotationType.Caret)]
    [InlineData("Redact", PdfAnnotationType.Other)]
    public void SubtypesMapOntoTheTypedEnum(string subtype, PdfAnnotationType expected)
    {
        var doc = Extract(Build($"<</Type/Annot/Subtype/{subtype}/Rect[0 0 10 10]/Contents(c)>>"));
        var a = Assert.Single(doc.Annotations!);
        Assert.Equal(expected, a.AnnotationType);
        Assert.Equal(subtype, a.Subtype);
    }

    [Theory]
    [InlineData("[0.5]", "#808080")]
    [InlineData("[1 0 0]", "#ff0000")]
    [InlineData("[0 0 0 1]", "#000000")]
    [InlineData("[0 0]", null)]
    public void ColoursAreNormalisedToHex(string color, string? expected)
    {
        var doc = Extract(Build($"<</Type/Annot/Subtype/Square/Rect[0 0 10 10]/C{color}>>"));
        Assert.Equal(expected, Assert.Single(doc.Annotations!).Color);
    }

    [Fact]
    public void AnnotationsAreOffUnlessAskedFor()
    {
        byte[] bytes = Build("<</Type/Annot/Subtype/Text/Rect[0 0 10 10]/Contents(note)>>");

        var off = Extract(bytes, annotations: false);
        Assert.Null(off.Annotations);
        Assert.DoesNotContain("Annotations", off.Content, StringComparison.Ordinal);

        var onByDefault = new Extractor().Extract(ExtractInput.FromBytes(bytes, "application/pdf"), new ExtractionConfig());
        Assert.Null(onByDefault.Results[0].Annotations);
    }

    [Fact]
    public void EveryRendererAppendsAnAnnotationsSection()
    {
        byte[] bytes = Build(
            "<</Type/Annot/Subtype/Highlight/Rect[107 698 145 714]/QuadPoints[107 714 145 714 107 698 145 698]/T(Carol)>>");

        Assert.Contains("Annotations:\nHighlight (page 1) by Carol: World", Extract(bytes).Content, StringComparison.Ordinal);
        Assert.Contains("## Annotations\n\n- **Highlight** (page 1) by Carol: World",
            Extract(bytes, OutputFormat.Markdown).Content, StringComparison.Ordinal);
        Assert.Contains("<h2>Annotations</h2>\n<ul>\n<li><strong>Highlight</strong> (page 1) by Carol: World</li>",
            Extract(bytes, OutputFormat.Html).Content, StringComparison.Ordinal);
        Assert.Contains("- _Highlight_ (page 1) by Carol: World",
            Extract(bytes, OutputFormat.Djot).Content, StringComparison.Ordinal);
        Assert.Contains("\"annotations\":[{\"annotation_type\":\"highlight\"",
            Extract(bytes, OutputFormat.Json).Content, StringComparison.Ordinal);
    }

    [Fact]
    public void ADamagedAnnotsArrayCostsOnlyTheAnnotations()
    {
        var builder = new SyntheticPdf();
        builder.Add("<</Type/Catalog/Pages 2 0 R>>");
        builder.Add("<</Type/Pages/Kids[3 0 R]/Count 1>>");
        builder.Add("<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1 4 0 R>>>>/Contents 5 0 R/Annots 99 0 R>>");
        builder.Add("<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>");
        builder.AddStream("", PageText);

        var doc = Extract(builder.Build());
        Assert.Contains("Hello World again", doc.Content, StringComparison.Ordinal);
        Assert.Null(doc.Annotations);
    }

    // ---- real documents (test_documents/pdf/features) ----

    private static string? Fixture(string name)
    {
        var roots = new List<string> { CorpusPresenceTests.CorpusRoot };
        if (Environment.GetEnvironmentVariable("XRAY_TEST_DOCUMENTS") is { Length: > 0 } env) roots.Add(env);
        return roots.Select(r => Path.Combine(r, "pdf", "features", name)).FirstOrDefault(File.Exists);
    }

    [Fact]
    public void TheAnnotatedFixtureYieldsEveryKindWithItsCoveredText()
    {
        if (Fixture("annotations.pdf") is not { } path) return;

        var doc = Extract(File.ReadAllBytes(path));
        var all = doc.Annotations!;

        string? Marked(PdfAnnotationType t) => all.First(a => a.AnnotationType == t).MarkedText;
        Assert.Equal("Revenue grew strongly", Marked(PdfAnnotationType.Highlight));
        Assert.Equal("Costs were", Marked(PdfAnnotationType.Underline));
        Assert.Equal("across all regions.", Marked(PdfAnnotationType.StrikeOut));
        Assert.Equal("the third quarter.", Marked(PdfAnnotationType.Squiggly));

        var note = all.First(a => a.Content == "Check these numbers with finance.");
        Assert.Equal("Alice", note.Author);
        Assert.Equal("#ffff00", note.Color);
        Assert.Equal("Review", note.Subject);
        Assert.Equal("D:20240115120000Z", note.Modified);

        Assert.Contains(all, a => a.AnnotationType == PdfAnnotationType.Stamp && a.Content == "Approved by management");
        Assert.Contains(all, a => a.AnnotationType == PdfAnnotationType.Caret && a.Author == "Eve");
        Assert.Contains(all, a => a.Content == "Second page note" && a.PageNumber == 2);

        // The hidden note is extracted, but flagged so a consumer can drop it.
        var hidden = all.Single(a => a.Content == "Hidden note");
        Assert.False(hidden.IsVisible);
        Assert.DoesNotContain(all.Where(a => a.IsVisible), a => a.Content == "Hidden note");

        // The popup attached to the first note is not an annotation of its own.
        Assert.DoesNotContain(all, a => a.Subtype == "Popup");

        Assert.Equal(
            new[] { "https://example.com/report?q=1", "#page=2", "mailto:team@example.com" },
            doc.Uris!.Select(u => u.Url));
    }
}
