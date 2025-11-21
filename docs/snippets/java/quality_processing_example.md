```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .enableQualityProcessing(true)
    .build();

ExtractionResult result = Kreuzberg.extractFileSync("scanned_document.pdf", null, config);

Object qualityObj = result.getMetadata().get("quality_score");
double qualityScore = qualityObj != null ? ((Number)qualityObj).doubleValue() : 0.0;

if (qualityScore < 0.5) {
    System.out.printf("Warning: Low quality extraction (%.2f)%n", qualityScore);
    System.out.println("Consider re-scanning with higher DPI or adjusting OCR settings");
} else {
    System.out.printf("Quality score: %.2f%n", qualityScore);
}
```
