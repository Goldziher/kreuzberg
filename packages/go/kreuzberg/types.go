package kreuzberg

import "encoding/json"

// ExtractionResult mirrors the Rust ExtractionResult struct returned by the core API.
type ExtractionResult struct {
	Content           string           `json:"content"`
	MimeType          string           `json:"mime_type"`
	Metadata          Metadata         `json:"metadata"`
	Tables            []Table          `json:"tables"`
	DetectedLanguages []string         `json:"detected_languages,omitempty"`
	Chunks            []Chunk          `json:"chunks,omitempty"`
	Images            []ExtractedImage `json:"images,omitempty"`
	Success           bool             `json:"success"`
}

// Table represents a detected table in the source document.
type Table struct {
	Cells      [][]string `json:"cells"`
	Markdown   string     `json:"markdown"`
	PageNumber int        `json:"page_number"`
}

// Chunk contains chunked content plus optional embeddings and metadata.
type Chunk struct {
	Content   string        `json:"content"`
	Embedding []float32     `json:"embedding,omitempty"`
	Metadata  ChunkMetadata `json:"metadata"`
}

// ChunkMetadata provides positional information for a chunk.
type ChunkMetadata struct {
	CharStart   int  `json:"char_start"`
	CharEnd     int  `json:"char_end"`
	TokenCount  *int `json:"token_count,omitempty"`
	ChunkIndex  int  `json:"chunk_index"`
	TotalChunks int  `json:"total_chunks"`
}

// ExtractedImage represents an extracted image, optionally with nested OCR results.
type ExtractedImage struct {
	Data             []byte            `json:"data"`
	Format           string            `json:"format"`
	ImageIndex       int               `json:"image_index"`
	PageNumber       *int              `json:"page_number,omitempty"`
	Width            *uint32           `json:"width,omitempty"`
	Height           *uint32           `json:"height,omitempty"`
	Colorspace       *string           `json:"colorspace,omitempty"`
	BitsPerComponent *uint32           `json:"bits_per_component,omitempty"`
	IsMask           bool              `json:"is_mask"`
	Description      *string           `json:"description,omitempty"`
	OCRResult        *ExtractionResult `json:"ocr_result,omitempty"`
}

// Metadata aggregates document metadata and format-specific payloads.
type Metadata struct {
	Language           *string                     `json:"language,omitempty"`
	Date               *string                     `json:"date,omitempty"`
	Subject            *string                     `json:"subject,omitempty"`
	FormatType         string                      `json:"format_type,omitempty"`
	Pdf                *PdfMetadata                `json:"-"`
	Excel              *ExcelMetadata              `json:"-"`
	Email              *EmailMetadata              `json:"-"`
	Pptx               *PptxMetadata               `json:"-"`
	Archive            *ArchiveMetadata            `json:"-"`
	Image              *ImageMetadata              `json:"-"`
	XML                *XMLMetadata                `json:"-"`
	Text               *TextMetadata               `json:"-"`
	HTML               *HtmlMetadata               `json:"-"`
	OCR                *OcrMetadata                `json:"-"`
	ImagePreprocessing *ImagePreprocessingMetadata `json:"image_preprocessing,omitempty"`
	JSONSchema         json.RawMessage             `json:"json_schema,omitempty"`
	Error              *ErrorMetadata              `json:"error,omitempty"`
	Additional         map[string]json.RawMessage  `json:"-"`
}

// PdfMetadata contains metadata extracted from PDF documents.
type PdfMetadata struct {
	Title       *string  `json:"title,omitempty"`
	Subject     *string  `json:"subject,omitempty"`
	Authors     []string `json:"authors,omitempty"`
	Keywords    []string `json:"keywords,omitempty"`
	CreatedAt   *string  `json:"created_at,omitempty"`
	ModifiedAt  *string  `json:"modified_at,omitempty"`
	CreatedBy   *string  `json:"created_by,omitempty"`
	Producer    *string  `json:"producer,omitempty"`
	PageCount   *int     `json:"page_count,omitempty"`
	PDFVersion  *string  `json:"pdf_version,omitempty"`
	IsEncrypted *bool    `json:"is_encrypted,omitempty"`
	Width       *int64   `json:"width,omitempty"`
	Height      *int64   `json:"height,omitempty"`
	Summary     *string  `json:"summary,omitempty"`
}

// ExcelMetadata lists sheets inside spreadsheet documents.
type ExcelMetadata struct {
	SheetCount int      `json:"sheet_count"`
	SheetNames []string `json:"sheet_names"`
}

// EmailMetadata captures envelope data for EML/MSG messages.
type EmailMetadata struct {
	FromEmail   *string  `json:"from_email,omitempty"`
	FromName    *string  `json:"from_name,omitempty"`
	ToEmails    []string `json:"to_emails"`
	CcEmails    []string `json:"cc_emails"`
	BccEmails   []string `json:"bcc_emails"`
	MessageID   *string  `json:"message_id,omitempty"`
	Attachments []string `json:"attachments"`
}

// ArchiveMetadata summarizes archive contents.
type ArchiveMetadata struct {
	Format         string   `json:"format"`
	FileCount      int      `json:"file_count"`
	FileList       []string `json:"file_list"`
	TotalSize      int      `json:"total_size"`
	CompressedSize *int     `json:"compressed_size,omitempty"`
}

// ImageMetadata describes standalone image documents.
type ImageMetadata struct {
	Width  uint32            `json:"width"`
	Height uint32            `json:"height"`
	Format string            `json:"format"`
	EXIF   map[string]string `json:"exif"`
}

// XMLMetadata provides statistics for XML documents.
type XMLMetadata struct {
	ElementCount   int      `json:"element_count"`
	UniqueElements []string `json:"unique_elements"`
}

// TextMetadata contains counts for plain text and Markdown documents.
type TextMetadata struct {
	LineCount      int         `json:"line_count"`
	WordCount      int         `json:"word_count"`
	CharacterCount int         `json:"character_count"`
	Headers        []string    `json:"headers,omitempty"`
	Links          [][2]string `json:"links,omitempty"`
	CodeBlocks     [][2]string `json:"code_blocks,omitempty"`
}

//revive:disable-next-line var-naming
type HtmlMetadata struct {
	Title              *string `json:"title,omitempty"`
	Description        *string `json:"description,omitempty"`
	Keywords           *string `json:"keywords,omitempty"`
	Author             *string `json:"author,omitempty"`
	Canonical          *string `json:"canonical,omitempty"`
	BaseHref           *string `json:"base_href,omitempty"`
	OGTitle            *string `json:"og_title,omitempty"`
	OGDescription      *string `json:"og_description,omitempty"`
	OGImage            *string `json:"og_image,omitempty"`
	OGURL              *string `json:"og_url,omitempty"`
	OGType             *string `json:"og_type,omitempty"`
	OGSiteName         *string `json:"og_site_name,omitempty"`
	TwitterCard        *string `json:"twitter_card,omitempty"`
	TwitterTitle       *string `json:"twitter_title,omitempty"`
	TwitterDescription *string `json:"twitter_description,omitempty"`
	TwitterImage       *string `json:"twitter_image,omitempty"`
	TwitterSite        *string `json:"twitter_site,omitempty"`
	TwitterCreator     *string `json:"twitter_creator,omitempty"`
	LinkAuthor         *string `json:"link_author,omitempty"`
	LinkLicense        *string `json:"link_license,omitempty"`
	LinkAlternate      *string `json:"link_alternate,omitempty"`
}

// PptxMetadata summarizes slide decks.
type PptxMetadata struct {
	Title       *string  `json:"title,omitempty"`
	Author      *string  `json:"author,omitempty"`
	Description *string  `json:"description,omitempty"`
	Summary     *string  `json:"summary,omitempty"`
	Fonts       []string `json:"fonts"`
}

// OcrMetadata records OCR settings/results associated with an extraction.
type OcrMetadata struct {
	Language     string `json:"language"`
	PSM          int    `json:"psm"`
	OutputFormat string `json:"output_format"`
	TableCount   int    `json:"table_count"`
	TableRows    *int   `json:"table_rows,omitempty"`
	TableCols    *int   `json:"table_cols,omitempty"`
}

// ImagePreprocessingMetadata tracks OCR preprocessing steps.
type ImagePreprocessingMetadata struct {
	OriginalDimensions [2]int     `json:"original_dimensions"`
	OriginalDPI        [2]float64 `json:"original_dpi"`
	TargetDPI          int        `json:"target_dpi"`
	ScaleFactor        float64    `json:"scale_factor"`
	AutoAdjusted       bool       `json:"auto_adjusted"`
	FinalDPI           int        `json:"final_dpi"`
	NewDimensions      *[2]int    `json:"new_dimensions,omitempty"`
	ResampleMethod     string     `json:"resample_method"`
	DimensionClamped   bool       `json:"dimension_clamped"`
	CalculatedDPI      *int       `json:"calculated_dpi,omitempty"`
	SkippedResize      bool       `json:"skipped_resize"`
	ResizeError        *string    `json:"resize_error,omitempty"`
}

// ErrorMetadata describes failures in batch operations.
type ErrorMetadata struct {
	ErrorType string `json:"error_type"`
	Message   string `json:"message"`
}
