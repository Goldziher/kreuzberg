using Kreuzberg;

var version = KreuzbergClient.GetVersion();
Console.WriteLine($"Kreuzberg version: {version}");

var result = KreuzbergClient.ExtractFileSync("document.pdf");
Console.WriteLine($"Extraction successful: {result.Success}");
