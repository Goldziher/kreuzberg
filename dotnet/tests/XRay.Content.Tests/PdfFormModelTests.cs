using XRay.Content.Core;
using XRay.Content.Extractors;
using XRay.Content.Internal.Pdf;
using XRay.Content.Types;
using Xunit;

namespace XRay.Content.Tests;

/// <summary>The typed AcroForm model: every field kind with its value, options, page and flags.</summary>
public class PdfFormModelTests
{
    /// <summary>A page with the given field dictionaries as objects 6 onwards, all in /AcroForm.</summary>
    private static PdfDocument Open(params string[] fields)
    {
        var builder = new SyntheticPdf();
        var refs = string.Join(" ", Enumerable.Range(6, fields.Length).Select(n => $"{n} 0 R"));
        builder.Add("<</Type/Catalog/Pages 2 0 R/AcroForm<</Fields[" + refs + "]>>>>");
        builder.Add("<</Type/Pages/Kids[3 0 R]/Count 1>>");
        builder.Add($"<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Annots[{refs}]>>");
        builder.Add("<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>");
        builder.AddStream("", "BT ET");
        foreach (var f in fields) builder.Add(f);
        return PdfDocument.Open(builder.Build());
    }

    private const string Widget = "/Type/Annot/Subtype/Widget/Rect[10 20 110 40]";

    [Fact]
    public void ATextFieldCarriesItsValuePageAndRectangle()
    {
        var fields = PdfFormModel.Extract(Open($"<<{Widget}/FT/Tx/T(name)/V(Alice)/DV(Bob)/Ff 3>>"));

        var f = Assert.Single(fields);
        Assert.Equal("text", f.FieldType);
        Assert.Equal("name", f.Name);
        Assert.Equal("Alice", f.Value);
        Assert.Equal("Bob", f.DefaultValue);
        Assert.Equal(1u, f.PageNumber);
        Assert.Equal(110, f.BoundingBox!.X1);
        Assert.True(f.ReadOnly);
        Assert.True(f.Required);
    }

    [Theory]
    [InlineData("/V/Yes/AS/Yes", true, "true")]
    [InlineData("/V/Off/AS/Off", false, "false")]
    [InlineData("/AS/On", true, "true")]
    [InlineData("", false, "false")]
    public void ACheckboxIsOnUnlessItsStateIsOff(string entries, bool expected, string value)
    {
        var f = Assert.Single(PdfFormModel.Extract(Open($"<<{Widget}/FT/Btn/T(box){entries}>>")));
        Assert.Equal("checkbox", f.FieldType);
        Assert.Equal(expected, f.Checked);
        Assert.Equal(value, f.Value);
    }

    [Fact]
    public void ARadioGroupReportsItsSelectedExportValueAndOptions()
    {
        // The group is the parent; its two widgets are anonymous kids with /AP states.
        var builder = new SyntheticPdf();
        builder.Add("<</Type/Catalog/Pages 2 0 R/AcroForm<</Fields[6 0 R]>>>>");
        builder.Add("<</Type/Pages/Kids[3 0 R]/Count 1>>");
        builder.Add("<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Annots[7 0 R 8 0 R]>>");
        builder.Add("<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>");
        builder.AddStream("", "BT ET");
        builder.Add("<</FT/Btn/Ff 32768/T(size)/V/large/Kids[7 0 R 8 0 R]>>");
        builder.Add($"<<{Widget}/Parent 6 0 R/AS/Off/AP<</N<</small 0 0 R/Off 0 0 R>>>>>>");
        builder.Add($"<<{Widget}/Parent 6 0 R/AS/large/AP<</N<</large 0 0 R/Off 0 0 R>>>>>>");

        var f = Assert.Single(PdfFormModel.Extract(PdfDocument.Open(builder.Build())));
        Assert.Equal("radio", f.FieldType);
        Assert.Equal("large", f.Value);
        Assert.Equal(new[] { "small", "large" }, f.Options!.Select(o => o.Export));
        Assert.Equal(1u, f.PageNumber);
    }

    [Fact]
    public void AChoiceFieldMapsExportValuesToDisplayText()
    {
        var f = Assert.Single(PdfFormModel.Extract(Open(
            $"<<{Widget}/FT/Ch/Ff 131072/T(country)/V(DE)/Opt[[(DE)(Germany)][(FR)(France)]]>>")));

        Assert.Equal("choice", f.FieldType);
        Assert.True(f.IsCombo);
        Assert.Equal("Germany", f.Value);
        Assert.Equal(new[] { "DE" }, f.Values);
        Assert.Equal(new[] { "Germany", "France" }, f.Options!.Select(o => o.Display));
    }

    [Fact]
    public void AMultiSelectListBoxJoinsItsSelections()
    {
        var f = Assert.Single(PdfFormModel.Extract(Open(
            $"<<{Widget}/FT/Ch/Ff 2097152/T(topics)/V[(a)(c)]/Opt[[(a)(Alpha)][(b)(Beta)][(c)(Gamma)]]>>")));

        Assert.False(f.IsCombo);
        Assert.True(f.MultiSelect);
        Assert.Equal("Alpha, Gamma", f.Value);
        Assert.Equal(new[] { "a", "c" }, f.Values);
    }

    [Fact]
    public void ButtonsAndSignaturesAreTypedToo()
    {
        var fields = PdfFormModel.Extract(Open(
            $"<<{Widget}/FT/Btn/Ff 65536/T(go)>>",
            $"<<{Widget}/FT/Sig/T(sig)/V<</Type/Sig/Filter/Adobe.PPKLite>>>>",
            $"<<{Widget}/FT/Sig/T(unsigned)>>"));

        Assert.Equal(new[] { "push_button", "signature", "signature" }, fields.Select(f => f.FieldType));
        Assert.Equal("signed", fields[1].Value);
        Assert.Null(fields[2].Value);
    }

    [Fact]
    public void KidsInheritTypeFlagsAndValueFromTheirParentAndQualifyTheirNames()
    {
        var builder = new SyntheticPdf();
        builder.Add("<</Type/Catalog/Pages 2 0 R/AcroForm<</Fields[6 0 R]>>>>");
        builder.Add("<</Type/Pages/Kids[3 0 R]/Count 1>>");
        builder.Add("<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Annots[8 0 R]>>");
        builder.Add("<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>");
        builder.AddStream("", "BT ET");
        builder.Add("<</T(form)/FT/Tx/Kids[7 0 R]>>");
        builder.Add("<</T(first)/Parent 6 0 R/Ff 1/V(inherited type)/Kids[8 0 R]>>");   // 7: group of one named field
        builder.Add($"<<{Widget}/Parent 7 0 R>>");                                       // 8: plain widget

        var f = Assert.Single(PdfFormModel.Extract(PdfDocument.Open(builder.Build())));
        Assert.Equal("form.first", f.Name);
        Assert.Equal("text", f.FieldType);
        Assert.True(f.ReadOnly);
        Assert.Equal("inherited type", f.Value);
    }

    [Fact]
    public void ACyclicFieldTreeTerminates()
    {
        var builder = new SyntheticPdf();
        builder.Add("<</Type/Catalog/Pages 2 0 R/AcroForm<</Fields[6 0 R]>>>>");
        builder.Add("<</Type/Pages/Kids[3 0 R]/Count 1>>");
        builder.Add("<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]>>");
        builder.Add("<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>");
        builder.AddStream("", "BT ET");
        builder.Add("<</T(a)/Kids[7 0 R]>>");
        builder.Add("<</T(b)/Kids[6 0 R]>>");

        Assert.Empty(PdfFormModel.Extract(PdfDocument.Open(builder.Build())));
    }

    [Fact]
    public void TheExtractorExposesTheFieldsOnTheDocument()
    {
        var builder = new SyntheticPdf();
        builder.Add("<</Type/Catalog/Pages 2 0 R/AcroForm<</Fields[6 0 R]>>>>");
        builder.Add("<</Type/Pages/Kids[3 0 R]/Count 1>>");
        builder.Add("<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Annots[6 0 R]>>");
        builder.Add("<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>");
        builder.AddStream("", "BT ET");
        builder.Add($"<<{Widget}/FT/Tx/T(city)/V(Berlin)>>");

        var result = new Extractor().Extract(ExtractInput.FromBytes(builder.Build(), "application/pdf"), new ExtractionConfig());
        Assert.Empty(result.Errors);
        var field = Assert.Single(result.Results[0].FormFields!);
        Assert.Equal("city", field.Name);
        Assert.Equal("Berlin", field.Value);
    }

    // ---- real document (test_documents/pdf/features/forms.pdf) ----

    private static string? Fixture(string name)
    {
        var roots = new List<string> { CorpusPresenceTests.CorpusRoot };
        if (Environment.GetEnvironmentVariable("XRAY_TEST_DOCUMENTS") is { Length: > 0 } env) roots.Add(env);
        return roots.Select(r => Path.Combine(r, "pdf", "features", name)).FirstOrDefault(File.Exists);
    }

    [Fact]
    public void TheFormsFixtureYieldsEveryFieldKind()
    {
        if (Fixture("forms.pdf") is not { } path) return;

        var result = new Extractor().Extract(ExtractInput.FromUri(path), new ExtractionConfig());
        var fields = result.Results[0].FormFields!.ToDictionary(f => f.Name);

        Assert.Equal("Ada Lovelace", fields["full_name"].Value);
        Assert.True(fields["subscribe"].Checked);
        Assert.False(fields["terms"].Checked);
        Assert.Equal("large", fields["size"].Value);
        Assert.Equal(new[] { "small", "large" }, fields["size"].Options!.Select(o => o.Export));
        Assert.Equal("Germany", fields["country"].Value);
        Assert.Equal(new[] { "DE" }, fields["country"].Values);
        Assert.Equal("Alpha, Gamma", fields["topics"].Value);
        Assert.True(fields["topics"].MultiSelect);
    }
}
