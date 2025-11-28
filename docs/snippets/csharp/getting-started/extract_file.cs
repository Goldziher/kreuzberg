using Kreuzberg;

var result = await KreuzbergClient.ExtractFileAsync("document.pdf");

Console.WriteLine(result.Content);
Console.WriteLine($"Tables: {result.Tables.Count}");
Console.WriteLine($"Metadata: {result.Metadata.FormatType}");
