```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.TokenReductionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .tokenReduction(TokenReductionConfig.builder()
        .mode("moderate")
        .preserveImportantWords(true)
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("verbose_document.pdf", config);

Object originalTokens = result.getMetadata().get("original_token_count");
Object reducedTokens = result.getMetadata().get("token_count");
Object reductionRatio = result.getMetadata().get("token_reduction_ratio");

System.out.println("Reduced from " + originalTokens + " to " + reducedTokens + " tokens");
System.out.println("Reduction: " + ((Number)reductionRatio).doubleValue() * 100 + "%");
```
