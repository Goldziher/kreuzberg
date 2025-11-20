```java
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.PdfConfig;
import java.util.Arrays;

ExtractionConfig config = ExtractionConfig.builder()
    .pdfOptions(PdfConfig.builder()
        .extractImages(true)
        .extractMetadata(true)
        .passwords(Arrays.asList("password1", "password2"))
        .build())
    .build();
```
