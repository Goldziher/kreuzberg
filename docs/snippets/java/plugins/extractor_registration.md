```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.KreuzbergException;
import java.io.IOException;

public class CustomExtractorExample {
    public static void main(String[] args) {
        try {
            ExtractionResult result = Kreuzberg.extractFile("document.json");
            System.out.println("Extracted content length: " + result.getContent().length());
        } catch (IOException | KreuzbergException e) {
            e.printStackTrace();
        }
    }
}
```
