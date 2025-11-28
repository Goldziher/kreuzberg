```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.concurrent.CompletableFuture;

try {
    byte[] data = Files.readAllBytes(Paths.get("document.pdf"));

    CompletableFuture<ExtractionResult> future = Kreuzberg.extractBytesAsync(
        data,
        "application/pdf",
        null
    );

    future.thenAccept(result -> System.out.println(result.getContent()))
        .join();
} catch (IOException e) {
    e.printStackTrace();
}
```
