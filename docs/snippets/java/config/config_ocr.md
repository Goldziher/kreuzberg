```java title="Java"
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.OcrConfig;
import dev.kreuzberg.config.TesseractConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .ocr(OcrConfig.builder()
        .backend("tesseract")
        .language("eng+fra")
        .tesseractConfig(TesseractConfig.builder()
            .psm(3)
            .build())
        .build())
    .build();
```
