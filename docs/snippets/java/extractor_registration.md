```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import java.lang.foreign.Arena;

public class CustomExtractorExample {
    public static void main(String[] args) {
        try (Arena arena = Arena.ofConfined()) {
            // Register custom extractor with priority 50
            Kreuzberg.registerDocumentExtractor("custom-json-extractor", 50, arena);

            // Use in extraction
            ExtractionResult result = Kreuzberg.extractFileSync("document.json");
            System.out.println("Extracted content length: " + result.content().length());
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}
```
