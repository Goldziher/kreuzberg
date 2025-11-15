package dev.kreuzberg;

/**
 * Exception thrown when OCR processing fails.
 *
 * <p>OCR errors occur during optical character recognition, such as:</p>
 * <ul>
 *   <li>OCR backend initialization failures</li>
 *   <li>Image preprocessing errors</li>
 *   <li>Language model loading issues</li>
 *   <li>OCR engine crashes</li>
 * </ul>
 *
 * @since 4.0.0
 */
public final class OcrException extends KreuzbergException {
    /**
     * Constructs a new OCR exception with the specified message.
     *
     * @param message the detail message explaining why OCR processing failed
     */
    public OcrException(String message) {
        super(message);
    }

    /**
     * Constructs a new OCR exception with the specified message and cause.
     *
     * @param message the detail message
     * @param cause the cause of the OCR failure
     */
    public OcrException(String message, Throwable cause) {
        super(message, cause);
    }
}
