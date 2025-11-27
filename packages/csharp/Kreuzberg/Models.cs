using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization;

namespace Kreuzberg;

public sealed class ExtractionResult
{
    [JsonPropertyName("content")]
    public string Content { get; set; } = string.Empty;

    [JsonPropertyName("mime_type")]
    public string MimeType { get; set; } = string.Empty;

    [JsonPropertyName("metadata")]
    public Metadata Metadata { get; set; } = new();

    [JsonPropertyName("tables")]
    public List<Table> Tables { get; set; } = new();

    [JsonPropertyName("detected_languages")]
    public List<string>? DetectedLanguages { get; set; }

    [JsonPropertyName("chunks")]
    public List<Chunk>? Chunks { get; set; }

    [JsonPropertyName("images")]
    public List<ExtractedImage>? Images { get; set; }

    [JsonPropertyName("success")]
    public bool Success { get; set; }
}

public sealed class Table
{
    [JsonPropertyName("cells")]
    public List<List<string>> Cells { get; set; } = new();

    [JsonPropertyName("markdown")]
    public string Markdown { get; set; } = string.Empty;

    [JsonPropertyName("page_number")]
    public int PageNumber { get; set; }
}

public sealed class Chunk
{
    [JsonPropertyName("content")]
    public string Content { get; set; } = string.Empty;

    [JsonPropertyName("embedding")]
    public float[]? Embedding { get; set; }

    [JsonPropertyName("metadata")]
    public ChunkMetadata Metadata { get; set; } = new();
}

public sealed class ChunkMetadata
{
    [JsonPropertyName("char_start")]
    public int CharStart { get; set; }

    [JsonPropertyName("char_end")]
    public int CharEnd { get; set; }

    [JsonPropertyName("token_count")]
    public int? TokenCount { get; set; }

    [JsonPropertyName("chunk_index")]
    public int ChunkIndex { get; set; }

    [JsonPropertyName("total_chunks")]
    public int TotalChunks { get; set; }
}

public sealed class ExtractedImage
{
    [JsonPropertyName("data")]
    public byte[] Data { get; set; } = Array.Empty<byte>();

    [JsonPropertyName("format")]
    public string Format { get; set; } = string.Empty;

    [JsonPropertyName("image_index")]
    public int ImageIndex { get; set; }

    [JsonPropertyName("page_number")]
    public int? PageNumber { get; set; }

    [JsonPropertyName("width")]
    public uint? Width { get; set; }

    [JsonPropertyName("height")]
    public uint? Height { get; set; }

    [JsonPropertyName("colorspace")]
    public string? Colorspace { get; set; }

    [JsonPropertyName("bits_per_component")]
    public uint? BitsPerComponent { get; set; }

    [JsonPropertyName("is_mask")]
    public bool IsMask { get; set; }

    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonPropertyName("ocr_result")]
    public ExtractionResult? OcrResult { get; set; }
}

public enum FormatType
{
    Unknown,
    [JsonPropertyName("pdf")]
    Pdf,
    [JsonPropertyName("excel")]
    Excel,
    [JsonPropertyName("email")]
    Email,
    [JsonPropertyName("pptx")]
    Pptx,
    [JsonPropertyName("archive")]
    Archive,
    [JsonPropertyName("image")]
    Image,
    [JsonPropertyName("xml")]
    Xml,
    [JsonPropertyName("text")]
    Text,
    [JsonPropertyName("html")]
    Html,
    [JsonPropertyName("ocr")]
    Ocr,
}

public sealed class FormatMetadata
{
    public FormatType Type { get; set; } = FormatType.Unknown;
    public PdfMetadata? Pdf { get; set; }
    public ExcelMetadata? Excel { get; set; }
    public EmailMetadata? Email { get; set; }
    public PptxMetadata? Pptx { get; set; }
    public ArchiveMetadata? Archive { get; set; }
    public ImageMetadata? Image { get; set; }
    public XmlMetadata? Xml { get; set; }
    public TextMetadata? Text { get; set; }
    public HtmlMetadata? Html { get; set; }
    public OcrMetadata? Ocr { get; set; }
}

public sealed class Metadata
{
    [JsonPropertyName("language")]
    public string? Language { get; set; }

    [JsonPropertyName("date")]
    public string? Date { get; set; }

    [JsonPropertyName("subject")]
    public string? Subject { get; set; }

    [JsonPropertyName("format_type")]
    public FormatType FormatType { get; set; } = FormatType.Unknown;

    [JsonIgnore]
    public FormatMetadata Format { get; set; } = new();

    [JsonPropertyName("image_preprocessing")]
    public ImagePreprocessingMetadata? ImagePreprocessing { get; set; }

    [JsonPropertyName("json_schema")]
    public JsonNode? JsonSchema { get; set; }

    [JsonPropertyName("error")]
    public ErrorMetadata? Error { get; set; }

    [JsonExtensionData]
    public Dictionary<string, JsonNode?>? Additional { get; set; }
}

public sealed class ImagePreprocessingMetadata
{
    [JsonPropertyName("original_dimensions")]
    public int[]? OriginalDimensions { get; set; }

    [JsonPropertyName("original_dpi")]
    public double[]? OriginalDpi { get; set; }

    [JsonPropertyName("target_dpi")]
    public int TargetDpi { get; set; }

    [JsonPropertyName("scale_factor")]
    public double ScaleFactor { get; set; }

    [JsonPropertyName("auto_adjusted")]
    public bool AutoAdjusted { get; set; }

    [JsonPropertyName("final_dpi")]
    public int FinalDpi { get; set; }

    [JsonPropertyName("new_dimensions")]
    public int[]? NewDimensions { get; set; }

    [JsonPropertyName("resample_method")]
    public string? ResampleMethod { get; set; }

    [JsonPropertyName("dimension_clamped")]
    public bool DimensionClamped { get; set; }

    [JsonPropertyName("calculated_dpi")]
    public int? CalculatedDpi { get; set; }

    [JsonPropertyName("skipped_resize")]
    public bool SkippedResize { get; set; }

    [JsonPropertyName("resize_error")]
    public string? ResizeError { get; set; }
}

public sealed class ErrorMetadata
{
    [JsonPropertyName("error_type")]
    public string ErrorType { get; set; } = string.Empty;

    [JsonPropertyName("message")]
    public string Message { get; set; } = string.Empty;
}

public sealed class PdfMetadata
{
    [JsonPropertyName("title")]
    public string? Title { get; set; }

    [JsonPropertyName("subject")]
    public string? Subject { get; set; }

    [JsonPropertyName("author")]
    public string? Author { get; set; }

    [JsonPropertyName("keywords")]
    public string? Keywords { get; set; }

    [JsonPropertyName("creator")]
    public string? Creator { get; set; }

    [JsonPropertyName("producer")]
    public string? Producer { get; set; }

    [JsonPropertyName("creation_date")]
    public string? CreationDate { get; set; }

    [JsonPropertyName("modification_date")]
    public string? ModificationDate { get; set; }

    [JsonPropertyName("page_count")]
    public int? PageCount { get; set; }
}

public sealed class ExcelMetadata
{
    [JsonPropertyName("sheet_count")]
    public int? SheetCount { get; set; }

    [JsonPropertyName("sheet_names")]
    public List<string>? SheetNames { get; set; }
}

public sealed class EmailMetadata
{
    [JsonPropertyName("from_email")]
    public string? FromEmail { get; set; }

    [JsonPropertyName("from_name")]
    public string? FromName { get; set; }

    [JsonPropertyName("to_emails")]
    public List<string>? ToEmails { get; set; }

    [JsonPropertyName("cc_emails")]
    public List<string>? CcEmails { get; set; }

    [JsonPropertyName("bcc_emails")]
    public List<string>? BccEmails { get; set; }

    [JsonPropertyName("message_id")]
    public string? MessageId { get; set; }

    [JsonPropertyName("attachments")]
    public List<string>? Attachments { get; set; }
}

public sealed class ArchiveMetadata
{
    [JsonPropertyName("format")]
    public string? Format { get; set; }

    [JsonPropertyName("file_count")]
    public int? FileCount { get; set; }

    [JsonPropertyName("file_list")]
    public List<string>? FileList { get; set; }

    [JsonPropertyName("total_size")]
    public int? TotalSize { get; set; }

    [JsonPropertyName("compressed_size")]
    public int? CompressedSize { get; set; }
}

public sealed class ImageMetadata
{
    [JsonPropertyName("width")]
    public uint Width { get; set; }

    [JsonPropertyName("height")]
    public uint Height { get; set; }

    [JsonPropertyName("format")]
    public string Format { get; set; } = string.Empty;

    [JsonPropertyName("exif")]
    public Dictionary<string, string>? Exif { get; set; }
}

public sealed class XmlMetadata
{
    [JsonPropertyName("element_count")]
    public int? ElementCount { get; set; }

    [JsonPropertyName("unique_elements")]
    public List<string>? UniqueElements { get; set; }
}

public sealed class TextMetadata
{
    [JsonPropertyName("line_count")]
    public int? LineCount { get; set; }

    [JsonPropertyName("word_count")]
    public int? WordCount { get; set; }

    [JsonPropertyName("character_count")]
    public int? CharacterCount { get; set; }

    [JsonPropertyName("headers")]
    public List<string>? Headers { get; set; }

    [JsonPropertyName("links")]
    public List<List<string>>? Links { get; set; }

    [JsonPropertyName("code_blocks")]
    public List<List<string>>? CodeBlocks { get; set; }
}

public sealed class HtmlMetadata
{
    [JsonPropertyName("title")]
    public string? Title { get; set; }

    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonPropertyName("keywords")]
    public string? Keywords { get; set; }

    [JsonPropertyName("author")]
    public string? Author { get; set; }

    [JsonPropertyName("canonical")]
    public string? Canonical { get; set; }

    [JsonPropertyName("base_href")]
    public string? BaseHref { get; set; }

    [JsonPropertyName("og_title")]
    public string? OgTitle { get; set; }

    [JsonPropertyName("og_description")]
    public string? OgDescription { get; set; }

    [JsonPropertyName("og_image")]
    public string? OgImage { get; set; }

    [JsonPropertyName("og_url")]
    public string? OgUrl { get; set; }

    [JsonPropertyName("og_type")]
    public string? OgType { get; set; }

    [JsonPropertyName("og_site_name")]
    public string? OgSiteName { get; set; }

    [JsonPropertyName("twitter_card")]
    public string? TwitterCard { get; set; }

    [JsonPropertyName("twitter_title")]
    public string? TwitterTitle { get; set; }

    [JsonPropertyName("twitter_description")]
    public string? TwitterDescription { get; set; }

    [JsonPropertyName("twitter_image")]
    public string? TwitterImage { get; set; }

    [JsonPropertyName("twitter_site")]
    public string? TwitterSite { get; set; }

    [JsonPropertyName("twitter_creator")]
    public string? TwitterCreator { get; set; }

    [JsonPropertyName("link_author")]
    public string? LinkAuthor { get; set; }

    [JsonPropertyName("link_license")]
    public string? LinkLicense { get; set; }

    [JsonPropertyName("link_alternate")]
    public string? LinkAlternate { get; set; }
}

public sealed class PptxMetadata
{
    [JsonPropertyName("title")]
    public string? Title { get; set; }

    [JsonPropertyName("author")]
    public string? Author { get; set; }

    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonPropertyName("summary")]
    public string? Summary { get; set; }

    [JsonPropertyName("fonts")]
    public List<string>? Fonts { get; set; }
}

public sealed class OcrMetadata
{
    [JsonPropertyName("language")]
    public string? Language { get; set; }

    [JsonPropertyName("psm")]
    public int? Psm { get; set; }

    [JsonPropertyName("output_format")]
    public string? OutputFormat { get; set; }

    [JsonPropertyName("table_count")]
    public int? TableCount { get; set; }

    [JsonPropertyName("table_rows")]
    public int? TableRows { get; set; }

    [JsonPropertyName("table_cols")]
    public int? TableCols { get; set; }
}

public sealed class ExtractionConfig
{
    [JsonPropertyName("use_cache")]
    public bool? UseCache { get; set; }

    [JsonPropertyName("enable_quality_processing")]
    public bool? EnableQualityProcessing { get; set; }

    [JsonPropertyName("ocr")]
    public OcrConfig? Ocr { get; set; }

    [JsonPropertyName("force_ocr")]
    public bool? ForceOcr { get; set; }

    [JsonPropertyName("chunking")]
    public ChunkingConfig? Chunking { get; set; }

    [JsonPropertyName("images")]
    public ImageExtractionConfig? Images { get; set; }

    [JsonPropertyName("pdf_options")]
    public PdfConfig? PdfOptions { get; set; }

    [JsonPropertyName("token_reduction")]
    public TokenReductionConfig? TokenReduction { get; set; }

    [JsonPropertyName("language_detection")]
    public LanguageDetectionConfig? LanguageDetection { get; set; }

    [JsonPropertyName("postprocessor")]
    public PostProcessorConfig? Postprocessor { get; set; }

    [JsonPropertyName("html_options")]
    public HtmlConversionOptions? HtmlOptions { get; set; }

    [JsonPropertyName("keywords")]
    public KeywordConfig? Keywords { get; set; }

    [JsonPropertyName("max_concurrent_extractions")]
    public int? MaxConcurrentExtractions { get; set; }
}

public sealed class OcrConfig
{
    [JsonPropertyName("backend")]
    public string? Backend { get; set; }

    [JsonPropertyName("language")]
    public string? Language { get; set; }

    [JsonPropertyName("tesseract_config")]
    public TesseractConfig? TesseractConfig { get; set; }
}

public sealed class TesseractConfig
{
    [JsonPropertyName("language")]
    public string? Language { get; set; }

    [JsonPropertyName("psm")]
    public int? Psm { get; set; }

    [JsonPropertyName("output_format")]
    public string? OutputFormat { get; set; }

    [JsonPropertyName("oem")]
    public int? Oem { get; set; }

    [JsonPropertyName("min_confidence")]
    public double? MinConfidence { get; set; }

    [JsonPropertyName("preprocessing")]
    public ImagePreprocessingConfig? Preprocessing { get; set; }

    [JsonPropertyName("enable_table_detection")]
    public bool? EnableTableDetection { get; set; }

    [JsonPropertyName("table_min_confidence")]
    public double? TableMinConfidence { get; set; }

    [JsonPropertyName("table_column_threshold")]
    public int? TableColumnThreshold { get; set; }

    [JsonPropertyName("table_row_threshold_ratio")]
    public double? TableRowThresholdRatio { get; set; }

    [JsonPropertyName("use_cache")]
    public bool? UseCache { get; set; }

    [JsonPropertyName("classify_use_pre_adapted_templates")]
    public bool? ClassifyUsePreAdaptedTemplates { get; set; }

    [JsonPropertyName("language_model_ngram_on")]
    public bool? LanguageModelNgramOn { get; set; }

    [JsonPropertyName("tessedit_dont_blkrej_good_wds")]
    public bool? TesseditDontBlkrejGoodWds { get; set; }

    [JsonPropertyName("tessedit_dont_rowrej_good_wds")]
    public bool? TesseditDontRowrejGoodWds { get; set; }

    [JsonPropertyName("tessedit_enable_dict_correction")]
    public bool? TesseditEnableDictCorrection { get; set; }

    [JsonPropertyName("tessedit_char_whitelist")]
    public string? TesseditCharWhitelist { get; set; }

    [JsonPropertyName("tessedit_char_blacklist")]
    public string? TesseditCharBlacklist { get; set; }

    [JsonPropertyName("tessedit_use_primary_params_model")]
    public bool? TesseditUsePrimaryParamsModel { get; set; }

    [JsonPropertyName("textord_space_size_is_variable")]
    public bool? TextordSpaceSizeIsVariable { get; set; }

    [JsonPropertyName("thresholding_method")]
    public bool? ThresholdingMethod { get; set; }
}

public sealed class ImagePreprocessingConfig
{
    [JsonPropertyName("target_dpi")]
    public int? TargetDpi { get; set; }

    [JsonPropertyName("auto_rotate")]
    public bool? AutoRotate { get; set; }

    [JsonPropertyName("deskew")]
    public bool? Deskew { get; set; }

    [JsonPropertyName("denoise")]
    public bool? Denoise { get; set; }

    [JsonPropertyName("contrast_enhance")]
    public bool? ContrastEnhance { get; set; }

    [JsonPropertyName("binarization_method")]
    public string? BinarizationMode { get; set; }

    [JsonPropertyName("invert_colors")]
    public bool? InvertColors { get; set; }
}

public sealed class ChunkingConfig
{
    [JsonPropertyName("max_chars")]
    public int? MaxChars { get; set; }

    [JsonPropertyName("max_overlap")]
    public int? MaxOverlap { get; set; }

    [JsonPropertyName("chunk_size")]
    public int? ChunkSize { get; set; }

    [JsonPropertyName("chunk_overlap")]
    public int? ChunkOverlap { get; set; }

    [JsonPropertyName("preset")]
    public string? Preset { get; set; }

    [JsonPropertyName("embedding")]
    public Dictionary<string, object?>? Embedding { get; set; }

    [JsonPropertyName("enabled")]
    public bool? Enabled { get; set; }
}

public sealed class ImageExtractionConfig
{
    [JsonPropertyName("extract_images")]
    public bool? ExtractImages { get; set; }

    [JsonPropertyName("target_dpi")]
    public int? TargetDpi { get; set; }

    [JsonPropertyName("max_image_dimension")]
    public int? MaxImageDimension { get; set; }

    [JsonPropertyName("auto_adjust_dpi")]
    public bool? AutoAdjustDpi { get; set; }

    [JsonPropertyName("min_dpi")]
    public int? MinDpi { get; set; }

    [JsonPropertyName("max_dpi")]
    public int? MaxDpi { get; set; }
}

public sealed class PdfConfig
{
    [JsonPropertyName("extract_images")]
    public bool? ExtractImages { get; set; }

    [JsonPropertyName("passwords")]
    public List<string>? Passwords { get; set; }

    [JsonPropertyName("extract_metadata")]
    public bool? ExtractMetadata { get; set; }
}

public sealed class TokenReductionConfig
{
    [JsonPropertyName("mode")]
    public string? Mode { get; set; }

    [JsonPropertyName("preserve_important_words")]
    public bool? PreserveImportantWords { get; set; }
}

public sealed class LanguageDetectionConfig
{
    [JsonPropertyName("enabled")]
    public bool? Enabled { get; set; }

    [JsonPropertyName("min_confidence")]
    public double? MinConfidence { get; set; }

    [JsonPropertyName("detect_multiple")]
    public bool? DetectMultiple { get; set; }
}

public sealed class PostProcessorConfig
{
    [JsonPropertyName("enabled")]
    public bool? Enabled { get; set; }

    [JsonPropertyName("enabled_processors")]
    public List<string>? EnabledProcessors { get; set; }

    [JsonPropertyName("disabled_processors")]
    public List<string>? DisabledProcessors { get; set; }
}

public sealed class HtmlConversionOptions
{
    [JsonPropertyName("heading_style")]
    public string? HeadingStyle { get; set; }

    [JsonPropertyName("list_indent_type")]
    public string? ListIndentType { get; set; }

    [JsonPropertyName("list_indent_width")]
    public int? ListIndentWidth { get; set; }

    [JsonPropertyName("bullets")]
    public string? Bullets { get; set; }

    [JsonPropertyName("strong_em_symbol")]
    public string? StrongEmSymbol { get; set; }

    [JsonPropertyName("escape_asterisks")]
    public bool? EscapeAsterisks { get; set; }

    [JsonPropertyName("escape_underscores")]
    public bool? EscapeUnderscores { get; set; }

    [JsonPropertyName("escape_misc")]
    public bool? EscapeMisc { get; set; }

    [JsonPropertyName("escape_ascii")]
    public bool? EscapeAscii { get; set; }

    [JsonPropertyName("code_language")]
    public string? CodeLanguage { get; set; }

    [JsonPropertyName("autolinks")]
    public bool? Autolinks { get; set; }

    [JsonPropertyName("default_title")]
    public string? DefaultTitle { get; set; }

    [JsonPropertyName("br_in_tables")]
    public bool? BrInTables { get; set; }

    [JsonPropertyName("hocr_spatial_tables")]
    public bool? HocrSpatialTables { get; set; }

    [JsonPropertyName("highlight_style")]
    public string? HighlightStyle { get; set; }

    [JsonPropertyName("extract_metadata")]
    public bool? ExtractMetadata { get; set; }

    [JsonPropertyName("whitespace_mode")]
    public string? WhitespaceMode { get; set; }

    [JsonPropertyName("strip_newlines")]
    public bool? StripNewlines { get; set; }

    [JsonPropertyName("wrap")]
    public bool? Wrap { get; set; }

    [JsonPropertyName("wrap_width")]
    public int? WrapWidth { get; set; }

    [JsonPropertyName("convert_as_inline")]
    public bool? ConvertAsInline { get; set; }

    [JsonPropertyName("sub_symbol")]
    public string? SubSymbol { get; set; }

    [JsonPropertyName("sup_symbol")]
    public string? SupSymbol { get; set; }

    [JsonPropertyName("newline_style")]
    public string? NewlineStyle { get; set; }

    [JsonPropertyName("code_block_style")]
    public string? CodeBlockStyle { get; set; }

    [JsonPropertyName("keep_inline_images_in")]
    public List<string>? KeepInlineImagesIn { get; set; }

    [JsonPropertyName("encoding")]
    public string? Encoding { get; set; }

    [JsonPropertyName("debug")]
    public bool? Debug { get; set; }

    [JsonPropertyName("strip_tags")]
    public List<string>? StripTags { get; set; }

    [JsonPropertyName("preserve_tags")]
    public List<string>? PreserveTags { get; set; }

    [JsonPropertyName("preprocessing")]
    public HtmlPreprocessingOptions? Preprocessing { get; set; }
}

public sealed class HtmlPreprocessingOptions
{
    [JsonPropertyName("enabled")]
    public bool? Enabled { get; set; }

    [JsonPropertyName("preset")]
    public string? Preset { get; set; }

    [JsonPropertyName("remove_navigation")]
    public bool? RemoveNavigation { get; set; }

    [JsonPropertyName("remove_forms")]
    public bool? RemoveForms { get; set; }
}

public sealed class KeywordConfig
{
    [JsonPropertyName("algorithm")]
    public string? Algorithm { get; set; }

    [JsonPropertyName("max_keywords")]
    public int? MaxKeywords { get; set; }

    [JsonPropertyName("min_score")]
    public double? MinScore { get; set; }

    [JsonPropertyName("ngram_range")]
    public List<int>? NgramRange { get; set; }

    [JsonPropertyName("language")]
    public string? Language { get; set; }

    [JsonPropertyName("yake_params")]
    public Dictionary<string, object?>? YakeParams { get; set; }

    [JsonPropertyName("rake_params")]
    public Dictionary<string, object?>? RakeParams { get; set; }
}

public sealed class BytesWithMime
{
    public byte[] Data { get; }
    public string MimeType { get; }

    public BytesWithMime(byte[] data, string mimeType)
    {
        Data = data ?? throw new ArgumentNullException(nameof(data));
        MimeType = mimeType ?? throw new ArgumentNullException(nameof(mimeType));
    }
}

public interface IPostProcessor
{
    string Name { get; }
    int Priority { get; }
    ExtractionResult Process(ExtractionResult result);
}

public interface IValidator
{
    string Name { get; }
    int Priority { get; }
    void Validate(ExtractionResult result);
}

public interface IOcrBackend
{
    string Name { get; }
    string Process(ReadOnlySpan<byte> imageBytes, OcrConfig? config);
}
