```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;
import java.nio.file.Path;

public final class ConfigFileExample {
    public static void main(String[] args) throws Exception {
        ExtractionConfig config = Kreuzberg.loadExtractionConfigFromFile(Path.of("kreuzberg.toml"));
        ExtractionResult result = Kreuzberg.extractFile(Path.of("document.pdf"), config);
        System.out.printf("Detected MIME: %s%n", result.getMimeType());
    }
}
```
