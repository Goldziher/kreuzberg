```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.KreuzbergException;
import java.io.IOException;

try {
    ExtractionResult result = Kreuzberg.extractFile("document.pdf");
    System.out.println("Extracted: " + result.getContent()
        .substring(0, Math.min(100, result.getContent().length())));
} catch (IOException e) {
    System.err.println("File not found: " + e.getMessage());
} catch (KreuzbergException e) {
    System.err.println("Extraction failed: " + e.getMessage());
}

try {
    byte[] pdfBytes = new byte[] { };
    ExtractionResult result = Kreuzberg.extractBytes(pdfBytes, "application/pdf", null);
    System.out.println("Extracted " + result.getContent().length() + " characters");
} catch (KreuzbergException e) {
    System.err.println("Extraction failed: " + e.getMessage());
}
```
