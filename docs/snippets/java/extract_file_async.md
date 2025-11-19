```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import java.util.concurrent.CompletableFuture;

public class Example {
    public static void main(String[] args) {
        CompletableFuture<ExtractionResult> future =
            Kreuzberg.extractFileAsync("document.pdf");

        future.thenAccept(result -> {
            System.out.println(result.getContent());
            System.out.println("Tables: " + result.getTables().size());
        }).join();
    }
}
```
