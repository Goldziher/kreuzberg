using Kreuzberg;
using System.Text;

var result = Kreuzberg.ExtractFileSync("document.pdf");

if (result.Metadata.Pages?.Boundaries != null)
{
    var contentBytes = Encoding.UTF8.GetBytes(result.Content);

    foreach (var boundary in result.Metadata.Pages.Boundaries.Take(3))
    {
        var pageBytes = contentBytes[boundary.ByteStart..boundary.ByteEnd];
        var pageText = Encoding.UTF8.GetString(pageBytes);

        Console.WriteLine($"Page {boundary.PageNumber}:");
        Console.WriteLine($"  Byte range: {boundary.ByteStart}-{boundary.ByteEnd}");
        Console.WriteLine($"  Preview: {pageText[..100]}...");
    }
}
