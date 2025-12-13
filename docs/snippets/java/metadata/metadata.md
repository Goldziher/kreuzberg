```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.KreuzbergException;
import java.io.IOException;
import java.util.Map;

public class Main {
    public static void main(String[] args) {
        try {
            ExtractionResult result = Kreuzberg.extractFileSync("document.pdf");

            // Access PDF metadata
            @SuppressWarnings("unchecked")
            Map<String, Object> pdfMeta = (Map<String, Object>) result.getMetadata().get("pdf");
            if (pdfMeta != null) {
                System.out.println("Pages: " + pdfMeta.get("page_count"));
                System.out.println("Author: " + pdfMeta.get("author"));
                System.out.println("Title: " + pdfMeta.get("title"));
            }

            // Access HTML metadata
            ExtractionResult htmlResult = Kreuzberg.extractFileSync("page.html");
            @SuppressWarnings("unchecked")
            Map<String, Object> htmlMeta = (Map<String, Object>) htmlResult.getMetadata().get("html");
            if (htmlMeta != null) {
                System.out.println("Title: " + htmlMeta.get("title"));
                System.out.println("Description: " + htmlMeta.get("description"));
                System.out.println("Open Graph Image: " + htmlMeta.get("og_image"));
            }
        } catch (IOException | KreuzbergException e) {
            System.err.println("Extraction failed: " + e.getMessage());
        }
    }
}
```
