```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;

ExtractionConfig config = Kreuzberg.discoverExtractionConfig();
ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);
```
