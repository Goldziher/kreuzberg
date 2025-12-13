```java title="Java"
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.ImageExtractionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .imageExtraction(ImageExtractionConfig.builder()
        .extractImages(true)
        .targetDpi(200)
        .maxImageDimension(2048)
        .autoAdjustDpi(true)
        .build())
    .build();
```
