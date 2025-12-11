```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import java.io.IOException;

public class InstallVerify {
    public static void main(String[] args) throws IOException {
        System.out.println("Kreuzberg FFI bindings loaded successfully");

        ExtractionResult result = Kreuzberg.extractFile("sample.pdf");
        System.out.println("Installation verified!");
        System.out.println("Extracted " + result.getContent().length() + " characters");
    }
}
```
