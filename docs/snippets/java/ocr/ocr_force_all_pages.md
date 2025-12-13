```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.OcrConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .ocr(OcrConfig.builder()
        .backend("tesseract")
        .build())
    .forceOcr(true)
    .build();

ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);
System.out.println(result.getContent());
```
