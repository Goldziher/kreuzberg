package dev.kreuzberg;

import dev.kreuzberg.config.ExtractionConfig;
import java.io.IOException;
import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.Linker;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.ValueLayout;
import java.lang.invoke.MethodHandle;
import java.lang.invoke.MethodHandles;
import java.lang.invoke.MethodType;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.List;
import java.util.Map;

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
    private static final Linker LINKER = Linker.nativeLinker();

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

            return processExtractionResult(resultPtr);
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
     * Extract text and metadata from a file with custom configuration.
     *
     * @param path the path to the file to extract
     * @param config the extraction configuration
     * @return the extraction result
     * @throws IOException if the file does not exist or cannot be read
     * @throws KreuzbergException if the extraction fails
     */
    public static ExtractionResult extractFile(String path, ExtractionConfig config)
            throws IOException, KreuzbergException {
        return extractFile(Path.of(path), config);
    }

    /**
     * Extract text and metadata from a file with custom configuration.
     *
     * @param path the path to the file to extract
     * @param config the extraction configuration
     * @return the extraction result
     * @throws IOException if the file does not exist or cannot be read
     * @throws KreuzbergException if the extraction fails
     */
    public static ExtractionResult extractFile(Path path, ExtractionConfig config)
            throws IOException, KreuzbergException {
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

        // Serialize config to JSON
        String configJson = toJson(config.toMap());

        try (Arena arena = Arena.ofConfined()) {
            // Convert path and config to C strings
            MemorySegment pathSegment = KreuzbergFFI.allocateCString(arena, path.toString());
            MemorySegment configSegment = KreuzbergFFI.allocateCString(arena, configJson);

            // Call C function
            MemorySegment resultPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_EXTRACT_FILE_SYNC_WITH_CONFIG
                .invoke(pathSegment, configSegment);

            return processExtractionResult(resultPtr);
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error during extraction", e);
        }
    }

    /**
     * Extract text and metadata from a file with post-processors and validators.
     *
     * @param path the path to the file to extract
     * @param config the extraction configuration (may be null for defaults)
     * @param postProcessors list of post-processors to apply (may be null or empty)
     * @param validators list of validators to run (may be null or empty)
     * @return the extraction result after processing and validation
     * @throws IOException if the file does not exist or cannot be read
     * @throws KreuzbergException if the extraction fails
     * @throws ValidationException if validation fails
     */
    public static ExtractionResult extractFile(
            Path path,
            ExtractionConfig config,
            List<PostProcessor> postProcessors,
            List<Validator> validators
    ) throws IOException, KreuzbergException {
        // Perform base extraction
        ExtractionResult result = config != null
            ? extractFile(path, config)
            : extractFile(path);

        // Apply post-processors
        if (postProcessors != null && !postProcessors.isEmpty()) {
            for (PostProcessor processor : postProcessors) {
                result = processor.process(result);
            }
        }

        // Run validators
        if (validators != null && !validators.isEmpty()) {
            for (Validator validator : validators) {
                validator.validate(result);
            }
        }

        return result;
    }

    /**
     * Extract text and metadata from a file with post-processors.
     *
     * @param path the path to the file to extract
     * @param postProcessors list of post-processors to apply
     * @return the extraction result after processing
     * @throws IOException if the file does not exist or cannot be read
     * @throws KreuzbergException if the extraction fails
     */
    public static ExtractionResult extractFile(Path path, List<PostProcessor> postProcessors)
            throws IOException, KreuzbergException {
        return extractFile(path, null, postProcessors, null);
    }

    /**
     * Extract text and metadata from a file with post-processors (varargs).
     *
     * @param path the path to the file to extract
     * @param postProcessors post-processors to apply
     * @return the extraction result after processing
     * @throws IOException if the file does not exist or cannot be read
     * @throws KreuzbergException if the extraction fails
     */
    public static ExtractionResult extractFile(Path path, PostProcessor... postProcessors)
            throws IOException, KreuzbergException {
        return extractFile(path, null, Arrays.asList(postProcessors), null);
    }

    /**
     * Extract text and metadata from a file with post-processors and validators.
     *
     * @param path the path to the file to extract
     * @param postProcessors list of post-processors to apply
     * @param validators list of validators to run
     * @return the extraction result after processing and validation
     * @throws IOException if the file does not exist or cannot be read
     * @throws KreuzbergException if the extraction fails
     * @throws ValidationException if validation fails
     */
    public static ExtractionResult extractFileWithValidation(
            Path path,
            List<PostProcessor> postProcessors,
            List<Validator> validators
    ) throws IOException, KreuzbergException {
        return extractFile(path, null, postProcessors, validators);
    }

    /**
     * Extract text and metadata from a file with configuration and post-processors.
     *
     * @param path the path to the file to extract
     * @param config the extraction configuration
     * @param postProcessors list of post-processors to apply
     * @return the extraction result after processing
     * @throws IOException if the file does not exist or cannot be read
     * @throws KreuzbergException if the extraction fails
     */
    public static ExtractionResult extractFile(
            Path path,
            ExtractionConfig config,
            List<PostProcessor> postProcessors
    ) throws IOException, KreuzbergException {
        return extractFile(path, config, postProcessors, null);
    }

    /**
     * Register a custom OCR backend for use in extraction.
     *
     * <p>The registered backend will be available for use via the backend name in OcrConfig.
     * The Arena parameter controls the lifecycle of the callback - the backend remains registered
     * as long as the Arena is alive.</p>
     *
     * @param name the backend name (e.g., "my-ocr", "custom-tesseract")
     * @param backend the OCR backend implementation
     * @param arena the Arena that manages the callback lifecycle
     * @return true if registration succeeded, false otherwise
     * @throws IllegalArgumentException if name is null or empty
     * @throws KreuzbergException if registration fails
     *
     * @since 4.0.0
     */
    public static void registerOcrBackend(String name, OcrBackend backend, Arena arena)
            throws KreuzbergException {
        if (name == null || name.isEmpty()) {
            throw new IllegalArgumentException("Backend name cannot be null or empty");
        }
        if (backend == null) {
            throw new IllegalArgumentException("Backend cannot be null");
        }
        if (arena == null) {
            throw new IllegalArgumentException("Arena cannot be null");
        }

        try {
            // Convert backend name to C string
            MemorySegment nameSegment = KreuzbergFFI.allocateCString(arena, name);

            // Create upcall stub for the backend
            // The callback signature matches the Rust side:
            // (image_bytes: *const u8, image_length: usize, config_json: *const c_char) -> *mut c_char
            FunctionDescriptor callbackDescriptor = FunctionDescriptor.of(
                ValueLayout.ADDRESS,  // return: *mut c_char (text result)
                ValueLayout.ADDRESS,  // image_bytes: *const u8
                ValueLayout.JAVA_LONG,  // image_length: usize
                ValueLayout.ADDRESS   // config_json: *const c_char
            );

            MethodHandle backendMethod = MethodHandles.lookup().bind(backend, "processImage",
                MethodType.methodType(MemorySegment.class, MemorySegment.class, long.class, MemorySegment.class));

            MemorySegment callbackPointer = LINKER.upcallStub(backendMethod, callbackDescriptor, arena);

            // Register with Rust
            boolean success = (boolean) KreuzbergFFI.KREUZBERG_REGISTER_OCR_BACKEND.invoke(
                nameSegment,
                callbackPointer
            );

            if (!success) {
                String error = getLastError();
                throw new KreuzbergException("Failed to register OCR backend: " + error);
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error during OCR backend registration", e);
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

    /**
     * Processes the extraction result from the native library.
     *
     * @param resultPtr the pointer to the C extraction result
     * @return the extraction result
     * @throws KreuzbergException if extraction failed
     * @throws Throwable if an unexpected error occurs
     */
    private static ExtractionResult processExtractionResult(MemorySegment resultPtr)
            throws KreuzbergException, Throwable {
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
    }

    /**
     * Converts a Map to JSON string (simple implementation without external dependencies).
     *
     * @param map the map to convert
     * @return JSON string representation
     */
    private static String toJson(Map<String, Object> map) {
        StringBuilder json = new StringBuilder("{");
        boolean first = true;
        for (Map.Entry<String, Object> entry : map.entrySet()) {
            if (!first) {
                json.append(",");
            }
            first = false;
            json.append("\"").append(entry.getKey()).append("\":");
            json.append(toJsonValue(entry.getValue()));
        }
        json.append("}");
        return json.toString();
    }

    @SuppressWarnings("unchecked")
    private static String toJsonValue(Object value) {
        if (value == null) {
            return "null";
        } else if (value instanceof String) {
            return "\"" + escapeJson((String) value) + "\"";
        } else if (value instanceof Number || value instanceof Boolean) {
            return value.toString();
        } else if (value instanceof List) {
            StringBuilder json = new StringBuilder("[");
            boolean first = true;
            for (Object item : (List<?>) value) {
                if (!first) {
                    json.append(",");
                }
                first = false;
                json.append(toJsonValue(item));
            }
            json.append("]");
            return json.toString();
        } else if (value instanceof Map) {
            return toJson((Map<String, Object>) value);
        } else {
            return "\"" + escapeJson(value.toString()) + "\"";
        }
    }

    private static String escapeJson(String str) {
        return str.replace("\\", "\\\\")
                  .replace("\"", "\\\"")
                  .replace("\n", "\\n")
                  .replace("\r", "\\r")
                  .replace("\t", "\\t");
    }
}
