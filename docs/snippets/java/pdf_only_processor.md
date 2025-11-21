```java
PostProcessor pdfOnly = result -> {
    // PDF-specific processing
    if (!result.mimeType().equals("application/pdf")) {
        return result;  // Skip non-PDF documents
    }

    // Perform PDF-specific enrichment
    Map<String, Object> metadata = new HashMap<>(result.getMetadata());
    metadata.put("pdf_processed", true);

    return new ExtractionResult(
        result.content(),
        result.mimeType(),
        result.language(),
        result.date(),
        result.subject(),
        result.getTables(),
        result.getDetectedLanguages(),
        metadata
    );
};
```
