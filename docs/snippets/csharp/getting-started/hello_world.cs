using Kreuzberg;

var result = KreuzbergClient.ExtractFileSync("document.pdf");

Console.WriteLine(result.Content);
