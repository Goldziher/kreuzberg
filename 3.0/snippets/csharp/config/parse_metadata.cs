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
    var title = result.Metadata.Format.Pdf.Title;
    var author = result.Metadata.Format.Pdf.Author;
    var pageCount = result.Metadata.Format.Pdf.PageCount;
}
