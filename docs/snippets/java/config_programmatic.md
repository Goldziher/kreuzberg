```java
import dev.kreuzberg.ChunkingConfig;
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.OcrConfig;
import dev.kreuzberg.TesseractConfig;
import dev.kreuzberg.extraction.ExtractionResult;

public final class ProgrammaticConfigExample {
    public static void main(String[] args) throws Exception {
        OcrConfig ocr = new OcrConfig.Builder()
                .backend("tesseract")
                .language("eng+deu")
                .tesseract(new TesseractConfig.Builder().psm(6).build())
                .build();

        ChunkingConfig chunking = new ChunkingConfig.Builder()
                .maxChars(1000)
                .maxOverlap(200)
                .build();

        ExtractionConfig config = new ExtractionConfig.Builder()
                .useCache(true)
                .ocr(ocr)
                .chunking(chunking)
                .enableQualityProcessing(true)
                .build();

        ExtractionResult result = Kreuzberg.extractFileSync("document.pdf", config);
        System.out.printf("Content length: %d%n", result.content().length());
    }
}
```
