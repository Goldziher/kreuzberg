package dev.kreuzberg;

/**
 * Exception thrown when extraction result validation fails.
 *
 * <p>This exception indicates that an extraction result did not meet
 * validation criteria (e.g., minimum content length, required metadata, quality checks).</p>
 *
 * @since 4.0.0
 */
public final class ValidationException extends KreuzbergException {
    /**
     * Constructs a new validation exception with the specified message.
     *
     * @param message the detail message explaining why validation failed
     */
    public ValidationException(String message) {
        super(message);
    }

    /**
     * Constructs a new validation exception with the specified message and cause.
     *
     * @param message the detail message
     * @param cause the cause of the validation failure
     */
    public ValidationException(String message, Throwable cause) {
        super(message, cause);
    }
}
