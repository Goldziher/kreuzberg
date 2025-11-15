package dev.kreuzberg;

/**
 * Exception thrown when image processing operations fail.
 *
 * <p>Image processing errors occur during image manipulation, such as:</p>
 * <ul>
 *   <li>Image decoding failures</li>
 *   <li>Unsupported image formats</li>
 *   <li>Image resizing/scaling errors</li>
 *   <li>DPI adjustment failures</li>
 *   <li>Color space conversion issues</li>
 * </ul>
 *
 * @since 4.0.0
 */
public final class ImageProcessingException extends KreuzbergException {
    /**
     * Constructs a new image processing exception with the specified message.
     *
     * @param message the detail message explaining why image processing failed
     */
    public ImageProcessingException(String message) {
        super(message);
    }

    /**
     * Constructs a new image processing exception with the specified message and cause.
     *
     * @param message the detail message
     * @param cause the cause of the image processing failure
     */
    public ImageProcessingException(String message, Throwable cause) {
        super(message, cause);
    }
}
