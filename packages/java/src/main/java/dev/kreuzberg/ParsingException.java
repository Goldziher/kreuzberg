package dev.kreuzberg;

/**
 * Exception thrown when document parsing fails.
 *
 * <p>Parsing errors occur when a document is corrupted, malformed, or cannot
 * be processed by the extraction engine. This includes issues like:</p>
 * <ul>
 *   <li>Corrupted PDF files</li>
 *   <li>Invalid XML/JSON syntax</li>
 *   <li>Unsupported file format versions</li>
 *   <li>Encrypted documents without valid passwords</li>
 * </ul>
 *
 * @since 4.0.0
 */
public final class ParsingException extends KreuzbergException {
    /**
     * Constructs a new parsing exception with the specified message.
     *
     * @param message the detail message explaining why parsing failed
     */
    public ParsingException(String message) {
        super(message);
    }

    /**
     * Constructs a new parsing exception with the specified message and cause.
     *
     * @param message the detail message
     * @param cause the cause of the parsing failure
     */
    public ParsingException(String message, Throwable cause) {
        super(message, cause);
    }
}
