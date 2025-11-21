```java
import dev.kreuzberg.*;
import java.lang.foreign.Arena;
import java.util.HashMap;
import java.util.Map;

public class WordCountExample {
    public static void main(String[] args) {
        try (Arena arena = Arena.ofConfined()) {
            // Define post-processor
            PostProcessor wordCount = result -> {
                long count = result.content().split("\\s+").length;

                Map<String, Object> metadata = new HashMap<>(result.getMetadata());
                metadata.put("word_count", count);

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

            // Register with priority 50 (default)
            Kreuzberg.registerPostProcessor("word-count", wordCount, 50, arena);

            // Use in extraction
            ExtractionResult result = Kreuzberg.extractFileSync("document.pdf");
            System.out.println("Word count: " + result.getMetadata().get("word_count"));
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}
```
