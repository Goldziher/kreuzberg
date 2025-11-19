```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.KreuzbergException;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;

public class Example {
    public static void main(String[] args) {
        try {
            byte[] data = Files.readAllBytes(Paths.get("document.pdf"));

            ExtractionResult result = Kreuzberg.extractBytesSync(
                data,
                "application/pdf"
            );
            System.out.println(result.getContent());
        } catch (IOException | KreuzbergException e) {
            e.printStackTrace();
        }
    }
}
```
