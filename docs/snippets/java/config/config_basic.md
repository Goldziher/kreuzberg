```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .useCache(true)
    .enableQualityProcessing(true)
    .build();
ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);
```
