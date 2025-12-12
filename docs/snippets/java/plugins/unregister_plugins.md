```java title="Java"
import dev.kreuzberg.Kreuzberg;

try {
    // Unregister specific plugins
    Kreuzberg.unregisterPostProcessor("word-count");
    Kreuzberg.unregisterValidator("min-length");
} catch (KreuzbergException e) {
    System.err.println("Failed to unregister: " + e.getMessage());
}
```
