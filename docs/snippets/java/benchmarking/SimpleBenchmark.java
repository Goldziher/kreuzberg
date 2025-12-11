```java
import com.kreuzberg.*;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.Callable;
import java.util.concurrent.ForkJoinPool;

public final class SimpleBenchmark {
  private SimpleBenchmark() {}

  public static void main(String[] args) throws Exception {
    ExtractionConfig config = new ExtractionConfig.Builder()
      .useCache(false)
      .build();

    Kreuzberg kreuzberg = new Kreuzberg(config);
    String filePath = "document.pdf";
    int numRuns = 10;

    System.out.println("Sync extraction (" + numRuns + " runs):");
    long start = System.nanoTime();
    for (int i = 0; i < numRuns; i++) {
      kreuzberg.extractFile(filePath);
    }
    double syncDuration = (System.nanoTime() - start) / 1_000_000_000.0;
    double avgSync = syncDuration / numRuns;
    System.out.println("  - Total time: " + String.format("%.3f", syncDuration) + "s");
    System.out.println("  - Average: " + String.format("%.3f", avgSync) + "s per extraction");

    System.out.println("\nAsync extraction (" + numRuns + " parallel runs):");
    List<Callable<ExtractionResult>> tasks = new ArrayList<>();
    for (int i = 0; i < numRuns; i++) {
      tasks.add(() -> kreuzberg.extractFile(filePath));
    }

    start = System.nanoTime();
    ForkJoinPool.commonPool().invokeAll(tasks);
    double asyncDuration = (System.nanoTime() - start) / 1_000_000_000.0;
    System.out.println("  - Total time: " + String.format("%.3f", asyncDuration) + "s");
    System.out.println("  - Average: " + String.format("%.3f", asyncDuration / numRuns) + "s per extraction");
    System.out.println("  - Speedup: " + String.format("%.1f", syncDuration / asyncDuration) + "x");

    ExtractionConfig cacheConfig = new ExtractionConfig.Builder()
      .useCache(true)
      .build();
    Kreuzberg kreuzbergCached = new Kreuzberg(cacheConfig);

    System.out.println("\nFirst extraction (populates cache)...");
    start = System.nanoTime();
    kreuzbergCached.extractFile(filePath);
    double firstDuration = (System.nanoTime() - start) / 1_000_000_000.0;
    System.out.println("  - Time: " + String.format("%.3f", firstDuration) + "s");

    System.out.println("Second extraction (from cache)...");
    start = System.nanoTime();
    kreuzbergCached.extractFile(filePath);
    double cachedDuration = (System.nanoTime() - start) / 1_000_000_000.0;
    System.out.println("  - Time: " + String.format("%.3f", cachedDuration) + "s");
    System.out.println("  - Cache speedup: " + String.format("%.1f", firstDuration / cachedDuration) + "x");
  }
}
```
