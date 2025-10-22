//! Basic Extraction Example
//!
//! Demonstrates basic document extraction with Kreuzberg Rust crate.

use kreuzberg::{
    batch_extract_files, batch_extract_files_sync, extract_bytes, extract_bytes_sync,
    extract_file, extract_file_sync, ChunkingConfig, ExtractionConfig, OcrConfig,
};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    // Synchronous extraction - simplest approach
    println!("=== Synchronous Extraction ===");
    let result = extract_file_sync("document.pdf", None, &ExtractionConfig::default())?;
    println!("Content length: {} characters", result.content.len());
    println!("MIME type: {}", result.mime_type);
    println!("First 200 chars: {}...", &result.content[..200.min(result.content.len())]);

    // With configuration
    println!("\n=== With Configuration ===");
    let config = ExtractionConfig {
        enable_quality_processing: true,
        use_cache: true,
        ..Default::default()
    };
    let result = extract_file_sync("document.pdf", None, &config)?;
    println!("Extracted {} characters with quality processing", result.content.len());

    // Async extraction - for I/O-bound workloads
    println!("\n=== Async Extraction ===");
    let result = extract_file("document.pdf", None, &ExtractionConfig::default()).await?;
    println!("Async extracted: {} characters", result.content.len());

    // Extract from bytes
    println!("\n=== Extract from Bytes ===");
    let data = std::fs::read("document.pdf")?;
    let result = extract_bytes_sync(&data, "application/pdf", &ExtractionConfig::default())?;
    println!("Extracted from bytes: {} characters", result.content.len());

    // Extract from bytes (async)
    println!("\n=== Extract from Bytes (Async) ===");
    let data = tokio::fs::read("document.pdf").await?;
    let result = extract_bytes(&data, "application/pdf", &ExtractionConfig::default()).await?;
    println!("Async extracted from bytes: {} characters", result.content.len());

    // Access metadata
    println!("\n=== Metadata ===");
    let result = extract_file_sync("document.pdf", None, &ExtractionConfig::default())?;
    if let Some(pdf_metadata) = &result.metadata.pdf {
        println!("PDF Pages: {}", pdf_metadata.page_count);
        if let Some(author) = &pdf_metadata.author {
            println!("Author: {}", author);
        }
        if let Some(title) = &pdf_metadata.title {
            println!("Title: {}", title);
        }
    }

    // OCR extraction
    println!("\n=== OCR Extraction ===");
    let ocr_config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = extract_file_sync("scanned_document.pdf", None, &ocr_config)?;
    println!("OCR extracted: {} characters", result.content.len());

    // Batch processing
    println!("\n=== Batch Processing ===");
    let files = vec!["document1.pdf", "document2.docx", "document3.txt"];
    let results = batch_extract_files_sync(&files, &ExtractionConfig::default())?;
    for (file, result) in files.iter().zip(results.iter()) {
        println!("{}: {} chars", file, result.content.len());
    }

    // Async batch processing
    println!("\n=== Async Batch Processing ===");
    let files = vec!["document1.pdf", "document2.docx"];
    let results = batch_extract_files(&files, &ExtractionConfig::default()).await?;
    println!("Processed {} files asynchronously", results.len());

    // Chunking
    println!("\n=== Content Chunking ===");
    let chunking_config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_chars: 1000,
            max_overlap: 100,
        }),
        ..Default::default()
    };
    let result = extract_file_sync("large_document.pdf", None, &chunking_config)?;
    if let Some(chunks) = result.chunks {
        println!("Document split into {} chunks", chunks.len());
        for (i, chunk) in chunks.iter().take(3).enumerate() {
            println!("  Chunk {}: {} chars", i + 1, chunk.len());
        }
    }

    // Error handling
    println!("\n=== Error Handling ===");
    match extract_file_sync("nonexistent.pdf", None, &ExtractionConfig::default()) {
        Ok(_) => println!("File extracted successfully"),
        Err(e) => eprintln!("Extraction error: {}", e),
    }

    Ok(())
}
