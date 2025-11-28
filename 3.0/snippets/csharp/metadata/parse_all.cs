using Kreuzberg;

var config = new ExtractionConfig
{
    PdfOptions = new PdfConfig
    {
        ExtractMetadata = true
    }
};

var result = KreuzbergClient.ExtractFileSync("document.pdf", config);

// Access PDF metadata
if (result.Metadata?.Format.Pdf != null)
{
    var pdfMeta = result.Metadata.Format.Pdf;
    Console.WriteLine($"Pages: {pdfMeta.PageCount}");
    Console.WriteLine($"Author: {pdfMeta.Author}");
    Console.WriteLine($"Title: {pdfMeta.Title}");
    Console.WriteLine($"Subject: {pdfMeta.Subject}");
    Console.WriteLine($"Created: {pdfMeta.CreatedDate:O}");
}

// Access HTML metadata
var htmlResult = KreuzbergClient.ExtractFileSync("page.html", config);
if (htmlResult.Metadata?.Format.Html != null)
{
    var htmlMeta = htmlResult.Metadata.Format.Html;
    Console.WriteLine($"Title: {htmlMeta.Title}");
    Console.WriteLine($"Description: {htmlMeta.Description}");
    Console.WriteLine($"Open Graph Image: {htmlMeta.OgImage}");
}
