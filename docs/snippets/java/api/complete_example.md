```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.*;

ExtractionConfig config = ExtractionConfig.builder()
    .useCache(true)
    .enableQualityProcessing(true)
    .forceOcr(false)
    .ocr(OcrConfig.builder()
        .backend("tesseract")
        .language("eng+fra")
        .build())
    .pdfOptions(PdfConfig.builder()
        .extractImages(true)
        .extractMetadata(true)
        .build())
    .imageExtraction(ImageExtractionConfig.builder()
        .extractImages(true)
        .targetDpi(150)
        .maxImageDimension(4096)
        .build())
    .imagePreprocessing(ImagePreprocessingConfig.builder()
        .targetDpi(300)
        .denoise(true)
        .deskew(true)
        .contrastEnhance(true)
        .build())
    .chunking(ChunkingConfig.builder()
        .maxChars(1000)
        .maxOverlap(200)
        .build())
    .tokenReduction(TokenReductionConfig.builder()
        .mode("moderate")
        .preserveImportantWords(true)
        .build())
    .languageDetection(LanguageDetectionConfig.builder()
        .enabled(true)
        .minConfidence(0.8)
        .build())
    .postprocessor(PostProcessorConfig.builder()
        .enabled(true)
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);
```
