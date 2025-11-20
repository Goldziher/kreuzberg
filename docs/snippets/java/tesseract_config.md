```java
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.OcrConfig;
import dev.kreuzberg.config.TesseractConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .ocr(OcrConfig.builder()
        .language("eng+fra+deu")
        .tesseractConfig(TesseractConfig.builder()
            .psm(6)
            .oem(1)
            .minConfidence(0.8)
            .tesseditCharWhitelist("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,!?")
            .enableTableDetection(true)
            .build())
        .build())
    .build();
```
