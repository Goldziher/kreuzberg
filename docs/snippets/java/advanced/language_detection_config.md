```java
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.LanguageDetectionConfig;
import java.math.BigDecimal;

ExtractionConfig config = ExtractionConfig.builder()
    .languageDetection(LanguageDetectionConfig.builder()
        .enabled(true)
        .minConfidence(new BigDecimal("0.8"))
        .detectMultiple(false)
        .build())
    .build();
```
