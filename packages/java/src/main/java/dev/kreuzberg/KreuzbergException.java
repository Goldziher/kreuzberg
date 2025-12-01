package dev.kreuzberg;

/**
 * Exception thrown when Kreuzberg extraction operations fail.
 *
 * <p>This exception wraps errors from the native Kreuzberg library,
 * including parsing errors, unsupported formats, and internal errors.
 * When a panic occurs in the native library, the exception will include
 * panic context information accessible via {@link #getPanicContext()}.</p>
 *
 * @since 4.0.0
 */
public class KreuzbergException extends Exception {
    private final ErrorCode errorCode;
    private final PanicContext panicContext;

    /**
     * Creates a new KreuzbergException with the specified message.
     *
     * @param message the error message
     */
    public KreuzbergException(String message) {
        this(message, ErrorCode.GENERIC_ERROR, null);
    }

    /**
     * Creates a new KreuzbergException with the specified message and cause.
     *
     * @param message the error message
     * @param cause the underlying cause
     */
    public KreuzbergException(String message, Throwable cause) {
        this(message, ErrorCode.GENERIC_ERROR, null, cause);
    }

    /**
     * Creates a new KreuzbergException with the specified message and error code.
     *
     * @param message the error message
     * @param errorCode the error code
     * @since 4.0.0
     */
    public KreuzbergException(String message, ErrorCode errorCode) {
        this(message, errorCode, null);
    }

    /**
     * Creates a new KreuzbergException with the specified message, error code, and panic context.
     *
     * @param message the error message
     * @param errorCode the error code
     * @param panicContext the panic context, or null if no panic occurred
     * @since 4.0.0
     */
    public KreuzbergException(String message, ErrorCode errorCode, PanicContext panicContext) {
        super(buildFullMessage(message, panicContext));
        this.errorCode = errorCode;
        this.panicContext = panicContext;
    }

    /**
     * Creates a new KreuzbergException with the specified message, error code, panic context, and cause.
     *
     * @param message the error message
     * @param errorCode the error code
     * @param panicContext the panic context, or null if no panic occurred
     * @param cause the underlying cause
     * @since 4.0.0
     */
    public KreuzbergException(String message, ErrorCode errorCode, PanicContext panicContext, Throwable cause) {
        super(buildFullMessage(message, panicContext), cause);
        this.errorCode = errorCode;
        this.panicContext = panicContext;
    }

    /**
     * Builds the full exception message including panic context if present.
     *
     * @param message the base error message
     * @param panicContext the panic context, or null
     * @return the full message to use for the exception
     */
    private static String buildFullMessage(String message, PanicContext panicContext) {
        if (panicContext != null) {
            return message + "\n[Panic Context: " + panicContext + "]";
        }
        return message;
    }

    /**
     * Returns the error code for this exception.
     *
     * @return the error code
     * @since 4.0.0
     */
    public ErrorCode getErrorCode() {
        return errorCode;
    }

    /**
     * Returns the panic context for this exception, if a panic occurred.
     *
     * @return the panic context, or null if no panic occurred
     * @since 4.0.0
     */
    public PanicContext getPanicContext() {
        return panicContext;
    }
}
