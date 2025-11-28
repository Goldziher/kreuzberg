using Kreuzberg;

var config = new ExtractionConfig
{
    PdfOptions = new PdfConfig
    {
        ExtractMetadata = true
    }
};

var result = KreuzbergClient.ExtractFileSync("document.pdf", config);

if (result.Metadata?.Format.Pdf != null)
{
    var createdDate = result.Metadata.Format.Pdf.CreatedDate;
    if (createdDate.HasValue)
    {
        Console.WriteLine($"Created: {createdDate.Value:O}");
    }
}
