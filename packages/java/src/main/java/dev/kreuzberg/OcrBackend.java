package dev.kreuzberg;

import java.util.Collections;
import java.util.List;

/**
 * Interface for custom OCR backend implementations.
 *
 * <p>OCR backends process images and extract text using optical character recognition.
 * They are called when OCR is enabled in the extraction configuration.</p>
 *
 * <h2>Usage</h2>
 * <pre>{@code
 * OcrBackend customBackend = (imageBytes, configJson) -> {
 *     String language = parseLanguage(configJson);
 *     return myOcrEngine.recognize(imageBytes, language);
 * };
 *
 * Kreuzberg.registerOcrBackend("custom-ocr", customBackend, List.of("eng", "deu"));
 * }</pre>
 *
 * <h2>Thread Safety</h2>
 * <p>Implementations must be thread-safe as they may be called concurrently
 * from multiple threads during batch extraction operations.</p>
 *
 * <h2>Error Handling</h2>
 * <p>If OCR fails, throw a {@link KreuzbergException}. The extraction will fail
 * with the exception message.</p>
 *
 * @since 4.0.0
 */
@FunctionalInterface
public interface OcrBackend {
    /**
     * Process image bytes and extract text via OCR.
     *
     * <p>The configuration is provided as a JSON string encoded from {@link dev.kreuzberg.config.OcrConfig}.</p>
     *
     * @param imageBytes raw image data
     * @param configJson JSON string containing OCR configuration
     * @return extracted text, or null on failure
     */
    String processImage(byte[] imageBytes, String configJson);

    /**
     * Languages supported by the backend.
     *
     * @return language codes (empty means all languages)
     */
    default List<String> supportedLanguages() {
        return Collections.emptyList();
    }
}
