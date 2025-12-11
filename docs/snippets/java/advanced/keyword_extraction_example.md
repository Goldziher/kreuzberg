```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.KeywordConfig;
import dev.kreuzberg.config.KeywordAlgorithm;
import java.util.List;
import java.util.Map;

ExtractionConfig config = ExtractionConfig.builder()
    .keywords(KeywordConfig.builder()
        .algorithm(KeywordAlgorithm.YAKE)
        .maxKeywords(10)
        .minScore(0.3)
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("research_paper.pdf", config);

Map<String, Object> metadata = result.getMetadata() != null ? result.getMetadata() : Map.of();

if (metadata.containsKey("keywords")) {
    List<Map<String, Object>> keywords = (List<Map<String, Object>>) metadata.get("keywords");
    for (Map<String, Object> kw : keywords) {
        String text = (String) kw.get("text");
        Double score = ((Number) kw.get("score")).doubleValue();
        System.out.println(text + ": " + String.format("%.3f", score));
    }
}
```
