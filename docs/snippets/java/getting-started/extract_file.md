```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;
import java.io.IOException;

public class ExtractFile {
    public static void main(String[] args) throws IOException {
        ExtractionConfig config = ExtractionConfig.builder()
            .useCache(true)
            .enableQualityProcessing(true)
            .build();

        ExtractionResult result = Kreuzberg.extractFile("contract.pdf", config);

        System.out.println("Extracted " + result.getContent().length() + " characters");
        System.out.println("Quality score: " + result.getMetadata().get("quality_score"));
        System.out.println("Processing time: " + result.getMetadata().get("processing_time") + "ms");
    }
}
```
