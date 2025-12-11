```java
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.LanguageDetectionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .languageDetection(LanguageDetectionConfig.builder()
        .enabled(true)
        .minConfidence(0.9)
        .detectMultiple(true)
        .build())
    .build();
```
