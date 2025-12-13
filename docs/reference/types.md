# Type Reference

Complete type definitions and documentation for Kreuzberg across all language bindings.

## ExtractionResult

Primary extraction result containing document content, metadata, and structured data elements. All extraction operations return this unified type with format-agnostic content and format-specific metadata.

### Rust

```rust title="extraction_result.rs"
pub struct ExtractionResult {
    pub content: String,
    pub mime_type: String,
    pub metadata: Metadata,
    pub tables: Vec<Table>,
    pub detected_languages: Option<Vec<String>>,
    pub chunks: Option<Vec<Chunk>>,
    pub images: Option<Vec<ExtractedImage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<Vec<PageContent>>,
}
```

### Python

```python title="extraction_result.py"
class ExtractionResult(TypedDict):
    """Main result containing extracted content, metadata, and structured data."""
    content: str
    mime_type: str
    metadata: Metadata
    tables: list[Table]
    detected_languages: list[str] | None
    chunks: list[Chunk] | None
    images: list[ExtractedImage] | None
    pages: list[PageContent] | None
```

### TypeScript

```typescript title="extraction_result.ts"
export interface ExtractionResult {
    content: string;
    mimeType: string;
    metadata: Metadata;
    tables: Table[];
    detectedLanguages: string[] | null;
    chunks: Chunk[] | null;
    images: ExtractedImage[] | null;
    pages?: PageContent[];
}
```

### Ruby

```ruby title="extraction_result.rb"
class Kreuzberg::Result
    attr_reader :content, :mime_type, :metadata, :tables
    attr_reader :detected_languages, :chunks, :images, :pages
end
```

### Java

```java title="ExtractionResult.java"
public record ExtractionResult(
    String content,
    String mimeType,
    Metadata metadata,
    List<Table> tables,
    List<String> detectedLanguages,
    List<Chunk> chunks,
    List<ExtractedImage> images,
    List<PageContent> pages
) {}
```

### Go

```go title="extraction_result.go"
type ExtractionResult struct {
    Content           string           `json:"content"`
    MimeType          string           `json:"mime_type"`
    Metadata          Metadata         `json:"metadata"`
    Tables            []Table          `json:"tables"`
    DetectedLanguages []string         `json:"detected_languages,omitempty"`
    Chunks            []Chunk          `json:"chunks,omitempty"`
    Images            []ExtractedImage `json:"images,omitempty"`
    Pages             []PageContent    `json:"pages,omitempty"`
}
```

## Metadata

Document metadata with discriminated union pattern. The `format_type` field determines which format-specific fields are populated, enabling type-safe access to PDF, Excel, Email, and other format-specific metadata.

### Rust

```rust title="metadata.rs"
pub struct Metadata {
    pub title: Option<String>,
    pub subject: Option<String>,
    pub authors: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
    pub language: Option<String>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub created_by: Option<String>,
    pub modified_by: Option<String>,
    pub pages: Option<PageStructure>,
    pub date: Option<String>,
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

```python title="metadata.py"
class Metadata(TypedDict, total=False):
    """Document metadata with format-specific fields and processing info."""
    title: str | None
    subject: str | None
    authors: list[str] | None
    keywords: list[str] | None
    language: str | None
    created_at: str | None
    modified_at: str | None
    created_by: str | None
    modified_by: str | None
    pages: PageStructure | None
    date: str | None
    format_type: Literal["pdf", "excel", "email", "pptx", "archive", "image", "xml", "text", "html", "ocr"]
    # Format-specific fields are included at root level based on format_type
    image_preprocessing: ImagePreprocessingMetadata | None
    json_schema: dict[str, Any] | None
    error: ErrorMetadata | None
```

### TypeScript

```typescript title="metadata.ts"
export interface Metadata {
    title?: string | null;
    subject?: string | null;
    authors?: string[] | null;
    keywords?: string[] | null;
    language?: string | null;
    createdAt?: string | null;
    modifiedAt?: string | null;
    createdBy?: string | null;
    modifiedBy?: string | null;
    pages?: PageStructure | null;
    date?: string | null;
    format_type?: "pdf" | "excel" | "email" | "pptx" | "archive" | "image" | "xml" | "text" | "html" | "ocr";
    // Format-specific fields are included at root level based on format_type
    image_preprocessing?: ImagePreprocessingMetadata | null;
    json_schema?: Record<string, unknown> | null;
    error?: ErrorMetadata | null;
    [key: string]: any;
}
```

### Ruby

```ruby title="metadata.rb"
# Metadata is returned as a Hash from the native extension
result.metadata  # Hash with string keys and mixed values
# Check format_type to determine which format-specific fields are available
result.metadata["format_type"]  # "pdf", "excel", "email", etc.
```

### Java

```java title="Metadata.java"
public final class Metadata {
    private final Optional<String> title;
    private final Optional<String> subject;
    private final Optional<List<String>> authors;
    private final Optional<List<String>> keywords;
    private final Optional<String> language;
    private final Optional<String> createdAt;
    private final Optional<String> modifiedAt;
    private final Optional<String> createdBy;
    private final Optional<String> modifiedBy;
    private final Optional<PageStructure> pages;
    private final Optional<String> date;
    private final Optional<FormatMetadata> format;
    private final Optional<ImagePreprocessingMetadata> imagePreprocessing;
    private final Optional<Map<String, Object>> jsonSchema;
    private final Optional<ErrorMetadata> error;
}

public final class FormatMetadata {
    private final FormatType type;
    private final Optional<PdfMetadata> pdf;
    private final Optional<ExcelMetadata> excel;
    private final Optional<EmailMetadata> email;
    // Additional Optional fields for each supported format type
}
```

### Go

```go title="metadata.go"
type Metadata struct {
    Title              *string                     `json:"title,omitempty"`
    Subject            *string                     `json:"subject,omitempty"`
    Authors            []string                    `json:"authors,omitempty"`
    Keywords           []string                    `json:"keywords,omitempty"`
    Language           *string                     `json:"language,omitempty"`
    CreatedAt          *string                     `json:"created_at,omitempty"`
    ModifiedAt         *string                     `json:"modified_at,omitempty"`
    CreatedBy          *string                     `json:"created_by,omitempty"`
    ModifiedBy         *string                     `json:"modified_by,omitempty"`
    Pages              *PageStructure              `json:"pages,omitempty"`
    Date               *string                     `json:"date,omitempty"`
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
    // Additional pointer fields for each supported format type
}
```

### Metadata.pages Field

Contains page structure information when page tracking is available. This field provides detailed boundaries and metadata for individual pages/slides/sheets within multi-page documents.

**Type**: `Option<PageStructure>` (Rust), `PageStructure | None` (Python), `PageStructure | null` (TypeScript), `Optional<PageStructure>` (Java), `*PageStructure` (Go)

**When populated**: Only when the document format supports page tracking (PDF, PPTX, DOCX, XLSX) and extraction is successful.

**Available fields**:
- `total_count`: Total number of pages/slides/sheets in the document
- `unit_type`: Type of paginated unit ("page", "slide", or "sheet")
- `boundaries`: Byte offset boundaries for each page (enables O(1) lookups from byte positions to page numbers)
- `pages`: Detailed per-page metadata including dimensions, titles, and content counts

**Example usage**:

```rust
if let Some(page_structure) = metadata.pages {
    println!("Document has {} pages", page_structure.total_count);
    if let Some(boundaries) = page_structure.boundaries {
        for boundary in boundaries {
            println!("Page {}: bytes {} to {}", boundary.page_number, boundary.byte_start, boundary.byte_end);
        }
    }
}
```

```python
if metadata.get("pages"):
    page_structure = metadata["pages"]
    print(f"Document has {page_structure['total_count']} pages")
    if page_structure.get("boundaries"):
        for boundary in page_structure["boundaries"]:
            print(f"Page {boundary['page_number']}: bytes {boundary['byte_start']}-{boundary['byte_end']}")
```

```typescript
if (metadata.pages) {
    console.log(`Document has ${metadata.pages.totalCount} pages`);
    if (metadata.pages.boundaries) {
        for (const boundary of metadata.pages.boundaries) {
            console.log(`Page ${boundary.pageNumber}: bytes ${boundary.byteStart}-${boundary.byteEnd}`);
        }
    }
}
```

```java
metadata.pages().ifPresent(pageStructure -> {
    System.out.println("Document has " + pageStructure.getTotalCount() + " pages");
    pageStructure.getBoundaries().ifPresent(boundaries -> {
        for (PageBoundary boundary : boundaries) {
            System.out.println("Page " + boundary.pageNumber() + ": bytes " +
                boundary.byteStart() + "-" + boundary.byteEnd());
        }
    });
});
```

```go
if metadata.Pages != nil {
    fmt.Printf("Document has %d pages\n", metadata.Pages.TotalCount)
    if metadata.Pages.Boundaries != nil {
        for _, boundary := range metadata.Pages.Boundaries {
            fmt.Printf("Page %d: bytes %d-%d\n", boundary.PageNumber, boundary.ByteStart, boundary.ByteEnd)
        }
    }
}
```

## PageStructure

Unified representation of page/slide/sheet structure with byte-accurate boundaries. Tracks the logical structure of multi-page documents, enabling precise page-to-content mapping and efficient chunk-to-page lookups.

### Rust

```rust title="page_structure.rs"
pub struct PageStructure {
    pub total_count: usize,
    pub unit_type: PageUnitType,
    pub boundaries: Option<Vec<PageBoundary>>,
    pub pages: Option<Vec<PageInfo>>,
}
```

### Python

```python title="page_structure.py"
class PageStructure(TypedDict, total=False):
    total_count: int
    unit_type: str  # "page", "slide", "sheet"
    boundaries: list[PageBoundary] | None
    pages: list[PageInfo] | None
```

### TypeScript

```typescript title="page_structure.ts"
interface PageStructure {
  totalCount: number;
  unitType: "page" | "slide" | "sheet";
  boundaries?: PageBoundary[];
  pages?: PageInfo[];
}
```

### Ruby

```ruby title="page_structure.rb"
class PageStructure < Dry::Struct
  attribute :total_count, Types::Integer
  attribute :unit_type, Types::String.enum("page", "slide", "sheet")
  attribute :boundaries, Types::Array.of(PageBoundary).optional
  attribute :pages, Types::Array.of(PageInfo).optional
end
```

### Java

```java title="PageStructure.java"
public final class PageStructure {
    private final long totalCount;
    private final PageUnitType unitType;
    private final List<PageBoundary> boundaries;
    private final List<PageInfo> pages;

    public Optional<List<PageBoundary>> getBoundaries() { }
    public Optional<List<PageInfo>> getPages() { }
}
```

### Go

```go title="page_structure.go"
type PageStructure struct {
    TotalCount int              `json:"total_count"`
    UnitType   string           `json:"unit_type"`
    Boundaries []PageBoundary   `json:"boundaries,omitempty"`
    Pages      []PageInfo       `json:"pages,omitempty"`
}
```

### C#

```csharp title="PageStructure.cs"
public record PageStructure
{
    public required int TotalCount { get; init; }
    public required string UnitType { get; init; }
    public List<PageBoundary>? Boundaries { get; init; }
    public List<PageInfo>? Pages { get; init; }
}
```

**Fields:**
- `total_count`: Total number of pages/slides/sheets
- `unit_type`: Distinction between Page/Slide/Sheet
- `boundaries`: Byte offset ranges for each page (enables O(1) lookups)
- `pages`: Per-page metadata (dimensions, counts, visibility)

## PageBoundary

Byte offset range for a single page/slide/sheet. Enables O(1) page lookups and precise chunk-to-page mapping.

WARNING: Byte offsets are UTF-8 safe boundaries. Do not use them as character indices.

### Rust

```rust title="page_boundary.rs"
pub struct PageBoundary {
    pub byte_start: usize,
    pub byte_end: usize,
    pub page_number: usize,
}
```

### Python

```python title="page_boundary.py"
class PageBoundary(TypedDict):
    byte_start: int
    byte_end: int
    page_number: int
```

### TypeScript

```typescript title="page_boundary.ts"
interface PageBoundary {
  byteStart: number;
  byteEnd: number;
  pageNumber: number;
}
```

### Ruby

```ruby title="page_boundary.rb"
class PageBoundary < Dry::Struct
  attribute :byte_start, Types::Integer
  attribute :byte_end, Types::Integer
  attribute :page_number, Types::Integer
end
```

### Java

```java title="PageBoundary.java"
public record PageBoundary(
    long byteStart,
    long byteEnd,
    long pageNumber
) {}
```

### Go

```go title="page_boundary.go"
type PageBoundary struct {
    ByteStart  int `json:"byte_start"`
    ByteEnd    int `json:"byte_end"`
    PageNumber int `json:"page_number"`
}
```

### C#

```csharp title="PageBoundary.cs"
public record PageBoundary
{
    public required int ByteStart { get; init; }
    public required int ByteEnd { get; init; }
    public required int PageNumber { get; init; }
}
```

**Fields:**
- `byte_start`: UTF-8 byte offset (inclusive)
- `byte_end`: UTF-8 byte offset (exclusive)
- `page_number`: 1-indexed page number

## PageInfo

Detailed per-page metadata. Contains format-specific metadata for individual pages/slides/sheets.

### Rust

```rust title="page_info.rs"
pub struct PageInfo {
    pub number: usize,
    pub title: Option<String>,
    pub dimensions: Option<(f64, f64)>,
    pub image_count: Option<usize>,
    pub table_count: Option<usize>,
    pub hidden: Option<bool>,
}
```

### Python

```python title="page_info.py"
class PageInfo(TypedDict, total=False):
    number: int
    title: str | None
    dimensions: tuple[float, float] | None
    image_count: int | None
    table_count: int | None
    hidden: bool | None
```

### TypeScript

```typescript title="page_info.ts"
interface PageInfo {
  number: number;
  title?: string;
  dimensions?: [number, number];
  imageCount?: number;
  tableCount?: number;
  hidden?: boolean;
}
```

### Ruby

```ruby title="page_info.rb"
class PageInfo < Dry::Struct
  attribute :number, Types::Integer
  attribute :title, Types::String.optional
  attribute :dimensions, Types::Array.of(Types::Float).optional
  attribute :image_count, Types::Integer.optional
  attribute :table_count, Types::Integer.optional
  attribute :hidden, Types::Bool.optional
end
```

### Java

```java title="PageInfo.java"
public record PageInfo(
    int number,
    Optional<String> title,
    Optional<double[]> dimensions,
    Optional<Integer> imageCount,
    Optional<Integer> tableCount,
    Optional<Boolean> hidden
) {}
```

### Go

```go title="page_info.go"
type PageInfo struct {
    Number      int       `json:"number"`
    Title       *string   `json:"title,omitempty"`
    Dimensions  []float64 `json:"dimensions,omitempty"`
    ImageCount  *int      `json:"image_count,omitempty"`
    TableCount  *int      `json:"table_count,omitempty"`
    Hidden      *bool     `json:"hidden,omitempty"`
}
```

### C#

```csharp title="PageInfo.cs"
public record PageInfo
{
    public required int Number { get; init; }
    public string? Title { get; init; }
    public (double Width, double Height)? Dimensions { get; init; }
    public int? ImageCount { get; init; }
    public int? TableCount { get; init; }
    public bool? Hidden { get; init; }
}
```

**Fields:**
- `number`: 1-indexed page number
- `title`: Page/slide title (PPTX)
- `dimensions`: Width and height in points (PDF, PPTX)
- `image_count`: Number of images on page
- `table_count`: Number of tables on page
- `hidden`: Whether page/slide is hidden (PPTX)

## PageUnitType

Enum distinguishing page types across document formats.

### Rust

```rust title="page_unit_type.rs"
pub enum PageUnitType {
    Page,
    Slide,
    Sheet,
}
```

### Python

```python title="page_unit_type.py"
# String literal type
PageUnitType = Literal["page", "slide", "sheet"]
```

### TypeScript

```typescript title="page_unit_type.ts"
type PageUnitType = "page" | "slide" | "sheet";
```

### Ruby

```ruby title="page_unit_type.rb"
module PageUnitType
  PAGE = "page"
  SLIDE = "slide"
  SHEET = "sheet"
end
```

### Java

```java title="PageUnitType.java"
public enum PageUnitType {
    PAGE,
    SLIDE,
    SHEET
}
```

### Go

```go title="page_unit_type.go"
type PageUnitType string

const (
    PageUnitTypePage  PageUnitType = "page"
    PageUnitTypeSlide PageUnitType = "slide"
    PageUnitTypeSheet PageUnitType = "sheet"
)
```

### C#

```csharp title="PageUnitType.cs"
public enum PageUnitType
{
    Page,
    Slide,
    Sheet
}
```

**Values:**
- `Page`: Standard document pages (PDF, DOCX)
- `Slide`: Presentation slides (PPTX)
- `Sheet`: Spreadsheet sheets (XLSX)

## Format-Specific Metadata

### PDF Metadata

Document properties extracted from PDF files including title, author, creation dates, and page count. Available when `format_type == "pdf"`.

#### Rust

```rust title="pdf_metadata.rs"
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

```python title="pdf_metadata.py"
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

```typescript title="pdf_metadata.ts"
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

```java title="PdfMetadata.java"
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

```go title="pdf_metadata.go"
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

Spreadsheet workbook information including sheet count and sheet names. Available when `format_type == "excel"`.

#### Rust

```rust title="excel_metadata.rs"
pub struct ExcelMetadata {
    pub sheet_count: usize,
    pub sheet_names: Vec<String>,
}
```

#### Python

```python title="excel_metadata.py"
class ExcelMetadata(TypedDict, total=False):
    sheet_count: int
    sheet_names: list[str]
```

#### TypeScript

```typescript title="excel_metadata.ts"
export interface ExcelMetadata {
    sheetCount?: number;
    sheetNames?: string[];
}
```

#### Java

```java title="ExcelMetadata.java"
public record ExcelMetadata(
    int sheetCount,
    List<String> sheetNames
) {}
```

#### Go

```go title="excel_metadata.go"
type ExcelMetadata struct {
    SheetCount int      `json:"sheet_count"`
    SheetNames []string `json:"sheet_names"`
}
```

### Email Metadata

Email message headers and recipient information including sender, recipients, message ID, and attachment lists. Available when `format_type == "email"`.

#### Rust

```rust title="email_metadata.rs"
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

```python title="email_metadata.py"
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

```typescript title="email_metadata.ts"
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

```java title="EmailMetadata.java"
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

```go title="email_metadata.go"
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

Archive file properties including format type, file count, file list, and compression information. Available when `format_type == "archive"`.

#### Rust

```rust title="archive_metadata.rs"
pub struct ArchiveMetadata {
    pub format: String,
    pub file_count: usize,
    pub file_list: Vec<String>,
    pub total_size: usize,
    pub compressed_size: Option<usize>,
}
```

#### Python

```python title="archive_metadata.py"
class ArchiveMetadata(TypedDict, total=False):
    format: str
    file_count: int
    file_list: list[str]
    total_size: int
    compressed_size: int | None
```

#### TypeScript

```typescript title="archive_metadata.ts"
export interface ArchiveMetadata {
    format?: string;
    fileCount?: number;
    fileList?: string[];
    totalSize?: number;
    compressedSize?: number | null;
}
```

#### Java

```java title="ArchiveMetadata.java"
public record ArchiveMetadata(
    String format,
    int fileCount,
    List<String> fileList,
    int totalSize,
    Optional<Integer> compressedSize
) {}
```

#### Go

```go title="archive_metadata.go"
type ArchiveMetadata struct {
    Format         string   `json:"format"`
    FileCount      int      `json:"file_count"`
    FileList       []string `json:"file_list"`
    TotalSize      int      `json:"total_size"`
    CompressedSize *int     `json:"compressed_size,omitempty"`
}
```

### Image Metadata

Image properties including dimensions, format type, and EXIF metadata extracted from image files. Available when `format_type == "image"`.

#### Rust

```rust title="image_metadata.rs"
pub struct ImageMetadata {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub exif: HashMap<String, String>,
}
```

#### Python

```python title="image_metadata.py"
class ImageMetadata(TypedDict, total=False):
    width: int
    height: int
    format: str
    exif: dict[str, str]
```

#### TypeScript

```typescript title="image_metadata.ts"
export interface ImageMetadata {
    width?: number;
    height?: number;
    format?: string;
    exif?: Record<string, string>;
}
```

#### Java

```java title="ImageMetadata.java"
public record ImageMetadata(
    int width,
    int height,
    String format,
    Map<String, String> exif
) {}
```

#### Go

```go title="image_metadata.go"
type ImageMetadata struct {
    Width  uint32            `json:"width"`
    Height uint32            `json:"height"`
    Format string            `json:"format"`
    EXIF   map[string]string `json:"exif"`
}
```

### HTML Metadata

Web page metadata including title, description, Open Graph tags, Twitter Card properties, and link relationships. Available when `format_type == "html"`.

#### Rust

```rust title="html_metadata.rs"
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

```python title="html_metadata.py"
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

```typescript title="html_metadata.ts"
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

```java title="HtmlMetadata.java"
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

```go title="html_metadata.go"
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

Text document statistics and structure including line/word/character counts, headers, links, and code blocks. Available when `format_type == "text"`.

#### Rust

```rust title="text_metadata.rs"
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

```python title="text_metadata.py"
class TextMetadata(TypedDict, total=False):
    line_count: int
    word_count: int
    character_count: int
    headers: list[str] | None
    links: list[tuple[str, str]] | None
    code_blocks: list[tuple[str, str]] | None
```

#### TypeScript

```typescript title="text_metadata.ts"
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

```java title="TextMetadata.java"
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

```go title="text_metadata.go"
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

Presentation metadata including title, author, description, summary, and font information. Available when `format_type == "pptx"`.

#### Rust

```rust title="pptx_metadata.rs"
pub struct PptxMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub summary: Option<String>,
    pub fonts: Vec<String>,
}
```

#### Python

```python title="pptx_metadata.py"
class PptxMetadata(TypedDict, total=False):
    title: str | None
    author: str | None
    description: str | None
    summary: str | None
    fonts: list[str]
```

#### TypeScript

```typescript title="pptx_metadata.ts"
export interface PptxMetadata {
    title?: string | null;
    author?: string | null;
    description?: string | null;
    summary?: string | null;
    fonts?: string[];
}
```

#### Java

```java title="PptxMetadata.java"
public record PptxMetadata(
    Optional<String> title,
    Optional<String> author,
    Optional<String> description,
    Optional<String> summary,
    List<String> fonts
) {}
```

#### Go

```go title="pptx_metadata.go"
type PptxMetadata struct {
    Title       *string  `json:"title,omitempty"`
    Author      *string  `json:"author,omitempty"`
    Description *string  `json:"description,omitempty"`
    Summary     *string  `json:"summary,omitempty"`
    Fonts       []string `json:"fonts"`
}
```

### OCR Metadata

Optical Character Recognition processing metadata including language, page segmentation mode, output format, and table detection results. Available when `format_type == "ocr"`.

#### Rust

```rust title="ocr_metadata.rs"
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

```python title="ocr_metadata.py"
class OcrMetadata(TypedDict, total=False):
    language: str
    psm: int
    output_format: str
    table_count: int
    table_rows: int | None
    table_cols: int | None
```

#### TypeScript

```typescript title="ocr_metadata.ts"
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

```java title="OcrMetadata.java"
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

```go title="ocr_metadata.go"
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

Structured table data extracted from documents with cell contents in 2D array format, markdown representation, and source page number.

### Rust

```rust title="table.rs"
pub struct Table {
    pub cells: Vec<Vec<String>>,
    pub markdown: String,
    pub page_number: usize,
}
```

### Python

```python title="table.py"
class Table(TypedDict):
    cells: list[list[str]]
    markdown: str
    page_number: int
```

### TypeScript

```typescript title="table.ts"
export interface Table {
    cells: string[][];
    markdown: string;
    pageNumber: number;
}
```

### Ruby

```ruby title="table.rb"
Kreuzberg::Result::Table = Struct.new(:cells, :markdown, :page_number, keyword_init: true)
```

### Java

```java title="Table.java"
public record Table(
    List<List<String>> cells,
    String markdown,
    int pageNumber
) {}
```

### Go

```go title="table.go"
type Table struct {
    Cells      [][]string `json:"cells"`
    Markdown   string     `json:"markdown"`
    PageNumber int        `json:"page_number"`
}
```

## Chunk

Text chunk for RAG and vector search applications, containing content segment, optional embedding vector, and position metadata for precise document referencing.

### Rust

```rust title="chunk.rs"
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

```python title="chunk.py"
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

```typescript title="chunk.ts"
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

```ruby title="chunk.rb"
Kreuzberg::Result::Chunk = Struct.new(
    :content, :char_start, :char_end, :token_count,
    :chunk_index, :total_chunks, :embedding,
    keyword_init: true
)
```

### Java

```java title="Chunk.java"
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

```go title="chunk.go"
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

Binary image data extracted from documents with format metadata, dimensions, colorspace information, and optional nested OCR extraction results.

### Rust

```rust title="extracted_image.rs"
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

```python title="extracted_image.py"
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

```typescript title="extracted_image.ts"
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

```ruby title="extracted_image.rb"
Kreuzberg::Result::Image = Struct.new(
    :data, :format, :image_index, :page_number, :width, :height,
    :colorspace, :bits_per_component, :is_mask, :description, :ocr_result,
    keyword_init: true
)
```

### Java

```java title="ExtractedImage.java"
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

```go title="extracted_image.go"
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

Comprehensive extraction pipeline configuration controlling OCR, chunking, image processing, language detection, and all processing features.

#### Rust

```rust title="extraction_config.rs"
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

```python title="extraction_config.py"
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

```typescript title="extraction_config.ts"
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

```java title="ExtractionConfig.java"
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

```go title="extraction_config.go"
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

OCR engine selection and language configuration for Tesseract, EasyOCR, and PaddleOCR backends.

#### Rust

```rust title="ocr_config.rs"
pub struct OcrConfig {
    pub backend: String,  // "tesseract", "easyocr", "paddleocr"
    pub language: String, // e.g., "eng", "deu", "fra"
    pub tesseract_config: Option<TesseractConfig>,
}
```

#### Python

```python title="ocr_config.py"
@dataclass
class OcrConfig:
    backend: str = "tesseract"
    language: str = "eng"
    tesseract_config: TesseractConfig | None = None
```

#### TypeScript

```typescript title="ocr_config.ts"
export interface OcrConfig {
    backend: string;
    language?: string;
    tesseractConfig?: TesseractConfig;
}
```

#### Java

```java title="OcrConfig.java"
public record OcrConfig(
    String backend,
    String language,
    Optional<TesseractConfig> tesseractConfig
) {}
```

#### Go

```go title="ocr_config.go"
type OcrConfig struct {
    Backend            string
    Language           string
    TesseractConfig    *TesseractConfig
}
```

### TesseractConfig

Advanced Tesseract OCR engine parameters including page segmentation mode, preprocessing, table detection, and character whitelisting/blacklisting.

#### Rust

```rust title="tesseract_config.rs"
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

Text chunking configuration for RAG pipelines with character limits, overlap control, and optional embedding generation.

#### Rust

```rust title="chunking_config.rs"
pub struct ChunkingConfig {
    pub max_chars: usize,
    pub max_overlap: usize,
    pub embedding: Option<EmbeddingConfig>,
    pub preset: Option<String>,
}
```

#### Python

```python title="chunking_config.py"
@dataclass
class ChunkingConfig:
    max_chars: int = 1000
    max_overlap: int = 200
    embedding: EmbeddingConfig | None = None
    preset: str | None = None
```

#### TypeScript

```typescript title="chunking_config.ts"
export interface ChunkingConfig {
    maxChars?: number;
    maxOverlap?: number;
    embedding?: EmbeddingConfig;
    preset?: string;
}
```

#### Java

```java title="ChunkingConfig.java"
public record ChunkingConfig(
    int maxChars,
    int maxOverlap,
    Optional<EmbeddingConfig> embedding,
    Optional<String> preset
) {}
```

#### Go

```go title="chunking_config.go"
type ChunkingConfig struct {
    MaxChars   int
    MaxOverlap int
    Embedding  *EmbeddingConfig
    Preset     *string
}
```

### EmbeddingConfig

Vector embedding configuration supporting FastEmbed models with normalization, batch processing, and custom model selection.

#### Rust

```rust title="embedding_config.rs"
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

```python title="embedding_config.py"
@dataclass
class EmbeddingConfig:
    model: EmbeddingModelType = field(default_factory=lambda: Preset("balanced"))
    normalize: bool = True
    batch_size: int = 32
    show_download_progress: bool = False
    cache_dir: Path | None = None

@dataclass
class EmbeddingModelType:
    # Discriminated union: Preset, FastEmbed, or Custom model types
    pass
```

#### TypeScript

```typescript title="embedding_config.ts"
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

Image extraction and preprocessing settings including DPI targeting, dimension limits, and automatic DPI adjustment for OCR quality.

#### Rust

```rust title="image_extraction_config.rs"
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

```python title="image_extraction_config.py"
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

```typescript title="image_extraction_config.ts"
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

```java title="ImageExtractionConfig.java"
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

```go title="image_extraction_config.go"
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

PDF-specific extraction options including image extraction control, password support for encrypted PDFs, and metadata extraction flags.

#### Rust

```rust title="pdf_config.rs"
pub struct PdfConfig {
    pub extract_images: bool,
    pub passwords: Option<Vec<String>>,
    pub extract_metadata: bool,
}
```

#### Python

```python title="pdf_config.py"
@dataclass
class PdfConfig:
    extract_images: bool = False
    passwords: list[str] | None = None
    extract_metadata: bool = True
```

#### TypeScript

```typescript title="pdf_config.ts"
export interface PdfConfig {
    extractImages?: boolean;
    passwords?: string[];
    extractMetadata?: boolean;
}
```

#### Ruby

```ruby title="pdf_config.rb"
class Kreuzberg::Config::PdfConfig
    attr_accessor :extract_images, :passwords, :extract_metadata
end
```

#### Java

```java title="PdfConfig.java"
public final class PdfConfig {
    private final boolean extractImages;
    private final List<String> passwords;
    private final boolean extractMetadata;

    public static Builder builder() { }
}
```

#### Go

```go title="pdf_config.go"
type PdfConfig struct {
    ExtractImages   *bool    `json:"extract_images,omitempty"`
    Passwords       []string `json:"passwords,omitempty"`
    ExtractMetadata *bool    `json:"extract_metadata,omitempty"`
}
```

### TokenReductionConfig

Token reduction settings for output optimization while preserving semantically important words and phrases.

#### Rust

```rust title="token_reduction_config.rs"
pub struct TokenReductionConfig {
    pub mode: String,
    pub preserve_important_words: bool,
}
```

#### Python

```python title="token_reduction_config.py"
@dataclass
class TokenReductionConfig:
    mode: str = "off"
    preserve_important_words: bool = True
```

#### TypeScript

```typescript title="token_reduction_config.ts"
export interface TokenReductionConfig {
    mode?: string;
    preserveImportantWords?: boolean;
}
```

#### Ruby

```ruby title="token_reduction_config.rb"
class Kreuzberg::Config::TokenReductionConfig
    attr_accessor :mode, :preserve_important_words
end
```

#### Java

```java title="TokenReductionConfig.java"
public final class TokenReductionConfig {
    private final String mode;
    private final boolean preserveImportantWords;

    public static Builder builder() { }
}
```

#### Go

```go title="token_reduction_config.go"
type TokenReductionConfig struct {
    Mode                   string `json:"mode,omitempty"`
    PreserveImportantWords *bool  `json:"preserve_important_words,omitempty"`
}
```

### LanguageDetectionConfig

Automatic language identification configuration with confidence thresholds and multi-language detection support.

#### Rust

```rust title="language_detection_config.rs"
pub struct LanguageDetectionConfig {
    pub enabled: bool,
    pub min_confidence: f64,
    pub detect_multiple: bool,
}
```

#### Python

```python title="language_detection_config.py"
@dataclass
class LanguageDetectionConfig:
    enabled: bool = True
    min_confidence: float = 0.8
    detect_multiple: bool = False
```

#### TypeScript

```typescript title="language_detection_config.ts"
export interface LanguageDetectionConfig {
    enabled?: boolean;
    minConfidence?: number;
    detectMultiple?: boolean;
}
```

#### Ruby

```ruby title="language_detection_config.rb"
class Kreuzberg::Config::LanguageDetectionConfig
    attr_accessor :enabled, :min_confidence, :detect_multiple
end
```

#### Java

```java title="LanguageDetectionConfig.java"
public final class LanguageDetectionConfig {
    private final boolean enabled;
    private final double minConfidence;
    private final boolean detectMultiple;

    public static Builder builder() { }
}
```

#### Go

```go title="language_detection_config.go"
type LanguageDetectionConfig struct {
    Enabled        *bool    `json:"enabled,omitempty"`
    MinConfidence  *float64 `json:"min_confidence,omitempty"`
    DetectMultiple *bool    `json:"detect_multiple,omitempty"`
}
```

### KeywordConfig

Automatic keyword and keyphrase extraction using YAKE or RAKE algorithms with configurable scoring, n-gram ranges, and language support.

#### Rust

```rust title="keyword_config.rs"
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

```python title="keyword_config.py"
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

```typescript title="keyword_config.ts"
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

```ruby title="keyword_config.rb"
class Kreuzberg::Config::KeywordConfig
    attr_accessor :algorithm, :max_keywords, :min_score,
                  :ngram_range, :language, :yake_params, :rake_params
end
```

#### Java

```java title="KeywordConfig.java"
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

```go title="keyword_config.go"
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

Image preprocessing transformation log tracking original and final DPI, scaling factors, dimension changes, and any processing errors.

#### Rust

```rust title="image_preprocessing_metadata.rs"
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

```python title="image_preprocessing_metadata.py"
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

```typescript title="image_preprocessing_metadata.ts"
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

```ruby title="image_preprocessing_metadata.rb"
class Kreuzberg::Result::ImagePreprocessingMetadata
    attr_reader :original_dimensions, :original_dpi, :target_dpi, :scale_factor,
                :auto_adjusted, :final_dpi, :new_dimensions, :resample_method,
                :dimension_clamped, :calculated_dpi, :skipped_resize, :resize_error
end
```

#### Java

```java title="ImagePreprocessingMetadata.java"
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

```go title="image_preprocessing_metadata.go"
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

Image preprocessing configuration for OCR quality enhancement including rotation, deskewing, denoising, contrast adjustment, and binarization methods.

#### Rust

```rust title="image_preprocessing_config.rs"
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

```python title="image_preprocessing_config.py"
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

```typescript title="image_preprocessing_config.ts"
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

```ruby title="image_preprocessing_config.rb"
class Kreuzberg::Config::ImagePreprocessingConfig
    attr_accessor :target_dpi, :auto_rotate, :deskew, :denoise,
                  :contrast_enhance, :binarization_method, :invert_colors
end
```

#### Java

```java title="ImagePreprocessingConfig.java"
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

```go title="image_preprocessing_config.go"
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

Error information captured during batch operations providing error type classification and detailed error messages.

#### Rust

```rust title="error_metadata.rs"
pub struct ErrorMetadata {
    pub error_type: String,
    pub message: String,
}
```

#### Python

```python title="error_metadata.py"
class ErrorMetadata(TypedDict, total=False):
    error_type: str
    message: str
```

#### TypeScript

```typescript title="error_metadata.ts"
export interface ErrorMetadata {
    errorType?: string;
    message?: string;
}
```

#### Ruby

```ruby title="error_metadata.rb"
class Kreuzberg::Result::ErrorMetadata
    attr_reader :error_type, :message
end
```

#### Java

```java title="ErrorMetadata.java"
public record ErrorMetadata(
    String errorType,
    String message
) {}
```

#### Go

```go title="error_metadata.go"
type ErrorMetadata struct {
    ErrorType string `json:"error_type"`
    Message   string `json:"message"`
}
```

### XmlMetadata

XML document structure statistics including total element count and unique element type inventory.

#### Rust

```rust title="xml_metadata.rs"
pub struct XmlMetadata {
    pub element_count: usize,
    pub unique_elements: Vec<String>,
}
```

#### Python

```python title="xml_metadata.py"
class XmlMetadata(TypedDict, total=False):
    element_count: int
    unique_elements: list[str]
```

#### TypeScript

```typescript title="xml_metadata.ts"
export interface XmlMetadata {
    elementCount?: number;
    uniqueElements?: string[];
}
```

#### Ruby

```ruby title="xml_metadata.rb"
class Kreuzberg::Result::XmlMetadata
    attr_reader :element_count, :unique_elements
end
```

#### Java

```java title="XmlMetadata.java"
public record XmlMetadata(
    int elementCount,
    List<String> uniqueElements
) {}
```

#### Go

```go title="xml_metadata.go"
type XmlMetadata struct {
    ElementCount  int      `json:"element_count"`
    UniqueElements []string `json:"unique_elements"`
}
```

### PostProcessorConfig

Post-processing pipeline control allowing selective enabling or disabling of individual text processors.

#### Rust

```rust title="post_processor_config.rs"
pub struct PostProcessorConfig {
    pub enabled: bool,
    pub enabled_processors: Option<Vec<String>>,
    pub disabled_processors: Option<Vec<String>>,
}
```

#### Python

```python title="post_processor_config.py"
@dataclass
class PostProcessorConfig:
    enabled: bool = True
    enabled_processors: list[str] | None = None
    disabled_processors: list[str] | None = None
```

#### TypeScript

```typescript title="post_processor_config.ts"
export interface PostProcessorConfig {
    enabled?: boolean;
    enabledProcessors?: string[];
    disabledProcessors?: string[];
}
```

#### Ruby

```ruby title="post_processor_config.rb"
class Kreuzberg::Config::PostProcessorConfig
    attr_accessor :enabled, :enabled_processors, :disabled_processors
end
```

#### Java

```java title="PostProcessorConfig.java"
public final class PostProcessorConfig {
    private final boolean enabled;
    private final List<String> enabledProcessors;
    private final List<String> disabledProcessors;

    public static Builder builder() { }
}
```

#### Go

```go title="post_processor_config.go"
type PostProcessorConfig struct {
    Enabled            *bool    `json:"enabled,omitempty"`
    EnabledProcessors  []string `json:"enabled_processors,omitempty"`
    DisabledProcessors []string `json:"disabled_processors,omitempty"`
}
```

## Type Mappings

Cross-language type equivalents showing how Kreuzberg types map across Rust, Python, TypeScript, Ruby, Java, and Go:

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

### Language-Specific Optional Field Handling

Each language binding uses its idiomatic approach for representing optional and nullable values:

**Rust**: Uses `Option<T>` explicitly. `None` represents absence. Mandatory at compile-time.

**Python**: Uses `T | None` type hints. Can be `None` at runtime. TypedDict with `total=False` makes all fields optional.

**TypeScript**: Uses `T | null` or `T | undefined`. Properties marked `?` are optional. Nullable with `null` literal.

**Ruby**: Everything is nullable. Use `nil` for absence. No type system enforcement.

**Java**: Uses `Optional<T>` for explicit optionality. Records with `Optional` fields. Checked at compile-time for clarity.

**Go**: Uses pointers (`*T`) for optional values. `nil` represents absence. Primitive types can't be nil (use pointers).

### Practical Examples: Accessing Optional Metadata Fields

Demonstrating idiomatic null-safe field access patterns across all supported languages:

```rust title="optional_field_access.rs"
// Rust: Pattern matching for safe optional field access
if let Some(title) = metadata.format.pdf.title {
    println!("Title: {}", title);
}
```

```python title="optional_field_access.py"
# Python: Dictionary-based metadata access with safe get method
if metadata.get("title"):
    print(f"Title: {metadata['title']}")
```

```typescript title="optional_field_access.ts"
// TypeScript: Nullish coalescing for default values
console.log(metadata.title ?? "No title");
```

```ruby title="optional_field_access.rb"
# Ruby: Conditional output with truthy check
puts "Title: #{result.metadata["title"]}" if result.metadata["title"]
```

```java title="OptionalFieldAccess.java"
// Java: Functional-style Optional handling with ifPresent
metadata.title()
    .ifPresent(title -> System.out.println("Title: " + title));
```

```go title="optional_field_access.go"
// Go: Nil-safe pointer dereferencing with nested checks
if metadata.Pdf != nil && metadata.Pdf.Title != nil {
    fmt.Println("Title:", *metadata.Pdf.Title)
}
```
