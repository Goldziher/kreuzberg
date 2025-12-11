```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import java.io.IOException;
import java.util.Map;

public class BasicUsage {
    public static void main(String[] args) throws IOException {
        ExtractionResult result = Kreuzberg.extractFile("document.pdf");

        System.out.println("Content:");
        System.out.println(result.getContent());

        System.out.println("\nMetadata:");
        Map<String, Object> metadata = result.getMetadata();
        if (metadata != null) {
            System.out.println("Title: " + metadata.get("title"));
            System.out.println("Author: " + metadata.get("author"));
        }

        System.out.println("\nTables found: " + result.getTables().size());
        System.out.println("Images found: " + result.getImages().size());
    }
}
```
