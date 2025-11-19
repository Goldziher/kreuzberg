```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.BytesWithMime;
import dev.kreuzberg.KreuzbergException;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

try {
    List<String> files = Arrays.asList("doc1.pdf", "doc2.docx");

    List<BytesWithMime> dataList = new ArrayList<>();
    for (String file : files) {
        byte[] data = Files.readAllBytes(Paths.get(file));
        String mimeType = file.endsWith(".pdf") ? "application/pdf" :
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
        dataList.add(new BytesWithMime(data, mimeType));
    }

    List<ExtractionResult> results = Kreuzberg.batchExtractBytesSync(dataList);

    for (int i = 0; i < results.size(); i++) {
        ExtractionResult result = results.get(i);
        System.out.println("Document " + (i + 1) + ": " + result.getContent().length() + " characters");
    }
} catch (IOException | KreuzbergException e) {
    e.printStackTrace();
}
```
