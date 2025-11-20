```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;

ExtractionConfig config = ExtractionConfig.discover();
ExtractionResult result = Kreuzberg.extractFile("document.pdf", null, config);
```
