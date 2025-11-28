```java
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.PostProcessor;
import org.junit.jupiter.api.Test;
import java.util.HashMap;
import java.util.Map;
import static org.junit.jupiter.api.Assertions.*;

class PostProcessorTest {
    @Test
    void testWordCountProcessor() {
        PostProcessor processor = result -> {
            long count = result.getContent().split("\\s+").length;

            Map<String, Object> metadata = new HashMap<>(result.getMetadata());
            metadata.put("word_count", count);

            return result;
        };

        ExtractionResult input = new ExtractionResult(
            "Hello world test",
            "text/plain",
            new HashMap<>(),
            java.util.List.of(),
            java.util.List.of(),
            java.util.List.of(),
            java.util.List.of(),
            true
        );

        ExtractionResult output = processor.process(input);

        assertEquals(3, output.getMetadata().get("word_count"));
    }
}
```
