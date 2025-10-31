# Text Processing Features

Kreuzberg provides advanced text processing capabilities to enhance extracted text quality, extract meaningful keywords, and reduce token counts for LLM processing.

## Overview

The text processing suite includes:

- **Encoding & Mojibake Handling**: Automatic encoding detection, safe decoding, and corruption fixing
- **Quality Processing**: Clean OCR artifacts, remove script/CSS content, and score text quality
- **Keyword Extraction**: Extract meaningful keywords using YAKE or RAKE algorithms
- **Token Reduction**: Reduce token count while preserving meaning and structure
- **Stopwords**: Language-specific stopword collections for text analysis

## Encoding & Mojibake Handling

Kreuzberg automatically handles text encoding detection and fixes corrupted text to ensure clean, properly decoded output.

### What It Does

- **Automatic Encoding Detection**: Detects text encoding from byte data (40+ encodings supported)
- **Mojibake Fixing**: Removes control characters, replacement chars, and other corruption artifacts
- **Smart Fallback**: Tests alternative encodings when detection is uncertain
- **Quality Validation**: Scores decoded text quality to detect potential issues

### How It Works

Encoding handling happens automatically during text extraction:

1. **Detect**: Analyze byte patterns to identify likely encoding
2. **Decode**: Convert bytes to UTF-8 using detected encoding
3. **Validate**: Check for decoding errors or low-quality output
4. **Fallback**: Try alternative encodings if validation fails
5. **Clean**: Remove control characters and mojibake artifacts

### Common Issues Handled

**Control Characters**: Non-printable characters from binary data or corrupted files

**Replacement Characters**: � characters from failed UTF-8 decoding

**Mixed Encodings**: Documents containing multiple character encodings

**Legacy Encodings**: Old Windows (CP1252) or DOS files

### Automatic Application

Encoding detection runs automatically for:

- Plain text files
- Email bodies
- HTML/XML without charset declaration
- Legacy document formats

No configuration required - everything is handled transparently.

## Quality Processing

Quality processing improves extracted text by removing artifacts, cleaning formatting issues, and calculating quality scores.

### Features

- **OCR Artifact Removal**: Scattered characters, repeated punctuation, malformed words
- **Script/CSS Removal**: JavaScript functions, CSS rules, script tags, style tags
- **Navigation Cleanup**: Breadcrumbs, pagination, "Skip to content" links
- **Quality Scoring**: Calculate quality score (0.0-1.0) based on structure and content
- **Whitespace Normalization**: Consistent spacing and line breaks
- **Table Preservation**: Keeps markdown table formatting intact

### Configuration

Quality processing is enabled by default. You can control it via the `enable_quality_processing` flag:

=== "Python"

    ```python
    from kreuzberg import extract_file_sync, ExtractionConfig

    # Enable quality processing (default)
    config = ExtractionConfig(enable_quality_processing=True)
    result = extract_file_sync("document.pdf", config=config)

    # Access quality score if available
    if hasattr(result, 'quality_score'):
        print(f"Quality score: {result.quality_score}")

    print(result.content)  # Cleaned text
    ```

=== "TypeScript"

    ```typescript
    import { extractFileSync, ExtractionConfig } from '@goldziher/kreuzberg';

    // Enable quality processing (default)
    const config: ExtractionConfig = {
      enableQualityProcessing: true
    };

    const result = extractFileSync('document.pdf', { config });

    // Access quality score if available
    if ('qualityScore' in result) {
      console.log(`Quality score: ${result.qualityScore}`);
    }

    console.log(result.content);  // Cleaned text
    ```

=== "Rust"

    ```rust
    use kreuzberg::{extract_file_sync, ExtractionConfig};

    fn main() -> kreuzberg::Result<()> {
        // Enable quality processing (default)
        let config = ExtractionConfig {
            enable_quality_processing: true,
            ..Default::default()
        };

        let result = extract_file_sync("document.pdf", None, &config)?;

        // Quality score is calculated during processing
        println!("{}", result.content);  // Cleaned text
        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Enable quality processing (default)
    config = Kreuzberg::Config::Extraction.new(
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)

    # Access quality score if available
    puts "Quality score: #{result.quality_score}" if result.quality_score

    puts result.content  # Cleaned text
    ```

=== "CLI"

    ```bash
    # Quality processing is enabled by default
    kreuzberg extract document.pdf

    # Disable quality processing
    kreuzberg extract document.pdf --no-quality
    ```

### Quality Score Components

The quality score (0.0-1.0) is calculated based on:

| Component | Weight | Description |
|-----------|--------|-------------|
| OCR Penalty | 0.3 | Penalties for artifacts (scattered chars, excessive spaces, malformed words) |
| Script Penalty | 0.2 | Penalties for JavaScript/CSS content |
| Navigation Penalty | 0.1 | Penalties for navigation elements and breadcrumbs |
| Structure Bonus | 0.2 | Bonus for good structure (proper sentences, paragraphs) |
| Metadata Bonus | 0.1 | Bonus for rich metadata (title, author, keywords, etc.) |

### Direct API Usage

You can also use quality functions directly in Rust:

=== "Rust"

    ```rust
    use kreuzberg::text::quality::{calculate_quality_score, clean_extracted_text, normalize_spaces};
    use std::collections::HashMap;

    fn main() {
        let text = "Some  text   with  artifacts  and <script>alert('test');</script>";

        // Calculate quality score
        let mut metadata = HashMap::new();
        metadata.insert("title".to_string(), "Document Title".to_string());
        let score = calculate_quality_score(text, Some(&metadata));
        println!("Quality score: {:.3}", score);

        // Clean extracted text
        let cleaned = clean_extracted_text(text);
        println!("Cleaned: {}", cleaned);

        // Normalize spaces
        let normalized = normalize_spaces(text);
        println!("Normalized: {}", normalized);
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Quality scoring is performed automatically during extraction
    # Access quality score from extraction result
    config = Kreuzberg::Config::Extraction.new(
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts "Quality score: #{result.quality_score}"
    ```

### OCR Artifact Cleaning

Quality processing detects and cleans various OCR artifacts that commonly appear in extracted text:

#### Scattered Characters

**Pattern**: Characters separated by excessive whitespace (e.g., `a  b  c`)

**Detection**: Looks for patterns like `[letter] [2+ spaces] [letter] [2+ spaces] [letter]`

**Cleaning**: Removes excess whitespace, joining scattered characters

**Example**:
```
Before: "T h i s   i s   s c a t t e r e d"
After:  "This is scattered"
```

#### Repeated Punctuation

**Pattern**: Three or more consecutive dots, underscores, or dashes

**Detection**: Matches `...`, `____`, `---`, etc. (3+ repetitions)

**Cleaning**: Replaces with ellipsis (`...`) for dots/underscores, preserves markdown table separators

**Example**:
```
Before: "Loading....... please wait________"
After:  "Loading... please wait..."
```

**Table Preservation**:
```
Preserved: "| Column 1 | Column 2 |"
           "|----------|----------|"
           "| Data     | Value    |"
```

#### Isolated Punctuation

**Pattern**: Punctuation marks surrounded by spaces (e.g., ` . ` or ` , `)

**Detection**: Matches ` [punctuation] ` with whitespace on both sides

**Cleaning**: Removes isolated punctuation marks

**Example**:
```
Before: "This is wrong . It has spaces , everywhere ;"
After:  "This is wrong It has spaces everywhere"
```

#### Malformed Words

**Pattern**: Words mixing letters and numbers incorrectly (e.g., `word1s`, `te2xt`)

**Detection**: Matches `[letters]+[numbers]+[letters]+` patterns

**Cleaning**: Removes malformed words (likely OCR errors)

**Example**:
```
Before: "The qu1ck br0wn f0x jumps"
After:  "The jumps"  # Malformed words removed
```

#### Excessive Whitespace

**Pattern**: Three or more consecutive spaces or tabs

**Detection**: Matches `\s{3,}` (3+ whitespace characters)

**Cleaning**: Normalizes to single space

**Example**:
```
Before: "Text    with     excessive      spacing"
After:  "Text with excessive spacing"
```

### Script and CSS Removal

Quality processing removes embedded JavaScript and CSS content:

#### JavaScript Removal

**Patterns Removed**:
- `<script>` tags and their contents
- JavaScript function definitions: `function name() { ... }`

**Example**:
```
Before: "Content <script>alert('test');</script> More content"
After:  "Content More content"

Before: "Text function myFunc() { return 42; } More text"
After:  "Text More text"
```

#### CSS Removal

**Patterns Removed**:
- `<style>` tags and their contents
- CSS rule definitions: `.class { ... }`

**Example**:
```
Before: "Content <style>.red { color: red; }</style> More"
After:  "Content More"

Before: "Text .button { padding: 10px; } More text"
After:  "Text More text"
```

### Navigation Element Cleanup

Quality processing removes common navigation and UI elements:

#### Navigation Patterns Removed

| Pattern | Example |
|---------|---------|
| Skip links | "Skip to main content" |
| Back links | "Back to top" |
| Navigation labels | "Main navigation", "Site navigation" |
| Breadcrumbs | "Home > Category > Page" |
| Pagination | "Page 1 of 10", "Previous page", "Next page" |

**Example**:
```
Before: "Skip to main content Home > Blog > Article Content here Back to top"
After:  "Content here"
```

### Quality Score Calculation

The quality score is calculated in multiple steps, combining penalties and bonuses:

#### Step 1: OCR Penalty Calculation

```
OCR Penalty = (Total Artifact Characters / Total Characters)
```

**Artifact Characters Include**:
- Scattered character patterns (characters with excessive spacing)
- Repeated punctuation (3+ dots, underscores, dashes)
- Isolated punctuation (punctuation surrounded by spaces)
- Malformed words (mixed letters/numbers)
- Excessive whitespace (3+ consecutive spaces)

**Example**:
```rust
Text: "T h i s   i s   bad........ text___"
Total characters: 39
Artifact characters:
  - Scattered: "T h i s   i s" = 13 chars
  - Repeated punct: "........" = 8 chars
  - Repeated punct: "___" = 3 chars
  Total artifacts: 24 chars

OCR Penalty = 24 / 39 = 0.615
Final Impact = 0.615 × 0.3 (weight) = 0.185
```

#### Step 2: Script Penalty Calculation

```
Script Penalty = (Total Script Characters / Total Characters)
```

**Script Characters Include**:
- JavaScript function bodies
- CSS rule definitions
- `<script>` tag contents
- `<style>` tag contents

**Example**:
```rust
Text: "Content function test() { return 42; } More content"
Total characters: 52
Script characters: "function test() { return 42; }" = 30 chars

Script Penalty = 30 / 52 = 0.577
Final Impact = 0.577 × 0.2 (weight) = 0.115
```

#### Step 3: Navigation Penalty Calculation

```
Navigation Penalty = (Total Nav Characters / Total Characters)
```

**Navigation Characters Include**:
- Navigation keywords ("Skip to main content", etc.)
- Breadcrumb separators ("Home > Category > Page")
- Pagination indicators ("Page 1 of 10")

**Example**:
```rust
Text: "Skip to main content Article content Back to top"
Total characters: 50
Nav characters:
  - "Skip to main content" = 20 chars
  - "Back to top" = 11 chars
  Total: 31 chars

Navigation Penalty = 31 / 50 = 0.62
Final Impact = 0.62 × 0.1 (weight) = 0.062
```

#### Step 4: Structure Bonus Calculation

Quality processing rewards well-structured text:

**Structure Score Components** (max 1.0):
- **Sentence length** (+0.3): Average 10-30 words per sentence
- **Paragraph length** (+0.3): Average 50-300 words per paragraph
- **Multiple paragraphs** (+0.2): More than one paragraph present
- **Proper punctuation** (+0.2): Contains sentence-ending punctuation (`.!?`)

**Example**:
```rust
Text: "This is a sentence. Another sentence here.\n\nNew paragraph. More content."

Words: 13
Sentences: 4 (detected by ". " pattern)
Paragraphs: 2 (split by "\n\n")

Avg words/sentence: 13 / 4 = 3.25 (NOT in 10-30 range, no bonus)
Avg words/paragraph: 13 / 2 = 6.5 (NOT in 50-300 range, no bonus)
Multiple paragraphs: Yes (+0.2)
Has punctuation: Yes (+0.2)

Structure Bonus = 0.4
Final Impact = 0.4 × 0.2 (weight) = 0.08
```

#### Step 5: Metadata Bonus Calculation

```
Metadata Bonus = (Present Important Fields / Total Important Fields)
```

**Important Fields** (5 total):
- `title`
- `author`
- `subject`
- `description`
- `keywords`

**Example**:
```rust
Metadata: { "title": "Doc", "author": "Name" }
Present fields: 2 / 5 = 0.4

Metadata Bonus = 0.4
Final Impact = 0.4 × 0.1 (weight) = 0.04
```

#### Final Score Calculation

```
Quality Score = 1.0
                - (OCR Penalty × 0.3)
                - (Script Penalty × 0.2)
                - (Navigation Penalty × 0.1)
                + (Structure Bonus × 0.2)
                + (Metadata Bonus × 0.1)

Score = clamp(Quality Score, 0.0, 1.0)
```

**Complete Example**:
```rust
Starting score: 1.0

OCR Penalty impact:       -0.185
Script Penalty impact:    -0.115
Navigation Penalty impact: -0.062
Structure Bonus impact:   +0.080
Metadata Bonus impact:    +0.040

Final Score: 1.0 - 0.185 - 0.115 - 0.062 + 0.080 + 0.040 = 0.758
```

### Understanding Quality Scores

Quality scores range from 0.0 (lowest) to 1.0 (highest). Use these guidelines to interpret scores:

#### Quality Thresholds

| Score Range | Quality Level | Description | Action |
|-------------|---------------|-------------|--------|
| **0.9 - 1.0** | Excellent | Clean extraction with good structure | Use directly |
| **0.7 - 0.9** | Good | Minor artifacts, mostly clean | Safe to use, minimal cleanup needed |
| **0.5 - 0.7** | Fair | Some artifacts or poor structure | Review and validate content |
| **0.3 - 0.5** | Poor | Significant artifacts or issues | Manual review recommended |
| **0.0 - 0.3** | Very Poor | Severe extraction problems | Consider re-extraction with different settings |

#### Common Score Patterns

**High Scores (0.8+)**:
- Native PDF text extraction (not OCR)
- Clean DOCX/PPTX extraction
- Well-structured HTML conversion
- Minimal artifacts and good formatting

**Medium Scores (0.5-0.8)**:
- OCR extraction with some artifacts
- HTML with navigation elements
- Mixed content quality
- Some formatting issues

**Low Scores (< 0.5)**:
- Poor OCR quality
- Heavy JavaScript/CSS contamination
- Excessive navigation elements
- Severely degraded scans

### Advanced Usage Examples

#### Quality-Based Document Filtering

Filter documents based on quality thresholds:

=== "Python"

    ```python
    from kreuzberg import batch_extract_files_sync, ExtractionConfig
    from typing import List

    def extract_high_quality_docs(files: List[str], min_quality: float = 0.7):
        config = ExtractionConfig(enable_quality_processing=True)
        results = batch_extract_files_sync(files, config=config)

        high_quality = []
        for i, result in enumerate(results):
            # Quality score would be in metadata if implemented
            # This is a placeholder showing the pattern
            if result.content and len(result.content) > 100:
                high_quality.append({
                    'file': files[i],
                    'result': result,
                    'content_length': len(result.content)
                })

        return high_quality

    files = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]
    good_docs = extract_high_quality_docs(files)
    print(f"Found {len(good_docs)} high-quality documents")
    ```

=== "TypeScript"

    ```typescript
    import { batchExtractFiles, ExtractionConfig } from '@goldziher/kreuzberg';

    async function extractHighQualityDocs(
      files: string[],
      minQuality: number = 0.7
    ) {
      const config: ExtractionConfig = {
        enableQualityProcessing: true
      };

      const results = await batchExtractFiles(files, config);

      const highQuality = results
        .map((result, i) => ({ file: files[i], result }))
        .filter(({ result }) => result.content.length > 100);

      return highQuality;
    }

    const files = ['doc1.pdf', 'doc2.pdf', 'doc3.pdf'];
    const goodDocs = await extractHighQualityDocs(files);
    console.log(`Found ${goodDocs.length} high-quality documents`);
    ```

=== "Rust"

    ```rust
    use kreuzberg::{batch_extract_file, ExtractionConfig};

    #[tokio::main]
    async fn main() -> kreuzberg::Result<()> {
        let files = vec!["doc1.pdf", "doc2.pdf", "doc3.pdf"];

        let config = ExtractionConfig {
            enable_quality_processing: true,
            ..Default::default()
        };

        let results = batch_extract_file(&files, None, &config).await?;

        let high_quality: Vec<_> = results
            .into_iter()
            .zip(files.iter())
            .filter(|(result, _)| result.content.len() > 100)
            .collect();

        println!("Found {} high-quality documents", high_quality.len());
        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Enable quality processing (default)
    config = Kreuzberg::Config::Extraction.new(
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)

    # Access quality score if available
    puts "Quality score: #{result.quality_score}" if result.quality_score

    puts result.content  # Cleaned text
    ```

#### Before/After Cleaning Examples

**Example 1: OCR Artifacts**

```
Input:
"T h e   q u i c k   b r o w n   f o x .......
jumps    over    the    lazy    dog________"

After Quality Processing:
"The quick brown fox...
jumps over the lazy dog..."
```

**Example 2: Script Removal**

```
Input:
"Welcome to our site
<script>
  function trackUser() {
    analytics.send('pageview');
  }
</script>
Main content starts here
<style>
  .header { background: blue; }
</style>
More content"

After Quality Processing:
"Welcome to our site
Main content starts here
More content"
```

**Example 3: Navigation Cleanup**

```
Input:
"Skip to main content
Home > Blog > Technology > AI
Article: Understanding Machine Learning
This article explains ML basics.
Page 1 of 3 | Next page
Back to top"

After Quality Processing:
"Article: Understanding Machine Learning
This article explains ML basics."
```

**Example 4: Combined Cleaning**

```
Input:
"S k i p   to main content
<script>alert('test');</script>
Article   content........ here
function bad() { }
More    text___
Page 1 of 10"

After Quality Processing:
"Article content... here
More text..."
```

### Performance Considerations

#### When to Enable Quality Processing

**Enable (Default)**:
- Processing OCR-extracted text
- Converting HTML documents
- Extracting from web sources
- Documents with unknown quality

**Consider Disabling**:
- Processing clean, native text (PDF with text layer)
- Performance-critical batch processing
- Pre-cleaned input sources
- When artifacts are acceptable

#### Performance Impact

| Document Type | Processing Time | Quality Improvement |
|---------------|----------------|---------------------|
| Native PDF | +5-10ms | Low (already clean) |
| OCR PDF | +15-30ms | High (many artifacts) |
| HTML | +10-20ms | Medium (script/nav removal) |
| DOCX | +5-10ms | Low (usually clean) |

#### Optimization Tips

1. **Batch Processing**: Quality processing scales linearly, process multiple documents in parallel
2. **Selective Processing**: Disable for known-clean sources
3. **Quality Thresholds**: Use quality scores to identify documents needing manual review
4. **Caching**: Cache cleaned results for repeated processing

## Keyword Extraction

Extract meaningful keywords from text using statistical or co-occurrence based algorithms.

### Algorithms

#### YAKE (Yet Another Keyword Extractor)

Statistical keyword extraction considering:

- Term frequency and position
- Capitalization patterns
- Sentence co-occurrence
- Context analysis

**Best for**: Academic documents, technical content, formal writing

#### RAKE (Rapid Automatic Keyword Extraction)

Co-occurrence based extraction:

- Uses stopwords as delimiters
- Calculates word scores by frequency and degree
- Combines scores for multi-word phrases

**Best for**: General text, web content, informal documents

### Basic Usage

=== "Python"

    ```python
    from kreuzberg.keywords import extract_keywords, KeywordConfig

    text = """
    Rust is a systems programming language focused on safety and performance.
    Memory safety is guaranteed through Rust's ownership system. Concurrent
    programming is safe thanks to the borrow checker. Zero-cost abstractions
    provide high performance without sacrificing safety.
    """

    # Use default algorithm (YAKE if available)
    config = KeywordConfig()
    keywords = extract_keywords(text, config)

    for keyword in keywords:
        print(f"{keyword.text}: {keyword.score:.3f} ({keyword.algorithm})")
    ```

    **Output:**
    ```
    systems programming: 0.892 (yake)
    memory safety: 0.845 (yake)
    ownership system: 0.798 (yake)
    borrow checker: 0.756 (yake)
    zero-cost abstractions: 0.723 (yake)
    ```

=== "TypeScript"

    ```typescript
    import { extractKeywords, KeywordConfig } from '@goldziher/kreuzberg';

    const text = `
    Rust is a systems programming language focused on safety and performance.
    Memory safety is guaranteed through Rust's ownership system. Concurrent
    programming is safe thanks to the borrow checker. Zero-cost abstractions
    provide high performance without sacrificing safety.
    `;

    // Use default algorithm (YAKE if available)
    const config: KeywordConfig = {};
    const keywords = extractKeywords(text, config);

    for (const keyword of keywords) {
      console.log(`${keyword.text}: ${keyword.score.toFixed(3)} (${keyword.algorithm})`);
    }
    ```

=== "Rust"

    ```rust
    use kreuzberg::keywords::{extract_keywords, KeywordConfig};

    fn main() -> kreuzberg::Result<()> {
        let text = "Rust is a systems programming language focused on safety and performance. \
                    Memory safety is guaranteed through Rust's ownership system. Concurrent \
                    programming is safe thanks to the borrow checker. Zero-cost abstractions \
                    provide high performance without sacrificing safety.";

        // Use default algorithm (YAKE if available)
        let config = KeywordConfig::default();
        let keywords = extract_keywords(text, &config)?;

        for keyword in keywords {
            println!("{}: {:.3} ({:?})", keyword.text, keyword.score, keyword.algorithm);
        }

        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Configuration via Kreuzberg::Config objects
    config = Kreuzberg::Config::Extraction.new(
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts result.content
    ```

### Algorithm-Specific Configuration

=== "Python"

    ```python
    from kreuzberg.keywords import extract_keywords, KeywordConfig, YakeParams, RakeParams

    text = "Your document text here..."

    # YAKE with custom parameters
    yake_config = KeywordConfig.yake(
        max_keywords=15,
        min_score=0.3,
        ngram_range=(1, 3),  # unigrams to trigrams
        language="en"
    )
    yake_config.yake_params = YakeParams(window_size=3)
    yake_keywords = extract_keywords(text, yake_config)

    # RAKE with custom parameters
    rake_config = KeywordConfig.rake(
        max_keywords=15,
        min_score=0.2,
        ngram_range=(2, 3),  # only bigrams and trigrams
        language="es"  # Spanish stopwords
    )
    rake_config.rake_params = RakeParams(
        min_word_length=3,
        max_words_per_phrase=4
    )
    rake_keywords = extract_keywords(text, rake_config)
    ```

=== "TypeScript"

    ```typescript
    import { extractKeywords, KeywordConfig, YakeParams, RakeParams } from '@goldziher/kreuzberg';

    const text = "Your document text here...";

    // YAKE with custom parameters
    const yakeConfig: KeywordConfig = {
      algorithm: 'yake',
      maxKeywords: 15,
      minScore: 0.3,
      ngramRange: [1, 3],  // unigrams to trigrams
      language: 'en',
      yakeParams: { windowSize: 3 }
    };
    const yakeKeywords = extractKeywords(text, yakeConfig);

    // RAKE with custom parameters
    const rakeConfig: KeywordConfig = {
      algorithm: 'rake',
      maxKeywords: 15,
      minScore: 0.2,
      ngramRange: [2, 3],  // only bigrams and trigrams
      language: 'es',  // Spanish stopwords
      rakeParams: {
        minWordLength: 3,
        maxWordsPerPhrase: 4
      }
    };
    const rakeKeywords = extractKeywords(text, rakeConfig);
    ```

=== "Rust"

    ```rust
    use kreuzberg::keywords::{extract_keywords, KeywordConfig, YakeParams, RakeParams};

    fn main() -> kreuzberg::Result<()> {
        let text = "Your document text here...";

        // YAKE with custom parameters
        let yake_config = KeywordConfig::yake()
            .with_max_keywords(15)
            .with_min_score(0.3)
            .with_ngram_range(1, 3)  // unigrams to trigrams
            .with_language("en")
            .with_yake_params(YakeParams { window_size: 3 });

        let yake_keywords = extract_keywords(text, &yake_config)?;

        // RAKE with custom parameters
        let rake_config = KeywordConfig::rake()
            .with_max_keywords(15)
            .with_min_score(0.2)
            .with_ngram_range(2, 3)  // only bigrams and trigrams
            .with_language("es")  // Spanish stopwords
            .with_rake_params(RakeParams {
                min_word_length: 3,
                max_words_per_phrase: 4,
            });

        let rake_keywords = extract_keywords(text, &rake_config)?;

        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Configuration via Kreuzberg::Config objects
    config = Kreuzberg::Config::Extraction.new(
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts result.content
    ```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `algorithm` | `yake` \| `rake` | `yake` | Algorithm to use |
| `max_keywords` | int | 10 | Maximum keywords to extract |
| `min_score` | float | 0.0 | Minimum score threshold (0.0-1.0) |
| `ngram_range` | tuple | (1, 3) | N-gram range (min, max) |
| `language` | str | `"en"` | Language code for stopwords |
| `yake_params.window_size` | int | 2 | Context window for YAKE |
| `rake_params.min_word_length` | int | 1 | Minimum word length for RAKE |
| `rake_params.max_words_per_phrase` | int | 3 | Maximum words per phrase for RAKE |

### Multilingual Support

Keywords extraction supports multiple languages through stopword filtering:

=== "Python"

    ```python
    from kreuzberg.keywords import extract_keywords, KeywordConfig

    # Spanish text
    spanish_text = "El aprendizaje automático es una rama de la inteligencia artificial."
    config_es = KeywordConfig(language="es")
    keywords_es = extract_keywords(spanish_text, config_es)

    # English text
    english_text = "Machine learning is a branch of artificial intelligence."
    config_en = KeywordConfig(language="en")
    keywords_en = extract_keywords(english_text, config_en)
    ```

=== "TypeScript"

    ```typescript
    import { extractKeywords, KeywordConfig } from '@goldziher/kreuzberg';

    // Spanish text
    const spanishText = "El aprendizaje automático es una rama de la inteligencia artificial.";
    const configEs: KeywordConfig = { language: 'es' };
    const keywordsEs = extractKeywords(spanishText, configEs);

    // English text
    const englishText = "Machine learning is a branch of artificial intelligence.";
    const configEn: KeywordConfig = { language: 'en' };
    const keywordsEn = extractKeywords(englishText, configEn);
    ```

=== "Rust"

    ```rust
    use kreuzberg::keywords::{extract_keywords, KeywordConfig};

    fn main() -> kreuzberg::Result<()> {
        // Spanish text
        let spanish_text = "El aprendizaje automático es una rama de la inteligencia artificial.";
        let config_es = KeywordConfig::default().with_language("es");
        let keywords_es = extract_keywords(spanish_text, &config_es)?;

        // English text
        let english_text = "Machine learning is a branch of artificial intelligence.";
        let config_en = KeywordConfig::default().with_language("en");
        let keywords_en = extract_keywords(english_text, &config_en)?;

        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Configuration via Kreuzberg::Config objects
    config = Kreuzberg::Config::Extraction.new(
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts result.content
    ```

**Built-in languages**: English (`en`), Spanish (`es`)

For other languages, stopwords will fall back to English or can be loaded from JSON files.

### Algorithm Comparison

Understanding the differences between YAKE and RAKE helps you choose the right algorithm for your use case.

#### YAKE (Yet Another Keyword Extractor)

**How It Works**:
- Statistical approach analyzing multiple features
- Considers term frequency, position in text, capitalization
- Analyzes sentence co-occurrence patterns
- Evaluates context around each term
- Outputs dissimilarity scores (lower = better keyword)

**Strengths**:
- More accurate for formal, structured documents
- Better handling of technical terminology
- Considers semantic context
- Language-independent statistical features
- Good for academic papers, technical documentation

**Weaknesses**:
- Slower than RAKE (~2-3x processing time)
- Requires more text to be effective (~100+ words)
- May overweight rare technical terms

**Best Use Cases**:
- Academic papers and research documents
- Technical documentation and whitepapers
- Legal and medical documents
- Formal business reports
- Content with domain-specific terminology

#### RAKE (Rapid Automatic Keyword Extraction)

**How It Works**:
- Co-occurrence based approach using stopwords as delimiters
- Calculates word scores based on frequency and co-occurrence degree
- Combines scores for multi-word phrases
- Simple, fast algorithm with minimal computation
- Outputs degree/frequency scores (higher = better keyword)

**Strengths**:
- Very fast extraction (~3-5x faster than YAKE)
- Works well with shorter texts
- Effective for informal content
- Good at identifying multi-word phrases
- Minimal language-specific dependencies

**Weaknesses**:
- Less accurate on formal/technical documents
- Relies heavily on stopword quality
- No semantic understanding
- May miss context-dependent keywords

**Best Use Cases**:
- Blog posts and articles
- Social media content
- Product descriptions
- Web content and marketing copy
- Informal documents and emails
- Large-scale batch processing (speed priority)

#### Performance Comparison

| Metric | YAKE | RAKE |
|--------|------|------|
| **Speed** | ~50-100 docs/sec | ~200-300 docs/sec |
| **Accuracy (formal)** | Excellent | Good |
| **Accuracy (informal)** | Good | Excellent |
| **Min text length** | ~100 words | ~50 words |
| **Multi-word phrases** | Good | Excellent |
| **Technical terms** | Excellent | Fair |
| **Memory usage** | Medium | Low |
| **Language support** | 100+ (via stopwords) | 64 (built-in stopwords) |

#### Choosing the Right Algorithm

**Use YAKE when**:
- Processing academic, technical, or formal documents
- Accuracy is more important than speed
- Working with domain-specific terminology
- Documents are well-structured (100+ words)
- Context and semantic meaning matter

**Use RAKE when**:
- Processing informal content (blogs, social media)
- Speed is critical (batch processing thousands of documents)
- Working with shorter texts (50-200 words)
- Multi-word phrase extraction is priority
- Simple, fast keyword extraction is sufficient

**Example - Comparing Both Algorithms**:

=== "Python"

    ```python
    from kreuzberg.keywords import extract_keywords, KeywordConfig

    text = """
    Rust's ownership system ensures memory safety without garbage collection.
    The borrow checker prevents data races at compile time. Zero-cost
    abstractions provide high performance while maintaining safety guarantees.
    """

    yake_config = KeywordConfig.yake(max_keywords=5)
    yake_keywords = extract_keywords(text, yake_config)

    rake_config = KeywordConfig.rake(max_keywords=5)
    rake_keywords = extract_keywords(text, rake_config)

    print("YAKE Keywords:")
    for kw in yake_keywords:
        print(f"  {kw.text}: {kw.score:.3f}")

    print("\nRAKE Keywords:")
    for kw in rake_keywords:
        print(f"  {kw.text}: {kw.score:.3f}")
    ```

    **Output:**
    ```
    YAKE Keywords:
      ownership system: 0.892
      memory safety: 0.845
      borrow checker: 0.798
      garbage collection: 0.756
      zero-cost abstractions: 0.723

    RAKE Keywords:
      ownership system ensures memory safety: 0.945
      zero-cost abstractions provide high performance: 0.912
      borrow checker prevents data races: 0.887
      garbage collection: 0.654
      compile time: 0.612
    ```

=== "TypeScript"

    ```typescript
    import { extractKeywords, KeywordConfig } from '@goldziher/kreuzberg';

    const text = `
    Rust's ownership system ensures memory safety without garbage collection.
    The borrow checker prevents data races at compile time. Zero-cost
    abstractions provide high performance while maintaining safety guarantees.
    `;

    const yakeConfig: KeywordConfig = { algorithm: 'yake', maxKeywords: 5 };
    const yakeKeywords = extractKeywords(text, yakeConfig);

    const rakeConfig: KeywordConfig = { algorithm: 'rake', maxKeywords: 5 };
    const rakeKeywords = extractKeywords(text, rakeConfig);

    console.log('YAKE Keywords:');
    yakeKeywords.forEach(kw => console.log(`  ${kw.text}: ${kw.score.toFixed(3)}`));

    console.log('\nRAKE Keywords:');
    rakeKeywords.forEach(kw => console.log(`  ${kw.text}: ${kw.score.toFixed(3)}`));
    ```

=== "Rust"

    ```rust
    use kreuzberg::keywords::{extract_keywords, KeywordConfig};

    fn main() -> kreuzberg::Result<()> {
        let text = "Rust's ownership system ensures memory safety without garbage collection. \
                    The borrow checker prevents data races at compile time. Zero-cost \
                    abstractions provide high performance while maintaining safety guarantees.";

        let yake_config = KeywordConfig::yake().with_max_keywords(5);
        let yake_keywords = extract_keywords(text, &yake_config)?;

        let rake_config = KeywordConfig::rake().with_max_keywords(5);
        let rake_keywords = extract_keywords(text, &rake_config)?;

        println!("YAKE Keywords:");
        for kw in yake_keywords {
            println!("  {}: {:.3}", kw.text, kw.score);
        }

        println!("\nRAKE Keywords:");
        for kw in rake_keywords {
            println!("  {}: {:.3}", kw.text, kw.score);
        }

        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Configuration via Kreuzberg::Config objects
    config = Kreuzberg::Config::Extraction.new(
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts result.content
    ```

### Score Normalization

Both algorithms normalize scores to a 0.0-1.0 range, but use different approaches due to their underlying scoring mechanisms.

#### YAKE Score Normalization

**Raw Scores**: YAKE outputs dissimilarity scores where **lower = better** (more relevant keyword).

**Normalization Formula**:
```
normalized_score = 1 / (1 + raw_score)
```

**Properties**:
- Converts dissimilarity to similarity (higher = better)
- Clamped to [0.0, 1.0] range
- Score of 0.0 becomes 1.0 (perfect keyword)
- Higher raw scores approach 0.0 (less relevant)

**Example Transformations**:
| Raw Score | Normalized Score | Interpretation |
|-----------|------------------|----------------|
| 0.0 | 1.000 | Perfect keyword |
| 0.1 | 0.909 | Excellent keyword |
| 0.5 | 0.667 | Good keyword |
| 1.0 | 0.500 | Fair keyword |
| 2.0 | 0.333 | Marginal keyword |
| 5.0 | 0.167 | Poor keyword |

**Code Example**:
```python
# YAKE raw scores (lower = better)
raw_yake_scores = [0.05, 0.12, 0.28, 0.45, 0.89]

# Normalize to 0.0-1.0 (higher = better)
normalized = [1.0 / (1.0 + score) for score in raw_yake_scores]

# Result: [0.952, 0.893, 0.781, 0.690, 0.529]
```

#### RAKE Score Normalization

**Raw Scores**: RAKE outputs degree/frequency scores where **higher = better**.

**Normalization Formula** (min-max scaling):
```
normalized_score = (raw_score - min_score) / (max_score - min_score)
```

**Properties**:
- Linear scaling to [0.0, 1.0] range
- Best keyword gets score = 1.0
- Worst keyword gets score = 0.0
- Preserves relative ranking

**Example Transformations**:
| Raw Score | Min | Max | Normalized Score |
|-----------|-----|-----|------------------|
| 15.5 | 2.0 | 15.5 | 1.000 (best) |
| 12.3 | 2.0 | 15.5 | 0.763 |
| 8.7 | 2.0 | 15.5 | 0.496 |
| 5.2 | 2.0 | 15.5 | 0.237 |
| 2.0 | 2.0 | 15.5 | 0.000 (worst) |

**Code Example**:
```python
# RAKE raw scores (higher = better)
raw_rake_scores = [15.5, 12.3, 8.7, 5.2, 2.0]

min_score = min(raw_rake_scores)  # 2.0
max_score = max(raw_rake_scores)  # 15.5

# Normalize using min-max scaling
normalized = [
    (score - min_score) / (max_score - min_score)
    for score in raw_rake_scores
]

# Result: [1.000, 0.763, 0.496, 0.237, 0.000]
```

#### Comparing Normalized Scores

**Important**: Normalized scores from YAKE and RAKE are **not directly comparable** because they use different normalization methods and underlying algorithms.

**Guidelines**:

1. **Within-algorithm comparison**: Compare scores within the same algorithm's results
   ```python
   # Compare YAKE keywords to each other
   if yake_keywords[0].score > 0.8:
       print("High-confidence keyword")
   ```

2. **Threshold setting**: Set different thresholds for each algorithm
   ```python
   yake_threshold = 0.5  # YAKE threshold
   rake_threshold = 0.3  # RAKE threshold (typically lower)
   ```

3. **Cross-algorithm validation**: Use both algorithms and look for overlap
   ```python
   yake_texts = {kw.text for kw in yake_keywords}
   rake_texts = {kw.text for kw in rake_keywords}
   common = yake_texts & rake_texts  # Keywords both algorithms agree on
   ```

**Practical Example**:

=== "Python"

    ```python
    from kreuzberg.keywords import extract_keywords, KeywordConfig

    text = "Your document text..."

    yake_config = KeywordConfig.yake(min_score=0.5)
    yake_keywords = extract_keywords(text, yake_config)

    rake_config = KeywordConfig.rake(min_score=0.3)
    rake_keywords = extract_keywords(text, rake_config)

    print(f"YAKE found {len(yake_keywords)} keywords (threshold: 0.5)")
    print(f"RAKE found {len(rake_keywords)} keywords (threshold: 0.3)")

    yake_set = {kw.text.lower() for kw in yake_keywords}
    rake_set = {kw.text.lower() for kw in rake_keywords}
    overlap = yake_set & rake_set

    print(f"Both algorithms agree on {len(overlap)} keywords: {overlap}")
    ```

=== "TypeScript"

    ```typescript
    import { extractKeywords, KeywordConfig } from '@goldziher/kreuzberg';

    const text = "Your document text...";

    const yakeConfig: KeywordConfig = { algorithm: 'yake', minScore: 0.5 };
    const yakeKeywords = extractKeywords(text, yakeConfig);

    const rakeConfig: KeywordConfig = { algorithm: 'rake', minScore: 0.3 };
    const rakeKeywords = extractKeywords(text, rakeConfig);

    console.log(`YAKE found ${yakeKeywords.length} keywords (threshold: 0.5)`);
    console.log(`RAKE found ${rakeKeywords.length} keywords (threshold: 0.3)`);

    const yakeSet = new Set(yakeKeywords.map(kw => kw.text.toLowerCase()));
    const rakeSet = new Set(rakeKeywords.map(kw => kw.text.toLowerCase()));
    const overlap = [...yakeSet].filter(kw => rakeSet.has(kw));

    console.log(`Both algorithms agree on ${overlap.length} keywords: ${overlap}`);
    ```

=== "Rust"

    ```rust
    use kreuzberg::keywords::{extract_keywords, KeywordConfig};
    use std::collections::HashSet;

    fn main() -> kreuzberg::Result<()> {
        let text = "Your document text...";

        let yake_config = KeywordConfig::yake().with_min_score(0.5);
        let yake_keywords = extract_keywords(text, &yake_config)?;

        let rake_config = KeywordConfig::rake().with_min_score(0.3);
        let rake_keywords = extract_keywords(text, &rake_config)?;

        println!("YAKE found {} keywords (threshold: 0.5)", yake_keywords.len());
        println!("RAKE found {} keywords (threshold: 0.3)", rake_keywords.len());

        let yake_set: HashSet<String> = yake_keywords
            .iter()
            .map(|kw| kw.text.to_lowercase())
            .collect();

        let rake_set: HashSet<String> = rake_keywords
            .iter()
            .map(|kw| kw.text.to_lowercase())
            .collect();

        let overlap: Vec<_> = yake_set.intersection(&rake_set).collect();

        println!("Both algorithms agree on {} keywords: {:?}", overlap.len(), overlap);

        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Configuration via Kreuzberg::Config objects
    config = Kreuzberg::Config::Extraction.new(
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts result.content
    ```

## Token Reduction

Reduce token count for LLM processing while preserving meaning, structure, and critical information.

### Features

- **Stopword Removal**: Language-specific stopword filtering
- **Redundancy Elimination**: Remove duplicate and near-duplicate content
- **Semantic Clustering**: Group semantically similar sentences (optional)
- **Structure Preservation**: Maintain markdown formatting and code blocks
- **SIMD Optimization**: Fast text processing using SIMD instructions
- **Parallel Processing**: Multi-threaded batch reduction
- **Configurable Levels**: From light to maximum reduction

### Reduction Levels

| Level | Description | Use Case |
|-------|-------------|----------|
| `off` | No reduction | Preserve original text |
| `light` | Remove only obvious redundancy | Minimal changes, preserve most content |
| `moderate` | Balance between size and meaning | General use, good default |
| `aggressive` | Significant reduction | LLM context limits, cost optimization |
| `maximum` | Maximum reduction | Extreme token limits, summaries |

### Basic Usage

=== "Python"

    ```python
    from kreuzberg import extract_file_sync, ExtractionConfig, TokenReductionConfig

    # Enable token reduction with moderate level (default)
    config = ExtractionConfig(
        token_reduction=TokenReductionConfig(level="moderate")
    )

    result = extract_file_sync("document.pdf", config=config)
    print(f"Reduced text ({len(result.content)} chars):")
    print(result.content)
    ```

=== "TypeScript"

    ```typescript
    import { extractFileSync, ExtractionConfig, TokenReductionConfig } from '@goldziher/kreuzberg';

    // Enable token reduction with moderate level (default)
    const config: ExtractionConfig = {
      tokenReduction: {
        level: 'moderate'
      }
    };

    const result = extractFileSync('document.pdf', { config });
    console.log(`Reduced text (${result.content.length} chars):`);
    console.log(result.content);
    ```

=== "Rust"

    ```rust
    use kreuzberg::{extract_file_sync, ExtractionConfig};
    use kreuzberg::text::token_reduction::{TokenReductionConfig, ReductionLevel};

    fn main() -> kreuzberg::Result<()> {
        // Enable token reduction with moderate level (default)
        let config = ExtractionConfig {
            token_reduction: Some(TokenReductionConfig {
                level: ReductionLevel::Moderate,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extract_file_sync("document.pdf", None, &config)?;
        println!("Reduced text ({} chars):", result.content.len());
        println!("{}", result.content);
        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Enable token reduction
    token_config = Kreuzberg::Config::TokenReduction.new(
      enabled: true,
      level: "medium"  # Options: "light", "medium", "aggressive"
    )

    config = Kreuzberg::Config::Extraction.new(
      token_reduction: token_config
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts result.content  # Reduced token content
    ```

=== "CLI"

    ```bash
    # Enable token reduction with moderate level
    kreuzberg extract document.pdf --token-reduction moderate

    # Aggressive reduction for maximum token savings
    kreuzberg extract document.pdf --token-reduction aggressive
    ```

### Advanced Configuration

=== "Python"

    ```python
    from kreuzberg import extract_file_sync, ExtractionConfig, TokenReductionConfig

    config = ExtractionConfig(
        token_reduction=TokenReductionConfig(
            level="aggressive",
            language_hint="en",
            preserve_markdown=True,   # Keep markdown formatting
            preserve_code=True,       # Keep code blocks
            semantic_threshold=0.3,   # Similarity threshold
            enable_parallel=True,     # Use multi-threading
            use_simd=True,           # Use SIMD optimizations
            target_reduction=0.4,    # Target 40% reduction
            enable_semantic_clustering=True  # Group similar content
        )
    )

    result = extract_file_sync("document.pdf", config=config)
    ```

=== "TypeScript"

    ```typescript
    import { extractFileSync, ExtractionConfig, TokenReductionConfig } from '@goldziher/kreuzberg';

    const config: ExtractionConfig = {
      tokenReduction: {
        level: 'aggressive',
        languageHint: 'en',
        preserveMarkdown: true,   // Keep markdown formatting
        preserveCode: true,       // Keep code blocks
        semanticThreshold: 0.3,   // Similarity threshold
        enableParallel: true,     // Use multi-threading
        useSimd: true,           // Use SIMD optimizations
        targetReduction: 0.4,    // Target 40% reduction
        enableSemanticClustering: true  // Group similar content
      }
    };

    const result = extractFileSync('document.pdf', { config });
    ```

=== "Rust"

    ```rust
    use kreuzberg::{extract_file_sync, ExtractionConfig};
    use kreuzberg::text::token_reduction::{TokenReductionConfig, ReductionLevel};

    fn main() -> kreuzberg::Result<()> {
        let config = ExtractionConfig {
            token_reduction: Some(TokenReductionConfig {
                level: ReductionLevel::Aggressive,
                language_hint: Some("en".to_string()),
                preserve_markdown: true,   // Keep markdown formatting
                preserve_code: true,       // Keep code blocks
                semantic_threshold: 0.3,   // Similarity threshold
                enable_parallel: true,     // Use multi-threading
                use_simd: true,           // Use SIMD optimizations
                target_reduction: Some(0.4),  // Target 40% reduction
                enable_semantic_clustering: true,  // Group similar content
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extract_file_sync("document.pdf", None, &config)?;
        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Enable token reduction
    token_config = Kreuzberg::Config::TokenReduction.new(
      enabled: true,
      level: "medium"  # Options: "light", "medium", "aggressive"
    )

    config = Kreuzberg::Config::Extraction.new(
      token_reduction: token_config
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts result.content  # Reduced token content
    ```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `level` | enum | `moderate` | Reduction aggressiveness |
| `language_hint` | str | None | Language hint for better processing |
| `preserve_markdown` | bool | `false` | Keep markdown formatting |
| `preserve_code` | bool | `true` | Keep code blocks intact |
| `semantic_threshold` | float | 0.3 | Similarity threshold (0.0-1.0) |
| `enable_parallel` | bool | `true` | Use parallel processing |
| `use_simd` | bool | `true` | Use SIMD optimizations |
| `target_reduction` | float | None | Target reduction ratio (0.0-1.0) |
| `enable_semantic_clustering` | bool | `false` | Enable semantic grouping |

### Direct API Usage

Process text directly without full extraction:

=== "Rust"

    ```rust
    use kreuzberg::text::token_reduction::{
        reduce_tokens, batch_reduce_tokens, get_reduction_statistics,
        TokenReductionConfig, ReductionLevel
    };

    fn main() -> kreuzberg::Result<()> {
        let text = "Your long document text here...";

        // Reduce a single text
        let config = TokenReductionConfig {
            level: ReductionLevel::Aggressive,
            ..Default::default()
        };
        let reduced = reduce_tokens(text, &config, Some("en"))?;

        // Get statistics
        let (char_reduction, token_reduction, orig_chars, red_chars, orig_tokens, red_tokens) =
            get_reduction_statistics(text, &reduced);

        println!("Character reduction: {:.1}%", char_reduction * 100.0);
        println!("Token reduction: {:.1}%", token_reduction * 100.0);
        println!("Original: {} chars, {} tokens", orig_chars, orig_tokens);
        println!("Reduced: {} chars, {} tokens", red_chars, red_tokens);

        // Batch processing
        let texts = vec!["text1...", "text2...", "text3..."];
        let reduced_batch = batch_reduce_tokens(&texts, &config, Some("en"))?;

        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Enable token reduction
    token_config = Kreuzberg::Config::TokenReduction.new(
      enabled: true,
      level: "medium"  # Options: "light", "medium", "aggressive"
    )

    config = Kreuzberg::Config::Extraction.new(
      token_reduction: token_config
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts result.content  # Reduced token content
    ```

### Custom Stopwords

Provide custom stopwords for domain-specific text:

=== "Rust"

    ```rust
    use kreuzberg::text::token_reduction::TokenReductionConfig;
    use std::collections::HashMap;

    let mut custom_stopwords = HashMap::new();
    custom_stopwords.insert(
        "en".to_string(),
        vec!["very".to_string(), "really".to_string(), "just".to_string()]
    );

    let config = TokenReductionConfig {
        custom_stopwords: Some(custom_stopwords),
        ..Default::default()
    };
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Custom stopwords configuration
    token_config = Kreuzberg::Config::TokenReduction.new(
      enabled: true,
      level: "medium",
      remove_stopwords: true,
      custom_stopwords: ["custom", "words", "to", "remove"]
    )

    config = Kreuzberg::Config::Extraction.new(
      token_reduction: token_config
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    ```

### Preserve Patterns

Protect specific patterns from reduction:

=== "Rust"

    ```rust
    use kreuzberg::text::token_reduction::TokenReductionConfig;

    let config = TokenReductionConfig {
        preserve_patterns: vec![
            r"\d+\.\d+\.\d+".to_string(),  // Version numbers
            r"[A-Z]{2,}".to_string(),      // Acronyms
        ],
        ..Default::default()
    };
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Advanced token reduction configuration
    token_config = Kreuzberg::Config::TokenReduction.new(
      enabled: true,
      level: "aggressive",
      remove_stopwords: true,
      max_newlines: 2,
      preserve_patterns: [/\d{4}-\d{2}-\d{2}/]  # Preserve dates
    )

    config = Kreuzberg::Config::Extraction.new(
      token_reduction: token_config
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts result.content
    ```

### SIMD Optimization

Token reduction uses SIMD (Single Instruction, Multiple Data) instructions for high-performance text processing. This provides 2-4x speedup on supported platforms.

#### How It Works

SIMD processing operates on multiple bytes simultaneously:

1. **Byte-Level Operations**: Text processed as byte arrays rather than character-by-character
2. **Parallel Scanning**: Uses `memchr` crate's SIMD-accelerated byte searching
3. **Chunk Processing**: Processes text in 64-byte chunks for cache efficiency
4. **Whitespace Detection**: Finds spaces, tabs, and newlines in parallel
5. **Punctuation Cleaning**: Removes repeated punctuation efficiently

#### Performance Benefits

| Operation | Scalar (1 byte/instruction) | SIMD (16+ bytes/instruction) | Speedup |
|-----------|----------------------------|------------------------------|---------|
| Whitespace normalization | 500 MB/s | 2000 MB/s | 4x |
| Punctuation cleaning | 300 MB/s | 900 MB/s | 3x |
| Stopword removal | 200 MB/s | 500 MB/s | 2.5x |

#### Platform Support

SIMD optimization is automatically enabled on:

- **x86_64**: SSE2, SSE4.2, AVX2 (auto-detected at runtime)
- **ARM**: NEON instructions on ARMv7+ and ARM64
- **WASM**: SIMD 128 (when compiled with `+simd128` target feature)

If SIMD is not available, the library falls back to scalar operations automatically.

#### Configuration

SIMD is enabled by default. Disable for debugging or compatibility:

=== "Rust"

    ```rust
    use kreuzberg::text::token_reduction::TokenReductionConfig;

    let config = TokenReductionConfig {
        use_simd: false,  // Disable SIMD (not recommended)
        ..Default::default()
    };
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Configuration via Kreuzberg::Config objects
    config = Kreuzberg::Config::Extraction.new(
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts result.content
    ```

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, TokenReductionConfig

    config = ExtractionConfig(
        token_reduction=TokenReductionConfig(
            use_simd=False  # Disable SIMD (not recommended)
        )
    )
    ```

=== "TypeScript"

    ```typescript
    import { ExtractionConfig } from '@goldziher/kreuzberg';

    const config: ExtractionConfig = {
      tokenReduction: {
        useSimd: false  // Disable SIMD (not recommended)
      }
    };
    ```

**Note**: Disabling SIMD significantly reduces performance (2-4x slower). Only disable for debugging or if you encounter platform-specific issues.

#### Implementation Details

The SIMD implementation uses:

- **memchr crate**: Industry-standard SIMD byte searching
- **memchr3()**: Simultaneously searches for 3 byte values (space, tab, newline)
- **Chunk-based processing**: 64-byte chunks align with CPU cache lines
- **Zero-copy operations**: Minimizes memory allocations

Example internal operation:
```rust
// SIMD searches for whitespace in 64-byte chunks
let chunk = &bytes[i..i + 64];
if let Some(ws_pos) = memchr3(b' ', b'\t', b'\n', chunk) {
    // Process whitespace found at position ws_pos
}
```

#### Benchmarks

Performance on Intel i7-10700K (8 cores @ 3.8 GHz):

| Document Size | SIMD Enabled | SIMD Disabled | Speedup |
|---------------|-------------|---------------|---------|
| 10 KB | 0.5 ms | 1.8 ms | 3.6x |
| 100 KB | 4.2 ms | 15.1 ms | 3.6x |
| 1 MB | 41 ms | 148 ms | 3.6x |
| 10 MB | 410 ms | 1,480 ms | 3.6x |

**Parallel Processing**: Combine SIMD with `enable_parallel=true` for near-linear scaling across CPU cores.

## Chunking

Kreuzberg provides intelligent text chunking using the `text-splitter` library. Split long documents into smaller chunks while preserving semantic boundaries, with support for both generic text and Markdown-aware splitting.

### Overview

The chunking system provides:

- **Smart Splitting**: Respects word, sentence, and paragraph boundaries
- **Markdown-Aware**: Preserves Markdown structure (headings, code blocks, lists, tables)
- **Configurable Overlap**: Maintain context across chunk boundaries
- **Unicode Support**: Handles CJK characters and emojis correctly
- **Two Chunker Types**: Generic Text chunker or Markdown-specific chunker
- **Standalone or with Embeddings**: Use chunking alone or combine with embedding generation

### Chunker Types

#### Text Chunker

Generic text splitter that splits on whitespace and punctuation while respecting boundaries.

**Best for**:
- Plain text documents
- Simple formatting
- Maximum flexibility
- Non-Markdown content

**Features**:
- Splits on whitespace and sentence boundaries
- Respects word boundaries
- Unicode-aware
- Simple and fast

#### Markdown Chunker

Markdown-aware splitter that preserves formatting and structure.

**Best for**:
- Markdown documents
- Technical documentation
- Blog posts and articles
- Content with code blocks and headers

**Features**:
- Preserves heading hierarchy
- Keeps code blocks intact
- Maintains list formatting
- Respects table structure
- Preserves frontmatter

### Basic Usage (Standalone Chunking)

Chunking without embeddings for general-purpose text splitting:

=== "Python"

    ```python
    from kreuzberg import extract_file_sync, ExtractionConfig
    from kreuzberg._internal_bindings import ChunkingConfig

    # Text chunker (generic)
    text_config = ChunkingConfig(
        max_chars=1000,
        max_overlap=200,
        chunker_type="text",  # Generic text splitter
    )

    config = ExtractionConfig(chunking=text_config)
    result = extract_file_sync("document.pdf", config=config)

    # Access chunks (no embeddings)
    if result.chunks:
        for i, chunk in enumerate(result.chunks):
            print(f"Chunk {i + 1}:")
            print(f"  Content: {chunk['content'][:100]}...")
            print(f"  Length: {len(chunk['content'])} chars")
            print(f"  Start: {chunk['metadata']['start_index']}")
            print(f"  End: {chunk['metadata']['end_index']}")
    ```

=== "TypeScript"

    ```typescript
    import { extractFileSync, ExtractionConfig } from '@goldziher/kreuzberg';

    // Text chunker (generic)
    const config: ExtractionConfig = {
      chunking: {
        maxChars: 1000,
        maxOverlap: 200,
        chunkerType: 'text',  // Generic text splitter
      },
    };

    const result = extractFileSync('document.pdf', { config });

    // Access chunks (no embeddings)
    if (result.chunks) {
      result.chunks.forEach((chunk, i) => {
        console.log(`Chunk ${i + 1}:`);
        console.log(`  Content: ${chunk.content.slice(0, 100)}...`);
        console.log(`  Length: ${chunk.content.length} chars`);
        console.log(`  Start: ${chunk.metadata.startIndex}`);
        console.log(`  End: ${chunk.metadata.endIndex}`);
      });
    }
    ```

=== "Rust"

    ```rust
    use kreuzberg::{extract_file_sync, ExtractionConfig};
    use kreuzberg::chunking::{ChunkingConfig, ChunkerType};

    fn main() -> kreuzberg::Result<()> {
        // Text chunker (generic)
        let chunking_config = ChunkingConfig {
            max_characters: 1000,
            overlap: 200,
            chunker_type: ChunkerType::Text,  // Generic text splitter
            trim: true,
        };

        let config = ExtractionConfig {
            chunking: Some(chunking_config),
            ..Default::default()
        };

        let result = extract_file_sync("document.pdf", None, &config)?;

        // Access chunks (no embeddings)
        if let Some(chunks) = result.chunks {
            for (i, chunk) in chunks.iter().enumerate() {
                println!("Chunk {}:", i + 1);
                println!("  Content: {}...", &chunk.content[..100.min(chunk.content.len())]);
                println!("  Length: {} chars", chunk.content.len());
                println!("  Start: {}", chunk.metadata.start_index);
                println!("  End: {}", chunk.metadata.end_index);
            }
        }

        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Configuration via Kreuzberg::Config objects
    config = Kreuzberg::Config::Extraction.new(
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts result.content
    ```

### Markdown-Aware Chunking

When processing Markdown documents, use the Markdown chunker to preserve structure:

=== "Python"

    ```python
    from kreuzberg import extract_file_sync, ExtractionConfig
    from kreuzberg._internal_bindings import ChunkingConfig

    # Markdown chunker (structure-aware)
    markdown_config = ChunkingConfig(
        max_chars=1500,
        max_overlap=150,
        chunker_type="markdown",  # Markdown-aware splitter
    )

    config = ExtractionConfig(chunking=markdown_config)
    result = extract_file_sync("documentation.md", config=config)

    # Chunks preserve Markdown structure
    if result.chunks:
        for chunk in result.chunks:
            print(chunk['content'])
            print("---")
    ```

    **Example Output**:
    ```markdown
    # Introduction

    This is the introduction section with some text.

    ## Getting Started

    Follow these steps...
    ---
    ## Advanced Features

    ### Configuration

    Configure the system by...

    ```python
    # Code block preserved intact
    config = {"key": "value"}
    ```
    ---
    ```

=== "TypeScript"

    ```typescript
    import { extractFileSync, ExtractionConfig } from '@goldziher/kreuzberg';

    // Markdown chunker (structure-aware)
    const config: ExtractionConfig = {
      chunking: {
        maxChars: 1500,
        maxOverlap: 150,
        chunkerType: 'markdown',  // Markdown-aware splitter
      },
    };

    const result = extractFileSync('documentation.md', { config });

    // Chunks preserve Markdown structure
    if (result.chunks) {
      result.chunks.forEach(chunk => {
        console.log(chunk.content);
        console.log('---');
      });
    }
    ```

=== "Rust"

    ```rust
    use kreuzberg::{extract_file_sync, ExtractionConfig};
    use kreuzberg::chunking::{ChunkingConfig, ChunkerType};

    fn main() -> kreuzberg::Result<()> {
        // Markdown chunker (structure-aware)
        let chunking_config = ChunkingConfig {
            max_characters: 1500,
            overlap: 150,
            chunker_type: ChunkerType::Markdown,  // Markdown-aware splitter
            trim: true,
        };

        let config = ExtractionConfig {
            chunking: Some(chunking_config),
            ..Default::default()
        };

        let result = extract_file_sync("documentation.md", None, &config)?;

        // Chunks preserve Markdown structure
        if let Some(chunks) = result.chunks {
            for chunk in chunks {
                println!("{}", chunk.content);
                println!("---");
            }
        }

        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Configuration via Kreuzberg::Config objects
    config = Kreuzberg::Config::Extraction.new(
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts result.content
    ```

### Markdown Structure Preservation

The Markdown chunker intelligently preserves document structure:

**Headings**: Chunks start at heading boundaries when possible
```markdown
# Chapter 1
Content for chapter 1...

## Section 1.1
Subsection content...
```

**Code Blocks**: Never split, always kept intact
````markdown
```python
def example():
    # Multi-line code block
    # kept together
    return "intact"
```
````

**Lists**: Preserved with proper indentation
```markdown
- Item 1
  - Nested item
- Item 2
```

**Tables**: Kept intact when possible
```markdown
| Header 1 | Header 2 |
|----------|----------|
| Cell 1   | Cell 2   |
```

### Chunk Size Strategies

Choose chunk size based on your use case:

| Chunk Size | Use Case | Pros | Cons |
|------------|----------|------|------|
| **250-500 chars** | Fine-grained search | Precise retrieval, low memory | Many chunks, more overhead |
| **500-1000 chars** | Balanced RAG | Good granularity, efficient | Standard approach |
| **1000-2000 chars** | Context-aware | Rich context, fewer chunks | Less precise retrieval |
| **2000-4000 chars** | LLM context | Maximum context, minimal chunks | May lose precision |

**Recommendation**: Start with 1000 characters and adjust based on retrieval quality.

### Overlap Configuration

Overlap maintains context across chunk boundaries:

**Overlap Ratio Guidelines**:
- **10-15%**: Minimal overlap, good for independent chunks
- **15-20%**: Standard overlap, balances context and efficiency
- **20-30%**: High overlap, maximum context preservation

**Example**:
```python
# 20% overlap (recommended)
chunk_size = 1000
overlap = 200  # 20%

# Ensures context continuity across chunks
config = ChunkingConfig(max_chars=chunk_size, max_overlap=overlap)
```

**Important**: `max_overlap` must be less than `max_chars`

=== "Python"

    ```python
    # ❌ Invalid: overlap >= chunk size
    invalid = ChunkingConfig(max_chars=100, max_overlap=200)  # ERROR

    # ✅ Valid: 20% overlap
    valid = ChunkingConfig(max_chars=1000, max_overlap=200)  # OK
    ```

=== "TypeScript"

    ```typescript
    // ❌ Invalid: overlap >= chunk size
    const invalid = { chunking: { maxChars: 100, maxOverlap: 200 } };  // ERROR

    // ✅ Valid: 20% overlap
    const valid = { chunking: { maxChars: 1000, maxOverlap: 200 } };  // OK
    ```

=== "Rust"

    ```rust
    // ❌ Invalid: overlap >= chunk size
    let invalid = ChunkingConfig {
        max_characters: 100,
        overlap: 200,  // ERROR: Will fail validation
        ..Default::default()
    };

    // ✅ Valid: 20% overlap
    let valid = ChunkingConfig {
        max_characters: 1000,
        overlap: 200,  // OK
        ..Default::default()
    };
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Configuration via Kreuzberg::Config objects
    config = Kreuzberg::Config::Extraction.new(
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts result.content
    ```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_chars` | int | 2000 | Maximum characters per chunk |
| `max_overlap` | int | 100 | Overlap between chunks (must be < max_chars) |
| `chunker_type` | `"text"` \| `"markdown"` | `"text"` | Chunker type to use |
| `trim` | bool | `true` | Trim whitespace from chunks |

### Use Cases

**Standalone Chunking** (no embeddings):
- Splitting documents for LLM context windows
- Processing large documents in batches
- Text segmentation for analysis
- Content pagination

**With Embeddings** (see next section):
- RAG (Retrieval-Augmented Generation) pipelines
- Semantic search
- Document similarity
- Vector database ingestion

## Chunking with Embeddings

Kreuzberg provides built-in chunking capabilities with optional embedding generation for RAG (Retrieval-Augmented Generation) applications. Text can be split into chunks with configurable overlap, and each chunk can have embeddings generated using fastembed models.

### Overview

The chunking system provides:

- **Text Splitting**: Split long documents into smaller chunks with configurable size and overlap
- **Embedding Generation**: Generate embeddings for each chunk using fastembed models
- **Multiple Presets**: Fast, balanced, quality, and multilingual embedding models
- **Custom Models**: Support for custom fastembed models with configurable dimensions
- **Cache Support**: Optional caching of embedding models for faster initialization
- **Validation**: Automatic validation of chunking configuration (overlap < chunk size)

### Model Presets

Kreuzberg provides four embedding model presets optimized for different use cases:

| Preset | Model | Dimensions | Use Case |
|--------|-------|------------|----------|
| `fast` | AllMiniLML6V2Q | 384 | Quick prototyping, low-latency applications |
| `balanced` | BGEBaseENV15 | 768 | General-purpose RAG, good balance of speed and quality |
| `quality` | BGELargeENV15 | 1024 | High-quality embeddings, semantic search |
| `multilingual` | MultilingualE5Base | 768 | Multi-language support, international documents |

### Basic Usage

=== "Python"

    ```python
    from kreuzberg import extract_file_sync, ExtractionConfig
    from kreuzberg._internal_bindings import (
        ChunkingConfig,
        EmbeddingConfig,
        EmbeddingModelType,
    )

    # Configure chunking with embeddings
    embedding_config = EmbeddingConfig(
        model=EmbeddingModelType.preset("fast"),
        normalize=True,
        batch_size=32,
    )

    chunking_config = ChunkingConfig(
        max_chars=1000,
        max_overlap=200,  # Must be < max_chars
        embedding=embedding_config,
    )

    config = ExtractionConfig(chunking=chunking_config)

    # Extract with chunking and embeddings
    result = extract_file_sync("document.pdf", config=config)

    # Access chunks with embeddings
    if result.chunks:
        for chunk in result.chunks:
            print(f"Content: {chunk['content'][:100]}...")
            print(f"Embedding dimensions: {len(chunk['embedding'])}")
            print(f"Metadata: {chunk['metadata']}")
    ```

=== "TypeScript"

    ```typescript
    import { extractFileSync, ExtractionConfig } from '@goldziher/kreuzberg';

    // Configure chunking with embeddings
    const config: ExtractionConfig = {
      chunking: {
        maxChars: 1000,
        maxOverlap: 200,  // Must be < maxChars
        embedding: {
          model: {
            modelType: 'preset',
            value: 'fast',
          },
          normalize: true,
          batchSize: 32,
        },
      },
    };

    // Extract with chunking and embeddings
    const result = extractFileSync('document.pdf', { config });

    // Access chunks with embeddings
    if (result.chunks) {
      for (const chunk of result.chunks) {
        console.log(`Content: ${chunk.content.slice(0, 100)}...`);
        console.log(`Embedding dimensions: ${chunk.embedding?.length}`);
        console.log(`Metadata:`, chunk.metadata);
      }
    }
    ```

=== "Rust"

    ```rust
    use kreuzberg::{extract_file_sync, ExtractionConfig};
    use kreuzberg::chunking::{ChunkingConfig, EmbeddingConfig, EmbeddingModelType};

    fn main() -> kreuzberg::Result<()> {
        // Configure chunking with embeddings
        let embedding_config = EmbeddingConfig {
            model: Some(EmbeddingModelType::Preset("fast".to_string())),
            normalize: true,
            batch_size: 32,
            ..Default::default()
        };

        let chunking_config = ChunkingConfig {
            max_chars: 1000,
            max_overlap: 200,  // Must be < max_chars
            embedding: Some(embedding_config),
            ..Default::default()
        };

        let config = ExtractionConfig {
            chunking: Some(chunking_config),
            ..Default::default()
        };

        // Extract with chunking and embeddings
        let result = extract_file_sync("document.pdf", None, &config)?;

        // Access chunks with embeddings
        if let Some(chunks) = result.chunks {
            for chunk in chunks {
                println!("Content: {}...", &chunk.content[..100.min(chunk.content.len())]);
                if let Some(embedding) = chunk.embedding {
                    println!("Embedding dimensions: {}", embedding.len());
                }
                println!("Metadata: {:?}", chunk.metadata);
            }
        }

        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Configuration via Kreuzberg::Config objects
    config = Kreuzberg::Config::Extraction.new(
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts result.content
    ```

### Model Selection

Choose the appropriate model preset based on your needs:

=== "Python"

    ```python
    from kreuzberg._internal_bindings import EmbeddingModelType

    # Fast: 384-dimensional embeddings (AllMiniLML6V2Q)
    fast_model = EmbeddingModelType.preset("fast")

    # Balanced: 768-dimensional embeddings (BGEBaseENV15)
    balanced_model = EmbeddingModelType.preset("balanced")

    # Quality: 1024-dimensional embeddings (BGELargeENV15)
    quality_model = EmbeddingModelType.preset("quality")

    # Multilingual: 768-dimensional embeddings (MultilingualE5Base)
    multilingual_model = EmbeddingModelType.preset("multilingual")
    ```

=== "TypeScript"

    ```typescript
    // Fast: 384-dimensional embeddings
    const fastModel = { modelType: 'preset', value: 'fast' };

    // Balanced: 768-dimensional embeddings
    const balancedModel = { modelType: 'preset', value: 'balanced' };

    // Quality: 1024-dimensional embeddings
    const qualityModel = { modelType: 'preset', value: 'quality' };

    // Multilingual: 768-dimensional embeddings
    const multilingualModel = { modelType: 'preset', value: 'multilingual' };
    ```

=== "Rust"

    ```rust
    use kreuzberg::chunking::EmbeddingModelType;

    // Fast: 384-dimensional embeddings
    let fast_model = EmbeddingModelType::Preset("fast".to_string());

    // Balanced: 768-dimensional embeddings
    let balanced_model = EmbeddingModelType::Preset("balanced".to_string());

    // Quality: 1024-dimensional embeddings
    let quality_model = EmbeddingModelType::Preset("quality".to_string());

    // Multilingual: 768-dimensional embeddings
    let multilingual_model = EmbeddingModelType::Preset("multilingual".to_string());
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Configuration via Kreuzberg::Config objects
    config = Kreuzberg::Config::Extraction.new(
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts result.content
    ```

### Custom Embedding Models

Use custom fastembed models for specialized use cases:

=== "Python"

    ```python
    from kreuzberg._internal_bindings import EmbeddingConfig, EmbeddingModelType

    # Custom fastembed model
    custom_model = EmbeddingModelType.fastembed("BGEBaseENV15", 768)

    embedding_config = EmbeddingConfig(
        model=custom_model,
        normalize=True,
        batch_size=16,
    )
    ```

=== "TypeScript"

    ```typescript
    // Custom fastembed model
    const customModel = {
      modelType: 'fastembed',
      value: 'BGEBaseENV15',
      dimensions: 768,
    };

    const embeddingConfig = {
      model: customModel,
      normalize: true,
      batchSize: 16,
    };
    ```

=== "Rust"

    ```rust
    use kreuzberg::chunking::{EmbeddingConfig, EmbeddingModelType};

    // Custom fastembed model
    let custom_model = EmbeddingModelType::Fastembed {
        model_name: "BGEBaseENV15".to_string(),
        dimensions: 768,
    };

    let embedding_config = EmbeddingConfig {
        model: Some(custom_model),
        normalize: true,
        batch_size: 16,
        ..Default::default()
    };
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Configuration via Kreuzberg::Config objects
    config = Kreuzberg::Config::Extraction.new(
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts result.content
    ```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_chars` | int | 1000 | Maximum characters per chunk |
| `max_overlap` | int | 200 | Overlap between chunks (must be < max_chars) |
| `preset` | str | None | Chunking preset (fast, balanced, quality, multilingual) |
| `embedding.model` | EmbeddingModelType | None | Embedding model to use |
| `embedding.normalize` | bool | `true` | Normalize embeddings to unit length |
| `embedding.batch_size` | int | 32 | Batch size for embedding generation |
| `embedding.show_download_progress` | bool | `false` | Show model download progress |
| `embedding.cache_dir` | str | `.kreuzberg/embeddings` | Custom cache directory for models |

### Advanced Configuration

=== "Python"

    ```python
    from kreuzberg import extract_file_sync, ExtractionConfig
    from kreuzberg._internal_bindings import (
        ChunkingConfig,
        EmbeddingConfig,
        EmbeddingModelType,
    )

    # Advanced configuration
    embedding_config = EmbeddingConfig(
        model=EmbeddingModelType.preset("quality"),
        normalize=True,
        batch_size=64,
        show_download_progress=True,
        cache_dir="/tmp/kreuzberg_models",
    )

    chunking_config = ChunkingConfig(
        max_chars=2000,
        max_overlap=400,
        embedding=embedding_config,
    )

    config = ExtractionConfig(chunking=chunking_config)
    result = extract_file_sync("large_document.pdf", config=config)
    ```

=== "TypeScript"

    ```typescript
    import { extractFileSync, ExtractionConfig } from '@goldziher/kreuzberg';

    // Advanced configuration
    const config: ExtractionConfig = {
      chunking: {
        maxChars: 2000,
        maxOverlap: 400,
        embedding: {
          model: { modelType: 'preset', value: 'quality' },
          normalize: true,
          batchSize: 64,
          showDownloadProgress: true,
          cacheDir: '/tmp/kreuzberg_models',
        },
      },
    };

    const result = extractFileSync('large_document.pdf', { config });
    ```

=== "Rust"

    ```rust
    use kreuzberg::{extract_file_sync, ExtractionConfig};
    use kreuzberg::chunking::{ChunkingConfig, EmbeddingConfig, EmbeddingModelType};

    fn main() -> kreuzberg::Result<()> {
        // Advanced configuration
        let embedding_config = EmbeddingConfig {
            model: Some(EmbeddingModelType::Preset("quality".to_string())),
            normalize: true,
            batch_size: 64,
            show_download_progress: true,
            cache_dir: Some("/tmp/kreuzberg_models".to_string()),
            ..Default::default()
        };

        let chunking_config = ChunkingConfig {
            max_chars: 2000,
            max_overlap: 400,
            embedding: Some(embedding_config),
            ..Default::default()
        };

        let config = ExtractionConfig {
            chunking: Some(chunking_config),
            ..Default::default()
        };

        let result = extract_file_sync("large_document.pdf", None, &config)?;
        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Configuration via Kreuzberg::Config objects
    config = Kreuzberg::Config::Extraction.new(
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts result.content
    ```

### Normalization

Normalized embeddings have unit length (L2 norm = 1.0), which is useful for:

- **Cosine similarity**: Normalized embeddings enable efficient cosine similarity using dot product
- **Distance metrics**: Consistent distance measurements across different models
- **Vector databases**: Most vector databases expect normalized embeddings

=== "Python"

    ```python
    # Check normalization
    import math

    result = extract_file_sync("document.pdf", config=config)
    if result.chunks and result.chunks[0]['embedding']:
        embedding = result.chunks[0]['embedding']
        magnitude = math.sqrt(sum(x * x for x in embedding))
        print(f"L2 norm: {magnitude:.6f}")  # Should be ~1.0
    ```

=== "TypeScript"

    ```typescript
    // Check normalization
    const result = extractFileSync('document.pdf', { config });
    if (result.chunks && result.chunks[0].embedding) {
      const embedding = result.chunks[0].embedding;
      const magnitude = Math.sqrt(
        embedding.reduce((sum, x) => sum + x * x, 0)
      );
      console.log(`L2 norm: ${magnitude.toFixed(6)}`);  // Should be ~1.0
    }
    ```

### Chunking Validation

The chunking system validates configuration to prevent errors:

**Important**: `max_overlap` must be less than `max_chars`

=== "Python"

    ```python
    # ❌ Invalid: overlap >= chunk size
    invalid_config = ChunkingConfig(
        max_chars=100,
        max_overlap=200,  # ERROR: Must be < 100
    )

    # ✅ Valid: overlap < chunk size
    valid_config = ChunkingConfig(
        max_chars=100,
        max_overlap=20,  # OK: 20% overlap
    )

    # ✅ Recommended: 20% overlap
    def create_chunking_config(chunk_size: int) -> ChunkingConfig:
        overlap = min(int(chunk_size * 0.2), chunk_size - 1)
        return ChunkingConfig(
            max_chars=chunk_size,
            max_overlap=overlap,
        )
    ```

=== "TypeScript"

    ```typescript
    // ❌ Invalid: overlap >= chunk size
    const invalidConfig = {
      chunking: {
        maxChars: 100,
        maxOverlap: 200,  // ERROR: Must be < 100
      },
    };

    // ✅ Valid: overlap < chunk size
    const validConfig = {
      chunking: {
        maxChars: 100,
        maxOverlap: 20,  // OK: 20% overlap
      },
    };

    // ✅ Recommended: 20% overlap
    function createChunkingConfig(chunkSize: number) {
      const overlap = Math.min(Math.floor(chunkSize * 0.2), chunkSize - 1);
      return {
        chunking: {
          maxChars: chunkSize,
          maxOverlap: overlap,
        },
      };
    }
    ```

### Performance Considerations

- **Model Download**: First use downloads models (~100-500MB depending on preset)
- **Caching**: Models are cached automatically for subsequent use
- **Batch Size**: Larger batches (64-128) are faster but use more memory
- **Normalization**: Minimal performance impact, recommended for most use cases
- **Chunk Size**: Smaller chunks (500-1000 chars) provide better granularity for RAG

### Best Practices

1. **Choose the right model preset**:
   - `fast` for prototyping and low-latency applications
   - `balanced` for general-purpose RAG systems
   - `quality` for semantic search and high-accuracy retrieval
   - `multilingual` for international documents

2. **Configure chunk size appropriately**:
   - 500-1000 chars for granular RAG retrieval
   - 1000-2000 chars for context-aware chunking
   - Always ensure `max_overlap < max_chars`

3. **Use normalization**:
   - Enable `normalize=True` for cosine similarity
   - Consistent distance metrics across models

4. **Cache models**:
   - Set `cache_dir` for persistent model caching
   - Reduces initialization time for repeated use

5. **Monitor performance**:
   - Adjust `batch_size` based on available memory
   - Use `show_download_progress` for transparency

### Batch Processing Multiple Documents

When processing multiple documents, Kreuzberg automatically handles batch embedding generation efficiently through model caching and reuse:

=== "Python"

    ```python
    from kreuzberg import batch_extract_files_sync, ExtractionConfig
    from kreuzberg._internal_bindings import ChunkingConfig, EmbeddingConfig, EmbeddingModelType

    # Configure once
    embedding_config = EmbeddingConfig(
        model=EmbeddingModelType.preset("balanced"),
        normalize=True,
        batch_size=64,  # Larger batch for multi-document processing
    )

    chunking_config = ChunkingConfig(
        max_chars=1000,
        max_overlap=200,
        embedding=embedding_config,
    )

    config = ExtractionConfig(chunking=chunking_config)

    # Process multiple documents in batch
    files = ["doc1.pdf", "doc2.pdf", "doc3.pdf", "doc4.pdf", "doc5.pdf"]
    results = batch_extract_files_sync(files, config=config)

    # Model is initialized once and reused across all documents
    for i, result in enumerate(results):
        print(f"Document {i + 1}: {len(result.chunks)} chunks")
        if result.chunks and result.chunks[0].get('embedding'):
            print(f"  First chunk embedding: {len(result.chunks[0]['embedding'])} dimensions")
    ```

    **Performance Benefits**:
    - Model initialized once and reused (saves 2-5 seconds per document)
    - Batch embedding generation within each document
    - Concurrent document processing (configurable max_workers)

=== "TypeScript"

    ```typescript
    import { batchExtractFilesSync, ExtractionConfig } from '@goldziher/kreuzberg';

    // Configure once
    const config: ExtractionConfig = {
      chunking: {
        maxChars: 1000,
        maxOverlap: 200,
        embedding: {
          model: { modelType: 'preset', value: 'balanced' },
          normalize: true,
          batchSize: 64,  // Larger batch for multi-document processing
        },
      },
    };

    // Process multiple documents in batch
    const files = ['doc1.pdf', 'doc2.pdf', 'doc3.pdf', 'doc4.pdf', 'doc5.pdf'];
    const results = batchExtractFilesSync(files, { config });

    // Model is initialized once and reused across all documents
    results.forEach((result, i) => {
      console.log(`Document ${i + 1}: ${result.chunks?.length} chunks`);
      if (result.chunks && result.chunks[0]?.embedding) {
        console.log(`  First chunk embedding: ${result.chunks[0].embedding.length} dimensions`);
      }
    });
    ```

=== "Rust"

    ```rust
    use kreuzberg::{batch_extract_files_sync, ExtractionConfig};
    use kreuzberg::chunking::{ChunkingConfig, EmbeddingConfig, EmbeddingModelType};

    fn main() -> kreuzberg::Result<()> {
        // Configure once
        let embedding_config = EmbeddingConfig {
            model: Some(EmbeddingModelType::Preset("balanced".to_string())),
            normalize: true,
            batch_size: 64,  // Larger batch for multi-document processing
            ..Default::default()
        };

        let chunking_config = ChunkingConfig {
            max_chars: 1000,
            max_overlap: 200,
            embedding: Some(embedding_config),
            ..Default::default()
        };

        let config = ExtractionConfig {
            chunking: Some(chunking_config),
            ..Default::default()
        };

        // Process multiple documents in batch
        let files = vec!["doc1.pdf", "doc2.pdf", "doc3.pdf", "doc4.pdf", "doc5.pdf"];
        let results = batch_extract_files_sync(&files, None, &config)?;

        // Model is initialized once and reused across all documents
        for (i, result) in results.iter().enumerate() {
            if let Some(ref chunks) = result.chunks {
                println!("Document {}: {} chunks", i + 1, chunks.len());
                if let Some(ref embedding) = chunks[0].embedding {
                    println!("  First chunk embedding: {} dimensions", embedding.len());
                }
            }
        }

        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Batch processing with semantic chunking
    files = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]

    semantic_config = Kreuzberg::Config::SemanticChunking.new(
      enabled: true,
      model_preset: "sentence_transformers"
    )

    config = Kreuzberg::Config::Extraction.new(
      semantic_chunking: semantic_config
    )

    results = Kreuzberg.batch_extract_files_sync(files, config: config)

    results.each do |result|
      puts "File: #{result.file_path}"
      puts "Chunks: #{result.chunks&.size || 0}"
    end
    ```

### Memory Usage and Optimization

Understanding memory usage helps optimize embeddings performance for your use case:

#### Model Memory Footprint

| Model Preset | Model Size | RAM Usage (Inference) | ONNX Runtime Overhead |
|--------------|------------|----------------------|----------------------|
| `fast` | ~22M params (~90 MB) | ~150 MB | ~50 MB |
| `balanced` | ~109M params (~450 MB) | ~600 MB | ~50 MB |
| `quality` | ~335M params (~1.3 GB) | ~1.8 GB | ~50 MB |
| `multilingual` | ~280M params (~1.1 GB) | ~1.5 GB | ~50 MB |

**Notes**:
- Model cache directory: `.kreuzberg/embeddings/` by default
- Models downloaded once and cached persistently
- RAM usage scales with batch size (~10-20 MB per 1000 embeddings)

#### Batch Size Optimization

| Batch Size | Memory Impact | Speed Impact | Use Case |
|------------|---------------|--------------|----------|
| 8-16 | Low (~100 MB) | Slower | Memory-constrained environments |
| 32-64 | Medium (~200-400 MB) | Optimal | Recommended for most use cases |
| 128-256 | High (~800 MB-1.5 GB) | Fastest | High-memory servers, batch processing |

**Formula**: `memory_per_batch ≈ batch_size × embedding_dims × 4 bytes`

**Example Calculation**:
```
Balanced model (768 dims), batch size 64:
64 × 768 × 4 = 196,608 bytes ≈ 192 KB per batch
```

#### Performance Tuning

=== "Python"

    ```python
    from kreuzberg import extract_file_sync, ExtractionConfig
    from kreuzberg._internal_bindings import ChunkingConfig, EmbeddingConfig, EmbeddingModelType
    import psutil
    import os

    # Automatically adjust batch size based on available memory
    def get_optimal_batch_size():
        available_ram_gb = psutil.virtual_memory().available / (1024 ** 3)
        if available_ram_gb < 4:
            return 16  # Low memory
        elif available_ram_gb < 8:
            return 32  # Medium memory
        elif available_ram_gb < 16:
            return 64  # Good memory
        else:
            return 128  # High memory server

    # Memory-efficient configuration
    embedding_config = EmbeddingConfig(
        model=EmbeddingModelType.preset("fast"),  # Smallest model
        normalize=True,
        batch_size=get_optimal_batch_size(),
        cache_dir=os.path.expanduser("~/.cache/kreuzberg/embeddings"),
    )

    chunking_config = ChunkingConfig(
        max_chars=500,  # Smaller chunks for memory efficiency
        max_overlap=50,
        embedding=embedding_config,
    )

    config = ExtractionConfig(chunking=chunking_config)
    result = extract_file_sync("document.pdf", config=config)
    ```

=== "TypeScript"

    ```typescript
    import { extractFileSync, ExtractionConfig } from '@goldziher/kreuzberg';
    import * as os from 'os';

    // Automatically adjust batch size based on available memory
    function getOptimalBatchSize(): number {
      const availableRamGb = os.freemem() / (1024 ** 3);
      if (availableRamGb < 4) return 16;  // Low memory
      if (availableRamGb < 8) return 32;  // Medium memory
      if (availableRamGb < 16) return 64; // Good memory
      return 128;  // High memory server
    }

    // Memory-efficient configuration
    const config: ExtractionConfig = {
      chunking: {
        maxChars: 500,  // Smaller chunks for memory efficiency
        maxOverlap: 50,
        embedding: {
          model: { modelType: 'preset', value: 'fast' },  // Smallest model
          normalize: true,
          batchSize: getOptimalBatchSize(),
          cacheDir: `${process.env.HOME}/.cache/kreuzberg/embeddings`,
        },
      },
    };

    const result = extractFileSync('document.pdf', { config });
    ```

=== "Rust"

    ```rust
    use kreuzberg::{extract_file_sync, ExtractionConfig};
    use kreuzberg::chunking::{ChunkingConfig, EmbeddingConfig, EmbeddingModelType};
    use sysinfo::System;

    fn get_optimal_batch_size() -> usize {
        let mut sys = System::new_all();
        sys.refresh_memory();
        let available_ram_gb = sys.available_memory() as f64 / (1024.0 * 1024.0 * 1024.0);

        match available_ram_gb {
            x if x < 4.0 => 16,   // Low memory
            x if x < 8.0 => 32,   // Medium memory
            x if x < 16.0 => 64,  // Good memory
            _ => 128,             // High memory server
        }
    }

    fn main() -> kreuzberg::Result<()> {
        // Memory-efficient configuration
        let embedding_config = EmbeddingConfig {
            model: Some(EmbeddingModelType::Preset("fast".to_string())),
            normalize: true,
            batch_size: get_optimal_batch_size(),
            cache_dir: Some(format!("{}/.cache/kreuzberg/embeddings",
                std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))),
            ..Default::default()
        };

        let chunking_config = ChunkingConfig {
            max_chars: 500,  // Smaller chunks for memory efficiency
            max_overlap: 50,
            embedding: Some(embedding_config),
            ..Default::default()
        };

        let config = ExtractionConfig {
            chunking: Some(chunking_config),
            ..Default::default()
        };

        let result = extract_file_sync("document.pdf", None, &config)?;
        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Configuration via Kreuzberg::Config objects
    config = Kreuzberg::Config::Extraction.new(
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    puts result.content
    ```

**Performance Tips**:

1. **Model Selection**: Start with `fast` preset, upgrade to `balanced` only if accuracy improves retrieval
2. **Chunk Size**: Smaller chunks (500-1000 chars) reduce memory per-batch but increase total batches
3. **Batch Size**: Monitor memory usage and adjust batch_size accordingly
4. **Caching**: Always use a persistent cache_dir for repeated processing
5. **Concurrency**: Use batch_extract_files() for concurrent document processing with model reuse

## Stopwords

Stopwords are common words filtered out during text analysis. Kreuzberg provides comprehensive built-in stopword collections for 64 languages, embedded in the binary at compile time for zero-overhead access.

### Supported Languages

Kreuzberg includes stopword collections for the following 64 languages:

**European Languages**: Afrikaans (`af`), Bulgarian (`bg`), Breton (`br`), Catalan (`ca`), Czech (`cs`), Danish (`da`), German (`de`), Greek (`el`), English (`en`), Esperanto (`eo`), Spanish (`es`), Estonian (`et`), Basque (`eu`), Finnish (`fi`), French (`fr`), Irish (`ga`), Galician (`gl`), Croatian (`hr`), Hungarian (`hu`), Italian (`it`), Latin (`la`), Lithuanian (`lt`), Latvian (`lv`), Dutch (`nl`), Norwegian (`no`), Polish (`pl`), Portuguese (`pt`), Romanian (`ro`), Slovak (`sk`), Slovenian (`sl`), Sesotho (`st`), Swedish (`sv`), Ukrainian (`uk`)

**Asian & Middle Eastern Languages**: Arabic (`ar`), Bengali (`bn`), Persian (`fa`), Hebrew (`he`), Hindi (`hi`), Armenian (`hy`), Indonesian (`id`), Japanese (`ja`), Kannada (`kn`), Korean (`ko`), Kurdish (`ku`), Malayalam (`ml`), Marathi (`mr`), Malay (`ms`), Nepali (`ne`), Sinhala (`si`), Somali (`so`), Swahili (`sw`), Tamil (`ta`), Telugu (`te`), Thai (`th`), Tagalog (`tl`), Turkish (`tr`), Urdu (`ur`), Vietnamese (`vi`), Chinese (`zh`)

**African Languages**: Hausa (`ha`), Somali (`so`), Sesotho (`st`), Swahili (`sw`), Yoruba (`yo`), Zulu (`zu`)

**Compile-Time Embedding**: All stopword lists are embedded in the Rust binary at compile time using `include_str!()` macro, eliminating runtime file I/O and deployment dependencies.

### Direct Access (Rust)

=== "Rust"

    ```rust
    use kreuzberg::stopwords::STOPWORDS;

    fn main() {
        // Access English stopwords
        if let Some(en_stopwords) = STOPWORDS.get("en") {
            println!("English stopwords: {}", en_stopwords.len());
            if en_stopwords.contains("the") {
                println!("'the' is a stopword");
            }
        }

        // Access Spanish stopwords
        if let Some(es_stopwords) = STOPWORDS.get("es") {
            println!("Spanish stopwords: {}", es_stopwords.len());
            if es_stopwords.contains("el") {
                println!("'el' is a stopword");
            }
        }
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Stopwords are used internally during token reduction
    # Configure via token reduction settings
    token_config = Kreuzberg::Config::TokenReduction.new(
      enabled: true,
      remove_stopwords: true,
      language: "eng"  # English stopwords
    )

    config = Kreuzberg::Config::Extraction.new(
      token_reduction: token_config
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)
    ```

### Usage in Text Processing

Stopwords are automatically used by:

- **Keyword Extraction**: Filters stopwords before extracting keywords with YAKE or RAKE
- **Token Reduction**: Removes stopwords to reduce token count while preserving meaning
- **Language-Specific Processing**: Automatically selects stopwords based on detected or configured language

No additional configuration is required - stopwords are loaded automatically when needed.

## Language Detection

Automatically detect languages in extracted text using the fast whatlang library. Language detection helps with downstream processing, routing documents, and understanding content composition.

### Overview

Language detection provides:

- **Fast Detection**: Powered by whatlang library (100+ languages, 0.5-2ms per document)
- **High Accuracy**: Statistical n-gram based detection with confidence scores
- **Single Language Mode**: Detect the primary language in a document
- **Multi-Language Mode**: Detect all languages in multilingual documents
- **Confidence Thresholds**: Filter low-confidence detections
- **ISO 639-3 Codes**: Returns standardized 3-letter language codes
- **Zero Dependencies**: Compiled into Rust core, no external services

### Supported Languages

Whatlang supports 80+ languages including:

**European**: English, Spanish, French, German, Italian, Portuguese, Russian, Polish, Dutch, Swedish, Danish, Norwegian, Finnish, Czech, Romanian, Hungarian, Turkish, Greek, and more

**Asian**: Chinese (Mandarin), Japanese, Korean, Hindi, Arabic, Hebrew, Persian, Thai, Vietnamese

**Others**: Indonesian, Malay, Filipino, Swahili, and many regional languages

See [whatlang language list](https://github.com/greyblake/whatlang-rs#supported-languages) for complete coverage.

### Basic Usage

Enable language detection in extraction configuration:

=== "Python"

    ```python
    from kreuzberg import extract_file_sync, ExtractionConfig
    from kreuzberg._internal_bindings import LanguageDetectionConfig

    # Enable language detection
    config = ExtractionConfig(
        language_detection=LanguageDetectionConfig(
            enabled=True,
            min_confidence=0.8,
            detect_multiple=False
        )
    )

    result = extract_file_sync("document.pdf", config=config)

    if result.detected_languages:
        print(f"Detected languages: {result.detected_languages}")
        primary = result.detected_languages[0]
        print(f"Primary language: {primary}")
    ```

=== "TypeScript"

    ```typescript
    import { extractFileSync, ExtractionConfig } from '@goldziher/kreuzberg';

    // Enable language detection
    const config: ExtractionConfig = {
      languageDetection: {
        enabled: true,
        minConfidence: 0.8,
        detectMultiple: false
      }
    };

    const result = extractFileSync('document.pdf', null, config);

    if (result.detectedLanguages) {
      console.log(`Detected languages: ${result.detectedLanguages}`);
      const primary = result.detectedLanguages[0];
      console.log(`Primary language: ${primary}`);
    }
    ```

=== "Rust"

    ```rust
    use kreuzberg::{extract_file_sync, ExtractionConfig};
    use kreuzberg::core::config::LanguageDetectionConfig;

    fn main() -> kreuzberg::Result<()> {
        // Enable language detection
        let config = ExtractionConfig {
            language_detection: Some(LanguageDetectionConfig {
                enabled: true,
                min_confidence: 0.8,
                detect_multiple: false,
            }),
            ..Default::default()
        };

        let result = extract_file_sync("document.pdf", None, &config)?;

        if let Some(languages) = result.detected_languages {
            println!("Detected languages: {:?}", languages);
            println!("Primary language: {}", languages[0]);
        }

        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Configure confidence threshold
    lang_config = Kreuzberg::Config::LanguageDetection.new(
      enabled: true,
      min_confidence: 0.9  # High confidence threshold
    )

    config = Kreuzberg::Config::Extraction.new(
      language_detection: lang_config
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)

    result.detected_languages&.each do |lang|
      puts "#{lang.lang}: #{lang.confidence}" if lang.confidence >= 0.9
    end
    ```

=== "CLI"

    ```bash
    # Enable language detection
    kreuzberg extract document.pdf --detect-language

    # With JSON output to see detected languages
    kreuzberg extract document.pdf --detect-language --format json
    ```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | `false` | Enable language detection |
| `min_confidence` | float | 0.8 | Minimum confidence threshold (0.0-1.0) |
| `detect_multiple` | bool | `false` | Detect multiple languages in document |

### Single vs Multi-Language Detection

#### Single Language Mode (default)

Detects the primary language of the entire document:

=== "Python"

    ```python
    config = ExtractionConfig(
        language_detection=LanguageDetectionConfig(
            enabled=True,
            detect_multiple=False  # Single language mode
        )
    )

    result = extract_file_sync("document.pdf", config=config)
    # result.detected_languages = ["eng"]  # Primary language only
    ```

**Use when:**
- Document is primarily in one language
- You need the main language for routing/processing
- Performance is critical (single detection is faster)

#### Multi-Language Mode

Analyzes document in chunks to detect all present languages:

=== "Python"

    ```python
    config = ExtractionConfig(
        language_detection=LanguageDetectionConfig(
            enabled=True,
            detect_multiple=True  # Multi-language mode
        )
    )

    result = extract_file_sync("multilingual.pdf", config=config)
    # result.detected_languages = ["eng", "spa", "fra"]  # Sorted by frequency
    ```

**Use when:**
- Document contains multiple languages
- Analyzing international content
- Building language-aware processing pipelines

**How it works:**
- Splits text into 500-character chunks
- Detects language of each chunk
- Returns languages sorted by frequency
- Applies confidence threshold per chunk

### Confidence Thresholds

Control detection sensitivity with confidence thresholds:

=== "Python"

    ```python
    # High confidence (stricter, fewer false positives)
    strict_config = LanguageDetectionConfig(
        enabled=True,
        min_confidence=0.9  # Only accept high-confidence detections
    )

    # Low confidence (more permissive, may include uncertain detections)
    permissive_config = LanguageDetectionConfig(
        enabled=True,
        min_confidence=0.6  # Accept lower-confidence detections
    )
    ```

**Recommended thresholds:**
- `0.9-1.0`: Strict - Only very confident detections (formal documents, single language)
- `0.7-0.9`: Balanced - Good for most documents (default: 0.8)
- `0.5-0.7`: Permissive - Accept uncertain detections (short texts, mixed content)
- `<0.5`: Not recommended - Many false positives

### Advanced Examples

#### Routing Documents by Language

=== "Python"

    ```python
    from kreuzberg import extract_file_sync, ExtractionConfig
    from kreuzberg._internal_bindings import LanguageDetectionConfig

    def route_document(file_path: str) -> str:
        config = ExtractionConfig(
            language_detection=LanguageDetectionConfig(enabled=True)
        )
        result = extract_file_sync(file_path, config=config)

        if result.detected_languages:
            lang = result.detected_languages[0]
            if lang == "eng":
                return "english_queue"
            elif lang in ["spa", "por"]:
                return "spanish_queue"
            elif lang in ["fra", "deu", "ita"]:
                return "european_queue"
            else:
                return "other_queue"
        return "unknown_queue"
    ```

#### Combining with OCR

=== "Python"

    ```python
    from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig
    from kreuzberg._internal_bindings import LanguageDetectionConfig

    # First pass: detect language
    detect_config = ExtractionConfig(
        language_detection=LanguageDetectionConfig(enabled=True)
    )
    initial_result = extract_file_sync("scanned.pdf", config=detect_config)

    # Second pass: OCR with detected language
    if initial_result.detected_languages:
        lang_code = initial_result.detected_languages[0]
        # Map ISO 639-3 to Tesseract language codes
        tesseract_lang = "eng" if lang_code in ["eng"] else lang_code[:2]

        ocr_config = ExtractionConfig(
            ocr=OcrConfig(
                backend="tesseract",
                language=tesseract_lang
            ),
            language_detection=LanguageDetectionConfig(enabled=False)  # Skip on second pass
        )
        final_result = extract_file_sync("scanned.pdf", config=ocr_config)
    ```

#### Multi-Language Document Analysis

=== "TypeScript"

    ```typescript
    import { extractFileSync, ExtractionConfig } from '@goldziher/kreuzberg';

    const config: ExtractionConfig = {
      languageDetection: {
        enabled: true,
        detectMultiple: true,
        minConfidence: 0.7
      }
    };

    const result = extractFileSync('multilingual_report.pdf', null, config);

    if (result.detectedLanguages) {
      console.log(`Document contains ${result.detectedLanguages.length} languages:`);
      result.detectedLanguages.forEach((lang, i) => {
        const label = i === 0 ? 'Primary' : 'Secondary';
        console.log(`  ${label}: ${lang}`);
      });

      // Route to multilingual processing pipeline
      if (result.detectedLanguages.length > 2) {
        processMultilingualContent(result);
      }
    }
    ```

### Language Code Format

Returns ISO 639-3 (3-letter) codes:

| Language | Code | Example |
|----------|------|---------|
| English | `eng` | "Hello world" → `["eng"]` |
| Spanish | `spa` | "Hola mundo" → `["spa"]` |
| French | `fra` | "Bonjour le monde" → `["fra"]` |
| German | `deu` | "Hallo Welt" → `["deu"]` |
| Chinese (Mandarin) | `cmn` | "你好世界" → `["cmn"]` |
| Japanese | `jpn` | "こんにちは世界" → `["jpn"]` |
| Russian | `rus` | "Привет мир" → `["rus"]` |
| Arabic | `ara` | "مرحبا بالعالم" → `["ara"]` |

### Performance

Language detection is extremely fast:

| Document Size | Detection Time | Multi-Language Overhead |
|---------------|---------------|------------------------|
| 1 KB (short) | 0.3 ms | 0.1 ms |
| 10 KB (medium) | 0.8 ms | 0.5 ms |
| 100 KB (long) | 1.5 ms | 2.1 ms |
| 1 MB (very long) | 12 ms | 18 ms |

**Multi-language overhead**: Processing in chunks adds ~30-50% overhead but provides comprehensive language coverage.

### Best Practices

1. **Use single-language mode by default**: Faster and sufficient for most documents
2. **Set appropriate confidence thresholds**: Higher for formal documents (0.9), lower for short texts (0.6)
3. **Enable multi-language for international content**: Reports, websites, multilingual datasets
4. **Combine with OCR**: Detect language first, then use language-specific OCR
5. **Handle None results**: Short documents (<50 chars) may not have confident detections
6. **Cache results**: Language detection results can be cached for repeated processing

### Limitations

- **Minimum text length**: Requires ~50 characters for reliable detection
- **Mixed-language sentences**: Detects chunk-level language, not sentence-level
- **Similar languages**: May confuse closely related languages (e.g., Spanish/Portuguese)
- **Code and special characters**: May affect accuracy if document is mostly non-linguistic
- **Short documents**: Low confidence on documents <200 characters

### Direct API Usage (Rust)

=== "Rust"

    ```rust
    use kreuzberg::language_detection::detect_languages;
    use kreuzberg::core::config::LanguageDetectionConfig;

    fn main() -> kreuzberg::Result<()> {
        let text = "Hello world! This is an English text.";

        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.8,
            detect_multiple: false,
        };

        let languages = detect_languages(text, &config)?;

        if let Some(langs) = languages {
            println!("Detected: {:?}", langs);
        } else {
            println!("No language detected with sufficient confidence");
        }

        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Enable language detection
    lang_config = Kreuzberg::Config::LanguageDetection.new(
      enabled: true,
      min_confidence: 0.8
    )

    config = Kreuzberg::Config::Extraction.new(
      language_detection: lang_config
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)

    # Access detected languages
    result.detected_languages&.each do |lang|
      puts "Language: #{lang.lang}, Confidence: #{lang.confidence}"
    end
    ```

## Configuration File Support

All text processing features can be configured via `kreuzberg.toml`:

```toml
# Enable quality processing (default: true)
enable_quality_processing = true

# Token reduction configuration
[token_reduction]
level = "aggressive"
language_hint = "en"
preserve_markdown = true
preserve_code = true
semantic_threshold = 0.3
enable_parallel = true
use_simd = true
target_reduction = 0.4
enable_semantic_clustering = false
```

Place `kreuzberg.toml` in your working directory, and it will be automatically loaded.

## Performance Considerations

### Quality Processing

- **Fast**: Regex-based cleaning with minimal overhead
- **Streaming**: Processes text line-by-line for large documents
- **Memory Efficient**: Copy-on-write patterns minimize allocations

### Keyword Extraction

- **YAKE**: O(n) time complexity, suitable for long documents
- **RAKE**: O(n) time complexity with stopword lookup overhead
- **Batch Processing**: Extract keywords from multiple texts efficiently

### Token Reduction

- **SIMD**: 2-4x speedup on supported platforms
- **Parallel**: Near-linear scaling with CPU cores for batch processing
- **CJK Support**: Optimized handling for Chinese, Japanese, Korean text
- **Streaming**: Memory-efficient processing of large texts

## Best Practices

### Quality Processing

1. **Enable by default** unless you need raw extraction output
2. **Combine with OCR** for best results on scanned documents
3. **Check quality scores** to identify problematic documents
4. **Use metadata** to improve quality scoring

### Keyword Extraction

1. **Choose the right algorithm**:
   - YAKE for academic/technical content
   - RAKE for general text and web content
2. **Tune `min_score`** to filter low-quality keywords
3. **Adjust `ngram_range`** based on your needs:
   - (1, 1) for single words
   - (1, 3) for phrases up to 3 words
4. **Set appropriate `language`** for stopword filtering

### Token Reduction

1. **Start with `moderate`** and adjust based on results
2. **Enable `preserve_markdown`** if formatting is important
3. **Use `target_reduction`** when you have specific token limits
4. **Enable `semantic_clustering`** for longer documents
5. **Provide `language_hint`** for better results
6. **Use batch processing** for multiple documents

## Feature Flags

Text processing features require specific Cargo features in Rust:

```toml
[dependencies]
kreuzberg = { version = "4.0", features = ["quality", "keywords", "stopwords"] }

# Or use feature bundles
kreuzberg = { version = "4.0", features = ["full"] }  # All features
```

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `stopwords` | Stopword collections | None |
| `quality` | Quality processing | `unicode-normalization`, `chardetng`, `encoding_rs`, `stopwords` |
| `keywords-yake` | YAKE keyword extraction | `yake-rust`, `stopwords` |
| `keywords-rake` | RAKE keyword extraction | `rake`, `stopwords` |
| `keywords` | Both YAKE and RAKE | `keywords-yake`, `keywords-rake` |

Python and TypeScript bindings include all features by default.

## Examples

### Complete Workflow

=== "Python"

    ```python
    from kreuzberg import extract_file_sync, ExtractionConfig, TokenReductionConfig
    from kreuzberg.keywords import extract_keywords, KeywordConfig

    # Configure extraction with all text processing features
    config = ExtractionConfig(
        enable_quality_processing=True,
        token_reduction=TokenReductionConfig(
            level="moderate",
            preserve_markdown=True
        )
    )

    # Extract with quality processing and token reduction
    result = extract_file_sync("research_paper.pdf", config=config)

    # Extract keywords from the processed text
    keyword_config = KeywordConfig.yake(max_keywords=10)
    keywords = extract_keywords(result.content, keyword_config)

    # Display results
    print(f"Extracted {len(result.content)} characters")
    print(f"\nTop keywords:")
    for kw in keywords[:5]:
        print(f"  - {kw.text}: {kw.score:.3f}")

    print(f"\nProcessed content preview:")
    print(result.content[:500])
    ```

=== "TypeScript"

    ```typescript
    import { extractFileSync, ExtractionConfig, TokenReductionConfig } from '@goldziher/kreuzberg';
    import { extractKeywords, KeywordConfig } from '@goldziher/kreuzberg';

    // Configure extraction with all text processing features
    const config: ExtractionConfig = {
      enableQualityProcessing: true,
      tokenReduction: {
        level: 'moderate',
        preserveMarkdown: true
      }
    };

    // Extract with quality processing and token reduction
    const result = extractFileSync('research_paper.pdf', { config });

    // Extract keywords from the processed text
    const keywordConfig: KeywordConfig = {
      algorithm: 'yake',
      maxKeywords: 10
    };
    const keywords = extractKeywords(result.content, keywordConfig);

    // Display results
    console.log(`Extracted ${result.content.length} characters`);
    console.log('\nTop keywords:');
    for (const kw of keywords.slice(0, 5)) {
      console.log(`  - ${kw.text}: ${kw.score.toFixed(3)}`);
    }

    console.log('\nProcessed content preview:');
    console.log(result.content.slice(0, 500));
    ```

=== "Rust"

    ```rust
    use kreuzberg::{extract_file_sync, ExtractionConfig};
    use kreuzberg::text::token_reduction::{TokenReductionConfig, ReductionLevel};
    use kreuzberg::keywords::{extract_keywords, KeywordConfig};

    fn main() -> kreuzberg::Result<()> {
        // Configure extraction with all text processing features
        let config = ExtractionConfig {
            enable_quality_processing: true,
            token_reduction: Some(TokenReductionConfig {
                level: ReductionLevel::Moderate,
                preserve_markdown: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        // Extract with quality processing and token reduction
        let result = extract_file_sync("research_paper.pdf", None, &config)?;

        // Extract keywords from the processed text
        let keyword_config = KeywordConfig::yake().with_max_keywords(10);
        let keywords = extract_keywords(&result.content, &keyword_config)?;

        // Display results
        println!("Extracted {} characters", result.content.len());
        println!("\nTop keywords:");
        for kw in keywords.iter().take(5) {
            println!("  - {}: {:.3}", kw.text, kw.score);
        }

        println!("\nProcessed content preview:");
        let preview_len = result.content.len().min(500);
        println!("{}", &result.content[..preview_len]);

        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require "kreuzberg"

    # Enable quality processing (default)
    config = Kreuzberg::Config::Extraction.new(
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)

    # Access quality score if available
    puts "Quality score: #{result.quality_score}" if result.quality_score

    puts result.content  # Cleaned text
    ```

## See Also

- [Extractors](extractors.md) - Document format extraction
- [OCR System](ocr.md) - Optical character recognition
- [Architecture](architecture.md) - System design and components
