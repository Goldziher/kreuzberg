```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.KreuzbergException;
import dev.kreuzberg.ValidationException;
import dev.kreuzberg.ParsingException;
import dev.kreuzberg.OcrException;
import dev.kreuzberg.MissingDependencyException;
import java.io.IOException;

try {
    ExtractionResult result = Kreuzberg.extractFileSync("document.pdf");
    System.out.println(result.getContent());
} catch (ValidationException e) {
    System.err.println("Invalid configuration: " + e.getMessage());
} catch (ParsingException e) {
    System.err.println("Failed to parse document: " + e.getMessage());
} catch (OcrException e) {
    System.err.println("OCR processing failed: " + e.getMessage());
} catch (MissingDependencyException e) {
    System.err.println("Missing dependency: " + e.getMessage());
} catch (KreuzbergException e) {
    System.err.println("Extraction error: " + e.getMessage());
} catch (IOException e) {
    System.err.println("System error: " + e.getMessage());
}
```
