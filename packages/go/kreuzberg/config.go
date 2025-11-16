package kreuzberg

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
	Postprocessor            *PostProcessorConfig     `json:"postprocessor,omitempty"`
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
	MaxChars     *int           `json:"max_chars,omitempty"`
	MaxOverlap   *int           `json:"max_overlap,omitempty"`
	ChunkSize    *int           `json:"chunk_size,omitempty"`
	ChunkOverlap *int           `json:"chunk_overlap,omitempty"`
	Preset       *string        `json:"preset,omitempty"`
	Embedding    map[string]any `json:"embedding,omitempty"`
	Enabled      *bool          `json:"enabled,omitempty"`
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
