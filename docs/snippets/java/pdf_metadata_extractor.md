```java
import dev.kreuzberg.*;
import java.lang.foreign.Arena;
import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.logging.Logger;

public class PdfMetadataExtractorExample {
    private static final Logger logger = Logger.getLogger(
        PdfMetadataExtractorExample.class.getName()
    );

    public static void main(String[] args) {
        try (Arena arena = Arena.ofConfined()) {
            AtomicInteger processedCount = new AtomicInteger(0);

            PostProcessor pdfMetadata = result -> {
                // Only process PDFs
                if (!result.mimeType().equals("application/pdf")) {
                    return result;
                }

                processedCount.incrementAndGet();

                // Extract PDF-specific metadata
                Map<String, Object> metadata = new HashMap<>(result.getMetadata());
                metadata.put("pdf_processed", true);
                metadata.put("processing_timestamp", System.currentTimeMillis());

                logger.info("Processed PDF: " + processedCount.get());

                return new ExtractionResult(
                    result.content(),
                    result.mimeType(),
                    result.language(),
                    result.date(),
                    result.subject(),
                    result.getTables(),
                    result.getDetectedLanguages(),
                    metadata
                );
            };

            // Register with priority 50 (default)
            Kreuzberg.registerPostProcessor("pdf-metadata-extractor", pdfMetadata, 50, arena);

            logger.info("PDF metadata extractor initialized");

            // Use in extraction
            ExtractionResult result = Kreuzberg.extractFileSync("document.pdf");
            System.out.println("PDF processed: " + result.getMetadata().get("pdf_processed"));

            logger.info("Processed " + processedCount.get() + " PDFs");
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}
```
