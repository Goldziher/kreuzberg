```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import java.io.IOException;

public class HelloWorld {
    public static void main(String[] args) throws IOException {
        ExtractionResult result = Kreuzberg.extractFile("document.pdf");
        System.out.println("Extracted content:");
        System.out.println(result.getContent().substring(0, Math.min(200, result.getContent().length())));
    }
}
```
