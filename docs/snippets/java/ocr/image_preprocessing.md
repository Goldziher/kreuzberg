```java title="Java"
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.ImagePreprocessingConfig;
import dev.kreuzberg.config.OcrConfig;
import dev.kreuzberg.config.TesseractConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .ocr(OcrConfig.builder()
        .tesseractConfig(TesseractConfig.builder()
            .preprocessing(ImagePreprocessingConfig.builder()
                .targetDpi(300)
                .denoise(true)
                .deskew(true)
                .contrastEnhance(true)
                .binarizationMethod("otsu")
                .build())
            .build())
        .build())
    .build();
```
