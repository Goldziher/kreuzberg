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
                // Enable OCR
                .ocr(OcrConfig.builder()
                    .backend("tesseract")
                    .language("eng+deu")  // Multiple languages
                    .build())

                // Enable chunking for LLM processing
                .chunking(ChunkingConfig.builder()
                    .maxChars(1000)
                    .maxOverlap(100)
                    .build())

                // Enable token reduction
                .tokenReduction(TokenReductionConfig.builder()
                    .mode("moderate")
                    .preserveImportantWords(true)
                    .build())

                // Enable language detection
                .languageDetection(LanguageDetectionConfig.builder()
                    .enabled(true)
                    .build())

                // Enable caching
                .useCache(true)

                // Enable quality processing
                .enableQualityProcessing(true)
                .build();

            ExtractionResult result = Kreuzberg.extractFileSync("document.pdf", null, config);

            // Access detected languages
            if (!result.getDetectedLanguages().isEmpty()) {
                System.out.println("Languages: " + result.getDetectedLanguages());
            }
        } catch (IOException | KreuzbergException e) {
            System.err.println("Extraction failed: " + e.getMessage());
        }
    }
}
```
