```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import java.nio.file.Path;
import java.util.concurrent.CompletableFuture;

public class Example {
    public static void main(String[] args) {
        CompletableFuture<ExtractionResult> future =
            Kreuzberg.extractFileAsync(Path.of("document.pdf"), null);

        future.thenAccept(result -> {
            System.out.println(result.getContent());
            System.out.println("Tables: " + result.getTables().size());
        }).join();
    }
}
```
