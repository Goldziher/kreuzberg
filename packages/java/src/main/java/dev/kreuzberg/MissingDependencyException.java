package dev.kreuzberg;

/**
 * Exception thrown when a required system dependency is missing.
 *
 * <p>Missing dependency errors occur when external tools or libraries are not available, such as:</p>
 * <ul>
 *   <li>LibreOffice (for DOC/PPT/XLS files)</li>
 *   <li>Tesseract OCR (for OCR processing)</li>
 *   <li>ImageMagick (for image processing)</li>
 *   <li>Poppler (for PDF rendering)</li>
 * </ul>
 *
 * @since 4.0.0
 */
public final class MissingDependencyException extends KreuzbergException {
    /**
     * Constructs a new missing dependency exception with the specified message.
     *
     * @param message the detail message explaining which dependency is missing
     */
    public MissingDependencyException(String message) {
        super(message);
    }

    /**
     * Constructs a new missing dependency exception with the specified message and cause.
     *
     * @param message the detail message
     * @param cause the cause of the missing dependency error
     */
    public MissingDependencyException(String message, Throwable cause) {
        super(message, cause);
    }
}
