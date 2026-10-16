using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using XRay.Content.Core;
using XRay.Content.Types;

// XRay.Content C# port — golden parity harness.
//
// Walks a test_documents tree for `{filename}-results-rust.json` golden files (produced by
// tools/xberg-reference-gen against the original Rust library), runs the C# extractor over the
// same source file in each output format, and diffs the result against the golden.
//
// Usage:
//   xray-content-testrunner <root-dir> [options]
//     --filter <substr>   only fixtures whose relative path contains <substr> (e.g. "docx")
//     --ext <ext>         only fixtures with this source extension (e.g. "docx")
//     --show <N>          print up to N example mismatches per dimension (default 5)
//     --diff              print a short unified-ish diff for mismatches
//     --strict-md         count markdown/html mismatches as failures (default: soft/reported)
//     --list-ok           also list fixtures that fully match

// The library never reads the environment itself — a knob hidden behind an env var inside
// library code is invisible to whoever links it. Harnesses opt in explicitly, which is what
// lets one variable drive this port and the Rust original through the same comparison.
XRayOptions.Default = XRayOptions.FromEnvironment();

// Single-file extract mode: `--extract <file> [--format plain|markdown|html|json]`
// Prints the C# extractor's output for one file (used to build the quality gallery).
if (args.Length >= 2 && args[0] == "--extract")
{
    var fmt = OutputFormat.Plain;
    for (int i = 2; i < args.Length - 1; i++)
        if (args[i] == "--format") fmt = OutputFormat.FromString(args[i + 1]);
    var res = new Extractor().Extract(ExtractInput.FromUri(args[1]), new ExtractionConfig { OutputFormat = fmt });
    Console.Out.Write(res.Results.FirstOrDefault()?.Content ?? "");
    return 0;
}

// Metadata dump mode: `--dump-metadata <file>` prints the C# metadata as JSON, so a
// mismatch can be diffed field-by-field against the golden.
if (args.Length >= 2 && args[0] == "--dump-metadata")
{
    var res = new Extractor().Extract(ExtractInput.FromUri(args[1]), new ExtractionConfig { OutputFormat = OutputFormat.Plain });
    Console.Out.Write(SerializeToNode(res.Results.FirstOrDefault()?.Metadata)?.ToJsonString() ?? "null");
    return 0;
}

// Layout-model fetch: `--fetch-layout-model <type> [<cacheDir>]` runs the ported model manager,
// printing where the verified file landed. Used to check the download, digest and publish path
// against the pinned checksums without a full layout run.
if (args.Length >= 2 && args[0] == "--fetch-layout-model")
{
    // The harness is a caller, so it is the one that may opt into the HF_* variables.
    var manager = new XRay.Content.Internal.Layout.LayoutModelManager(
        args.Length > 2 ? args[2] : null, XRay.Content.Core.XRayOptions.FromEnvironment());
    try
    {
        string path = manager.EnsureModelAsync(args[1]).GetAwaiter().GetResult();
        Console.WriteLine($"OK\t{args[1]}\t{path}\t{new FileInfo(path).Length}");
        return 0;
    }
    catch (Exception e)
    {
        Console.Error.WriteLine($"FAIL\t{args[1]}\t{e.Message}");
        return 1;
    }
}

// QR probe mode: `--dump-qr <image>...` prints what the ported detector/decoder found, in the
// same JSON shape as `tools/qr-probe`, so the two can be diffed.
if (args.Length >= 2 && args[0] == "--dump-qr")
{
    var qr = new System.Text.Json.Nodes.JsonObject();
    for (int i = 1; i < args.Length; i++)
    {
        var found = new System.Text.Json.Nodes.JsonArray();
        foreach (var code in XRay.Content.Internal.Qr.QrScanner.Detect(File.ReadAllBytes(args[i])))
            found.Add(new System.Text.Json.Nodes.JsonObject
            {
                ["payload"] = code.Payload,
                ["x"] = code.X,
                ["y"] = code.Y,
                ["width"] = code.Width,
                ["height"] = code.Height,
            });
        qr[args[i]] = found;
    }
    Console.Out.Write(qr.ToJsonString(new System.Text.Json.JsonSerializerOptions { WriteIndented = true }));
    return 0;
}

// YOLO probe mode: `--dump-yolo <image> <variant>=<model.onnx>...` runs the ported YOLO
// wrapper over each model and prints what it decoded, in the same JSON shape as
// `tools/yolo-probe`, so the two can be diffed.
if (args.Length >= 3 && args[0] == "--dump-yolo")
{
    using var page = SixLabors.ImageSharp.Image.Load<SixLabors.ImageSharp.PixelFormats.Rgb24>(args[1]);
    var byVariant = new System.Text.Json.Nodes.JsonObject();

    for (int i = 2; i < args.Length; i++)
    {
        int split = args[i].IndexOf('=');
        string name = args[i][..split];
        string path = args[i][(split + 1)..];

        var (variant, width, height) = name switch
        {
            "doclaynet" => (XRay.Content.Internal.Layout.YoloVariant.DocLayNet, 640, 640),
            "docstructbench" => (XRay.Content.Internal.Layout.YoloVariant.DocStructBench, 1024, 1024),
            "yolox" => (XRay.Content.Internal.Layout.YoloVariant.Yolox, 768, 1024),
            _ => throw new ArgumentException($"unknown variant {name}"),
        };

        var model = XRay.Content.Internal.Layout.YoloModel.FromFile(path, variant, width, height, name);
        var found = new System.Text.Json.Nodes.JsonArray();
        foreach (var detection in model.Detect(page))
            found.Add(new System.Text.Json.Nodes.JsonObject
            {
                ["class"] = XRay.Content.Internal.Layout.LayoutClassExtensions.WireName(detection.ClassName),
                ["confidence"] = detection.Confidence,
                ["x1"] = detection.Box.X1,
                ["y1"] = detection.Box.Y1,
                ["x2"] = detection.Box.X2,
                ["y2"] = detection.Box.Y2,
            });
        byVariant[name] = found;
    }

    Console.Out.Write(byVariant.ToJsonString(new System.Text.Json.JsonSerializerOptions { WriteIndented = true }));
    return 0;
}

// WordPerfect probe mode: `--dump-wpd <file>...` parses each document with the managed
// WordPerfect reader and prints the event stream plus the plain text it renders to, so both can
// be diffed against what libwpd produced.
if (args.Length >= 2 && args[0] == "--dump-wpd")
{
    var documents = new System.Text.Json.Nodes.JsonArray();
    for (int i = 1; i < args.Length; i++)
    {
        var entry = new System.Text.Json.Nodes.JsonObject { ["file"] = args[i] };
        try
        {
            var parsed = XRay.Content.Internal.WordPerfect.WordPerfectReader.Parse(File.ReadAllBytes(args[i]));
            var events = new System.Text.Json.Nodes.JsonArray();
            foreach (var e in parsed.Events)
                events.Add(e.Kind == XRay.Content.Internal.WordPerfect.WpdEventKind.Text
                    ? $"Text:{e.Text}"
                    : e.Kind.ToString());
            entry["format"] = XRay.Content.Internal.WordPerfect.WordPerfectReader.Detect(
                File.ReadAllBytes(args[i])).ToString();
            entry["events"] = events;
            entry["plain"] = XRay.Content.Internal.WordPerfect.WordPerfectReader.RenderPlain(parsed);
        }
        catch (Exception e)
        {
            entry["error"] = e.Message;
        }
        documents.Add(entry);
    }
    Console.Out.Write(documents.ToJsonString(new System.Text.Json.JsonSerializerOptions { WriteIndented = true }));
    return 0;
}

// SLANeXt probe mode: `--dump-slanet <model.onnx> <image>...` runs the ported table-structure
// model and prints its decoded token stream, grid shape and cell boxes.
if (args.Length >= 3 && args[0] == "--dump-slanet")
{
    var slanet = XRay.Content.Internal.Layout.SlanetModel.FromFile(args[1]);
    var pages = new System.Text.Json.Nodes.JsonArray();
    for (int i = 2; i < args.Length; i++)
    {
        using var image = SixLabors.ImageSharp.Image.Load<SixLabors.ImageSharp.PixelFormats.Rgb24>(args[i]);
        var recognized = slanet.Recognize(image);
        var cells = new System.Text.Json.Nodes.JsonArray();
        foreach (var cell in recognized.Cells)
            cells.Add(new System.Text.Json.Nodes.JsonObject
            {
                ["row"] = cell.Row,
                ["col"] = cell.Col,
                ["bbox"] = new System.Text.Json.Nodes.JsonArray(
                    cell.Box[0], cell.Box[1], cell.Box[2], cell.Box[3]),
            });
        pages.Add(new System.Text.Json.Nodes.JsonObject
        {
            ["file"] = args[i],
            ["num_rows"] = recognized.NumRows,
            ["num_cols"] = recognized.NumCols,
            ["confidence"] = recognized.Confidence,
            ["tokens"] = string.Join("", recognized.StructureTokens),
            ["cells"] = cells,
        });
    }
    Console.Out.Write(pages.ToJsonString(new System.Text.Json.JsonSerializerOptions { WriteIndented = true }));
    return 0;
}

// TATR probe mode: `--dump-tatr <model.onnx> <image>...` runs the ported table-structure model
// over each image and prints its detections plus the reconstructed grid shape, matching
// tools/tatr-probe so the two can be diffed.
if (args.Length >= 3 && args[0] == "--dump-tatr")
{
    var tatr = XRay.Content.Internal.Layout.TatrModel.FromFile(args[1]);
    var pages = new System.Text.Json.Nodes.JsonArray();
    for (int i = 2; i < args.Length; i++)
    {
        using var image = SixLabors.ImageSharp.Image.Load<SixLabors.ImageSharp.PixelFormats.Rgb24>(args[i]);
        var recognized = tatr.Recognize(image);

        static System.Text.Json.Nodes.JsonArray Render(
            IEnumerable<XRay.Content.Internal.Layout.TatrDetection> detections)
        {
            var array = new System.Text.Json.Nodes.JsonArray();
            foreach (var d in detections)
                array.Add(new System.Text.Json.Nodes.JsonObject
                {
                    ["class_name"] = d.ClassName.ToString(),
                    ["confidence"] = d.Confidence,
                    ["bbox"] = new System.Text.Json.Nodes.JsonArray(d.X1, d.Y1, d.X2, d.Y2),
                });
            return array;
        }

        var (grid, structure) = XRay.Content.Internal.Layout.TatrModel.BuildCellGridWithStructure(
            recognized, recognized.TableBox);
        pages.Add(new System.Text.Json.Nodes.JsonObject
        {
            ["file"] = args[i],
            ["rows"] = Render(recognized.Rows),
            ["columns"] = Render(recognized.Columns),
            ["headers"] = Render(recognized.Headers),
            ["spanning"] = Render(recognized.Spanning),
            ["table_bbox"] = recognized.TableBox is { } tb
                ? new System.Text.Json.Nodes.JsonArray(tb[0], tb[1], tb[2], tb[3])
                : null,
            ["grid_rows"] = grid.Count,
            ["grid_cols"] = grid.Count > 0 ? grid[0].Count : 0,
            ["header_row_count"] = structure.HeaderRowCount,
            ["spans"] = structure.Spans.Count,
        });
    }
    Console.Out.Write(pages.ToJsonString(new System.Text.Json.JsonSerializerOptions { WriteIndented = true }));
    return 0;
}

// ONNX op census: `--onnx-ops <model.onnx>` lists the distinct operators a graph uses and how
// many nodes use each, so a model that needs a kernel the port lacks names all of them at once
// rather than one exception per run.
if (args.Length >= 2 && args[0] == "--onnx-ops")
{
    var model = XRay.Content.Internal.Onnx.OnnxModel.Load(args[1]);
    var census = new SortedDictionary<string, int>(StringComparer.Ordinal);
    int total = 0;

    // Control-flow nodes carry their own graphs, and a body's operators are just as required as
    // the top level's — counting only top-level nodes reports a graph as runnable when it is not.
    void Walk(IEnumerable<XRay.Content.Internal.Onnx.OnnxNode> nodes)
    {
        foreach (var node in nodes)
        {
            census[node.OpType] = census.TryGetValue(node.OpType, out int n) ? n + 1 : 1;
            total++;
            foreach (var attribute in node.Attributes)
                if (attribute.Graph is { } subgraph) Walk(subgraph.Nodes);
        }
    }

    Walk(model.Nodes);
    foreach (var (op, count) in census) Console.WriteLine($"{count,6}  {op}");
    Console.WriteLine($"distinct={census.Count} nodes={total}");
    return 0;
}

// PP-DocLayout-V3 probe mode: `--dump-ppdoclayout <model.onnx> <image>...` runs the ported model
// and prints the same JSON shape as tools/ppdoclayout-probe so the two can be diffed.
if (args.Length >= 3 && args[0] == "--dump-ppdoclayout")
{
    var layoutModel = XRay.Content.Internal.Layout.PpDocLayoutV3Model.FromFile(args[1]);
    var pages = new System.Text.Json.Nodes.JsonArray();
    for (int i = 2; i < args.Length; i++)
    {
        using var image = SixLabors.ImageSharp.Image.Load<SixLabors.ImageSharp.PixelFormats.Rgb24>(args[i]);
        var found = new System.Text.Json.Nodes.JsonArray();
        foreach (var d in layoutModel.Detect(image))
            found.Add(new System.Text.Json.Nodes.JsonObject
            {
                ["class_name"] = XRay.Content.Internal.Layout.LayoutClassExtensions.WireName(d.ClassName),
                ["confidence"] = d.Confidence,
                ["bbox"] = new System.Text.Json.Nodes.JsonArray(d.Box.X1, d.Box.Y1, d.Box.X2, d.Box.Y2),
            });
        pages.Add(new System.Text.Json.Nodes.JsonObject { ["file"] = args[i], ["detections"] = found });
    }
    Console.Out.Write(pages.ToJsonString(new System.Text.Json.JsonSerializerOptions { WriteIndented = true }));
    return 0;
}

// Table-classifier probe mode: `--dump-tablecls <model.onnx> <image>...` classifies each image
// wired/wireless through the ported PP-LCNet path, printing the same JSON shape as
// tools/tablecls-probe so the two can be diffed.
if (args.Length >= 3 && args[0] == "--dump-tablecls")
{
    var classifier = XRay.Content.Internal.Layout.TableClassifier.FromFile(args[1]);
    var classified = new System.Text.Json.Nodes.JsonArray();
    for (int i = 2; i < args.Length; i++)
    {
        using var image = SixLabors.ImageSharp.Image.Load<SixLabors.ImageSharp.PixelFormats.Rgb24>(args[i]);
        var (type, rawWired, rawWireless) = classifier.ClassifyWithLogits(image);
        classified.Add(new System.Text.Json.Nodes.JsonObject
        {
            ["file"] = args[i],
            ["table_type"] = XRay.Content.Internal.Layout.TableTypeExtensions.Name(type),
            ["raw_wired"] = rawWired,
            ["raw_wireless"] = rawWireless,
        });
    }
    Console.Out.Write(classified.ToJsonString(new System.Text.Json.JsonSerializerOptions { WriteIndented = true }));
    return 0;
}

// Layout heuristics check: `--check-heuristics <probe.json>` reads the differential dump from
// tools/heuristics-probe — the same pages detected twice by the real engine, once with
// apply_heuristics off and once on — applies the ported heuristics to each raw set, and reports
// where the result differs from what upstream produced. Upstream's heuristics carry no tests of
// their own, so this real-detection differential is what verifies them.
if (args.Length >= 2 && args[0] == "--check-heuristics")
{
    using var probe = System.Text.Json.JsonDocument.Parse(File.ReadAllText(args[1]));

    static XRay.Content.Internal.Layout.LayoutClass ParseClass(string label)
    {
        foreach (XRay.Content.Internal.Layout.LayoutClass value
                 in Enum.GetValues<XRay.Content.Internal.Layout.LayoutClass>())
            if (XRay.Content.Internal.Layout.LayoutClassExtensions.WireName(value) == label) return value;
        throw new InvalidOperationException($"unknown layout class '{label}'");
    }

    static List<XRay.Content.Internal.Layout.LayoutDetection> ReadDetections(System.Text.Json.JsonElement page)
    {
        var list = new List<XRay.Content.Internal.Layout.LayoutDetection>();
        foreach (var d in page.GetProperty("detections").EnumerateArray())
        {
            var box = d.GetProperty("bbox");
            list.Add(new XRay.Content.Internal.Layout.LayoutDetection(
                ParseClass(d.GetProperty("class_name").GetString()!),
                d.GetProperty("confidence").GetSingle(),
                new XRay.Content.Internal.Layout.BBox(
                    box[0].GetSingle(), box[1].GetSingle(), box[2].GetSingle(), box[3].GetSingle())));
        }
        return list;
    }

    static string Render(XRay.Content.Internal.Layout.LayoutDetection d) =>
        string.Create(System.Globalization.CultureInfo.InvariantCulture,
            $"{XRay.Content.Internal.Layout.LayoutClassExtensions.WireName(d.ClassName)}@{d.Confidence:F6}{d.Box}");

    int pages = 0, matched = 0, rawTotal = 0, expectedTotal = 0;
    foreach (var entry in probe.RootElement.EnumerateArray())
    {
        pages++;
        var rawPage = entry.GetProperty("raw");
        var cookedPage = entry.GetProperty("cooked");
        var raw = ReadDetections(rawPage);
        var expected = ReadDetections(cookedPage);
        rawTotal += raw.Count;
        expectedTotal += expected.Count;

        var actual = XRay.Content.Internal.Layout.LayoutPostprocessing.ApplyHeuristics(
            raw, rawPage.GetProperty("page_width").GetSingle(), rawPage.GetProperty("page_height").GetSingle());

        var expectedText = expected.Select(Render).ToList();
        var actualText = actual.Select(Render).ToList();
        if (expectedText.SequenceEqual(actualText)) { matched++; continue; }

        Console.WriteLine($"MISMATCH {entry.GetProperty("file").GetString()}");
        Console.WriteLine($"  expected ({expectedText.Count}): {string.Join(", ", expectedText)}");
        Console.WriteLine($"  actual   ({actualText.Count}): {string.Join(", ", actualText)}");
    }

    Console.WriteLine($"pages={pages} matched={matched} raw={rawTotal} expected={expectedTotal}");
    return matched == pages ? 0 : 1;
}

// DocTags probe mode: `--dump-doctags <file>...` renders each file to DocTags and then feeds
// that stream back through the DocTags extractor, printing the same JSON shape as
// `tools/doctags-probe` so the two can be diffed. Both stages are printed, so a divergence pins
// to the renderer or to the parser.
if (args.Length >= 2 && args[0] == "--dump-doctags")
{
    var doctags = new System.Text.Json.Nodes.JsonObject();
    var dtExtractor = new Extractor();
    string RenderDocTags(ExtractInput input)
    {
        var cfg = new ExtractionConfig { OutputFormat = OutputFormat.DocTags };
        try
        {
            var r = dtExtractor.Extract(input, cfg);
            if (r.Errors.Count > 0) return $"<<error: {r.Errors[0].Message}>>";
            return r.Results.FirstOrDefault()?.Content ?? "";
        }
        catch (Exception e) { return $"<<error: {e.Message}>>"; }
    }
    for (int i = 1; i < args.Length; i++)
    {
        // A real Docling stream is fed in as DocTags bytes rather than by extension: the corpus
        // names them `*.doctags.txt`, which resolves as plain text, so the extractor would never
        // otherwise see one.
        string first = args[i].EndsWith(".doctags.txt", StringComparison.Ordinal)
            ? RenderDocTags(ExtractInput.FromBytes(File.ReadAllBytes(args[i]), "text/vnd.docling.doctags"))
            : RenderDocTags(ExtractInput.FromUri(args[i]));
        string second = RenderDocTags(ExtractInput.FromBytes(
            System.Text.Encoding.UTF8.GetBytes(first), "text/vnd.docling.doctags"));
        doctags[args[i]] = new System.Text.Json.Nodes.JsonObject
        {
            ["render"] = first,
            ["roundtrip"] = second,
        };
    }
    Console.Out.Write(doctags.ToJsonString(new System.Text.Json.JsonSerializerOptions { WriteIndented = true }));
    return 0;
}

// Styled-HTML probe mode: `--dump-styled-html <file>...` renders each file through
// `StyledHtmlRenderer` under every configuration `tools/htmlstyled-probe` covers, printing the
// same JSON shape so the two can be diffed byte for byte.
if (args.Length >= 2 && args[0] == "--dump-styled-html")
{
    var cases = new (string Name, HtmlOutputConfig Config)[]
    {
        ("unstyled-embed", new HtmlOutputConfig()),
        ("default-embed", new HtmlOutputConfig { Theme = HtmlTheme.Default }),
        ("github-embed", new HtmlOutputConfig { Theme = HtmlTheme.GitHub }),
        ("dark-embed", new HtmlOutputConfig { Theme = HtmlTheme.Dark }),
        ("light-embed", new HtmlOutputConfig { Theme = HtmlTheme.Light }),
        ("default-noembed", new HtmlOutputConfig { Theme = HtmlTheme.Default, EmbedCss = false }),
        ("unstyled-prefix", new HtmlOutputConfig { ClassPrefix = "zz-" }),
        ("unstyled-usercss", new HtmlOutputConfig { Css = ".kb-p { color: red; }" }),
    };
    var probe = new System.Text.Json.Nodes.JsonObject();
    var probeExtractor = new Extractor();
    for (int i = 1; i < args.Length; i++)
    {
        var perCase = new System.Text.Json.Nodes.JsonObject();
        foreach (var (name, html) in cases)
        {
            var cfg = new ExtractionConfig { OutputFormat = OutputFormat.Html, HtmlOutput = html };
            string content;
            try
            {
                var r = probeExtractor.Extract(ExtractInput.FromUri(args[i]), cfg);
                content = r.Results.FirstOrDefault()?.Content ?? "";
            }
            catch (Exception e) { content = $"<<error: {e.Message}>>"; }
            perCase[name] = content;
        }
        probe[args[i]] = perCase;
    }
    Console.Out.Write(probe.ToJsonString(new System.Text.Json.JsonSerializerOptions { WriteIndented = true }));
    return 0;
}

// Table dump mode: `--dump-tables <file>` prints the C# tables as JSON, so a mismatch can be
// diffed cell-by-cell against the golden's `tables` array.
if (args.Length >= 2 && args[0] == "--dump-tables")
{
    var res = new Extractor().Extract(ExtractInput.FromUri(args[1]), new ExtractionConfig { OutputFormat = OutputFormat.Plain });
    Console.Out.Write(SerializeToNode(res.Results.FirstOrDefault()?.Tables)?.ToJsonString() ?? "null");
    return 0;
}

// Path dump mode: `--dump-pdfpaths <file> [page] [--words]` prints the geometry one PDF page
// hands the ruled-table tier — every collected path with its classification, optionally the
// word boxes — plus what each tier makes of it. A table that does not come out right is almost
// always a page whose paths are not what they look like (a strikethrough is a line, a shading
// is a rectangle), and this is how to see that without guessing.
if (args.Length >= 2 && args[0] == "--dump-pdfpaths")
{
    int wantPage = args.Length >= 3 && int.TryParse(args[2], out int p) ? p : 0;
    bool withWords = args.Contains("--words");
    var pdf = XRay.Content.Internal.Pdf.PdfDocument.Open(File.ReadAllBytes(args[1]));
    if (pdf is null) { Console.Error.WriteLine($"not a readable PDF: {args[1]}"); return 2; }
    XRay.Content.Extractors.PdfExtractor.ExtractTextAndSegments(
        pdf, DateTime.UtcNow.AddMinutes(5).Ticks, XRayOptions.Default,
        out _, out var pageWords, out var pagePaths);

    for (int i = 0; i < pagePaths.Count; i++)
    {
        if (wantPage > 0 && i + 1 != wantPage) continue;
        Console.WriteLine($"## page {i + 1}: paths={pagePaths[i].Count} words={pageWords[i].Count}");
        foreach (var path in pagePaths[i])
        {
            var b = path.Bbox;
            var r = path.RenderedBbox();
            Console.WriteLine(
                $"  kind={string.Join(",", path.Operations.Select(o => o.Kind))} "
                + $"stroked={path.Stroked} filled={path.Filled} width={path.StrokeWidth:F2} "
                + $"primitive={path.IsTablePrimitive()} rule={path.IsRuleCandidate()} "
                + $"bbox=({b.Left:F1},{b.Top:F1},{b.Right:F1},{b.Bottom:F1}) "
                + $"rendered=({r.Left:F1},{r.Top:F1},{r.Right:F1},{r.Bottom:F1})");
        }
        if (withWords)
            foreach (var w in pageWords[i])
                Console.WriteLine($"  word {w.Text} "
                    + $"bbox=({w.Bbox.Left:F1},{w.Bbox.Top:F1},{w.Bbox.Right:F1},{w.Bbox.Bottom:F1})");

        int strict = XRay.Content.Internal.Pdf.PdfSpatialTables
            .DetectPageTables(pageWords[i], pagePaths[i], (uint)(i + 1),
                XRay.Content.Internal.Pdf.TableDetectionConfig.Strict()).Count;
        int bordered = XRay.Content.Internal.Pdf.PdfSpatialTables
            .DetectPageTables(pageWords[i], pagePaths[i], (uint)(i + 1),
                XRay.Content.Internal.Pdf.TableDetectionConfig.Bordered()).Count;
        Console.WriteLine($"  -> strict={strict} bordered={bordered}");
    }
    return 0;
}

var opts = ParseArgs(args);
if (opts is null) return 2;

// Goldens live next to their fixtures by default. `--goldens <dir>` reads them from a
// mirror of the fixture tree instead, which is how one corpus carries several golden sets —
// one per xberg feature configuration — without either overwriting the other.
var goldenRoot = opts.GoldenRoot ?? opts.Root;
var goldenFiles = Directory
    .EnumerateFiles(goldenRoot, "*-results-rust.json", SearchOption.AllDirectories)
    .OrderBy(p => p, StringComparer.Ordinal)
    .ToList();

if (goldenFiles.Count == 0)
{
    Console.Error.WriteLine($"No *-results-rust.json golden files under {goldenRoot}");
    return 2;
}

var extractor = new Extractor();
var stats = new Stats();
var examples = new Dictionary<string, List<string>>();
// --cluster: group plain-text mismatches by the text at their first divergence, so a
// systematic defect shows up as one bucket with a count instead of N separate fixtures.
var clusters = new Dictionary<string, (int Count, string Rel, string Want, string Have)>(StringComparer.Ordinal);

foreach (var goldenPath in goldenFiles)
{
    var goldenStem = goldenPath[..^"-results-rust.json".Length];
    // With `--goldens`, the golden sits at the fixture's relative position under the golden
    // root; the fixture itself is still under `--root`.
    var rel = Path.GetRelativePath(goldenRoot, goldenStem).Replace('\\', '/');
    var sourcePath = opts.GoldenRoot is null ? goldenStem : Path.Combine(opts.Root, rel);
    var ext = Path.GetExtension(sourcePath).TrimStart('.').ToLowerInvariant();

    if (opts.Filter is not null && !rel.Contains(opts.Filter, StringComparison.OrdinalIgnoreCase)) continue;
    if (opts.Ext is not null && !ext.Equals(opts.Ext, StringComparison.OrdinalIgnoreCase)) continue;
    if (!File.Exists(sourcePath)) { stats.NoSource++; continue; }

    JsonNode golden;
    try { golden = JsonNode.Parse(File.ReadAllText(goldenPath))!; }
    catch { stats.BadGolden++; continue; }

    stats.Total++;
    stats.ByExt.TryAdd(ext, new ExtStat());
    var es = stats.ByExt[ext];
    es.Total++;

    bool rustSuccess = golden["success"]?.GetValue<bool>() ?? true;

    // Run the C# extractor for each output format.
    var got = new Dictionary<string, string>();
    ExtractedDocument? plainDoc = null;
    bool csharpFailed = false;
    // The extractor reports a refusal as an error item rather than throwing, and a fixture that
    // starts being refused is exactly what a change to the security limits would do; without
    // this the sweep only shows it as a fixture quietly moving into the "both empty" bucket.
    string csharpError = "";
    foreach (var (name, fmt) in Formats())
    {
        try
        {
            // Per-file guard: a pathological fixture must not stall the whole sweep.
            var capturedPath = sourcePath;
            var task = System.Threading.Tasks.Task.Run(() =>
                extractor.Extract(ExtractInput.FromUri(capturedPath), Cfg.Make(fmt, opts)));
            if (!task.Wait(TimeSpan.FromSeconds(120)))
            {
                csharpFailed = true;
                got[name] = "";
                Console.Error.WriteLine($"TIMEOUT {name} {rel}");
                continue;
            }
            var doc = task.Result.Results.FirstOrDefault();
            got[name] = doc?.Content ?? "";
            if (name == "plain")
            {
                plainDoc = doc;
                if (task.Result.Errors.Count > 0)
                {
                    csharpFailed = true;
                    csharpError = $"{task.Result.Errors[0].ErrorType}: {task.Result.Errors[0].Message}";
                }
            }
        }
        catch (Exception ex)
        {
            csharpFailed = true;
            got[name] = "";
            if (csharpError.Length == 0) csharpError = ex.Message;
        }
    }

    // Both sides couldn't handle it → not a regression; count separately.
    bool csharpEmpty = string.IsNullOrEmpty(got["plain"]) && (plainDoc is null || plainDoc.Metadata.Format is null);
    if (!rustSuccess)
    {
        if (csharpFailed || csharpEmpty)
        {
            es.BothUnsupported++; stats.BothUnsupported++;
            // Rust extracts nothing from this fixture, so parity says nothing about it either
            // way — but if the port would have produced something with the security limits
            // lifted, then the limits are what emptied it, and that is worth naming. Only the
            // non-comparable set is re-run, which is small.
            if (opts.ListFail && !opts.NoSecurity)
            {
                try
                {
                    var loose = Cfg.Make(OutputFormat.Plain, new Options { NoSecurity = true });
                    var again = extractor.Extract(ExtractInput.FromUri(sourcePath), loose);
                    var got2 = again.Results.FirstOrDefault();
                    bool wouldHaveContent = !string.IsNullOrEmpty(got2?.Content)
                        || (got2 is not null && got2.Metadata.Format is not null);
                    if (wouldHaveContent && again.Errors.Count == 0)
                        Console.WriteLine($"SECEMPTY\t{ext}\t{rel}\t{csharpError}");
                }
                catch { /* the loose run failing too means the limits were not the cause */ }
            }
            // Rust extracts nothing here either, so this is not a parity failure — but a
            // fixture that moves into this bucket after a change is one the port stopped
            // handling, and `--list-fail` is where that has to be visible.
            if (opts.ListFail && csharpFailed)
                Console.WriteLine($"CSFAIL\t{ext}\t{rel}\t{csharpError}");
        }
        else { es.Extra++; stats.Extra++; }
        continue;
    }

    // Rust succeeded — measure each dimension.
    bool allHard = true;
    var failedDims = new List<string>();
    foreach (var (name, _) in Formats())
    {
        string want = golden["content"]?[name]?.GetValue<string>() ?? "";
        string have = got[name];
        bool soft = name is "markdown" or "html" && !opts.StrictMd;
        bool match = name is "plain" or "json"
            ? want == have
            : NormalizeText(want) == NormalizeText(have);

        var dim = es.Dim(name);
        if (match) dim.Match++;
        else
        {
            dim.Mismatch++;
            if (!soft) allHard = false; else failedDims.Add("~" + name);
            if (!soft) failedDims.Add(name);
            AddExample(examples, $"{name}", rel, want, have, opts);
            if (opts.Dump is not null)
            {
                var safe = rel.Replace('/', '_');
                Directory.CreateDirectory(opts.Dump);
                File.WriteAllText(Path.Combine(opts.Dump, $"{safe}.{name}.rust.txt"), want);
                File.WriteAllText(Path.Combine(opts.Dump, $"{safe}.{name}.cs.txt"), have);
            }
        }
    }

    // Metadata + tables (structural).
    bool metaMatch = JsonEquivalent(golden["metadata"], SerializeToNode(plainDoc?.Metadata), MetadataNormalizer);
    if (metaMatch) es.MetaMatch++; else { es.MetaMismatch++; allHard = false; failedDims.Add("metadata"); AddExample(examples, "metadata", rel, Compact(golden["metadata"]), Compact(SerializeToNode(plainDoc?.Metadata)), opts); }

    bool tablesMatch = JsonEquivalent(golden["tables"], SerializeToNode(plainDoc?.Tables), TablesNormalizer);
    if (tablesMatch) es.TablesMatch++; else { es.TablesMismatch++; allHard = false; failedDims.Add("tables"); AddExample(examples, "tables", rel, Compact(golden["tables"]), Compact(SerializeToNode(plainDoc?.Tables)), opts); }

    if (allHard) { es.Ok++; stats.Ok++; if (opts.ListOk) Console.WriteLine($"  ok  {rel}"); }
    else if (opts.ListFail)
        Console.WriteLine($"FAIL\t{Path.GetExtension(rel).TrimStart('.').ToLowerInvariant()}\t{rel}\t{string.Join(",", failedDims)}");

    // ── Content-parity (separate from byte-parity) ──────────────────────────
    // Byte-parity penalises us for cosmetic and even for being-more-correct-than-Rust
    // differences. Content-parity asks the honest question: is the extracted TEXT the
    // same, ignoring whitespace/spacing and reading-order? Measured on the plain text.
    {
        string rustPlain = golden["content"]?["plain"]?.GetValue<string>() ?? "";
        string csPlain = got["plain"];
        if (opts.Cluster)
        {
            // --cluster-format picks which rendered format the clusters are computed over,
            // so a defect that only shows up in markdown or html can be isolated too.
            string cw = golden["content"]?[opts.ClusterFormat]?.GetValue<string>() ?? "";
            string cg = got.TryGetValue(opts.ClusterFormat, out var cgv) ? cgv : "";
            if (cw != cg) RecordCluster(clusters, rel, cw, cg);
        }
        if (rustPlain == csPlain) { es.ContentExact++; stats.ContentExact++; stats.ContentClose++; }
        else if (NormalizeText(rustPlain) == NormalizeText(csPlain)) { es.ContentExact++; stats.ContentExact++; stats.ContentClose++; }
        else if (NormalizeSorted(rustPlain) == NormalizeSorted(csPlain)) { stats.ContentOrder++; stats.ContentClose++; }
        else
        {
            double r = Similarity(rustPlain, csPlain);
            if (r >= 0.95) stats.ContentClose++;
            else if (r >= 0.80) stats.ContentPartial++;
            else stats.ContentLow++;

            // Record real content losses: Rust extracted clean, substantial text and we
            // produced meaningfully less of it (not just reformatted / reordered).
            bool rClean = GarbageRatio(rustPlain) < 0.02 && NormalizeText(rustPlain).Length >= 40;
            if (r < 0.80 && rClean && NormalizeText(csPlain).Length < NormalizeText(rustPlain).Length * 0.9)
                stats.Misses.Add(new MissRow(ext, rel, rustPlain.Length, csPlain.Length, r));
        }

        // ── Catastrophe audit (judges OUR output on its own merits) ──────────
        // A catastrophe = output a human would call broken, independent of Rust parity:
        //   crash, timeout, empty-when-there-is-content, mojibake, severe under-extraction.
        // Only flag output WE broke relative to Rust — if the Rust reference is itself
        // garbage/binary/empty on a fixture, matching it is correct, not a catastrophe.
        string? cat = null;
        double csGarbage = GarbageRatio(csPlain);
        double rustGarbage = GarbageRatio(rustPlain);
        bool rustClean = rustGarbage < 0.02;
        bool rustHadContent = NormalizeText(rustPlain).Length >= 40 && rustClean;
        if (csharpFailed && csPlain.Length == 0) cat = "CRASH/TIMEOUT (threw or hung; Rust succeeded)";
        else if (csGarbage > 0.02 && csGarbage > rustGarbage + 0.02) cat = $"MOJIBAKE ({csGarbage:P0} vs Rust {rustGarbage:P0})";
        else if (rustHadContent && NormalizeText(csPlain).Length == 0) cat = "EMPTY (Rust extracted text, we got nothing)";
        else if (rustHadContent && csPlain.Length < rustPlain.Length * 0.25) cat = $"SEVERE UNDER-EXTRACTION ({csPlain.Length}/{rustPlain.Length} chars)";
        if (cat is not null)
        {
            stats.Catastrophes++;
            if (stats.CatastropheList.Count < 60) stats.CatastropheList.Add($"  [{ext}] {rel}  →  {cat}");
        }
    }
}

PrintReport(stats, examples, opts);
Console.WriteLine();
if (opts.Cluster)
{
    Console.WriteLine();
    Console.WriteLine($"─── First-divergence clusters ({clusters.Count} distinct) ───");
    foreach (var (key, c) in clusters.OrderByDescending(kv => kv.Value.Count).Take(25))
        Console.WriteLine($"  {c.Count,4}  rust={Quote(c.Want),-46} c#={Quote(c.Have),-46}  e.g. {c.Rel}");
    Console.WriteLine();
}

Console.WriteLine("─── Content parity (plain text; whitespace/order-normalized) ───");
int cTotal = stats.Total;
Console.WriteLine($"  content-identical (ws/order-normalized): {stats.ContentExact + stats.ContentOrder}  ({Pct(stats.ContentExact + stats.ContentOrder, cTotal)})");
Console.WriteLine($"  ≥95% similar (near-identical):           {stats.ContentClose}  ({Pct(stats.ContentClose, cTotal)})");
Console.WriteLine($"  80–95% similar (minor content drift):    {stats.ContentPartial}");
Console.WriteLine($"  <80% similar (real content miss):        {stats.ContentLow}");
Console.WriteLine();
Console.WriteLine($"─── Catastrophe audit (broken output, judged on its own merits) ───");
Console.WriteLine($"  catastrophes: {stats.Catastrophes}  ({Pct(stats.Catastrophes, cTotal)} of fixtures)");
foreach (var line in stats.CatastropheList) Console.WriteLine(line);

Console.WriteLine();
Console.WriteLine($"─── Content losses (Rust caught clean text we under-extract) — {stats.Misses.Count} fixtures ───");
// Group by extension: count + total chars lost, worst formats first.
foreach (var g in stats.Misses.GroupBy(m => m.Ext)
                              .Select(g => new { Ext = g.Key, N = g.Count(), Lost = g.Sum(m => (long)(m.RustLen - m.CsLen)) })
                              .OrderByDescending(g => g.Lost))
    Console.WriteLine($"  {g.Ext,-8} {g.N,4} fixtures   {g.Lost,10:N0} chars lost");
Console.WriteLine("  ── worst 40 individual fixtures (by chars lost) ──");
foreach (var m in stats.Misses.OrderByDescending(m => m.RustLen - m.CsLen).Take(40))
    Console.WriteLine($"  [{m.Ext,-6}] rust={m.RustLen,8:N0}  cs={m.CsLen,8:N0}  sim={m.Sim:F2}  {m.Rel}");
return 0;

static string Pct(int n, int d) => d == 0 ? "—" : $"{100.0 * n / d:F1}%";

// Fraction of chars that are U+FFFD (mojibake) or C0/C1 control (excluding \t\n\r).
static double GarbageRatio(string s)
{
    if (s.Length == 0) return 0;
    int bad = 0;
    foreach (char c in s)
        if (c == '�' || (c < 0x20 && c != '\t' && c != '\n' && c != '\r') || (c >= 0x7F && c <= 0x9F))
            bad++;
    return (double)bad / s.Length;
}

// Whitespace + line-order insensitive (catches pure reading-order differences).
static string NormalizeSorted(string s)
{
    var lines = s.Split('\n').Select(l => NormalizeText(l)).Where(l => l.Length > 0).ToList();
    lines.Sort(StringComparer.Ordinal);
    return string.Join("\n", lines);
}

static double Similarity(string a, string b)
{
    // Bounded LCS-ratio on capped inputs (cheap, good enough for bucketing).
    const int cap = 2500;  // LCS is O(n*m); a small cap keeps similarity bucketing cheap
    if (a.Length > cap) a = a[..cap];
    if (b.Length > cap) b = b[..cap];
    if (a.Length == 0 && b.Length == 0) return 1.0;
    int lcs = Lcs(a, b);
    return 2.0 * lcs / (a.Length + b.Length);
}

static int Lcs(string a, string b)
{
    var prev = new int[b.Length + 1];
    var cur = new int[b.Length + 1];
    for (int i = 1; i <= a.Length; i++)
    {
        for (int j = 1; j <= b.Length; j++)
            cur[j] = a[i - 1] == b[j - 1] ? prev[j - 1] + 1 : Math.Max(prev[j], cur[j - 1]);
        (prev, cur) = (cur, prev);
    }
    return prev[b.Length];
}

// ────────────────────────────────────────────────────────────────────────────
static (string name, OutputFormat fmt)[] Formats() =>
    [("plain", OutputFormat.Plain), ("markdown", OutputFormat.Markdown), ("html", OutputFormat.Html), ("json", OutputFormat.Json)];

static JsonNode? SerializeToNode(object? value)
{
    if (value is null) return null;
    var json = JsonSerializer.Serialize(value, value.GetType(), XRay.Content.Types.Json.Options);
    return JsonNode.Parse(json);
}

// Whitespace-insensitive text comparison for the not-yet-byte-exact markdown/html renderers.
static string NormalizeText(string s)
{
    var sb = new StringBuilder(s.Length);
    bool prevWs = false;
    foreach (char c in s)
    {
        if (char.IsWhiteSpace(c)) { if (!prevWs) sb.Append(' '); prevWs = true; }
        else { sb.Append(c); prevWs = false; }
    }
    return sb.ToString().Trim();
}

// Structural JSON comparison treating null == absent == empty([]/{}) as equal, after an
// optional per-domain normalization pass (drops volatile / not-yet-ported keys).
static bool JsonEquivalent(JsonNode? a, JsonNode? b, Action<JsonObject>? normalizeObj)
{
    a = Canonicalize(a?.DeepClone(), normalizeObj);
    b = Canonicalize(b?.DeepClone(), normalizeObj);
    return JsonNodeDeepEquals(a, b);
}

static JsonNode? Canonicalize(JsonNode? n, Action<JsonObject>? normalizeObj)
{
    switch (n)
    {
        case null: return null;
        case JsonObject obj:
            normalizeObj?.Invoke(obj);
            var keys = obj.Select(kv => kv.Key).ToList();
            foreach (var k in keys)
            {
                var child = Canonicalize(obj[k]?.DeepClone(), normalizeObj);
                if (IsEmpty(child)) obj.Remove(k);
                else obj[k] = child;
            }
            return obj.Count == 0 ? null : obj;
        case JsonArray arr:
            for (int i = 0; i < arr.Count; i++) arr[i] = Canonicalize(arr[i]?.DeepClone(), normalizeObj);
            return arr.Count == 0 ? null : arr;
        default: return n;
    }
}

static bool IsEmpty(JsonNode? n) => n is null || (n is JsonArray a && a.Count == 0) || (n is JsonObject o && o.Count == 0);

static bool JsonNodeDeepEquals(JsonNode? a, JsonNode? b)
{
    if (a is null && b is null) return true;
    if (a is null || b is null) return false;
    if (a is JsonObject oa && b is JsonObject ob)
    {
        if (oa.Count != ob.Count) return false;
        foreach (var kv in oa)
        {
            if (!ob.TryGetPropertyValue(kv.Key, out var bv)) return false;
            if (!JsonNodeDeepEquals(kv.Value, bv)) return false;
        }
        return true;
    }
    if (a is JsonArray aa && b is JsonArray ab)
    {
        if (aa.Count != ab.Count) return false;
        for (int i = 0; i < aa.Count; i++) if (!JsonNodeDeepEquals(aa[i], ab[i])) return false;
        return true;
    }
    return a.ToJsonString() == b.ToJsonString();
}

// Drop machine/runtime-volatile metadata bookkeeping before comparison.
static void MetadataNormalizer(JsonObject obj)
{
    if (obj.TryGetPropertyValue("additional", out var add) && add is JsonObject ao)
        foreach (var k in new[] { "source_uri", "final_uri", "source_index", "source_kind", "extraction_method" })
            ao.Remove(k);
    obj.Remove("extraction_duration_ms");
    obj.Remove("output_format");

    // Port additions to a PDF's metadata that upstream's goldens cannot carry: encryption detail,
    // the outline, the catalog language, custom Info entries and extra XMP properties. They add
    // information without changing any field upstream reports, so a golden comparison ignores them.
    // The metadata object is visited once as itself (where `additional` and `language` live) and
    // once as the nested `format` object (where the payload fields live); either may carry the tag.
    bool pdfHere = obj.TryGetPropertyValue("format_type", out var formatType)
        && formatType is JsonValue fv && fv.TryGetValue<string>(out var ft) && ft == "pdf";
    bool pdfNested = obj.TryGetPropertyValue("format", out var nestedFormat) && nestedFormat is JsonObject nf
        && nf.TryGetPropertyValue("format_type", out var nft) && nft is JsonValue nv
        && nv.TryGetValue<string>(out var nvs) && nvs == "pdf";
    if (pdfHere)
        foreach (var k in new[]
        {
            "requires_password", "authenticated_as", "encryption_algorithm", "encryption_key_bits",
            "permissions", "outline",
        })
            obj.Remove(k);
    if (pdfHere || pdfNested)
    {
        obj.Remove("language");
        if (obj.TryGetPropertyValue("additional", out var pdfAdd) && pdfAdd is JsonObject pdfAo)
            foreach (var k in new[] { "pdf_catalog_version", "pdf_info", "xmp" })
                pdfAo.Remove(k);
    }
}

// The per-table `markdown` string tracks the (not-yet-exact) markdown renderer; compare cells only.
static void TablesNormalizer(JsonObject obj) => obj.Remove("markdown");

static string Compact(JsonNode? n) => n?.ToJsonString() ?? "null";

// Bucket a mismatch by the first position where the two texts diverge, keyed on a short
// window of both sides so unrelated fixtures with the same defect land together.
static void RecordCluster(Dictionary<string, (int, string, string, string)> clusters,
    string rel, string want, string have)
{
    int n = Math.Min(want.Length, have.Length), i = 0;
    while (i < n && want[i] == have[i]) i++;
    string w = want[i..Math.Min(want.Length, i + 30)];
    string h = have[i..Math.Min(have.Length, i + 30)];
    string key = w[..Math.Min(w.Length, 14)] + "\u0000" + h[..Math.Min(h.Length, 14)];
    if (clusters.TryGetValue(key, out var cur)) clusters[key] = (cur.Item1 + 1, cur.Item2, cur.Item3, cur.Item4);
    else clusters[key] = (1, rel, w, h);
}

static void AddExample(Dictionary<string, List<string>> ex, string dim, string rel, string want, string have, Options o)
{
    if (!ex.TryGetValue(dim, out var list)) ex[dim] = list = new();
    if (list.Count >= o.Show) return;
    var sb = new StringBuilder();
    sb.Append("    • ").Append(rel);
    if (o.Diff)
    {
        sb.Append("\n      rust: ").Append(Trunc(want, 200));
        sb.Append("\n      c#  : ").Append(Trunc(have, 200));
    }
    list.Add(sb.ToString());
}

static string Quote(string s) => "\"" + s.Replace("\n", "\\n").Replace("\t", "\\t") + "\"";

static string Trunc(string s, int n) => s.Length <= n ? s.Replace("\n", "\\n") : s[..n].Replace("\n", "\\n") + "…";

static void PrintReport(Stats s, Dictionary<string, List<string>> ex, Options o)
{
    Console.WriteLine();
    Console.WriteLine("═══ XRay.Content C# parity report ═══");
    // `Total` counts every fixture walked, including those Rust itself could not extract.
    // Only the fixtures Rust did extract are comparable, and dividing by anything else
    // scores this port against documents upstream never read.
    int comparable = s.Total - s.BothUnsupported - s.Extra;
    Console.WriteLine($"Fixtures walked:                   {s.Total}");
    Console.WriteLine($"  comparable (rust extracted):     {comparable}");
    Console.WriteLine($"  fully matching (hard dims):      {s.Ok}" +
        (comparable > 0 ? $"  ({100.0 * s.Ok / comparable:F1}% of comparable)" : ""));
    Console.WriteLine($"  failing at least one hard dim:   {comparable - s.Ok}");
    Console.WriteLine($"  rust failed, C# also empty:      {s.BothUnsupported}");
    Console.WriteLine($"  rust failed, C# produced output: {s.Extra}");
    Console.WriteLine($"  source file missing:             {s.NoSource}");
    Console.WriteLine();
    Console.WriteLine($"{"ext",-10}{"n",5}{"ok",6}{"plain",8}{"md",8}{"html",8}{"json",8}{"meta",8}{"tables",8}");
    foreach (var (ext, es) in s.ByExt.OrderByDescending(k => k.Value.Total))
    {
        Console.WriteLine($"{ext,-10}{es.Total,5}{es.Ok,6}" +
            $"{Frac(es.Plain),8}{Frac(es.Markdown),8}{Frac(es.Html),8}{Frac(es.Json),8}" +
            $"{es.MetaMatch + "/" + (es.MetaMatch + es.MetaMismatch),8}{es.TablesMatch + "/" + (es.TablesMatch + es.TablesMismatch),8}");
    }
    if (ex.Count > 0)
    {
        Console.WriteLine();
        Console.WriteLine("Example mismatches:");
        foreach (var (dim, list) in ex)
        {
            Console.WriteLine($"  [{dim}]");
            foreach (var line in list) Console.WriteLine(line);
        }
    }
}

static string Frac(DimStat d) => d.Match + d.Mismatch == 0 ? "-" : $"{d.Match}/{d.Match + d.Mismatch}";

static Options? ParseArgs(string[] args)
{
    if (args.Length == 0)
    {
        Console.Error.WriteLine("Usage: xray-content-testrunner <root-dir> [--filter s] [--ext e] [--show N] [--diff] [--strict-md] [--list-ok]");
        return null;
    }
    var o = new Options { Root = args[0] };
    for (int i = 1; i < args.Length; i++)
    {
        switch (args[i])
        {
            case "--filter": o.Filter = args[++i]; break;
            case "--ext": o.Ext = args[++i]; break;
            case "--show": o.Show = int.Parse(args[++i]); break;
            case "--diff": o.Diff = true; break;
            case "--cluster": o.Cluster = true; break;
            case "--cluster-format": o.ClusterFormat = args[++i]; break;
            case "--strict-md": o.StrictMd = true; break;
            case "--list-ok": o.ListOk = true; break;
            case "--list-fail": o.ListFail = true; break;
            case "--dump": o.Dump = args[++i]; break;
            case "--goldens": o.GoldenRoot = args[++i]; break;
            case "--no-security": o.NoSecurity = true; break;
            case "--features":
                foreach (var f in args[++i].Split(',', StringSplitOptions.RemoveEmptyEntries))
                    if (f.Trim() == "code") o.SourceCodeDetection = true;
                break;
        }
    }
    if (!Directory.Exists(o.Root)) { Console.Error.WriteLine($"not a directory: {o.Root}"); return null; }
    return o;
}

static partial class Cfg
{
    /// <summary>The extraction config for one sweep run.</summary>
    public static ExtractionConfig Make(OutputFormat fmt, Options o) => new()
    {
        OutputFormat = fmt,
        SecurityLimits = o.NoSecurity ? Unbounded : null,
        // Upstream gates source-code detection behind the `tree-sitter` cargo feature, so a
        // golden set generated without it reads a `.py` file as plain text. The port decides at
        // run time instead, and the sweep has to be told which golden set it is measuring
        // against: `--features code` alongside `--goldens <extended tree>`.
        Options = new XRayOptions { SourceCodeDetection = o.SourceCodeDetection },
    };

    private static readonly SecurityLimits Unbounded = new()
    {
        MaxArchiveSize = long.MaxValue,
        MaxCompressionRatio = long.MaxValue,
        MaxFilesInArchive = long.MaxValue,
        MaxNestingDepth = long.MaxValue,
        MaxEntityLength = long.MaxValue,
        MaxContentSize = long.MaxValue,
        MaxIterations = long.MaxValue,
        MaxXmlDepth = long.MaxValue,
        MaxTableCells = long.MaxValue,
    };
}

sealed class Options
{
    public string Root = "";
    public string? Filter;
    public string? Ext;
    public int Show = 5;
    public bool Diff;
    public bool Cluster;
    public string ClusterFormat = "plain";
    public bool StrictMd;
    public bool ListOk;
    public bool ListFail;
    public string? Dump;

    /// <summary>Root of a mirrored golden tree (`--goldens`), or null to read them next to
    /// the fixtures.</summary>
    public string? GoldenRoot;

    /// <summary>Raise every security limit out of reach, so a sweep can be A/B'd against one
    /// with the limits in force and attribute any difference to them.</summary>
    public bool NoSecurity;

    /// <summary>Whether a source file resolves to the code extractor (`--features code`).
    /// Off by default, matching the golden set the default generator build produces.</summary>
    public bool SourceCodeDetection;
}

sealed class DimStat { public int Match; public int Mismatch; }

sealed class ExtStat
{
    public int Total, Ok, BothUnsupported, Extra, MetaMatch, MetaMismatch, TablesMatch, TablesMismatch;
    public int ContentExact;
    public readonly DimStat Plain = new(), Markdown = new(), Html = new(), Json = new();
    public DimStat Dim(string n) => n switch { "plain" => Plain, "markdown" => Markdown, "html" => Html, _ => Json };
}

sealed class Stats
{
    public int Total, Ok, BothUnsupported, Extra, NoSource, BadGolden;
    // Content-parity buckets (plain text, whitespace/order-normalized).
    public int ContentExact, ContentOrder, ContentClose, ContentPartial, ContentLow;
    public int Catastrophes;
    public readonly List<string> CatastropheList = new();
    // Fixtures where Rust extracted clean content that we under-extract (content loss).
    public readonly List<MissRow> Misses = new();
    public readonly Dictionary<string, ExtStat> ByExt = new();
}

readonly record struct MissRow(string Ext, string Rel, int RustLen, int CsLen, double Sim);
