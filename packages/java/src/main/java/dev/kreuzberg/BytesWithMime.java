package dev.kreuzberg;

/**
 * Container for byte array data with an associated MIME type.
 *
 * <p>This class is used for batch extraction operations where multiple
 * byte arrays need to be processed together.</p>
 *
 * @param data the document data as a byte array
 * @param mimeType the MIME type of the data (e.g., "application/pdf")
 */
public record BytesWithMime(byte[] data, String mimeType) {
    /**
     * Creates a new BytesWithMime instance.
     *
     * @param data the document data
     * @param mimeType the MIME type
     * @throws IllegalArgumentException if data or mimeType is null or empty
     */
    public BytesWithMime {
        if (data == null || data.length == 0) {
            throw new IllegalArgumentException("Data cannot be null or empty");
        }
        if (mimeType == null || mimeType.isEmpty()) {
            throw new IllegalArgumentException("MIME type cannot be null or empty");
        }
    }
}
