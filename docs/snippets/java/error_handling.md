```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.exception.*;
import java.io.IOException;

// Extract from file with comprehensive error handling
try {
    ExtractionResult result = Kreuzberg.extractFile("document.pdf");
    System.out.println("Extracted: " + result.content().substring(0, Math.min(100, result.content().length())));
} catch (ParsingException e) {
    System.err.println("Failed to parse document: " + e.getMessage());
} catch (OcrException e) {
    System.err.println("OCR processing failed: " + e.getMessage());
} catch (MissingDependencyException e) {
    System.err.println("Missing dependency: " + e.getMessage());
} catch (IOException e) {
    System.err.println("File not found: " + e.getMessage());
} catch (KreuzbergException e) {
    System.err.println("Extraction failed: " + e.getMessage());
}

// Extract from bytes with configuration
try {
    ExtractionConfig config = new ExtractionConfig();
    byte[] pdfBytes = new byte[] { /* PDF file bytes */ };
    ExtractionResult result = Kreuzberg.extractBytes(pdfBytes, "application/pdf", config);
    System.out.println("Extracted " + result.content().length() + " characters");
} catch (ValidationException e) {
    System.err.println("Invalid configuration: " + e.getMessage());
} catch (OcrException e) {
    System.err.println("OCR failed: " + e.getMessage());
} catch (KreuzbergException e) {
    System.err.println("Extraction failed: " + e.getMessage());
}
```
