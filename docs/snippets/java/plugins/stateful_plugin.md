```java title="Java"
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicInteger;

class StatefulPlugin implements PostProcessor {
    // Use atomic types for simple counters
    private final AtomicInteger callCount = new AtomicInteger(0);

    // Use concurrent collections for complex state
    private final ConcurrentHashMap<String, String> cache = new ConcurrentHashMap<>();

    @Override
    public ExtractionResult process(ExtractionResult result) {
        // Increment counter atomically
        callCount.incrementAndGet();

        // Update cache (thread-safe)
        cache.put("last_mime", result.mimeType());

        return result;
    }

    public int getCallCount() {
        return callCount.get();
    }
}
```
