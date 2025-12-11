```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.TokenReductionConfig;
import java.util.Map;

ExtractionConfig config = ExtractionConfig.builder()
    .tokenReduction(TokenReductionConfig.builder()
        .mode("moderate")
        .preserveMarkdown(true)
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("verbose_document.pdf", config);

Map<String, Object> metadata = result.getMetadata() != null ? result.getMetadata() : Map.of();

int original = metadata.containsKey("original_token_count")
    ? ((Number) metadata.get("original_token_count")).intValue()
    : 0;

int reduced = metadata.containsKey("token_count")
    ? ((Number) metadata.get("token_count")).intValue()
    : 0;

double ratio = metadata.containsKey("token_reduction_ratio")
    ? ((Number) metadata.get("token_reduction_ratio")).doubleValue()
    : 0.0;

System.out.println("Reduced from " + original + " to " + reduced + " tokens");
System.out.println(String.format("Reduction: %.1f%%", ratio * 100));
```
