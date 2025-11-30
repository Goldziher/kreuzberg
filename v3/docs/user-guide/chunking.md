# Content Chunking

Kreuzberg provides a powerful content chunking capability that allows you to split extracted text into smaller, more manageable chunks. This feature is particularly useful for processing large documents, working with language models that have token limits, or implementing semantic search functionality.

## Overview

Content chunking divides the extracted text into smaller segments while maintaining semantic coherence. Kreuzberg uses the `semantic-text-splitter` library to intelligently split text based on content type (plain text or markdown), respecting the document's structure.

## Configuration

Chunking is controlled through the `ExtractionConfig` class with these parameters:

- `chunk_content`: Boolean flag to enable/disable chunking (default: `False`)
- `max_chars`: Maximum number of characters per chunk (default: 4000)
- `max_overlap`: Number of characters to overlap between chunks (default: 200)

## Basic Usage

To enable chunking in your extraction process:

```python
from kreuzberg import extract_file, ExtractionConfig

# Enable chunking with default chunk size and overlap
result = await extract_file("large_document.pdf", config=ExtractionConfig(chunk_content=True))

# Access the full content
full_text = result.content

# Access individual chunks
for i, chunk in enumerate(result.chunks):
    print(f"Chunk {i+1}, length: {len(chunk)} characters")
    print(f"Preview: {chunk[:100]}...\n")
```

## Customizing Chunk Size and Overlap

You can customize the chunk size and overlap to suit your specific needs:

```python
from kreuzberg import extract_file, ExtractionConfig

# Custom chunk size (2000 characters) and overlap (100 characters)
result = await extract_file("large_document.pdf", config=ExtractionConfig(chunk_content=True, max_chars=2000, max_overlap=100))
```

## Format-Aware Chunking

Kreuzberg's chunking system is format-aware, meaning it handles different content types appropriately:

- **Markdown**: When extracting from formats that produce markdown output (like DOCX, PPTX), the chunker preserves markdown structure, avoiding breaks in the middle of headings, lists, or code blocks.
- **Plain Text**: For plain text output, the chunker attempts to split on natural boundaries like paragraph breaks and sentences.

## Use Cases

### Working with Large Language Models

When using LLMs with token limits, chunking allows you to process documents that would otherwise exceed those limits:

```python
from kreuzberg import extract_file, ExtractionConfig

# Extract with chunking enabled
result = await extract_file("large_report.pdf", config=ExtractionConfig(chunk_content=True))

# Process each chunk with an LLM
summaries = []
for chunk in result.chunks:
    # Process chunk with your LLM of choice
    summary = await process_with_llm(chunk)
    summaries.append(summary)

# Combine the results
final_summary = "\n\n".join(summaries)
```

### Semantic Search Implementation

Chunking is essential for implementing effective semantic search:

```python
from kreuzberg import extract_file, ExtractionConfig
import numpy as np

# Extract with chunking enabled
result = await extract_file("knowledge_base.pdf", config=ExtractionConfig(chunk_content=True))

# Create embeddings for each chunk (using a hypothetical embedding function)
embeddings = [create_embedding(chunk) for chunk in result.chunks]

# Search function
def semantic_search(query, chunks, embeddings, top_k=3):
    query_embedding = create_embedding(query)

    # Calculate similarity scores
    similarities = [np.dot(query_embedding, emb) for emb in embeddings]

    # Get indices of top results
    top_indices = sorted(range(len(similarities)), key=lambda i: similarities[i], reverse=True)[:top_k]

    # Return top chunks
    return [chunks[i] for i in top_indices]

# Example usage
results = semantic_search("renewable energy benefits", result.chunks, embeddings)
```

## Technical Details

Under the hood, Kreuzberg uses the `semantic-text-splitter` library which intelligently splits text while preserving semantic structure. The chunking process:

1. Identifies the content type (markdown or plain text)
1. Creates an appropriate splitter based on the content type
1. Splits the content according to the specified maximum size and overlap
1. Returns the chunks as a list of strings in the `ExtractionResult.chunks` field

The chunker is cached for performance, so creating multiple extraction results with the same chunking parameters is efficient.

## Best Practices

- **Choose appropriate chunk sizes**: Smaller chunks (1000-2000 characters) work well for precise semantic search, while larger chunks (4000-8000 characters) may be better for context-aware processing.
- **Set meaningful overlap**: Overlap ensures that context isn't lost between chunks. A good rule of thumb is 5-10% of your chunk size.
- **Consider content type**: Markdown content may require larger chunk sizes to preserve structure.
- **Test with your specific use case**: Optimal chunking parameters depend on your specific documents and use case.
