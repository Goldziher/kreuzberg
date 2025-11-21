```java
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class PostProcessorTest {
    @Test
    void testWordCountProcessor() {
        PostProcessor processor = result -> {
            long count = result.content().split("\\s+").length;

            Map<String, Object> metadata = new HashMap<>(result.getMetadata());
            metadata.put("word_count", count);

            return new ExtractionResult(
                result.content(),
                result.mimeType(),
                result.language(),
                result.date(),
                result.subject(),
                result.getTables(),
                result.getDetectedLanguages(),
                metadata
            );
        };

        ExtractionResult input = new ExtractionResult(
            "Hello world test",
            "text/plain",
            Optional.empty(),
            Optional.empty(),
            Optional.empty(),
            Collections.emptyList(),
            Collections.emptyList(),
            Collections.emptyMap()
        );

        ExtractionResult output = processor.process(input);

        assertEquals(3, output.getMetadata().get("word_count"));
    }
}
```
