```java
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.extraction.ExtractionResult;
import java.nio.file.Path;

public final class ConfigFileExample {
    public static void main(String[] args) throws Exception {
        ExtractionConfig config = Kreuzberg.loadExtractionConfigFromFile(Path.of("kreuzberg.toml"));
        ExtractionResult result = Kreuzberg.extractFileSync("document.pdf", config);
        System.out.printf("Detected MIME: %s%n", result.mimeType());
    }
}
```
