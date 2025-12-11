```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.KreuzbergException;
import dev.kreuzberg.config.*;
import java.io.IOException;

public class Main {
    public static void main(String[] args) {
        try {
            ExtractionConfig config = ExtractionConfig.builder()
                .ocr(OcrConfig.builder()
                    .backend("tesseract")
                    .language("eng+deu")
                    .build())
                .chunking(ChunkingConfig.builder()
                    .maxChars(1000)
                    .maxOverlap(100)
                    .build())
                .tokenReduction(TokenReductionConfig.builder()
                    .mode("moderate")
                    .preserveImportantWords(true)
                    .build())
                .languageDetection(LanguageDetectionConfig.builder()
                    .enabled(true)
                    .build())
                .useCache(true)
                .enableQualityProcessing(true)
                .build();

            ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);

            if (!result.getDetectedLanguages().isEmpty()) {
                System.out.println("Languages: " + result.getDetectedLanguages());
            }
        } catch (IOException | KreuzbergException e) {
            System.err.println("Extraction failed: " + e.getMessage());
        }
    }
}
```
