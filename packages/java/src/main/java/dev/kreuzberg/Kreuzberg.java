package dev.kreuzberg;

import java.io.IOException;
import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.ValueLayout;
import java.nio.file.Files;
import java.nio.file.Path;

/**
 * High-level Java API for Kreuzberg document intelligence library.
 *
 * <p>Kreuzberg is a powerful document extraction library that supports various file formats
 * including PDFs, Office documents, images, and more.</p>
 *
 * <h2>Basic Usage</h2>
 * <pre>{@code
 * ExtractionResult result = Kreuzberg.extractFile("document.pdf");
 * System.out.println(result.content());
 * System.out.println("Type: " + result.mimeType());
 * }</pre>
 *
 * <h2>Error Handling</h2>
 * <pre>{@code
 * try {
 *     ExtractionResult result = Kreuzberg.extractFile("document.pdf");
 *     // Process result
 * } catch (KreuzbergException e) {
 *     System.err.println("Extraction failed: " + e.getMessage());
 * } catch (IOException e) {
 *     System.err.println("File not found: " + e.getMessage());
 * }
 * }</pre>
 */
public final class Kreuzberg {
    private Kreuzberg() {
        // Private constructor to prevent instantiation
    }

    /**
     * Extract text and metadata from a file.
     *
     * @param path the path to the file to extract
     * @return the extraction result
     * @throws IOException if the file does not exist or cannot be read
     * @throws KreuzbergException if the extraction fails
     */
    public static ExtractionResult extractFile(String path) throws IOException, KreuzbergException {
        return extractFile(Path.of(path));
    }

    /**
     * Extract text and metadata from a file.
     *
     * @param path the path to the file to extract
     * @return the extraction result
     * @throws IOException if the file does not exist or cannot be read
     * @throws KreuzbergException if the extraction fails
     */
    public static ExtractionResult extractFile(Path path) throws IOException, KreuzbergException {
        // Validate file exists
        if (!Files.exists(path)) {
            throw new IOException("File not found: " + path);
        }
        if (!Files.isRegularFile(path)) {
            throw new IOException("Not a regular file: " + path);
        }
        if (!Files.isReadable(path)) {
            throw new IOException("File not readable: " + path);
        }

        try (Arena arena = Arena.ofConfined()) {
            // Convert path to C string
            MemorySegment pathSegment = KreuzbergFFI.allocateCString(arena, path.toString());

            // Call C function
            MemorySegment resultPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_EXTRACT_FILE_SYNC
                .invoke(pathSegment);

            // Check for null (error)
            if (resultPtr == null || resultPtr.address() == 0) {
                String error = getLastError();
                throw new KreuzbergException("Extraction failed: " + error);
            }

            try {
                // Read result fields
                MemorySegment result = resultPtr.reinterpret(KreuzbergFFI.C_EXTRACTION_RESULT_LAYOUT.byteSize());

                boolean success = result.get(ValueLayout.JAVA_BOOLEAN, KreuzbergFFI.SUCCESS_OFFSET);
                if (!success) {
                    String error = getLastError();
                    throw new KreuzbergException("Extraction failed: " + error);
                }

                // Read string fields
                MemorySegment contentPtr = result.get(ValueLayout.ADDRESS, KreuzbergFFI.CONTENT_OFFSET);
                MemorySegment mimeTypePtr = result.get(ValueLayout.ADDRESS, KreuzbergFFI.MIME_TYPE_OFFSET);
                MemorySegment languagePtr = result.get(ValueLayout.ADDRESS, KreuzbergFFI.LANGUAGE_OFFSET);
                MemorySegment datePtr = result.get(ValueLayout.ADDRESS, KreuzbergFFI.DATE_OFFSET);
                MemorySegment subjectPtr = result.get(ValueLayout.ADDRESS, KreuzbergFFI.SUBJECT_OFFSET);

                String content = KreuzbergFFI.readCString(contentPtr);
                String mimeType = KreuzbergFFI.readCString(mimeTypePtr);
                String language = KreuzbergFFI.readCString(languagePtr);
                String date = KreuzbergFFI.readCString(datePtr);
                String subject = KreuzbergFFI.readCString(subjectPtr);

                return ExtractionResult.of(content, mimeType, language, date, subject);
            } finally {
                // Free the result
                KreuzbergFFI.KREUZBERG_FREE_RESULT.invoke(resultPtr);
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error during extraction", e);
        }
    }

    /**
     * Get the Kreuzberg library version.
     *
     * @return the version string
     */
    public static String getVersion() {
        try {
            MemorySegment versionPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_VERSION.invoke();
            return KreuzbergFFI.readCString(versionPtr);
        } catch (Throwable e) {
            throw new RuntimeException("Failed to get version", e);
        }
    }

    /**
     * Gets the last error message from the native library.
     *
     * @return the error message, or a default message if none available
     */
    private static String getLastError() {
        try {
            MemorySegment errorPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_LAST_ERROR.invoke();
            String error = KreuzbergFFI.readCString(errorPtr);
            return error != null ? error : "Unknown error";
        } catch (Throwable e) {
            return "Unknown error (failed to retrieve error message)";
        }
    }
}
