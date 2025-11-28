```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.PostProcessor;
import dev.kreuzberg.KreuzbergException;
import java.io.IOException;
import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.logging.Logger;

public class PdfMetadataExtractorExample {
    private static final Logger logger = Logger.getLogger(
        PdfMetadataExtractorExample.class.getName()
    );

    public static void main(String[] args) {
        AtomicInteger processedCount = new AtomicInteger(0);

        PostProcessor pdfMetadata = result -> {
            if (!result.getMimeType().equals("application/pdf")) {
                return result;
            }

            processedCount.incrementAndGet();

            Map<String, Object> metadata = new HashMap<>(result.getMetadata());
            metadata.put("pdf_processed", true);
            metadata.put("processing_timestamp", System.currentTimeMillis());

            logger.info("Processed PDF: " + processedCount.get());

            return result;
        };

        try {
            Kreuzberg.registerPostProcessor("pdf-metadata-extractor", pdfMetadata, 50);

            logger.info("PDF metadata extractor initialized");

            ExtractionResult result = Kreuzberg.extractFile("document.pdf");
            System.out.println("PDF processed: " + result.getMetadata().get("pdf_processed"));

            logger.info("Processed " + processedCount.get() + " PDFs");
        } catch (IOException | KreuzbergException e) {
            e.printStackTrace();
        }
    }
}
```
