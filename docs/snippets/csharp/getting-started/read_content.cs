using Kreuzberg;

var data = File.ReadAllBytes("document.pdf");
var result = KreuzbergClient.ExtractBytesSync(data, "application/pdf");

Console.WriteLine(result.Content);
Console.WriteLine($"Success: {result.Success}");
Console.WriteLine($"Content Length: {result.Content.Length}");
