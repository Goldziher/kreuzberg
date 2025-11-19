```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.KreuzbergException;
import java.io.IOException;
import java.util.Arrays;
import java.util.List;

try {
    List<String> files = Arrays.asList("doc1.pdf", "doc2.docx", "doc3.pptx");

    List<ExtractionResult> results = Kreuzberg.batchExtractFilesSync(files);

    for (int i = 0; i < results.size(); i++) {
        ExtractionResult result = results.get(i);
        System.out.println("File " + (i + 1) + ": " + result.getContent().length() + " characters");
    }
} catch (IOException | KreuzbergException e) {
    e.printStackTrace();
}
```
