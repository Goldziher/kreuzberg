using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using XRay.Content.Core;
using XRay.Content.Internal.Markup;
using XRay.Content.Internal.MathMarkup;
using XRay.Content.Types;

namespace XRay.Content.Extractors;

/// <summary>
/// Jupyter Notebook (.ipynb) extractor. Ported from Rust `extractors/jupyter.rs`. Markdown cells
/// become headings/paragraphs, code cells become code blocks with output markers. Image decoding
/// (an OCR-adjacent concern) is omitted; images do not affect plain/json/meta/tables parity.
/// </summary>
public sealed class JupyterExtractor : IExtractor
{
    public IEnumerable<string> SupportedMimeTypes => new[] { "application/x-ipynb+json" };
    public int Priority => 50;

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        // Upstream's one-line growth guard: the input's own size is charged against
        // `max_content_size` before parsing, so a document too large to render is refused
        // rather than rendered and then found to be too large.
        SecurityBudget.FromConfig(config).AccountText(content.Length);
        var node = JsonNode.Parse(content.ToArray()) ?? throw new JsonException("invalid notebook");
        var notebook = node.AsObject();

        var additional = new Dictionary<string, JsonNode?>();
        string? languageName = null;

        var nbMeta = notebook["metadata"] as JsonObject;
        if (nbMeta is not null)
        {
            if (nbMeta.TryGetPropertyValue("kernelspec", out var ks) && ks is not null)
                additional["kernelspec"] = ks.DeepClone();
            if (nbMeta.TryGetPropertyValue("language_info", out var li) && li is not null)
            {
                additional["language_info"] = li.DeepClone();
                if (li is JsonObject lio)
                {
                    if (lio.TryGetPropertyValue("name", out var n) && n is not null) { additional["language_name"] = n.DeepClone(); languageName = (n as JsonValue)?.ToString(); }
                    if (lio.TryGetPropertyValue("version", out var v) && v is not null) additional["language_version"] = v.DeepClone();
                    if (lio.TryGetPropertyValue("mimetype", out var mt) && mt is not null) additional["language_mimetype"] = mt.DeepClone();
                }
            }
        }
        if (notebook.TryGetPropertyValue("nbformat", out var nf) && nf is not null) additional["nbformat"] = nf.DeepClone();
        if (notebook.TryGetPropertyValue("nbformat_minor", out var nfm) && nfm is not null) additional["nbformat_minor"] = nfm.DeepClone();

        var cells = notebook["cells"] as JsonArray;
        if (cells is not null)
        {
            additional["cell_count"] = JsonValue.Create(cells.Count);
            var cellsMeta = new JsonArray();
            for (int idx = 0; idx < cells.Count; idx++)
            {
                var cell = cells[idx] as JsonObject;
                string cellType = (cell?["cell_type"] as JsonValue)?.ToString() ?? "unknown";
                var entry = new JsonObject { ["index"] = idx, ["cell_type"] = cellType };
                // Whatever the cell carries of these, whatever its type — a markdown cell has an
                // id too, and reporting it only for code cells lost it for most of a notebook.
                foreach (var key in new[] { "id", "execution_count" })
                    if (cell is not null && cell.TryGetPropertyValue(key, out var value))
                        entry[key] = value?.DeepClone();
                var tags = (cell?["metadata"] as JsonObject)?["tags"] as JsonArray;
                if (tags is not null && tags.Count > 0)
                    entry["tags"] = tags.DeepClone();
                if (cell?["outputs"] as JsonArray is { Count: > 0 } cellOutputs)
                {
                    var outputsMeta = new JsonArray();
                    for (int oi = 0; oi < cellOutputs.Count; oi++)
                        outputsMeta.Add(OutputMetadata(cellOutputs[oi] as JsonObject, oi));
                    entry["outputs"] = outputsMeta;
                }
                cellsMeta.Add(entry);
            }
            additional["cells"] = cellsMeta;
        }

        // Plain and structured output take an output's text/plain repr and nothing else. The
        // richer representations are markup, and rendering them into a plain document would put
        // HTML tags in it.
        bool plain = config.OutputFormat.Which is OutputFormat.Kind.Plain or OutputFormat.Kind.Structured;
        var doc = BuildInternalDocument(notebook, plain);
        doc.MimeType = mimeType;
        doc.Metadata = new Metadata { Language = languageName };
        foreach (var (k, v) in additional)
            doc.Metadata.Additional[k] = JsonSerializer.SerializeToElement(v, Json.Options);
        return doc;
    }

    private static InternalDocument BuildInternalDocument(JsonObject notebook, bool plain)
    {
        var builder = new InternalDocumentBuilder("jupyter");
        var cells = notebook["cells"] as JsonArray;
        if (cells is null) return builder.Build();

        string? kernelLang =
            (((notebook["metadata"] as JsonObject)?["kernelspec"] as JsonObject)?["language"] as JsonValue)?.ToString()
            ?? (((notebook["metadata"] as JsonObject)?["language_info"] as JsonObject)?["name"] as JsonValue)?.ToString();

        foreach (var cellNode in cells)
        {
            Cancel.ThrowIfRequested();
            var cell = cellNode as JsonObject;
            if (cell is null) continue;
            string cellType = (cell["cell_type"] as JsonValue)?.ToString() ?? "unknown";
            string sourceText = ExtractSource(cell["source"]);
            string trimmed = sourceText.Trim();

            // A cell's id, tags and execution count are metadata: they are recorded on the
            // element and in the document's metadata, and are not content. Emitting them as
            // paragraphs put `[cell_id: 0ad1fbe7-…]` into the extracted text of every notebook.
            var tags = (cell["metadata"] as JsonObject)?["tags"] as JsonArray;

            // A cell is only empty if it has nothing else to contribute either: a code cell's
            // source may have been stripped while its saved outputs still carry real content.
            var cellOutputs = cell["outputs"] as JsonArray;
            bool hasOutputs = cellType == "code" && cellOutputs is { Count: > 0 };
            bool hasAttachments = cell["attachments"] is JsonObject att && att.Count > 0;
            if (trimmed.Length == 0 && !hasOutputs && !hasAttachments) continue;

            switch (cellType)
            {
                case "markdown":
                {
                    // Parsed by the markdown parser proper, not an ad-hoc line scan: a cell is
                    // markdown, so it gets the same treatment a .md file would — smart
                    // punctuation, emphasis, lists and all.
                    var cellEvents = Internal.Commonmark.MarkdownParser.Parse(trimmed);
                    builder.AppendDocument(MarkdownExtractor.BuildInternalDocument(cellEvents, null, "jupyter"));
                    break;
                }
                case "code":
                {
                    // A code cell with no source contributes only its outputs. An empty code
                    // element for it leaves a blank block between the cell before it and that
                    // output. The outputs still belong to the document, so this guards the code
                    // element alone rather than the whole cell.
                    if (trimmed.Length != 0)
                    {
                        uint idx = builder.PushCode(trimmed, kernelLang, null, null);
                        var attrs = new Dictionary<string, string>();
                        if (cell.TryGetPropertyValue("execution_count", out var ec))
                        {
                            if (ec is JsonValue ecv && ecv.GetValueKind() == JsonValueKind.Number) attrs["execution_count"] = ecv.ToJsonString();
                            else if (ec is null || ec.GetValueKind() == JsonValueKind.Null) attrs["execution_count"] = "null";
                        }
                        if (tags is not null && tags.Count > 0)
                        {
                            var tagStrs = tags.Select(t => (t as JsonValue)?.ToString()).Where(s => s is not null).Select(s => s!).ToList();
                            attrs["tags"] = string.Join(",", tagStrs);
                        }
                        if (attrs.Count > 0) builder.SetAttributes(idx, attrs);
                    }

                    // Each output contributes the richest representation it carries.
                    var outputs = cell["outputs"] as JsonArray;
                    if (outputs is not null)
                    {
                        foreach (var outNode in outputs)
                        {
                            if (outNode is not JsonObject output) continue;
                            PushOutputElement(builder, output, plain);
                        }
                    }
                    break;
                }
                default:
                    builder.PushParagraph(trimmed, new(), null, null);
                    break;
            }
        }

        return builder.Build();
    }

    /// <summary>What an output records about itself, as distinct from the text it carries.</summary>
    private static JsonObject OutputMetadata(JsonObject? output, int index)
    {
        var entry = new JsonObject { ["index"] = index };
        if (output is null) return entry;

        if (output.TryGetPropertyValue("output_type", out var outputType))
            entry["output_type"] = outputType?.DeepClone();
        foreach (var key in new[] { "name", "execution_count", "ename", "evalue" })
            if (output.TryGetPropertyValue(key, out var value))
                entry[key] = value?.DeepClone();
        if (output["data"] as JsonObject is JsonObject data)
        {
            var mimeTypes = new JsonArray();
            foreach (var mime in data.Select(kv => kv.Key).OrderBy(k => k, StringComparer.Ordinal))
                mimeTypes.Add(mime);
            entry["mime_types"] = mimeTypes;
        }
        return entry;
    }

    private static string ExtractSource(JsonNode? source)
    {
        if (source is JsonValue v && v.TryGetValue(out string? s)) return s ?? "";
        if (source is JsonArray arr)
        {
            var sb = new StringBuilder();
            foreach (var item in arr) if (item is JsonValue iv && iv.TryGetValue(out string? str)) sb.Append(str);
            return sb.ToString();
        }
        return "";
    }

    /// <summary>
    /// Emit one output as whichever of its representations says the most.
    /// </summary>
    /// <remarks>
    /// An output ships the same result under several MIME types, and the text/plain one is often
    /// a placeholder for an object — "&lt;IPython.core.display.HTML object&gt;" — where the
    /// text/html one is the output as its author meant it to be seen.
    /// <para>
    /// text/latex is deliberately not preferred. Upstream's current source takes it as a formula,
    /// but the reference outputs this port is validated against predate that and keep the
    /// text/plain repr — <c>z₀</c> rather than <c>$$z_{0}$$</c> — for every output carrying both.
    /// </para>
    /// </remarks>
    private static void PushOutputElement(InternalDocumentBuilder builder, JsonObject output, bool plain)
    {
        string outputType = (output["output_type"] as JsonValue)?.ToString() ?? "";
        switch (outputType)
        {
            case "stream":
            {
                string text = output.TryGetPropertyValue("text", out var t) ? ExtractSource(t).Trim() : "";
                if (text.Length != 0) builder.PushParagraph(text, new(), null, null);
                break;
            }
            case "execute_result":
            case "display_data":
            case "update_display_data":
            {
                if (output["data"] is not JsonObject data) break;
                if (!plain)
                {
                    // A tool that ships `text/latex` has already decided the output is math, and
                    // the LaTeX states the equation exactly — nothing richer can be recovered
                    // from the HTML or the repr of the same result. `text/latex` arrives
                    // delimited; the formula element holds bare LaTeX and the renderers put the
                    // delimiters back.
                    if (data.TryGetPropertyValue("text/latex", out var latex))
                    {
                        string text = ExtractSource(latex);
                        if (text.Trim().Length != 0)
                        {
                            string bare = MathMl.StripMathDelimiters(text);
                            if (bare.Length != 0) { builder.PushFormula(bare, null, null); break; }
                        }
                    }
                    if (data.TryGetPropertyValue("text/html", out var html))
                    {
                        string text = ExtractSource(html).Trim();
                        if (text.Length != 0) { builder.PushRawBlock("html", text, null); break; }
                    }
                    if (data.TryGetPropertyValue("text/markdown", out var md))
                    {
                        string text = ExtractSource(md).Trim();
                        if (text.Length != 0) { builder.PushParagraph(text, new(), null, null); break; }
                    }
                }
                if (data.TryGetPropertyValue("text/plain", out var p))
                {
                    string text = ExtractSource(p).Trim();
                    if (text.Length != 0) builder.PushParagraph(text, new(), null, null);
                }
                break;
            }
            case "error":
            {
                string text = CollectOutputText(output).Trim();
                if (text.Length != 0) builder.PushParagraph(text, new(), null, null);
                break;
            }
        }
    }

    private static string CollectOutputText(JsonObject output)
    {
        string outputType = (output["output_type"] as JsonValue)?.ToString() ?? "";
        switch (outputType)
        {
            case "stream":
                return output.TryGetPropertyValue("text", out var t) ? ExtractSource(t) : "";
            case "execute_result":
            case "display_data":
            case "update_display_data":
                if (output["data"] as JsonObject is JsonObject data && data.TryGetPropertyValue("text/plain", out var p))
                    return ExtractSource(p);
                return "";
            case "error":
            {
                // The traceback is the useful part of an error and belongs with it.
                string ename = (output["ename"] as JsonValue)?.ToString() ?? "Unknown";
                string evalue = (output["evalue"] as JsonValue)?.ToString() ?? "";
                var text = new StringBuilder($"Error ({ename}): {evalue}");
                if (output["traceback"] is JsonArray traceback)
                {
                    text.Append("\nTraceback:");
                    foreach (var line in traceback)
                        if (line is JsonValue lv && lv.TryGetValue(out string? ls) && ls is not null)
                            text.Append('\n').Append(ls);
                }
                return text.ToString();
            }
            default:
                return "";
        }
    }

    private static (byte level, string text)? ParseHeadingLine(string line)
    {
        string trimmed = line.TrimStart();
        int hashes = 0;
        while (hashes < trimmed.Length && trimmed[hashes] == '#') hashes++;
        if (hashes == 0 || hashes > 6) return null;
        string rest = trimmed.Substring(hashes);
        if (rest.Length != 0 && !rest.StartsWith(' ')) return null;
        return ((byte)hashes, rest.Trim());
    }

    // ── inline markdown scan (byte-based, mirrors the Rust byte-as-char behaviour) ──

    private static (string, List<TextAnnotation>) ScanMarkdownInline(string text)
    {
        byte[] b = Encoding.UTF8.GetBytes(text);
        int len = b.Length;
        var outBuf = new Utf8Buf();
        var anns = new List<TextAnnotation>();
        int i = 0;

        string Slice(int s, int e) => Encoding.UTF8.GetString(b, s, e - s);

        while (i < len)
        {
            if (i + 1 < len && b[i] == (byte)'*' && b[i + 1] == (byte)'*')
            {
                int end = FindClosing2(b, i + 2, (byte)'*', (byte)'*');
                if (end >= 0)
                {
                    uint start = outBuf.Len; outBuf.Append(Slice(i + 2, end)); uint e2 = outBuf.Len;
                    anns.Add(MarkupHelpers.Annotation(start, e2, MarkupHelpers.Bold));
                    i = end + 2; continue;
                }
            }
            if (b[i] == (byte)'*' && (i == 0 || b[i - 1] != (byte)'*'))
            {
                if (i + 1 < len && b[i + 1] != (byte)'*')
                {
                    int end = FindClosingSingleStar(b, i + 1);
                    if (end >= 0)
                    {
                        uint start = outBuf.Len; outBuf.Append(Slice(i + 1, end)); uint e2 = outBuf.Len;
                        anns.Add(MarkupHelpers.Annotation(start, e2, MarkupHelpers.Italic));
                        i = end + 1; continue;
                    }
                }
            }
            if (b[i] == (byte)'`' && (i + 1 >= len || b[i + 1] != (byte)'`'))
            {
                int end = FindClosingByte(b, i + 1, (byte)'`');
                if (end >= 0)
                {
                    uint start = outBuf.Len; outBuf.Append(Slice(i + 1, end)); uint e2 = outBuf.Len;
                    anns.Add(MarkupHelpers.Annotation(start, e2, MarkupHelpers.Code));
                    i = end + 1; continue;
                }
            }
            outBuf.AppendChar((char)b[i]);
            i++;
        }
        return (outBuf.ToString(), anns);
    }

    private static int FindClosing2(byte[] b, int start, byte d0, byte d1)
    {
        int i = start;
        while (i + 1 < b.Length) { if (b[i] == d0 && b[i + 1] == d1) return i; i++; }
        return -1;
    }
    private static int FindClosingSingleStar(byte[] b, int start)
    {
        int i = start;
        while (i < b.Length) { if (b[i] == (byte)'*' && (i + 1 >= b.Length || b[i + 1] != (byte)'*')) return i; i++; }
        return -1;
    }
    private static int FindClosingByte(byte[] b, int start, byte delim)
    {
        for (int i = start; i < b.Length; i++) if (b[i] == delim) return i;
        return -1;
    }
}
