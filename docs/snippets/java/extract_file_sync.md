```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.KreuzbergException;
import java.io.IOException;

try {
    ExtractionResult result = Kreuzberg.extractFileSync("document.pdf");

    System.out.println(result.getContent());
    System.out.println("Tables: " + result.getTables().size());
    System.out.println("Metadata: " + result.getMetadata());
} catch (IOException | KreuzbergException e) {
    e.printStackTrace();
}
```
