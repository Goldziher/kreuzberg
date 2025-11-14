package dev.kreuzberg;

import java.lang.foreign.MemorySegment;

/**
 * Interface for custom OCR backend implementations.
 *
 * <p>OCR backends process images and extract text using optical character recognition.
 * They are called when OCR is enabled in the extraction configuration.</p>
 *
 * <h2>Usage</h2>
 * <pre>{@code
 * // Create a global arena for the callback lifecycle
 * Arena callbackArena = Arena.ofAuto();
 *
 * OcrBackend customBackend = (imageBytes, imageLength, configJson) -> {
 *     try {
 *         // Read image bytes
 *         byte[] image = imageBytes.reinterpret(imageLength).toArray(ValueLayout.JAVA_BYTE);
 *
 *         // Parse config JSON
 *         String configStr = configJson.reinterpret(Long.MAX_VALUE).getString(0);
 *         // Parse configStr as JSON and extract language, etc.
 *
 *         // Perform OCR using your custom engine
 *         String text = myOcrEngine.recognize(image, language);
 *
 *         // Return result as C string
 *         return callbackArena.allocateFrom(text);
 *     } catch (Exception e) {
 *         // Return NULL on error
 *         return MemorySegment.NULL;
 *     }
 * };
 *
 * try (Arena arena = Arena.ofConfined()) {
 *     Kreuzberg.registerOcrBackend("custom-ocr", customBackend, arena);
 *
 *     // Use the custom backend in extraction
 *     ExtractionConfig config = ExtractionConfig.builder()
 *         .ocr(OcrConfig.builder().backend("custom-ocr").build())
 *         .build();
 *
 *     ExtractionResult result = Kreuzberg.extractFile("scanned.pdf", config);
 * }
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
     * <p>This method receives raw image data (PNG, JPEG, TIFF, etc.) as a native
     * memory segment and OCR configuration as JSON. It must return a pointer to
     * the extracted text as a C string.</p>
     *
     * <p>The memory segments are only valid for the duration of this call and should
     * not be stored or accessed after returning.</p>
     *
     * <p><strong>IMPORTANT:</strong> The returned MemorySegment must be allocated
     * via {@link Arena#allocateFrom(String)} using a global or confined arena, and
     * will be freed by the Rust side after use.</p>
     *
     * @param imageBytes native memory segment containing image data
     * @param imageLength length of the image data in bytes
     * @param configJson native memory segment containing JSON-encoded OcrConfig
     * @return memory segment containing null-terminated UTF-8 string with extracted text,
     *         or {@link MemorySegment#NULL} on error
     */
    MemorySegment processImage(MemorySegment imageBytes, long imageLength, MemorySegment configJson);
}
