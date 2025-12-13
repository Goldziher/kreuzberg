```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.PostProcessor;
import dev.kreuzberg.KreuzbergException;
import java.io.IOException;
import java.util.HashMap;
import java.util.Map;

public class WordCountExample {
    public static void main(String[] args) {
        PostProcessor wordCount = result -> {
            long count = result.getContent().split("\\s+").length;

            Map<String, Object> metadata = new HashMap<>(result.getMetadata());
            metadata.put("word_count", count);

            return result;
        };

        try {
            Kreuzberg.registerPostProcessor("word-count", wordCount, 50);

            ExtractionResult result = Kreuzberg.extractFile("document.pdf");
            System.out.println("Word count: " + result.getMetadata().get("word_count"));
        } catch (IOException | KreuzbergException e) {
            e.printStackTrace();
        }
    }
}
```
