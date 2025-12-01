package dev.kreuzberg;

/**
 * Enumeration of error codes returned by the Kreuzberg C library.
 *
 * <p>Error codes indicate the type of error that occurred during
 * FFI operations. Use {@link KreuzbergException#getErrorCode()} to retrieve
 * the code for a specific exception.</p>
 *
 * @since 4.0.0
 */
public enum ErrorCode {
    /**
     * No error occurred (0).
     */
    SUCCESS(0),

    /**
     * Generic error that doesn't fit other categories (1).
     */
    GENERIC_ERROR(1),

    /**
     * A panic occurred in the native library (2).
     */
    PANIC(2),

    /**
     * Invalid argument provided to the library (3).
     */
    INVALID_ARGUMENT(3),

    /**
     * I/O error during file or stream operations (4).
     */
    IO_ERROR(4),

    /**
     * Error during document parsing (5).
     */
    PARSING_ERROR(5),

    /**
     * Error during optical character recognition (6).
     */
    OCR_ERROR(6),

    /**
     * Required dependency is missing (7).
     */
    MISSING_DEPENDENCY(7);

    private final int code;

    ErrorCode(int code) {
        this.code = code;
    }

    /**
     * Returns the numeric code value.
     *
     * @return the error code as an integer
     */
    public int getCode() {
        return code;
    }

    private static final int CODE_GENERIC_ERROR = 1;
    private static final int CODE_PANIC = 2;
    private static final int CODE_INVALID_ARGUMENT = 3;
    private static final int CODE_IO_ERROR = 4;
    private static final int CODE_PARSING_ERROR = 5;
    private static final int CODE_OCR_ERROR = 6;
    private static final int CODE_MISSING_DEPENDENCY = 7;

    /**
     * Returns the ErrorCode for the given numeric code.
     *
     * @param code the numeric error code
     * @return the corresponding ErrorCode, or SUCCESS if code is not recognized
     */
    public static ErrorCode fromCode(int code) {
        return switch (code) {
            case CODE_GENERIC_ERROR -> GENERIC_ERROR;
            case CODE_PANIC -> PANIC;
            case CODE_INVALID_ARGUMENT -> INVALID_ARGUMENT;
            case CODE_IO_ERROR -> IO_ERROR;
            case CODE_PARSING_ERROR -> PARSING_ERROR;
            case CODE_OCR_ERROR -> OCR_ERROR;
            case CODE_MISSING_DEPENDENCY -> MISSING_DEPENDENCY;
            default -> SUCCESS;
        };
    }
}
