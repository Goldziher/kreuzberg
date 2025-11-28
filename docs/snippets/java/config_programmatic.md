```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ChunkingConfig;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.OcrConfig;
import dev.kreuzberg.config.TesseractConfig;

public final class ProgrammaticConfigExample {
    public static void main(String[] args) throws Exception {
        ExtractionConfig config = ExtractionConfig.builder()
            .ocr(OcrConfig.builder()
                .backend("tesseract")
                .language("eng+deu")
                .tesseractConfig(TesseractConfig.builder()
                    .psm(6)
                    .build())
                .build())
            .chunking(ChunkingConfig.builder()
                .maxChars(1000)
                .maxOverlap(200)
                .build())
            .useCache(true)
            .enableQualityProcessing(true)
            .build();

        ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);
        System.out.printf("Content length: %d%n", result.getContent().length());
    }
}
```
