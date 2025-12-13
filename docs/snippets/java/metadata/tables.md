```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.KreuzbergException;
import dev.kreuzberg.Table;
import java.io.IOException;
import java.util.List;

public class Main {
    public static void main(String[] args) {
        try {
            ExtractionResult result = Kreuzberg.extractFile("document.pdf");

            for (Table table : result.getTables()) {
                System.out.println("Table with " + table.cells().size() + " rows");
                System.out.println(table.markdown());

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
