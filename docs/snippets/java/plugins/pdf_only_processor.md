```java title="Java"
import dev.kreuzberg.PostProcessor;
import java.util.HashMap;
import java.util.Map;

PostProcessor pdfOnly = result -> {
    if (!result.getMimeType().equals("application/pdf")) {
        return result;
    }

    Map<String, Object> metadata = new HashMap<>(result.getMetadata());
    metadata.put("pdf_processed", true);

    return result;
};
```
