# Text Processing Features

Kreuzberg provides advanced text processing capabilities to enhance extracted text quality, extract meaningful keywords, and reduce token counts for LLM processing.

## Overview

The text processing suite includes:

- **Quality Processing**: Clean OCR artifacts, remove script/CSS content, and score text quality
- **Keyword Extraction**: Extract meaningful keywords using YAKE or RAKE algorithms
- **Token Reduction**: Reduce token count while preserving meaning and structure
- **Stopwords**: Language-specific stopword collections for text analysis

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

**Built-in languages**: English (`en`), Spanish (`es`)

For other languages, stopwords will fall back to English or can be loaded from JSON files.

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

## Stopwords

Stopwords are common words filtered out during text analysis. Kreuzberg provides built-in stopword collections for multiple languages.

### Supported Languages

- **English (`en`)**: 78+ common words
- **Spanish (`es`)**: 250+ common words

Additional languages can be loaded from JSON files in the `stopwords/` directory.

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

### Custom Stopwords

For languages not built-in, create JSON files:

```json
// stopwords/fr_stopwords.json
[
  "le", "la", "les", "un", "une", "des",
  "et", "ou", "mais", "donc", "car",
  "de", "du", "à", "au", "en"
]
```

Place the file in one of these locations:

- `kreuzberg/_token_reduction/stopwords/{lang}_stopwords.json`
- `../_token_reduction/stopwords/{lang}_stopwords.json`
- `_token_reduction/stopwords/{lang}_stopwords.json`
- `stopwords/{lang}_stopwords.json`

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

## See Also

- [Extractors](extractors.md) - Document format extraction
- [OCR System](ocr.md) - Optical character recognition
- [Architecture](architecture.md) - System design and components
