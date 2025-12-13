```java title="Java"
import java.util.logging.Logger;
import java.util.logging.Level;

class MyPlugin implements PostProcessor {
    private static final Logger logger = Logger.getLogger(MyPlugin.class.getName());

    @Override
    public ExtractionResult process(ExtractionResult result) {
        logger.info("Processing " + result.mimeType() +
            " (" + result.content().length() + " bytes)");

        // Processing...

        if (result.content().isEmpty()) {
            logger.warning("Processing resulted in empty content");
        }

        return result;
    }
}
```
