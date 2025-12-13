```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;
import java.util.Map;

ExtractionConfig config = ExtractionConfig.builder()
    .enableQualityProcessing(true)
    .build();

ExtractionResult result = Kreuzberg.extractFile("scanned_document.pdf", config);

Map<String, Object> metadata = result.getMetadata() != null ? result.getMetadata() : Map.of();

double qualityScore = metadata.containsKey("quality_score")
    ? ((Number) metadata.get("quality_score")).doubleValue()
    : 0.0;

if (qualityScore < 0.5) {
    System.out.println(String.format("Warning: Low quality extraction (%.2f)", qualityScore));
    System.out.println("Consider re-scanning with higher DPI or adjusting OCR settings");
} else {
    System.out.println(String.format("Quality score: %.2f", qualityScore));
}
```
