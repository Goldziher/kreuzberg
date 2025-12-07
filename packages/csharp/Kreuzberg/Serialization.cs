using System.Collections.Frozen;
using System.Runtime.CompilerServices;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization;

[assembly: InternalsVisibleTo("Kreuzberg.E2E")]

namespace Kreuzberg;

internal static class Serialization
{
    internal static readonly JsonSerializerOptions Options = new()
    {
        PropertyNameCaseInsensitive = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        WriteIndented = false,
    };

    private static readonly FrozenDictionary<FormatType, string[]> FormatFields = new Dictionary<FormatType, string[]>
    {
        { FormatType.Pdf, new[] { "title", "subject", "author", "keywords", "creator", "producer", "creation_date", "modification_date", "page_count" } },
        { FormatType.Excel, new[] { "sheet_count", "sheet_names" } },
        { FormatType.Email, new[] { "from_email", "from_name", "to_emails", "cc_emails", "bcc_emails", "message_id", "attachments" } },
        { FormatType.Pptx, new[] { "title", "author", "description", "summary", "fonts" } },
        { FormatType.Archive, new[] { "format", "file_count", "file_list", "total_size", "compressed_size" } },
        { FormatType.Image, new[] { "width", "height", "format", "exif" } },
        { FormatType.Xml, new[] { "element_count", "unique_elements" } },
        { FormatType.Text, new[] { "line_count", "word_count", "character_count", "headers", "links", "code_blocks" } },
        { FormatType.Html, new[] { "title", "description", "keywords", "author", "canonical", "base_href", "og_title", "og_description", "og_image", "og_url", "og_type", "og_site_name", "twitter_card", "twitter_title", "twitter_description", "twitter_image", "twitter_site", "twitter_creator", "link_author", "link_license", "link_alternate" } },
        { FormatType.Ocr, new[] { "language", "psm", "output_format", "table_count", "table_rows", "table_cols" } },
    }.ToFrozenDictionary();

    private static readonly FrozenSet<string> CoreMetadataKeys = new HashSet<string>
    {
        "language",
        "date",
        "subject",
        "format_type",
        "image_preprocessing",
        "json_schema",
        "error",
    }.ToFrozenSet(StringComparer.OrdinalIgnoreCase);

    internal static string SerializeResult(ExtractionResult result)
    {
        var root = new JsonObject
        {
            ["content"] = result.Content,
            ["mime_type"] = result.MimeType,
            ["metadata"] = BuildMetadataNode(result.Metadata),
            ["success"] = result.Success,
            ["tables"] = JsonSerializer.SerializeToNode(result.Tables, Options),
        };

        if (result.DetectedLanguages != null)
        {
            root["detected_languages"] = JsonSerializer.SerializeToNode(result.DetectedLanguages, Options);
        }

        if (result.Chunks != null)
        {
            root["chunks"] = JsonSerializer.SerializeToNode(result.Chunks, Options);
        }

        if (result.Images != null)
        {
            root["images"] = JsonSerializer.SerializeToNode(result.Images, Options);
        }

        return root.ToJsonString(Options);
    }

    internal static ExtractionResult ParseResult(string json)
    {
        using var document = JsonDocument.Parse(json);
        var root = document.RootElement;
        var result = new ExtractionResult
        {
            Content = root.GetPropertyOrDefault("content", string.Empty),
            MimeType = root.GetPropertyOrDefault("mime_type", string.Empty),
            Success = root.GetPropertyOrDefault("success", true),
        };

        if (root.TryGetProperty("tables", out var tables))
        {
            result.Tables = DeserializeElement<List<Table>>(tables) ?? new List<Table>();
        }

        if (root.TryGetProperty("detected_languages", out var langs))
        {
            result.DetectedLanguages = DeserializeElement<List<string>>(langs);
        }

        if (root.TryGetProperty("chunks", out var chunks))
        {
            result.Chunks = DeserializeElement<List<Chunk>>(chunks);
        }

        if (root.TryGetProperty("images", out var images))
        {
            result.Images = DeserializeElement<List<ExtractedImage>>(images);
        }

        if (root.TryGetProperty("metadata", out var metadata))
        {
            result.Metadata = ParseMetadata(metadata.GetRawText());
        }

        return result;
    }

    internal static ExtractionConfig ParseConfig(string json)
    {
        return JsonSerializer.Deserialize<ExtractionConfig>(json, Options) ?? new ExtractionConfig();
    }

    internal static Metadata ParseMetadata(string? metadataJson)
    {
        if (string.IsNullOrWhiteSpace(metadataJson))
        {
            return new Metadata();
        }

        using var document = JsonDocument.Parse(metadataJson);
        var root = document.RootElement;
        var metadata = new Metadata();
        var recognized = new HashSet<string>(CoreMetadataKeys, StringComparer.OrdinalIgnoreCase);

        if (root.TryGetProperty("language", out var language))
        {
            metadata.Language = language.GetString();
        }

        if (root.TryGetProperty("date", out var date))
        {
            metadata.Date = date.GetString();
        }

        if (root.TryGetProperty("subject", out var subject))
        {
            metadata.Subject = subject.GetString();
        }

        if (root.TryGetProperty("image_preprocessing", out var imagePre))
        {
            metadata.ImagePreprocessing = DeserializeElement<ImagePreprocessingMetadata>(imagePre);
        }

        if (root.TryGetProperty("json_schema", out var schema))
        {
            metadata.JsonSchema = ParseNode(schema);
        }

        if (root.TryGetProperty("error", out var error))
        {
            metadata.Error = DeserializeElement<ErrorMetadata>(error);
        }

        if (root.TryGetProperty("format_type", out var formatType))
        {
            metadata.FormatType = ParseFormat(formatType.GetString());
            metadata.Format.Type = metadata.FormatType;
            recognized.UnionWith(FormatFields.GetValueOrDefault(metadata.FormatType, Array.Empty<string>()));
        }

        ApplyFormatMetadata(root, metadata);
        var additional = new Dictionary<string, JsonNode?>(StringComparer.OrdinalIgnoreCase);
        foreach (var property in root.EnumerateObject())
        {
            if (CoreMetadataKeys.Contains(property.Name))
            {
                continue;
            }
            additional[property.Name] = ParseNode(property.Value);
        }

        if (additional.Count > 0)
        {
            metadata.Additional = additional;
        }

        return metadata;
    }

    private static void ApplyFormatMetadata(JsonElement root, Metadata metadata)
    {
        switch (metadata.FormatType)
        {
            case FormatType.Pdf:
                metadata.Format.Pdf = DeserializeElement<PdfMetadata>(root);
                break;
            case FormatType.Excel:
                metadata.Format.Excel = DeserializeElement<ExcelMetadata>(root);
                break;
            case FormatType.Email:
                metadata.Format.Email = DeserializeElement<EmailMetadata>(root);
                break;
            case FormatType.Pptx:
                metadata.Format.Pptx = DeserializeElement<PptxMetadata>(root);
                break;
            case FormatType.Archive:
                metadata.Format.Archive = DeserializeElement<ArchiveMetadata>(root);
                break;
            case FormatType.Image:
                metadata.Format.Image = DeserializeElement<ImageMetadata>(root);
                break;
            case FormatType.Xml:
                metadata.Format.Xml = DeserializeElement<XmlMetadata>(root);
                break;
            case FormatType.Text:
                metadata.Format.Text = DeserializeElement<TextMetadata>(root);
                break;
            case FormatType.Html:
                metadata.Format.Html = DeserializeElement<HtmlMetadata>(root);
                break;
            case FormatType.Ocr:
                metadata.Format.Ocr = DeserializeElement<OcrMetadata>(root);
                break;
            default:
                metadata.Format.Type = FormatType.Unknown;
                break;
        }
    }

    public static JsonNode BuildMetadataNode(Metadata metadata)
    {
        var node = new JsonObject
        {
            ["format_type"] = FormatTypeToString(metadata.FormatType),
        };

        if (!string.IsNullOrWhiteSpace(metadata.Language))
        {
            node["language"] = metadata.Language;
        }
        if (!string.IsNullOrWhiteSpace(metadata.Date))
        {
            node["date"] = metadata.Date;
        }
        if (!string.IsNullOrWhiteSpace(metadata.Subject))
        {
            node["subject"] = metadata.Subject;
        }
        if (metadata.ImagePreprocessing != null)
        {
            node["image_preprocessing"] = JsonSerializer.SerializeToNode(metadata.ImagePreprocessing, Options);
        }
        if (metadata.JsonSchema != null)
        {
            node["json_schema"] = metadata.JsonSchema;
        }
        if (metadata.Error != null)
        {
            node["error"] = JsonSerializer.SerializeToNode(metadata.Error, Options);
        }

        AddFormatFields(metadata, node);

        if (metadata.Additional != null)
        {
            foreach (var kvp in metadata.Additional)
            {
                node[kvp.Key] = kvp.Value;
            }
        }

        return node;
    }

    private static void AddFormatFields(Metadata metadata, JsonObject node)
    {
        void Merge<T>(T? payload)
        {
            if (payload == null)
            {
                return;
            }
            var raw = JsonSerializer.SerializeToNode(payload, Options) as JsonObject;
            if (raw == null)
            {
                return;
            }
            foreach (var kvp in raw)
            {
                node[kvp.Key] = kvp.Value?.DeepClone();
            }
        }

        switch (metadata.FormatType)
        {
            case FormatType.Pdf:
                Merge(metadata.Format.Pdf);
                break;
            case FormatType.Excel:
                Merge(metadata.Format.Excel);
                break;
            case FormatType.Email:
                Merge(metadata.Format.Email);
                break;
            case FormatType.Pptx:
                Merge(metadata.Format.Pptx);
                break;
            case FormatType.Archive:
                Merge(metadata.Format.Archive);
                break;
            case FormatType.Image:
                Merge(metadata.Format.Image);
                break;
            case FormatType.Xml:
                Merge(metadata.Format.Xml);
                break;
            case FormatType.Text:
                Merge(metadata.Format.Text);
                break;
            case FormatType.Html:
                Merge(metadata.Format.Html);
                break;
            case FormatType.Ocr:
                Merge(metadata.Format.Ocr);
                break;
            default:
                break;
        }
    }

    private static FormatType ParseFormat(string? format)
    {
        return format?.ToLowerInvariant() switch
        {
            "pdf" => FormatType.Pdf,
            "excel" => FormatType.Excel,
            "email" => FormatType.Email,
            "pptx" => FormatType.Pptx,
            "archive" => FormatType.Archive,
            "image" => FormatType.Image,
            "xml" => FormatType.Xml,
            "text" => FormatType.Text,
            "html" => FormatType.Html,
            "ocr" => FormatType.Ocr,
            _ => FormatType.Unknown,
        };
    }

    private static string? FormatTypeToString(FormatType format)
    {
        return format switch
        {
            FormatType.Pdf => "pdf",
            FormatType.Excel => "excel",
            FormatType.Email => "email",
            FormatType.Pptx => "pptx",
            FormatType.Archive => "archive",
            FormatType.Image => "image",
            FormatType.Xml => "xml",
            FormatType.Text => "text",
            FormatType.Html => "html",
            FormatType.Ocr => "ocr",
            _ => null,
        };
    }

    private static T? DeserializeElement<T>(JsonElement element)
    {
        return JsonSerializer.Deserialize<T>(element.GetRawText(), Options);
    }

    private static JsonNode? ParseNode(JsonElement element)
    {
        try
        {
            return JsonNode.Parse(element.GetRawText());
        }
        catch
        {
            return null;
        }
    }
}

internal static class JsonElementExtensions
{
    internal static T GetPropertyOrDefault<T>(this JsonElement element, string name, T defaultValue)
    {
        if (element.TryGetProperty(name, out var property))
        {
            try
            {
                return property.Deserialize<T>(Serialization.Options)!;
            }
            catch
            {
                return defaultValue;
            }
        }
        return defaultValue;
    }
}
