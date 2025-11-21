# Type Reference

Complete type definitions and documentation for Kreuzberg across all language bindings.

## ExtractionResult

The main result object returned from all document extraction operations. Contains extracted content, metadata, tables, chunks, and images.

### Rust

```rust
pub struct ExtractionResult {
    pub content: String,
    pub mime_type: String,
    pub metadata: Metadata,
    pub tables: Vec<Table>,
    pub detected_languages: Option<Vec<String>>,
    pub chunks: Option<Vec<Chunk>>,
    pub images: Option<Vec<ExtractedImage>>,
}
```

### Python

```python
class ExtractionResult(TypedDict):
    """Extraction result returned by all extraction functions."""
    content: str
    mime_type: str
    metadata: Metadata
    tables: list[Table]
    detected_languages: list[str] | None
    chunks: list[Chunk] | None
    images: list[ExtractedImage] | None
```

### TypeScript

```typescript
export interface ExtractionResult {
    content: string;
    mimeType: string;
    metadata: Metadata;
    tables: Table[];
    detectedLanguages: string[] | null;
    chunks: Chunk[] | null;
    images: ExtractedImage[] | null;
}
```

### Ruby

```ruby
class Kreuzberg::Result
    attr_reader :content, :mime_type, :metadata, :tables
    attr_reader :detected_languages, :chunks, :images
end
```

### Java

```java
public record ExtractionResult(
    String content,
    String mimeType,
    Metadata metadata,
    List<Table> tables,
    List<String> detectedLanguages,
    List<Chunk> chunks,
    List<ExtractedImage> images
) {}
```

### Go

```go
type ExtractionResult struct {
    Content           string           `json:"content"`
    MimeType          string           `json:"mime_type"`
    Metadata          Metadata         `json:"metadata"`
    Tables            []Table          `json:"tables"`
    DetectedLanguages []string         `json:"detected_languages,omitempty"`
    Chunks            []Chunk          `json:"chunks,omitempty"`
    Images            []ExtractedImage `json:"images,omitempty"`
}
```

## Metadata

Format-specific metadata extracted from documents, using a discriminated union pattern with `format_type` as the discriminator.

### Rust

```rust
pub struct Metadata {
    pub language: Option<String>,
    pub date: Option<String>,
    pub subject: Option<String>,
    pub format: Option<FormatMetadata>,
    pub image_preprocessing: Option<ImagePreprocessingMetadata>,
    pub json_schema: Option<serde_json::Value>,
    pub error: Option<ErrorMetadata>,
    pub additional: HashMap<String, serde_json::Value>,
}

pub enum FormatMetadata {
    #[cfg(feature = "pdf")]
    Pdf(PdfMetadata),
    Excel(ExcelMetadata),
    Email(EmailMetadata),
    Pptx(PptxMetadata),
    Archive(ArchiveMetadata),
    Image(ImageMetadata),
    Xml(XmlMetadata),
    Text(TextMetadata),
    Html(Box<HtmlMetadata>),
    Ocr(OcrMetadata),
}
```

### Python

```python
class Metadata(TypedDict, total=False):
    """Strongly-typed metadata for extraction results."""
    language: str
    date: str
    subject: str
    format_type: Literal["pdf", "excel", "email", "pptx", "archive", "image", "xml", "text", "html", "ocr"]
    # Format-specific fields flatten at root level
    title: str
    authors: list[str]
    keywords: list[str]
    # ... (all fields from format-specific metadata)
    image_preprocessing: ImagePreprocessingMetadata
    json_schema: dict[str, Any]
    error: ErrorMetadata
```

### TypeScript

```typescript
export interface Metadata {
    language?: string | null;
    date?: string | null;
    subject?: string | null;
    format_type?: "pdf" | "excel" | "email" | "pptx" | "archive" | "image" | "xml" | "text" | "html" | "ocr";
    // Format-specific fields flatten at root level
    title?: string | null;
    author?: string | null;
    // ... (all fields from format-specific metadata)
    image_preprocessing?: ImagePreprocessingMetadata | null;
    json_schema?: Record<string, unknown> | null;
    error?: ErrorMetadata | null;
    [key: string]: any;
}
```

### Ruby

```ruby
# Returned as Hash from native extension
result.metadata  # Hash<String, Any>
# Access format type to determine which fields are present
result.metadata["format_type"]  # "pdf", "excel", "email", etc.
```

### Java

```java
public final class Metadata {
    private final Optional<String> language;
    private final Optional<String> date;
    private final Optional<String> subject;
    private final FormatMetadata format;
    private final Optional<ImagePreprocessingMetadata> imagePreprocessing;
    private final Optional<Map<String, Object>> jsonSchema;
    private final Optional<ErrorMetadata> error;
}

public final class FormatMetadata {
    private final FormatType type;
    private final Optional<PdfMetadata> pdf;
    private final Optional<ExcelMetadata> excel;
    private final Optional<EmailMetadata> email;
    // ... (one for each format)
}
```

### Go

```go
type Metadata struct {
    Language           *string                     `json:"language,omitempty"`
    Date               *string                     `json:"date,omitempty"`
    Subject            *string                     `json:"subject,omitempty"`
    Format             FormatMetadata              `json:"-"`
    ImagePreprocessing *ImagePreprocessingMetadata `json:"image_preprocessing,omitempty"`
    JSONSchema         json.RawMessage             `json:"json_schema,omitempty"`
    Error              *ErrorMetadata              `json:"error,omitempty"`
    Additional         map[string]json.RawMessage `json:"-"`
}

type FormatMetadata struct {
    Type    FormatType
    Pdf     *PdfMetadata
    Excel   *ExcelMetadata
    Email   *EmailMetadata
    // ... (one for each format)
}
```

## Format-Specific Metadata

### PDF Metadata

Common fields across all formats; format-specific fields are available when `format_type == "pdf"`.

#### Rust

```rust
pub struct PdfMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub modification_date: Option<String>,
    pub page_count: Option<usize>,
}
```

#### Python

```python
class PdfMetadata(TypedDict, total=False):
    title: str | None
    author: str | None
    subject: str | None
    keywords: str | None
    creator: str | None
    producer: str | None
    creation_date: str | None
    modification_date: str | None
    page_count: int
```

#### TypeScript

```typescript
export interface PdfMetadata {
    title?: string | null;
    author?: string | null;
    subject?: string | null;
    keywords?: string | null;
    creator?: string | null;
    producer?: string | null;
    creationDate?: string | null;
    modificationDate?: string | null;
    pageCount?: number;
}
```

#### Java

```java
public record PdfMetadata(
    Optional<String> title,
    Optional<String> author,
    Optional<String> subject,
    Optional<String> keywords,
    Optional<String> creator,
    Optional<String> producer,
    Optional<String> creationDate,
    Optional<String> modificationDate,
    Optional<Integer> pageCount
) {}
```

#### Go

```go
type PdfMetadata struct {
    Title            *string `json:"title,omitempty"`
    Author           *string `json:"author,omitempty"`
    Subject          *string `json:"subject,omitempty"`
    Keywords         []string `json:"keywords,omitempty"`
    Creator          *string `json:"creator,omitempty"`
    Producer         *string `json:"producer,omitempty"`
    CreatedAt        *string `json:"created_at,omitempty"`
    ModifiedAt       *string `json:"modified_at,omitempty"`
    PageCount        *int    `json:"page_count,omitempty"`
}
```

### Excel Metadata

#### Rust

```rust
pub struct ExcelMetadata {
    pub sheet_count: usize,
    pub sheet_names: Vec<String>,
}
```

#### Python

```python
class ExcelMetadata(TypedDict, total=False):
    sheet_count: int
    sheet_names: list[str]
```

#### TypeScript

```typescript
export interface ExcelMetadata {
    sheetCount?: number;
    sheetNames?: string[];
}
```

#### Java

```java
public record ExcelMetadata(
    int sheetCount,
    List<String> sheetNames
) {}
```

#### Go

```go
type ExcelMetadata struct {
    SheetCount int      `json:"sheet_count"`
    SheetNames []string `json:"sheet_names"`
}
```

### Email Metadata

#### Rust

```rust
pub struct EmailMetadata {
    pub from_email: Option<String>,
    pub from_name: Option<String>,
    pub to_emails: Vec<String>,
    pub cc_emails: Vec<String>,
    pub bcc_emails: Vec<String>,
    pub message_id: Option<String>,
    pub attachments: Vec<String>,
}
```

#### Python

```python
class EmailMetadata(TypedDict, total=False):
    from_email: str | None
    from_name: str | None
    to_emails: list[str]
    cc_emails: list[str]
    bcc_emails: list[str]
    message_id: str | None
    attachments: list[str]
```

#### TypeScript

```typescript
export interface EmailMetadata {
    fromEmail?: string | null;
    fromName?: string | null;
    toEmails?: string[];
    ccEmails?: string[];
    bccEmails?: string[];
    messageId?: string | null;
    attachments?: string[];
}
```

#### Java

```java
public record EmailMetadata(
    Optional<String> fromEmail,
    Optional<String> fromName,
    List<String> toEmails,
    List<String> ccEmails,
    List<String> bccEmails,
    Optional<String> messageId,
    List<String> attachments
) {}
```

#### Go

```go
type EmailMetadata struct {
    FromEmail   *string  `json:"from_email,omitempty"`
    FromName    *string  `json:"from_name,omitempty"`
    ToEmails    []string `json:"to_emails"`
    CcEmails    []string `json:"cc_emails"`
    BccEmails   []string `json:"bcc_emails"`
    MessageID   *string  `json:"message_id,omitempty"`
    Attachments []string `json:"attachments"`
}
```

### Archive Metadata

#### Rust

```rust
pub struct ArchiveMetadata {
    pub format: String,
    pub file_count: usize,
    pub file_list: Vec<String>,
    pub total_size: usize,
    pub compressed_size: Option<usize>,
}
```

#### Python

```python
class ArchiveMetadata(TypedDict, total=False):
    format: str
    file_count: int
    file_list: list[str]
    total_size: int
    compressed_size: int | None
```

#### TypeScript

```typescript
export interface ArchiveMetadata {
    format?: string;
    fileCount?: number;
    fileList?: string[];
    totalSize?: number;
    compressedSize?: number | null;
}
```

#### Java

```java
public record ArchiveMetadata(
    String format,
    int fileCount,
    List<String> fileList,
    int totalSize,
    Optional<Integer> compressedSize
) {}
```

#### Go

```go
type ArchiveMetadata struct {
    Format         string   `json:"format"`
    FileCount      int      `json:"file_count"`
    FileList       []string `json:"file_list"`
    TotalSize      int      `json:"total_size"`
    CompressedSize *int     `json:"compressed_size,omitempty"`
}
```

### Image Metadata

#### Rust

```rust
pub struct ImageMetadata {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub exif: HashMap<String, String>,
}
```

#### Python

```python
class ImageMetadata(TypedDict, total=False):
    width: int
    height: int
    format: str
    exif: dict[str, str]
```

#### TypeScript

```typescript
export interface ImageMetadata {
    width?: number;
    height?: number;
    format?: string;
    exif?: Record<string, string>;
}
```

#### Java

```java
public record ImageMetadata(
    int width,
    int height,
    String format,
    Map<String, String> exif
) {}
```

#### Go

```go
type ImageMetadata struct {
    Width  uint32            `json:"width"`
    Height uint32            `json:"height"`
    Format string            `json:"format"`
    EXIF   map[string]string `json:"exif"`
}
```

### HTML Metadata

#### Rust

```rust
pub struct HtmlMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<String>,
    pub author: Option<String>,
    pub canonical: Option<String>,
    pub base_href: Option<String>,
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub og_image: Option<String>,
    pub og_url: Option<String>,
    pub og_type: Option<String>,
    pub og_site_name: Option<String>,
    pub twitter_card: Option<String>,
    pub twitter_title: Option<String>,
    pub twitter_description: Option<String>,
    pub twitter_image: Option<String>,
    pub twitter_site: Option<String>,
    pub twitter_creator: Option<String>,
    pub link_author: Option<String>,
    pub link_license: Option<String>,
    pub link_alternate: Option<String>,
}
```

#### Python

```python
class HtmlMetadata(TypedDict, total=False):
    title: str | None
    description: str | None
    keywords: str | None
    author: str | None
    canonical: str | None
    base_href: str | None
    og_title: str | None
    og_description: str | None
    og_image: str | None
    og_url: str | None
    og_type: str | None
    og_site_name: str | None
    twitter_card: str | None
    twitter_title: str | None
    twitter_description: str | None
    twitter_image: str | None
    twitter_site: str | None
    twitter_creator: str | None
    link_author: str | None
    link_license: str | None
    link_alternate: str | None
```

#### TypeScript

```typescript
export interface HtmlMetadata {
    title?: string | null;
    description?: string | null;
    keywords?: string | null;
    author?: string | null;
    canonical?: string | null;
    baseHref?: string | null;
    ogTitle?: string | null;
    ogDescription?: string | null;
    ogImage?: string | null;
    ogUrl?: string | null;
    ogType?: string | null;
    ogSiteName?: string | null;
    twitterCard?: string | null;
    twitterTitle?: string | null;
    twitterDescription?: string | null;
    twitterImage?: string | null;
    twitterSite?: string | null;
    twitterCreator?: string | null;
    linkAuthor?: string | null;
    linkLicense?: string | null;
    linkAlternate?: string | null;
}
```

#### Java

```java
public record HtmlMetadata(
    Optional<String> title,
    Optional<String> description,
    Optional<String> keywords,
    Optional<String> author,
    Optional<String> canonical,
    Optional<String> baseHref,
    Optional<String> ogTitle,
    Optional<String> ogDescription,
    Optional<String> ogImage,
    Optional<String> ogUrl,
    Optional<String> ogType,
    Optional<String> ogSiteName,
    Optional<String> twitterCard,
    Optional<String> twitterTitle,
    Optional<String> twitterDescription,
    Optional<String> twitterImage,
    Optional<String> twitterSite,
    Optional<String> twitterCreator,
    Optional<String> linkAuthor,
    Optional<String> linkLicense,
    Optional<String> linkAlternate
) {}
```

#### Go

```go
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
```

### Text/Markdown Metadata

#### Rust

```rust
pub struct TextMetadata {
    pub line_count: usize,
    pub word_count: usize,
    pub character_count: usize,
    pub headers: Option<Vec<String>>,
    pub links: Option<Vec<(String, String)>>,
    pub code_blocks: Option<Vec<(String, String)>>,
}
```

#### Python

```python
class TextMetadata(TypedDict, total=False):
    line_count: int
    word_count: int
    character_count: int
    headers: list[str] | None
    links: list[tuple[str, str]] | None
    code_blocks: list[tuple[str, str]] | None
```

#### TypeScript

```typescript
export interface TextMetadata {
    lineCount?: number;
    wordCount?: number;
    characterCount?: number;
    headers?: string[] | null;
    links?: [string, string][] | null;
    codeBlocks?: [string, string][] | null;
}
```

#### Java

```java
public record TextMetadata(
    int lineCount,
    int wordCount,
    int characterCount,
    Optional<List<String>> headers,
    Optional<List<String[]>> links,
    Optional<List<String[]>> codeBlocks
) {}
```

#### Go

```go
type TextMetadata struct {
    LineCount      int         `json:"line_count"`
    WordCount      int         `json:"word_count"`
    CharacterCount int         `json:"character_count"`
    Headers        []string    `json:"headers,omitempty"`
    Links          [][2]string `json:"links,omitempty"`
    CodeBlocks     [][2]string `json:"code_blocks,omitempty"`
}
```

### PowerPoint Metadata

#### Rust

```rust
pub struct PptxMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub summary: Option<String>,
    pub fonts: Vec<String>,
}
```

#### Python

```python
class PptxMetadata(TypedDict, total=False):
    title: str | None
    author: str | None
    description: str | None
    summary: str | None
    fonts: list[str]
```

#### TypeScript

```typescript
export interface PptxMetadata {
    title?: string | null;
    author?: string | null;
    description?: string | null;
    summary?: string | null;
    fonts?: string[];
}
```

#### Java

```java
public record PptxMetadata(
    Optional<String> title,
    Optional<String> author,
    Optional<String> description,
    Optional<String> summary,
    List<String> fonts
) {}
```

#### Go

```go
type PptxMetadata struct {
    Title       *string  `json:"title,omitempty"`
    Author      *string  `json:"author,omitempty"`
    Description *string  `json:"description,omitempty"`
    Summary     *string  `json:"summary,omitempty"`
    Fonts       []string `json:"fonts"`
}
```

### OCR Metadata

#### Rust

```rust
pub struct OcrMetadata {
    pub language: String,
    pub psm: i32,
    pub output_format: String,
    pub table_count: usize,
    pub table_rows: Option<usize>,
    pub table_cols: Option<usize>,
}
```

#### Python

```python
class OcrMetadata(TypedDict, total=False):
    language: str
    psm: int
    output_format: str
    table_count: int
    table_rows: int | None
    table_cols: int | None
```

#### TypeScript

```typescript
export interface OcrMetadata {
    language?: string;
    psm?: number;
    outputFormat?: string;
    tableCount?: number;
    tableRows?: number | null;
    tableCols?: number | null;
}
```

#### Java

```java
public record OcrMetadata(
    String language,
    int psm,
    String outputFormat,
    int tableCount,
    Optional<Integer> tableRows,
    Optional<Integer> tableCols
) {}
```

#### Go

```go
type OcrMetadata struct {
    Language     string `json:"language"`
    PSM          int    `json:"psm"`
    OutputFormat string `json:"output_format"`
    TableCount   int    `json:"table_count"`
    TableRows    *int   `json:"table_rows,omitempty"`
    TableCols    *int   `json:"table_cols,omitempty"`
}
```

## Table

Represents a table extracted from a document.

### Rust

```rust
pub struct Table {
    pub cells: Vec<Vec<String>>,
    pub markdown: String,
    pub page_number: usize,
}
```

### Python

```python
class Table(TypedDict):
    cells: list[list[str]]
    markdown: str
    page_number: int
```

### TypeScript

```typescript
export interface Table {
    cells: string[][];
    markdown: string;
    pageNumber: number;
}
```

### Ruby

```ruby
Kreuzberg::Result::Table = Struct.new(:cells, :markdown, :page_number, keyword_init: true)
```

### Java

```java
public record Table(
    List<List<String>> cells,
    String markdown,
    int pageNumber
) {}
```

### Go

```go
type Table struct {
    Cells      [][]string `json:"cells"`
    Markdown   string     `json:"markdown"`
    PageNumber int        `json:"page_number"`
}
```

## Chunk

Text chunk with optional embedding vector and metadata.

### Rust

```rust
pub struct Chunk {
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub metadata: ChunkMetadata,
}

pub struct ChunkMetadata {
    pub char_start: usize,
    pub char_end: usize,
    pub token_count: Option<usize>,
    pub chunk_index: usize,
    pub total_chunks: usize,
}
```

### Python

```python
class ChunkMetadata(TypedDict):
    char_start: int
    char_end: int
    token_count: int | None
    chunk_index: int
    total_chunks: int

class Chunk(TypedDict, total=False):
    content: str
    embedding: list[float] | None
    metadata: ChunkMetadata
```

### TypeScript

```typescript
export interface ChunkMetadata {
    charStart: number;
    charEnd: number;
    tokenCount?: number | null;
    chunkIndex: number;
    totalChunks: number;
}

export interface Chunk {
    content: string;
    embedding?: number[] | null;
    metadata: ChunkMetadata;
}
```

### Ruby

```ruby
Kreuzberg::Result::Chunk = Struct.new(
    :content, :char_start, :char_end, :token_count,
    :chunk_index, :total_chunks, :embedding,
    keyword_init: true
)
```

### Java

```java
public record ChunkMetadata(
    int charStart,
    int charEnd,
    Optional<Integer> tokenCount,
    int chunkIndex,
    int totalChunks
) {}

public record Chunk(
    String content,
    Optional<List<Float>> embedding,
    ChunkMetadata metadata
) {}
```

### Go

```go
type ChunkMetadata struct {
    CharStart   int  `json:"char_start"`
    CharEnd     int  `json:"char_end"`
    TokenCount  *int `json:"token_count,omitempty"`
    ChunkIndex  int  `json:"chunk_index"`
    TotalChunks int  `json:"total_chunks"`
}

type Chunk struct {
    Content   string        `json:"content"`
    Embedding []float32     `json:"embedding,omitempty"`
    Metadata  ChunkMetadata `json:"metadata"`
}
```

## ExtractedImage

Image extracted from a document, optionally with nested OCR results.

### Rust

```rust
pub struct ExtractedImage {
    pub data: Vec<u8>,
    pub format: String,
    pub image_index: usize,
    pub page_number: Option<usize>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub colorspace: Option<String>,
    pub bits_per_component: Option<u32>,
    pub is_mask: bool,
    pub description: Option<String>,
    pub ocr_result: Option<Box<ExtractionResult>>,
}
```

### Python

```python
class ExtractedImage(TypedDict, total=False):
    data: bytes
    format: str
    image_index: int
    page_number: int | None
    width: int | None
    height: int | None
    colorspace: str | None
    bits_per_component: int | None
    is_mask: bool
    description: str | None
    ocr_result: ExtractionResult | None
```

### TypeScript

```typescript
export interface ExtractedImage {
    data: Uint8Array;
    format: string;
    imageIndex: number;
    pageNumber?: number | null;
    width?: number | null;
    height?: number | null;
    colorspace?: string | null;
    bitsPerComponent?: number | null;
    isMask: boolean;
    description?: string | null;
    ocrResult?: ExtractionResult | null;
}
```

### Ruby

```ruby
Kreuzberg::Result::Image = Struct.new(
    :data, :format, :image_index, :page_number, :width, :height,
    :colorspace, :bits_per_component, :is_mask, :description, :ocr_result,
    keyword_init: true
)
```

### Java

```java
public record ExtractedImage(
    byte[] data,
    String format,
    int imageIndex,
    Optional<Integer> pageNumber,
    Optional<Integer> width,
    Optional<Integer> height,
    Optional<String> colorspace,
    Optional<Integer> bitsPerComponent,
    boolean isMask,
    Optional<String> description,
    Optional<ExtractionResult> ocrResult
) {}
```

### Go

```go
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
```

## Configuration Types

### ExtractionConfig

Main extraction configuration controlling all pipeline features.

#### Rust

```rust
pub struct ExtractionConfig {
    pub use_cache: bool,
    pub enable_quality_processing: bool,
    pub ocr: Option<OcrConfig>,
    pub force_ocr: bool,
    pub chunking: Option<ChunkingConfig>,
    pub images: Option<ImageExtractionConfig>,
    pub pdf_options: Option<PdfConfig>,
    pub token_reduction: Option<TokenReductionConfig>,
    pub language_detection: Option<LanguageDetectionConfig>,
    pub keywords: Option<KeywordConfig>,
    pub postprocessor: Option<PostProcessorConfig>,
    pub max_concurrent_extractions: Option<usize>,
}
```

#### Python

```python
@dataclass
class ExtractionConfig:
    use_cache: bool = True
    enable_quality_processing: bool = True
    ocr: OcrConfig | None = None
    force_ocr: bool = False
    chunking: ChunkingConfig | None = None
    images: ImageExtractionConfig | None = None
    pdf_options: PdfConfig | None = None
    token_reduction: TokenReductionConfig | None = None
    language_detection: LanguageDetectionConfig | None = None
    keywords: KeywordConfig | None = None
    postprocessor: PostProcessorConfig | None = None
    max_concurrent_extractions: int | None = None
```

#### TypeScript

```typescript
export interface ExtractionConfig {
    useCache?: boolean;
    enableQualityProcessing?: boolean;
    ocr?: OcrConfig;
    forceOcr?: boolean;
    chunking?: ChunkingConfig;
    images?: ImageExtractionConfig;
    pdfOptions?: PdfConfig;
    tokenReduction?: TokenReductionConfig;
    languageDetection?: LanguageDetectionConfig;
    keywords?: KeywordConfig;
    postprocessor?: PostProcessorConfig;
    maxConcurrentExtractions?: number;
}
```

#### Java

```java
public record ExtractionConfig(
    boolean useCache,
    boolean enableQualityProcessing,
    Optional<OcrConfig> ocr,
    boolean forceOcr,
    Optional<ChunkingConfig> chunking,
    Optional<ImageExtractionConfig> images,
    Optional<PdfConfig> pdfOptions,
    Optional<TokenReductionConfig> tokenReduction,
    Optional<LanguageDetectionConfig> languageDetection,
    Optional<KeywordConfig> keywords,
    Optional<PostProcessorConfig> postprocessor,
    Optional<Integer> maxConcurrentExtractions
) {}
```

#### Go

```go
type ExtractionConfig struct {
    UseCache                    bool
    EnableQualityProcessing     bool
    OCR                         *OcrConfig
    ForceOCR                    bool
    Chunking                    *ChunkingConfig
    Images                      *ImageExtractionConfig
    PDFOptions                  *PdfConfig
    TokenReduction              *TokenReductionConfig
    LanguageDetection           *LanguageDetectionConfig
    Keywords                    *KeywordConfig
    PostProcessor               *PostProcessorConfig
    MaxConcurrentExtractions    *int
}
```

### OcrConfig

OCR (Optical Character Recognition) configuration.

#### Rust

```rust
pub struct OcrConfig {
    pub backend: String,  // "tesseract", "easyocr", "paddleocr"
    pub language: String, // e.g., "eng", "deu", "fra"
    pub tesseract_config: Option<TesseractConfig>,
}
```

#### Python

```python
@dataclass
class OcrConfig:
    backend: str = "tesseract"
    language: str = "eng"
    tesseract_config: TesseractConfig | None = None
```

#### TypeScript

```typescript
export interface OcrConfig {
    backend: string;
    language?: string;
    tesseractConfig?: TesseractConfig;
}
```

#### Java

```java
public record OcrConfig(
    String backend,
    String language,
    Optional<TesseractConfig> tesseractConfig
) {}
```

#### Go

```go
type OcrConfig struct {
    Backend            string
    Language           string
    TesseractConfig    *TesseractConfig
}
```

### TesseractConfig

Fine-grained Tesseract OCR engine parameters.

#### Rust

```rust
pub struct TesseractConfig {
    pub language: String,
    pub psm: i32,  // Page Segmentation Mode (0-13)
    pub output_format: String,  // "text" or "markdown"
    pub oem: i32,  // OCR Engine Mode (0-3)
    pub min_confidence: f64,
    pub preprocessing: Option<ImagePreprocessingConfig>,
    pub enable_table_detection: bool,
    pub table_min_confidence: f64,
    pub table_column_threshold: i32,
    pub table_row_threshold_ratio: f64,
    pub use_cache: bool,
    pub classify_use_pre_adapted_templates: bool,
    pub language_model_ngram_on: bool,
    pub tessedit_dont_blkrej_good_wds: bool,
    pub tessedit_dont_rowrej_good_wds: bool,
    pub tessedit_enable_dict_correction: bool,
    pub tessedit_char_whitelist: String,
    pub tessedit_char_blacklist: String,
    pub tessedit_use_primary_params_model: bool,
    pub textord_space_size_is_variable: bool,
    pub thresholding_method: bool,
}
```

### ChunkingConfig

Text chunking configuration for creating overlapping chunks.

#### Rust

```rust
pub struct ChunkingConfig {
    pub max_chars: usize,
    pub max_overlap: usize,
    pub embedding: Option<EmbeddingConfig>,
    pub preset: Option<String>,
}
```

#### Python

```python
@dataclass
class ChunkingConfig:
    max_chars: int = 1000
    max_overlap: int = 200
    embedding: EmbeddingConfig | None = None
    preset: str | None = None
```

#### TypeScript

```typescript
export interface ChunkingConfig {
    maxChars?: number;
    maxOverlap?: number;
    embedding?: EmbeddingConfig;
    preset?: string;
}
```

#### Java

```java
public record ChunkingConfig(
    int maxChars,
    int maxOverlap,
    Optional<EmbeddingConfig> embedding,
    Optional<String> preset
) {}
```

#### Go

```go
type ChunkingConfig struct {
    MaxChars   int
    MaxOverlap int
    Embedding  *EmbeddingConfig
    Preset     *string
}
```

### EmbeddingConfig

Configuration for generating embeddings on text chunks.

#### Rust

```rust
pub struct EmbeddingConfig {
    pub model: EmbeddingModelType,
    pub normalize: bool,
    pub batch_size: usize,
    pub show_download_progress: bool,
    pub cache_dir: Option<PathBuf>,
}

pub enum EmbeddingModelType {
    Preset { name: String },
    FastEmbed { model: String, dimensions: usize },
    Custom { model_id: String, dimensions: usize },
}
```

#### Python

```python
@dataclass
class EmbeddingConfig:
    model: EmbeddingModelType = field(default_factory=lambda: Preset("balanced"))
    normalize: bool = True
    batch_size: int = 32
    show_download_progress: bool = False
    cache_dir: Path | None = None

@dataclass
class EmbeddingModelType:
    # One of: Preset, FastEmbed, Custom (discriminated union)
    pass
```

#### TypeScript

```typescript
export interface EmbeddingConfig {
    model: EmbeddingModelType;
    normalize?: boolean;
    batchSize?: number;
    showDownloadProgress?: boolean;
    cacheDir?: string;
}

export type EmbeddingModelType =
    | { type: "preset"; name: string }
    | { type: "fastembed"; model: string; dimensions: number }
    | { type: "custom"; modelId: string; dimensions: number };
```

### ImageExtractionConfig

Configuration for extracting and preprocessing images.

#### Rust

```rust
pub struct ImageExtractionConfig {
    pub extract_images: bool,
    pub target_dpi: i32,
    pub max_image_dimension: i32,
    pub auto_adjust_dpi: bool,
    pub min_dpi: i32,
    pub max_dpi: i32,
}
```

#### Python

```python
@dataclass
class ImageExtractionConfig:
    extract_images: bool = True
    target_dpi: int = 300
    max_image_dimension: int = 4096
    auto_adjust_dpi: bool = True
    min_dpi: int = 72
    max_dpi: int = 600
```

#### TypeScript

```typescript
export interface ImageExtractionConfig {
    extractImages?: boolean;
    targetDpi?: number;
    maxImageDimension?: number;
    autoAdjustDpi?: boolean;
    minDpi?: number;
    maxDpi?: number;
}
```

#### Java

```java
public record ImageExtractionConfig(
    boolean extractImages,
    int targetDpi,
    int maxImageDimension,
    boolean autoAdjustDpi,
    int minDpi,
    int maxDpi
) {}
```

#### Go

```go
type ImageExtractionConfig struct {
    ExtractImages      bool
    TargetDPI          int32
    MaxImageDimension  int32
    AutoAdjustDPI      bool
    MinDPI             int32
    MaxDPI             int32
}
```

### PdfConfig

PDF-specific extraction options.

#### Rust

```rust
pub struct PdfConfig {
    pub extract_images: bool,
    pub passwords: Option<Vec<String>>,
    pub extract_metadata: bool,
}
```

#### Python

```python
@dataclass
class PdfConfig:
    extract_images: bool = False
    passwords: list[str] | None = None
    extract_metadata: bool = True
```

#### TypeScript

```typescript
export interface PdfConfig {
    extractImages?: boolean;
    passwords?: string[];
    extractMetadata?: boolean;
}
```

#### Ruby

```ruby
class Kreuzberg::Config::PdfConfig
    attr_accessor :extract_images, :passwords, :extract_metadata
end
```

#### Java

```java
public final class PdfConfig {
    private final boolean extractImages;
    private final List<String> passwords;
    private final boolean extractMetadata;

    public static Builder builder() { }
}
```

#### Go

```go
type PdfConfig struct {
    ExtractImages   *bool    `json:"extract_images,omitempty"`
    Passwords       []string `json:"passwords,omitempty"`
    ExtractMetadata *bool    `json:"extract_metadata,omitempty"`
}
```

### TokenReductionConfig

Token reduction configuration for minimizing output size.

#### Rust

```rust
pub struct TokenReductionConfig {
    pub mode: String,
    pub preserve_important_words: bool,
}
```

#### Python

```python
@dataclass
class TokenReductionConfig:
    mode: str = "off"
    preserve_important_words: bool = True
```

#### TypeScript

```typescript
export interface TokenReductionConfig {
    mode?: string;
    preserveImportantWords?: boolean;
}
```

#### Ruby

```ruby
class Kreuzberg::Config::TokenReductionConfig
    attr_accessor :mode, :preserve_important_words
end
```

#### Java

```java
public final class TokenReductionConfig {
    private final String mode;
    private final boolean preserveImportantWords;

    public static Builder builder() { }
}
```

#### Go

```go
type TokenReductionConfig struct {
    Mode                   string `json:"mode,omitempty"`
    PreserveImportantWords *bool  `json:"preserve_important_words,omitempty"`
}
```

### LanguageDetectionConfig

Language detection configuration enabling automatic language identification.

#### Rust

```rust
pub struct LanguageDetectionConfig {
    pub enabled: bool,
    pub min_confidence: f64,
    pub detect_multiple: bool,
}
```

#### Python

```python
@dataclass
class LanguageDetectionConfig:
    enabled: bool = True
    min_confidence: float = 0.8
    detect_multiple: bool = False
```

#### TypeScript

```typescript
export interface LanguageDetectionConfig {
    enabled?: boolean;
    minConfidence?: number;
    detectMultiple?: boolean;
}
```

#### Ruby

```ruby
class Kreuzberg::Config::LanguageDetectionConfig
    attr_accessor :enabled, :min_confidence, :detect_multiple
end
```

#### Java

```java
public final class LanguageDetectionConfig {
    private final boolean enabled;
    private final double minConfidence;
    private final boolean detectMultiple;

    public static Builder builder() { }
}
```

#### Go

```go
type LanguageDetectionConfig struct {
    Enabled        *bool    `json:"enabled,omitempty"`
    MinConfidence  *float64 `json:"min_confidence,omitempty"`
    DetectMultiple *bool    `json:"detect_multiple,omitempty"`
}
```

### KeywordConfig

Keyword extraction configuration for automatic keyword/phrase extraction.

#### Rust

```rust
pub struct KeywordConfig {
    pub algorithm: KeywordAlgorithm,
    pub max_keywords: usize,
    pub min_score: f32,
    pub ngram_range: (usize, usize),
    pub language: Option<String>,
    pub yake_params: Option<YakeParams>,
    pub rake_params: Option<RakeParams>,
}
```

#### Python

```python
@dataclass
class YakeParams:
    window_size: int = 2

@dataclass
class RakeParams:
    min_word_length: int = 1
    max_words_per_phrase: int = 3

@dataclass
class KeywordConfig:
    algorithm: str = "yake"
    max_keywords: int = 10
    min_score: float = 0.0
    ngram_range: tuple[int, int] = (1, 3)
    language: str | None = "en"
    yake_params: YakeParams | None = None
    rake_params: RakeParams | None = None
```

#### TypeScript

```typescript
export interface YakeParams {
    windowSize?: number;
}

export interface RakeParams {
    minWordLength?: number;
    maxWordsPerPhrase?: number;
}

export interface KeywordConfig {
    algorithm?: KeywordAlgorithm;
    maxKeywords?: number;
    minScore?: number;
    ngramRange?: [number, number];
    language?: string;
    yakeParams?: YakeParams;
    rakeParams?: RakeParams;
}
```

#### Ruby

```ruby
class Kreuzberg::Config::KeywordConfig
    attr_accessor :algorithm, :max_keywords, :min_score,
                  :ngram_range, :language, :yake_params, :rake_params
end
```

#### Java

```java
public final class KeywordConfig {
    private final String algorithm;
    private final Integer maxKeywords;
    private final Double minScore;
    private final int[] ngramRange;
    private final String language;
    private final YakeParams yakeParams;
    private final RakeParams rakeParams;

    public static Builder builder() { }

    public static final class YakeParams { }
    public static final class RakeParams { }
}
```

#### Go

```go
type YakeParams struct {
    WindowSize *int `json:"window_size,omitempty"`
}

type RakeParams struct {
    MinWordLength     *int `json:"min_word_length,omitempty"`
    MaxWordsPerPhrase *int `json:"max_words_per_phrase,omitempty"`
}

type KeywordConfig struct {
    Algorithm   string      `json:"algorithm,omitempty"`
    MaxKeywords *int        `json:"max_keywords,omitempty"`
    MinScore    *float64    `json:"min_score,omitempty"`
    NgramRange  *[2]int     `json:"ngram_range,omitempty"`
    Language    *string     `json:"language,omitempty"`
    Yake        *YakeParams `json:"yake_params,omitempty"`
    Rake        *RakeParams `json:"rake_params,omitempty"`
}
```

### ImagePreprocessingMetadata

Image preprocessing transformation metadata tracking DPI changes and resizing.

#### Rust

```rust
pub struct ImagePreprocessingMetadata {
    pub original_dimensions: (usize, usize),
    pub original_dpi: (f64, f64),
    pub target_dpi: i32,
    pub scale_factor: f64,
    pub auto_adjusted: bool,
    pub final_dpi: i32,
    pub new_dimensions: Option<(usize, usize)>,
    pub resample_method: String,
    pub dimension_clamped: bool,
    pub calculated_dpi: Option<i32>,
    pub skipped_resize: bool,
    pub resize_error: Option<String>,
}
```

#### Python

```python
class ImagePreprocessingMetadata(TypedDict, total=False):
    original_dimensions: tuple[int, int]
    original_dpi: tuple[float, float]
    target_dpi: int
    scale_factor: float
    auto_adjusted: bool
    final_dpi: int
    new_dimensions: tuple[int, int] | None
    resample_method: str
    dimension_clamped: bool
    calculated_dpi: int | None
    skipped_resize: bool
    resize_error: str | None
```

#### TypeScript

```typescript
export interface ImagePreprocessingMetadata {
    originalDimensions?: [number, number];
    originalDpi?: [number, number];
    targetDpi?: number;
    scaleFactor?: number;
    autoAdjusted?: boolean;
    finalDpi?: number;
    newDimensions?: [number, number] | null;
    resampleMethod?: string;
    dimensionClamped?: boolean;
    calculatedDpi?: number | null;
    skippedResize?: boolean;
    resizeError?: string | null;
}
```

#### Ruby

```ruby
class Kreuzberg::Result::ImagePreprocessingMetadata
    attr_reader :original_dimensions, :original_dpi, :target_dpi, :scale_factor,
                :auto_adjusted, :final_dpi, :new_dimensions, :resample_method,
                :dimension_clamped, :calculated_dpi, :skipped_resize, :resize_error
end
```

#### Java

```java
public record ImagePreprocessingMetadata(
    int[] originalDimensions,
    double[] originalDpi,
    int targetDpi,
    double scaleFactor,
    boolean autoAdjusted,
    int finalDpi,
    Optional<int[]> newDimensions,
    String resampleMethod,
    boolean dimensionClamped,
    Optional<Integer> calculatedDpi,
    boolean skippedResize,
    Optional<String> resizeError
) {}
```

#### Go

```go
type ImagePreprocessingMetadata struct {
    OriginalDimensions [2]int    `json:"original_dimensions"`
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
```

### ImagePreprocessingConfig

Image preprocessing configuration for OCR quality enhancement.

#### Rust

```rust
pub struct ImagePreprocessingConfig {
    pub target_dpi: i32,
    pub auto_rotate: bool,
    pub deskew: bool,
    pub denoise: bool,
    pub contrast_enhance: bool,
    pub binarization_method: String,
    pub invert_colors: bool,
}
```

#### Python

```python
@dataclass
class ImagePreprocessingConfig:
    target_dpi: int = 300
    auto_rotate: bool = True
    deskew: bool = True
    denoise: bool = False
    contrast_enhance: bool = False
    binarization_method: str = "otsu"
    invert_colors: bool = False
```

#### TypeScript

```typescript
export interface ImagePreprocessingConfig {
    targetDpi?: number;
    autoRotate?: boolean;
    deskew?: boolean;
    denoise?: boolean;
    contrastEnhance?: boolean;
    binarizationMethod?: string;
    invertColors?: boolean;
}
```

#### Ruby

```ruby
class Kreuzberg::Config::ImagePreprocessingConfig
    attr_accessor :target_dpi, :auto_rotate, :deskew, :denoise,
                  :contrast_enhance, :binarization_method, :invert_colors
end
```

#### Java

```java
public final class ImagePreprocessingConfig {
    private final int targetDpi;
    private final boolean autoRotate;
    private final boolean deskew;
    private final boolean denoise;
    private final boolean contrastEnhance;
    private final String binarizationMethod;
    private final boolean invertColors;

    public static Builder builder() { }
}
```

#### Go

```go
type ImagePreprocessingConfig struct {
    TargetDPI        *int   `json:"target_dpi,omitempty"`
    AutoRotate       *bool  `json:"auto_rotate,omitempty"`
    Deskew           *bool  `json:"deskew,omitempty"`
    Denoise          *bool  `json:"denoise,omitempty"`
    ContrastEnhance  *bool  `json:"contrast_enhance,omitempty"`
    BinarizationMode string `json:"binarization_method,omitempty"`
    InvertColors     *bool  `json:"invert_colors,omitempty"`
}
```

### ErrorMetadata

Error metadata for batch operation failure tracking.

#### Rust

```rust
pub struct ErrorMetadata {
    pub error_type: String,
    pub message: String,
}
```

#### Python

```python
class ErrorMetadata(TypedDict, total=False):
    error_type: str
    message: str
```

#### TypeScript

```typescript
export interface ErrorMetadata {
    errorType?: string;
    message?: string;
}
```

#### Ruby

```ruby
class Kreuzberg::Result::ErrorMetadata
    attr_reader :error_type, :message
end
```

#### Java

```java
public record ErrorMetadata(
    String errorType,
    String message
) {}
```

#### Go

```go
type ErrorMetadata struct {
    ErrorType string `json:"error_type"`
    Message   string `json:"message"`
}
```

### XmlMetadata

XML document structure metadata.

#### Rust

```rust
pub struct XmlMetadata {
    pub element_count: usize,
    pub unique_elements: Vec<String>,
}
```

#### Python

```python
class XmlMetadata(TypedDict, total=False):
    element_count: int
    unique_elements: list[str]
```

#### TypeScript

```typescript
export interface XmlMetadata {
    elementCount?: number;
    uniqueElements?: string[];
}
```

#### Ruby

```ruby
class Kreuzberg::Result::XmlMetadata
    attr_reader :element_count, :unique_elements
end
```

#### Java

```java
public record XmlMetadata(
    int elementCount,
    List<String> uniqueElements
) {}
```

#### Go

```go
type XmlMetadata struct {
    ElementCount  int      `json:"element_count"`
    UniqueElements []string `json:"unique_elements"`
}
```

### PostProcessorConfig

Post-processor configuration for selective processor execution.

#### Rust

```rust
pub struct PostProcessorConfig {
    pub enabled: bool,
    pub enabled_processors: Option<Vec<String>>,
    pub disabled_processors: Option<Vec<String>>,
}
```

#### Python

```python
@dataclass
class PostProcessorConfig:
    enabled: bool = True
    enabled_processors: list[str] | None = None
    disabled_processors: list[str] | None = None
```

#### TypeScript

```typescript
export interface PostProcessorConfig {
    enabled?: boolean;
    enabledProcessors?: string[];
    disabledProcessors?: string[];
}
```

#### Ruby

```ruby
class Kreuzberg::Config::PostProcessorConfig
    attr_accessor :enabled, :enabled_processors, :disabled_processors
end
```

#### Java

```java
public final class PostProcessorConfig {
    private final boolean enabled;
    private final List<String> enabledProcessors;
    private final List<String> disabledProcessors;

    public static Builder builder() { }
}
```

#### Go

```go
type PostProcessorConfig struct {
    Enabled            *bool    `json:"enabled,omitempty"`
    EnabledProcessors  []string `json:"enabled_processors,omitempty"`
    DisabledProcessors []string `json:"disabled_processors,omitempty"`
}
```

## Type Mappings

Cross-language type equivalents:

| Purpose | Rust | Python | TypeScript | Ruby | Java | Go |
|---------|------|--------|------------|------|------|-----|
| String | `String` | `str` | `string` | `String` | `String` | `string` |
| Optional/Nullable | `Option<T>` | `T \| None` | `T \| null` | `T or nil` | `Optional<T>` | `*T` |
| Array/List | `Vec<T>` | `list[T]` | `T[]` | `Array` | `List<T>` | `[]T` |
| Tuple/Pair | `(T, U)` | `tuple[T, U]` | `[T, U]` | `Array` | `Pair<T,U>` | `[2]T` |
| Dictionary/Map | `HashMap<K,V>` | `dict[K, V]` | `Record<K, V>` | `Hash` | `Map<K, V>` | `map[K]V` |
| Integer | `i32`, `i64`, `usize` | `int` | `number` | `Integer` | `int`, `long` | `int`, `int64` |
| Float | `f32`, `f64` | `float` | `number` | `Float` | `float`, `double` | `float32`, `float64` |
| Boolean | `bool` | `bool` | `boolean` | `Boolean` | `boolean` | `bool` |
| Bytes | `Vec<u8>` | `bytes` | `Uint8Array` | `String` (binary) | `byte[]` | `[]byte` |
| Union/Enum | `enum` | `Literal` | `union` | `case` statement | `sealed class` | custom struct |

## Nullability and Optionals

### How Each Language Handles Optional Fields

**Rust**: Uses `Option<T>` explicitly. `None` represents absence. Mandatory at compile-time.

**Python**: Uses `T | None` type hints. Can be `None` at runtime. TypedDict with `total=False` makes all fields optional.

**TypeScript**: Uses `T | null` or `T | undefined`. Properties marked `?` are optional. Nullable with `null` literal.

**Ruby**: Everything is nullable. Use `nil` for absence. No type system enforcement.

**Java**: Uses `Optional<T>` for explicit optionality. Records with `Optional` fields. Checked at compile-time for clarity.

**Go**: Uses pointers (`*T`) for optional values. `nil` represents absence. Primitive types can't be nil (use pointers).

### Example: Processing Optional Metadata Fields

```rust
// Rust: explicit checking
if let Some(title) = metadata.format.pdf.title {
    println!("Title: {}", title);
}
```

```python
# Python: straightforward None checking
if metadata.get("title"):
    print(f"Title: {metadata['title']}")
```

```typescript
// TypeScript: optional chaining
console.log(metadata.title ?? "No title");
```

```ruby
# Ruby: simple truthy check
puts "Title: #{result.metadata["title"]}" if result.metadata["title"]
```

```java
// Java: Optional methods
metadata.title()
    .ifPresent(title -> System.out.println("Title: " + title));
```

```go
// Go: pointer nil checks
if metadata.Pdf != nil && metadata.Pdf.Title != nil {
    fmt.Println("Title:", *metadata.Pdf.Title)
}
```
