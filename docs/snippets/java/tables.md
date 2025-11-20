```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.KreuzbergException;
import dev.kreuzberg.Table;
import java.io.IOException;
import java.util.List;

public class Main {
    public static void main(String[] args) {
        try {
            ExtractionResult result = Kreuzberg.extractFileSync("document.pdf");

            // Iterate over tables
            for (Table table : result.getTables()) {
                System.out.println("Table with " + table.cells().size() + " rows");
                System.out.println(table.markdown());  // Markdown representation

                // Access cells
                for (List<String> row : table.cells()) {
                    System.out.println(row);
                }
            }
        } catch (IOException | KreuzbergException e) {
            System.err.println("Extraction failed: " + e.getMessage());
        }
    }
}
```
