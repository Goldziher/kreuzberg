package dev.kreuzberg;

/**
 * Exception thrown when Kreuzberg extraction operations fail.
 *
 * <p>This exception wraps errors from the native Kreuzberg library,
 * including parsing errors, unsupported formats, and internal errors.</p>
 */
public class KreuzbergException extends Exception {
    /**
     * Creates a new KreuzbergException with the specified message.
     *
     * @param message the error message
     */
    public KreuzbergException(String message) {
        super(message);
    }

    /**
     * Creates a new KreuzbergException with the specified message and cause.
     *
     * @param message the error message
     * @param cause the underlying cause
     */
    public KreuzbergException(String message, Throwable cause) {
        super(message, cause);
    }
}
