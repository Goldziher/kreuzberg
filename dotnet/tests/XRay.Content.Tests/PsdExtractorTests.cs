using System.Text.Json;
using XRay.Content.Core;
using XRay.Content.Extractors;
using XRay.Content.Types;
using Xunit;

namespace XRay.Content.Tests;

/// <summary>
/// The Photoshop extractor, an X-Ray addition with no upstream counterpart: the layer tree as
/// headings, type-layer text as paragraphs under its layer, document facts as metadata.
/// </summary>
/// <remarks>
/// The fixture-backed tests read <c>test_documents/psd/</c>, the Patchy and psd-tools fixtures in
/// curiosity-ai/test_documents, and skip when the corpus is absent, as the other fixture-backed
/// tests here do (<see cref="CorpusPresenceTests"/> is the one loud failure for that). The
/// synthetic tests need no corpus.
/// </remarks>
public sealed class PsdExtractorTests
{
    /// <summary>
    /// A fixture under <c>test_documents/psd/</c>, or null when the corpus does not have it. The
    /// submodule is looked for first; <c>XRAY_TEST_DOCUMENTS</c> names a corpus checkout elsewhere.
    /// </summary>
    private static string? Fixture(string name)
    {
        var roots = new List<string> { CorpusPresenceTests.CorpusRoot };
        if (Environment.GetEnvironmentVariable("XRAY_TEST_DOCUMENTS") is { Length: > 0 } env) roots.Add(env);
        return roots.Select(r => Path.Combine(r, "psd", name)).FirstOrDefault(File.Exists);
    }

    private static ExtractedDocument Extract(string path, OutputFormat format)
    {
        var result = new Extractor().Extract(ExtractInput.FromUri(path), new ExtractionConfig { OutputFormat = format });
        Assert.Empty(result.Errors);
        return Assert.Single(result.Results);
    }

    private static ExtractedDocument Extract(byte[] bytes, OutputFormat format, string? filename = null)
    {
        var result = new Extractor().Extract(
            ExtractInput.FromBytes(bytes, PsdExtractor.MimeType, filename), new ExtractionConfig { OutputFormat = format });
        Assert.Empty(result.Errors);
        return Assert.Single(result.Results);
    }

    private static T Additional<T>(ExtractedDocument doc, string key) =>
        doc.Metadata.Additional[key].Deserialize<T>()!;

    // --- real documents ---

    [Fact]
    public void MarkdownIsTheLayerTreeWithEachTypeLayersTextUnderItsHeading()
    {
        if (Fixture("qual_rca_pinout.psd") is not { } path) return;

        var doc = Extract(path, OutputFormat.Markdown);

        Assert.Equal(PsdExtractor.MimeType, doc.MimeType);
        Assert.Equal(
            "# qual_rca_pinout\n\n"
            + "## Layer 2\n\n"
            + "## 12345678910\n\n12345678910\n\n"
            + "## 5=Audio (W)\n\n5=Audio (W)\n\n"
            + "## 4=Audio (R)\n\n4=Audio (R)\n\n"
            + "## 9=Video\n\n9=Video\n\n"
            + "## 10=G\n\n10=G\n\n"
            + "## 1=G\n\n1=G\n\n"
            + "## Layer 1\n",
            doc.Content);
    }

    [Fact]
    public void PlainTextIsTheSameTreeWithoutMarkup()
    {
        if (Fixture("qual_rca_pinout.psd") is not { } path) return;

        var doc = Extract(path, OutputFormat.Plain);

        Assert.Equal(
            "qual_rca_pinout\n\n"
            + "  Layer 2\n\n"
            + "  12345678910\n    12345678910\n\n"
            + "  5=Audio (W)\n    5=Audio (W)\n\n"
            + "  4=Audio (R)\n    4=Audio (R)\n\n"
            + "  9=Video\n    9=Video\n\n"
            + "  10=G\n    10=G\n\n"
            + "  1=G\n    1=G\n\n"
            + "  Layer 1",
            doc.Content);
    }

    [Fact]
    public void GroupsNestTheirLayersOneLevelDownAndHiddenLayersStay()
    {
        // "Turn arrows" and four of the pixel layers are hidden; extraction reports what the file
        // holds, not what a render shows.
        if (Fixture("arrows.psd") is not { } path) return;

        var doc = Extract(path, OutputFormat.Markdown);

        Assert.Equal(
            "# arrows\n\n"
            + "## Turn arrows\n\n### Shape 5\n\n### Shape 4\n\n"
            + "## Move arrows\n\n### Shape 3 copy\n\n### Shape 3\n\n### Shape 2\n\n### Shape 1\n\n"
            + "## Layer 4\n\n## Layer 3 copy\n\n## Layer 3\n\n## Layer 5\n\n## Layer 2\n",
            doc.Content);
    }

    [Fact]
    public void EachPhotoshopParagraphIsItsOwnParagraph()
    {
        if (Fixture("photoshop-text-vertical-point.psd") is not { } path) return;

        Assert.Equal(
            "# photoshop-text-vertical-point\n\n## vt_point_ja_multi\n\n縦書き\n\nテスト\n\n## Background\n",
            Extract(path, OutputFormat.Markdown).Content);
        Assert.Equal(
            "photoshop-text-vertical-point\n\n  vt_point_ja_multi\n    縦書き\n\n    テスト\n\n  Background",
            Extract(path, OutputFormat.Plain).Content);
    }

    [Fact]
    public void AnEmbeddedSmartObjectsLayersNestUnderTheLayerThatPlacesIt()
    {
        if (Fixture("photoshop-smart-filter-unsharp-motion.psd") is not { } path) return;

        var doc = Extract(path, OutputFormat.Markdown);

        Assert.Equal(
            "# photoshop-smart-filter-unsharp-motion\n\n"
            + "## Unsharp Mask 175 2.5 7\n\n### Unfiltered source\n\n"
            + "## Motion Blur 37 12\n\n### Unfiltered source\n\n"
            + "## _Unfiltered source\n\n### Unfiltered source\n",
            doc.Content);
        Assert.Equal(["Unfiltered source.psb"], Additional<List<string>>(doc, "linked_files"));
        // The counts are the outer document's own.
        Assert.Equal(3, Additional<int>(doc, "layer_count"));
    }

    [Fact]
    public void DocumentFactsAreMetadataNotBody()
    {
        if (Fixture("qual_rca_pinout.psd") is not { } path) return;

        var doc = Extract(path, OutputFormat.Plain);

        Assert.Equal("qual_rca_pinout", doc.Metadata.Title);
        var image = Assert.IsType<ImageMetadata>(doc.Metadata.Format!.Payload);
        Assert.Equal(("PSD", 1745u, 1158u), (image.Format, image.Width, image.Height));
        Assert.Equal("Adobe Photoshop 27.4 (Windows)", image.Exif["Software"]);
        Assert.Equal("rgb", Additional<string>(doc, "color_mode"));
        Assert.Equal(8, Additional<int>(doc, "bit_depth"));
        Assert.Equal(8, Additional<int>(doc, "layer_count"));
        Assert.Equal(6, Additional<int>(doc, "text_layer_count"));
        Assert.Equal(["ArialMT"], Additional<List<string>>(doc, "fonts"));
    }

    [Fact]
    public void ALargeDocumentReportsItselfAsPsb()
    {
        if (Fixture("photoshop-basic.psb") is not { } path) return;

        var doc = Extract(path, OutputFormat.Plain);

        Assert.Equal(PsdExtractor.MimeType, doc.MimeType);
        Assert.Equal("PSB", Assert.IsType<ImageMetadata>(doc.Metadata.Format!.Payload).Format);
    }

    [Fact]
    public void ChannelAndPathNamesAreMetadata()
    {
        if (Fixture("photoshop-saved-channels.psd") is not { } channels
            || Fixture("photoshop-saved-paths.psd") is not { } paths) return;

        var channelDoc = Extract(channels, OutputFormat.Plain);
        Assert.Equal(["Duplicate", "Duplicate", "特色チャンネル"], Additional<List<string>>(channelDoc, "channel_names"));
        Assert.DoesNotContain("Duplicate", channelDoc.Content, StringComparison.Ordinal);

        var pathDoc = Extract(paths, OutputFormat.Plain);
        Assert.Equal(["Alpha Path", "Beta Path"], Additional<List<string>>(pathDoc, "path_names"));
        Assert.DoesNotContain("Alpha Path", pathDoc.Content, StringComparison.Ordinal);
    }

    [Fact]
    public void SlicesAreMetadataAndTheirUrlsAreLinks()
    {
        if (Fixture("psd-tools/slices.psd") is not { } path) return;

        var doc = Extract(path, OutputFormat.Markdown);

        Assert.Equal("# slices\n", doc.Content);
        var slices = Additional<List<Dictionary<string, string>>>(doc, "slices");
        Assert.Contains(slices, s => s.GetValueOrDefault("message") == "Hi there" && s.GetValueOrDefault("alt") == "Slice_1");
        Assert.Contains(slices, s => s.GetValueOrDefault("cell_text") == "HTML Text");
        var uri = Assert.Single(doc.Uris!);
        Assert.Equal(("http://example.com", Types.UriKind.Hyperlink), (uri.Url, uri.Kind));
    }

    // --- synthetic documents ---

    private static byte[] NestedDocument()
    {
        var psd = new SyntheticPsd();
        psd.Layers.Add(new SyntheticPsd.Layer
        {
            Name = "Header",
            Children =
            [
                new SyntheticPsd.Layer { Name = "Headline", Text = "Big news\nSecond paragraph" },
                new SyntheticPsd.Layer { Name = "Logo", Hidden = true },
            ],
        });
        psd.Layers.Add(new SyntheticPsd.Layer { Name = "Background" });
        return psd.WithChannelNames("Spot").Build();
    }

    [Fact]
    public void ASyntheticTreeRendersAsNestedHeadingsWithItsText()
    {
        var doc = Extract(NestedDocument(), OutputFormat.Markdown, "poster.psd");

        Assert.Equal(
            "# poster\n\n## Header\n\n### Headline\n\nBig news\n\nSecond paragraph\n\n### Logo\n\n## Background\n",
            doc.Content);
        Assert.Equal(["Spot"], Additional<List<string>>(doc, "channel_names"));
        Assert.Equal(4, Additional<int>(doc, "layer_count"));
        Assert.Equal(1, Additional<int>(doc, "text_layer_count"));
    }

    [Fact]
    public void ASyntheticTreeInPlainTextIndentsByDepth()
    {
        var doc = Extract(NestedDocument(), OutputFormat.Plain, "poster.psd");

        Assert.Equal(
            "poster\n\n  Header\n    Headline\n      Big news\n\n      Second paragraph\n\n    Logo\n  Background",
            doc.Content);
    }

    [Fact]
    public void WithoutATitleTheTopLevelLayersAreFirstLevelHeadings()
    {
        var doc = Extract(NestedDocument(), OutputFormat.Markdown);

        Assert.StartsWith("# Header\n\n## Headline\n\nBig news\n", doc.Content, StringComparison.Ordinal);
        Assert.Null(doc.Metadata.Title);
    }

    [Fact]
    public void TypeTextBelongsToItsLayersSectionInTheDocumentStructure()
    {
        var result = new Extractor().Extract(
            ExtractInput.FromBytes(NestedDocument(), PsdExtractor.MimeType, "poster.psd"),
            new ExtractionConfig { IncludeDocumentStructure = true });
        var structure = Assert.Single(result.Results).Document!;

        var headline = structure.Nodes.Single(n => n.Content.HeadingText == "Headline");
        var children = headline.Children.Select(i => structure.Nodes[(int)i].Content).ToList();
        Assert.Contains(children, c => c.Text == "Big news");
        Assert.Contains(children, c => c.Text == "Second paragraph");
    }

    [Fact]
    public void HeadingsStopAtLevelSix()
    {
        // Seven nested groups put the innermost layer at depth 8 below the title.
        var leaf = new SyntheticPsd.Layer { Name = "Leaf", Text = "deep" };
        for (int i = 7; i >= 1; i--) leaf = new SyntheticPsd.Layer { Name = $"G{i}", Children = [leaf] };
        var psd = new SyntheticPsd();
        psd.Layers.Add(leaf);

        var doc = Extract(psd.Build(), OutputFormat.Markdown, "deep.psd");

        Assert.Equal(
            "# deep\n\n## G1\n\n### G2\n\n#### G3\n\n##### G4\n\n###### G5\n\n###### G6\n\n###### G7\n\n###### Leaf\n\ndeep\n",
            doc.Content);
    }

    [Fact]
    public void AMalformedDocumentIsAParsingError()
    {
        byte[] truncated = "8BPS\0\u0001"u8.ToArray();

        Assert.Throws<InvalidDataException>(() =>
            new PsdExtractor().Extract(truncated, PsdExtractor.MimeType, new ExtractionConfig()));

        var result = new Extractor().Extract(
            ExtractInput.FromBytes(truncated, PsdExtractor.MimeType), new ExtractionConfig());
        Assert.Single(result.Errors);
    }

    [Fact]
    public void PhotoshopFilesAreDetectedAndRoutedToThePsdExtractor()
    {
        byte[] psd = NestedDocument();

        Assert.Equal(PsdExtractor.MimeType, Mime.DetectMimeTypeFromBytes(psd));
        Assert.Equal(PsdExtractor.MimeType, Mime.DetectMimeType("poster.psd", checkExists: false));
        Assert.Equal(PsdExtractor.MimeType, Mime.DetectMimeType("poster.PSB", checkExists: false));
        // Content decides over a wrong extension, as it does for every other signature.
        Assert.Equal(PsdExtractor.MimeType, Mime.ResolveWithContent("image/png", psd));
        Assert.IsType<PsdExtractor>(Registry.RegisterDefaults().ForMime(PsdExtractor.MimeType));
    }
}
