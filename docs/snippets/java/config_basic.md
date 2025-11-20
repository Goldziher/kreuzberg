```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .useCache(true)
    .enableQualityProcessing(true)
    .build();
ExtractionResult result = Kreuzberg.extractFileSync("document.pdf", null, config);
```
