package kreuzberg

// BoolPtr returns a pointer to a bool value. Useful for initializing nullable config fields.
func BoolPtr(b bool) *bool {
	return &b
}

// StringPtr returns a pointer to a string value. Useful for initializing nullable config fields.
func StringPtr(s string) *string {
	return &s
}

// IntPtr returns a pointer to an int value. Useful for initializing nullable config fields.
func IntPtr(i int) *int {
	return &i
}

// FloatPtr returns a pointer to a float64 value. Useful for initializing nullable config fields.
func FloatPtr(f float64) *float64 {
	return &f
}

// ExtractionConfig mirrors the Rust ExtractionConfig structure and is serialized to JSON
// before crossing the FFI boundary. Use pointer fields to omit values and rely on Kreu
// zberg defaults whenever possible.
type ExtractionConfig struct {
	UseCache                 *bool                    `json:"use_cache,omitempty"`
	EnableQualityProcessing  *bool                    `json:"enable_quality_processing,omitempty"`
	OCR                      *OCRConfig               `json:"ocr,omitempty"`
	ForceOCR                 *bool                    `json:"force_ocr,omitempty"`
	Chunking                 *ChunkingConfig          `json:"chunking,omitempty"`
	Images                   *ImageExtractionConfig   `json:"images,omitempty"`
	PdfOptions               *PdfConfig               `json:"pdf_options,omitempty"`
	TokenReduction           *TokenReductionConfig    `json:"token_reduction,omitempty"`
	LanguageDetection        *LanguageDetectionConfig `json:"language_detection,omitempty"`
	Keywords                 *KeywordConfig           `json:"keywords,omitempty"`
	Postprocessor            *PostProcessorConfig     `json:"postprocessor,omitempty"`
	HTMLOptions              *HTMLConversionOptions   `json:"html_options,omitempty"`
	Pages                    *PageConfig              `json:"pages,omitempty"`
	MaxConcurrentExtractions *int                     `json:"max_concurrent_extractions,omitempty"`
}

// OCRConfig selects and configures OCR backends.
type OCRConfig struct {
	Backend   string           `json:"backend,omitempty"`
	Language  *string          `json:"language,omitempty"`
	Tesseract *TesseractConfig `json:"tesseract_config,omitempty"`
}

// TesseractConfig exposes fine-grained controls for the Tesseract backend.
type TesseractConfig struct {
	Language                       string                    `json:"language,omitempty"`
	PSM                            *int                      `json:"psm,omitempty"`
	OutputFormat                   string                    `json:"output_format,omitempty"`
	OEM                            *int                      `json:"oem,omitempty"`
	MinConfidence                  *float64                  `json:"min_confidence,omitempty"`
	Preprocessing                  *ImagePreprocessingConfig `json:"preprocessing,omitempty"`
	EnableTableDetection           *bool                     `json:"enable_table_detection,omitempty"`
	TableMinConfidence             *float64                  `json:"table_min_confidence,omitempty"`
	TableColumnThreshold           *int                      `json:"table_column_threshold,omitempty"`
	TableRowThresholdRatio         *float64                  `json:"table_row_threshold_ratio,omitempty"`
	UseCache                       *bool                     `json:"use_cache,omitempty"`
	ClassifyUsePreAdaptedTemplates *bool                     `json:"classify_use_pre_adapted_templates,omitempty"`
	LanguageModelNgramOn           *bool                     `json:"language_model_ngram_on,omitempty"`
	TesseditDontBlkrejGoodWds      *bool                     `json:"tessedit_dont_blkrej_good_wds,omitempty"`
	TesseditDontRowrejGoodWds      *bool                     `json:"tessedit_dont_rowrej_good_wds,omitempty"`
	TesseditEnableDictCorrection   *bool                     `json:"tessedit_enable_dict_correction,omitempty"`
	TesseditCharWhitelist          string                    `json:"tessedit_char_whitelist,omitempty"`
	TesseditCharBlacklist          string                    `json:"tessedit_char_blacklist,omitempty"`
	TesseditUsePrimaryParamsModel  *bool                     `json:"tessedit_use_primary_params_model,omitempty"`
	TextordSpaceSizeIsVariable     *bool                     `json:"textord_space_size_is_variable,omitempty"`
	ThresholdingMethod             *bool                     `json:"thresholding_method,omitempty"`
}

// ImagePreprocessingConfig tunes DPI normalization and related steps for OCR.
type ImagePreprocessingConfig struct {
	TargetDPI        *int   `json:"target_dpi,omitempty"`
	AutoRotate       *bool  `json:"auto_rotate,omitempty"`
	Deskew           *bool  `json:"deskew,omitempty"`
	Denoise          *bool  `json:"denoise,omitempty"`
	ContrastEnhance  *bool  `json:"contrast_enhance,omitempty"`
	BinarizationMode string `json:"binarization_method,omitempty"`
	InvertColors     *bool  `json:"invert_colors,omitempty"`
}

// ChunkingConfig configures text chunking for downstream RAG/Retrieval workloads.
type ChunkingConfig struct {
	MaxChars     *int             `json:"max_chars,omitempty"`
	MaxOverlap   *int             `json:"max_overlap,omitempty"`
	ChunkSize    *int             `json:"chunk_size,omitempty"`
	ChunkOverlap *int             `json:"chunk_overlap,omitempty"`
	Preset       *string          `json:"preset,omitempty"`
	Embedding    *EmbeddingConfig `json:"embedding,omitempty"`
	Enabled      *bool            `json:"enabled,omitempty"`
}

// ImageExtractionConfig controls inline image extraction from PDFs/Office docs.
type ImageExtractionConfig struct {
	ExtractImages     *bool `json:"extract_images,omitempty"`
	TargetDPI         *int  `json:"target_dpi,omitempty"`
	MaxImageDimension *int  `json:"max_image_dimension,omitempty"`
	AutoAdjustDPI     *bool `json:"auto_adjust_dpi,omitempty"`
	MinDPI            *int  `json:"min_dpi,omitempty"`
	MaxDPI            *int  `json:"max_dpi,omitempty"`
}

// PdfConfig exposes PDF-specific options.
type PdfConfig struct {
	ExtractImages   *bool    `json:"extract_images,omitempty"`
	Passwords       []string `json:"passwords,omitempty"`
	ExtractMetadata *bool    `json:"extract_metadata,omitempty"`
}

// TokenReductionConfig governs token pruning before embeddings.
type TokenReductionConfig struct {
	Mode                   string `json:"mode,omitempty"`
	PreserveImportantWords *bool  `json:"preserve_important_words,omitempty"`
}

// LanguageDetectionConfig enables automatic language detection.
type LanguageDetectionConfig struct {
	Enabled        *bool    `json:"enabled,omitempty"`
	MinConfidence  *float64 `json:"min_confidence,omitempty"`
	DetectMultiple *bool    `json:"detect_multiple,omitempty"`
}

// PostProcessorConfig determines which post processors run.
type PostProcessorConfig struct {
	Enabled            *bool    `json:"enabled,omitempty"`
	EnabledProcessors  []string `json:"enabled_processors,omitempty"`
	DisabledProcessors []string `json:"disabled_processors,omitempty"`
}

// EmbeddingModelType configures embedding model selection.
type EmbeddingModelType struct {
	Type       string `json:"type"`                 // preset, fastembed, custom
	Name       string `json:"name,omitempty"`       // for preset
	Model      string `json:"model,omitempty"`      // for fastembed/custom
	ModelID    string `json:"model_id,omitempty"`   // alias for custom
	Dimensions *int   `json:"dimensions,omitempty"` // for fastembed/custom
}

// EmbeddingConfig configures embedding generation for chunks.
type EmbeddingConfig struct {
	Model                *EmbeddingModelType `json:"model,omitempty"`
	Normalize            *bool               `json:"normalize,omitempty"`
	BatchSize            *int                `json:"batch_size,omitempty"`
	ShowDownloadProgress *bool               `json:"show_download_progress,omitempty"`
	CacheDir             *string             `json:"cache_dir,omitempty"`
}

// KeywordConfig configures keyword extraction.
type KeywordConfig struct {
	Algorithm   string      `json:"algorithm,omitempty"` // yake | rake
	MaxKeywords *int        `json:"max_keywords,omitempty"`
	MinScore    *float64    `json:"min_score,omitempty"`
	NgramRange  *[2]int     `json:"ngram_range,omitempty"`
	Language    *string     `json:"language,omitempty"`
	Yake        *YakeParams `json:"yake_params,omitempty"`
	Rake        *RakeParams `json:"rake_params,omitempty"`
}

// YakeParams holds YAKE-specific tuning.
type YakeParams struct {
	WindowSize *int `json:"window_size,omitempty"`
}

// RakeParams holds RAKE-specific tuning.
type RakeParams struct {
	MinWordLength     *int `json:"min_word_length,omitempty"`
	MaxWordsPerPhrase *int `json:"max_words_per_phrase,omitempty"`
}

// HTMLPreprocessingOptions configures HTML cleaning.
type HTMLPreprocessingOptions struct {
	Enabled          *bool   `json:"enabled,omitempty"`
	Preset           *string `json:"preset,omitempty"` // minimal|standard|aggressive
	RemoveNavigation *bool   `json:"remove_navigation,omitempty"`
	RemoveForms      *bool   `json:"remove_forms,omitempty"`
}

// HTMLConversionOptions mirrors html_to_markdown_rs::ConversionOptions.
type HTMLConversionOptions struct {
	HeadingStyle       *string                   `json:"heading_style,omitempty"`
	ListIndentType     *string                   `json:"list_indent_type,omitempty"`
	ListIndentWidth    *int                      `json:"list_indent_width,omitempty"`
	Bullets            *string                   `json:"bullets,omitempty"`
	StrongEmSymbol     *string                   `json:"strong_em_symbol,omitempty"`
	EscapeAsterisks    *bool                     `json:"escape_asterisks,omitempty"`
	EscapeUnderscores  *bool                     `json:"escape_underscores,omitempty"`
	EscapeMisc         *bool                     `json:"escape_misc,omitempty"`
	EscapeASCII        *bool                     `json:"escape_ascii,omitempty"`
	CodeLanguage       *string                   `json:"code_language,omitempty"`
	Autolinks          *bool                     `json:"autolinks,omitempty"`
	DefaultTitle       *bool                     `json:"default_title,omitempty"`
	BrInTables         *bool                     `json:"br_in_tables,omitempty"`
	HocrSpatialTables  *bool                     `json:"hocr_spatial_tables,omitempty"`
	HighlightStyle     *string                   `json:"highlight_style,omitempty"`
	ExtractMetadata    *bool                     `json:"extract_metadata,omitempty"`
	WhitespaceMode     *string                   `json:"whitespace_mode,omitempty"`
	StripNewlines      *bool                     `json:"strip_newlines,omitempty"`
	Wrap               *bool                     `json:"wrap,omitempty"`
	WrapWidth          *int                      `json:"wrap_width,omitempty"`
	ConvertAsInline    *bool                     `json:"convert_as_inline,omitempty"`
	SubSymbol          *string                   `json:"sub_symbol,omitempty"`
	SupSymbol          *string                   `json:"sup_symbol,omitempty"`
	NewlineStyle       *string                   `json:"newline_style,omitempty"`
	CodeBlockStyle     *string                   `json:"code_block_style,omitempty"`
	KeepInlineImagesIn []string                  `json:"keep_inline_images_in,omitempty"`
	Encoding           *string                   `json:"encoding,omitempty"`
	Debug              *bool                     `json:"debug,omitempty"`
	StripTags          []string                  `json:"strip_tags,omitempty"`
	PreserveTags       []string                  `json:"preserve_tags,omitempty"`
	Preprocessing      *HTMLPreprocessingOptions `json:"preprocessing,omitempty"`
}

// PageConfig configures page tracking and extraction.
type PageConfig struct {
	ExtractPages      *bool   `json:"extract_pages,omitempty"`
	InsertPageMarkers *bool   `json:"insert_page_markers,omitempty"`
	MarkerFormat      *string `json:"marker_format,omitempty"`
}
