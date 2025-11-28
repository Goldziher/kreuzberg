```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.OcrConfig;
import dev.kreuzberg.config.ImagePreprocessingConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .ocr(OcrConfig.builder()
        .backend("tesseract")
        .build())
    .imagePreprocessing(ImagePreprocessingConfig.builder()
        .targetDpi(300)
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("scanned.pdf", config);
```
