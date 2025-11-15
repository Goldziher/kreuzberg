package dev.kreuzberg;

/**
 * Exception thrown when cache operations fail.
 *
 * <p>Cache errors are typically non-fatal and occur during caching operations, such as:</p>
 * <ul>
 *   <li>Cache directory creation failures</li>
 *   <li>Disk write errors</li>
 *   <li>Cache entry corruption</li>
 *   <li>Insufficient disk space</li>
 * </ul>
 *
 * <p>These errors are usually logged but don't prevent extraction from completing.</p>
 *
 * @since 4.0.0
 */
public final class CacheException extends KreuzbergException {
    /**
     * Constructs a new cache exception with the specified message.
     *
     * @param message the detail message explaining why the cache operation failed
     */
    public CacheException(String message) {
        super(message);
    }

    /**
     * Constructs a new cache exception with the specified message and cause.
     *
     * @param message the detail message
     * @param cause the cause of the cache failure
     */
    public CacheException(String message, Throwable cause) {
        super(message, cause);
    }
}
