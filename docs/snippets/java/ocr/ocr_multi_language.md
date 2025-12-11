```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.OcrConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .ocr(OcrConfig.builder()
        .backend("tesseract")
        .language("eng+deu+fra")
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("multilingual.pdf", config);
System.out.println(result.getContent());
```
