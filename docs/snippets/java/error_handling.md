```java
public ExtractionResult process(ExtractionResult result) throws KreuzbergException {
    // Validate inputs
    if (result.content().isEmpty()) {
        throw new ValidationException("Empty content");
    }

    // Handle errors with context
    try {
        String processed = parseContent(result.content());

        return result.withContent(processed);
    } catch (Exception e) {
        throw new ParsingException(
            "Failed to parse " + result.mimeType() + ": " + e.getMessage(),
            e
        );
    }
}
```
