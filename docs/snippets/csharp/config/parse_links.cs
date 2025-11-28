using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = true
};

var result = KreuzbergClient.ExtractFileSync("document.html", config);

if (result.Metadata?.Format.Text?.Links != null)
{
    foreach (var link in result.Metadata.Format.Text.Links)
    {
        var text = link[0];
        var url = link[1];
    }
}
