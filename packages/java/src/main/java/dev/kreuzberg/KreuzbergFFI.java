package dev.kreuzberg;

import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.MemoryLayout;
import java.lang.foreign.Linker;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.SymbolLookup;
import java.lang.foreign.StructLayout;
import java.lang.foreign.ValueLayout;
import java.lang.invoke.MethodHandle;
import java.util.Locale;

/**
 * Low-level FFI bindings to the Kreuzberg C library.
 *
 * This class provides direct access to the C functions exported by kreuzberg-ffi.
 * It uses the Java Foreign Function & Memory API (Panama) introduced in JDK 22.
 *
 * <p><strong>Internal API:</strong> This class is not intended for direct use.
 * Use the high-level {@link Kreuzberg} class instead.</p>
 */
final class KreuzbergFFI {
    private static final Linker LINKER = Linker.nativeLinker();
    private static final SymbolLookup LOOKUP;

    static final MethodHandle KREUZBERG_EXTRACT_FILE_SYNC;
    static final MethodHandle KREUZBERG_EXTRACT_FILE_SYNC_WITH_CONFIG;
    static final MethodHandle KREUZBERG_EXTRACT_BYTES_SYNC;
    static final MethodHandle KREUZBERG_EXTRACT_BYTES_SYNC_WITH_CONFIG;
    static final MethodHandle KREUZBERG_BATCH_EXTRACT_FILES_SYNC;
    static final MethodHandle KREUZBERG_BATCH_EXTRACT_BYTES_SYNC;
    static final MethodHandle KREUZBERG_LOAD_EXTRACTION_CONFIG_FROM_FILE;
    static final MethodHandle KREUZBERG_FREE_STRING;
    static final MethodHandle KREUZBERG_FREE_RESULT;
    static final MethodHandle KREUZBERG_FREE_BATCH_RESULT;
    static final MethodHandle KREUZBERG_LAST_ERROR;
    static final MethodHandle KREUZBERG_LAST_ERROR_CODE;
    static final MethodHandle KREUZBERG_LAST_PANIC_CONTEXT;
    static final MethodHandle KREUZBERG_VERSION;
    static final MethodHandle KREUZBERG_CLONE_STRING;
    static final MethodHandle KREUZBERG_REGISTER_POST_PROCESSOR;
    static final MethodHandle KREUZBERG_REGISTER_POST_PROCESSOR_WITH_STAGE;
    static final MethodHandle KREUZBERG_UNREGISTER_POST_PROCESSOR;
    static final MethodHandle KREUZBERG_CLEAR_POST_PROCESSORS;
    static final MethodHandle KREUZBERG_LIST_POST_PROCESSORS;
    static final MethodHandle KREUZBERG_REGISTER_VALIDATOR;
    static final MethodHandle KREUZBERG_UNREGISTER_VALIDATOR;
    static final MethodHandle KREUZBERG_CLEAR_VALIDATORS;
    static final MethodHandle KREUZBERG_LIST_VALIDATORS;
    static final MethodHandle KREUZBERG_REGISTER_OCR_BACKEND;
    static final MethodHandle KREUZBERG_REGISTER_OCR_BACKEND_WITH_LANGUAGES;
    static final MethodHandle KREUZBERG_UNREGISTER_OCR_BACKEND;
    static final MethodHandle KREUZBERG_LIST_OCR_BACKENDS;
    static final MethodHandle KREUZBERG_CLEAR_OCR_BACKENDS;
    static final MethodHandle KREUZBERG_LIST_DOCUMENT_EXTRACTORS;
    static final MethodHandle KREUZBERG_UNREGISTER_DOCUMENT_EXTRACTOR;
    static final MethodHandle KREUZBERG_CLEAR_DOCUMENT_EXTRACTORS;
    static final MethodHandle KREUZBERG_DETECT_MIME_TYPE;
    static final MethodHandle KREUZBERG_VALIDATE_MIME_TYPE;
    static final MethodHandle KREUZBERG_DETECT_MIME_TYPE_FROM_BYTES;
    static final MethodHandle KREUZBERG_GET_EXTENSIONS_FOR_MIME;
    static final MethodHandle KREUZBERG_CONFIG_DISCOVER;
    static final MethodHandle KREUZBERG_LIST_EMBEDDING_PRESETS;
    static final MethodHandle KREUZBERG_GET_EMBEDDING_PRESET;

    static final StructLayout C_EXTRACTION_RESULT_LAYOUT = MemoryLayout.structLayout(
        ValueLayout.ADDRESS.withName("content"),
        ValueLayout.ADDRESS.withName("mime_type"),
        ValueLayout.ADDRESS.withName("language"),
        ValueLayout.ADDRESS.withName("date"),
        ValueLayout.ADDRESS.withName("subject"),
        ValueLayout.ADDRESS.withName("tables_json"),
        ValueLayout.ADDRESS.withName("detected_languages_json"),
        ValueLayout.ADDRESS.withName("metadata_json"),
        ValueLayout.ADDRESS.withName("chunks_json"),
        ValueLayout.ADDRESS.withName("images_json"),
        ValueLayout.JAVA_BOOLEAN.withName("success"),
        MemoryLayout.paddingLayout(7)
    );

    static final long CONTENT_OFFSET = C_EXTRACTION_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("content"));
    static final long MIME_TYPE_OFFSET = C_EXTRACTION_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("mime_type"));
    static final long LANGUAGE_OFFSET = C_EXTRACTION_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("language"));
    static final long DATE_OFFSET = C_EXTRACTION_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("date"));
    static final long SUBJECT_OFFSET = C_EXTRACTION_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("subject"));
    static final long TABLES_OFFSET = C_EXTRACTION_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("tables_json"));
    static final long DETECTED_LANGUAGES_OFFSET = C_EXTRACTION_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("detected_languages_json"));
    static final long METADATA_OFFSET = C_EXTRACTION_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("metadata_json"));
    static final long CHUNKS_OFFSET = C_EXTRACTION_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("chunks_json"));
    static final long IMAGES_OFFSET = C_EXTRACTION_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("images_json"));
    static final long SUCCESS_OFFSET = C_EXTRACTION_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("success"));

    static final StructLayout C_BATCH_RESULT_LAYOUT = MemoryLayout.structLayout(
        ValueLayout.ADDRESS.withName("results"),
        ValueLayout.JAVA_LONG.withName("count"),
        ValueLayout.JAVA_BOOLEAN.withName("success"),
        MemoryLayout.paddingLayout(7)
    );

    static final long BATCH_RESULTS_PTR_OFFSET = C_BATCH_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("results"));
    static final long BATCH_COUNT_OFFSET = C_BATCH_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("count"));
    static final long BATCH_SUCCESS_OFFSET = C_BATCH_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("success"));

    static final StructLayout C_BYTES_WITH_MIME_LAYOUT = MemoryLayout.structLayout(
        ValueLayout.ADDRESS.withName("data"),
        ValueLayout.JAVA_LONG.withName("data_len"),
        ValueLayout.ADDRESS.withName("mime_type")
    );
    static final long BYTES_DATA_OFFSET = C_BYTES_WITH_MIME_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("data"));
    static final long BYTES_LEN_OFFSET = C_BYTES_WITH_MIME_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("data_len"));
    static final long BYTES_MIME_OFFSET = C_BYTES_WITH_MIME_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("mime_type"));
    static final long BYTES_WITH_MIME_ALIGNMENT = 8;

    static {
        try {
            loadNativeLibrary();
            LOOKUP = SymbolLookup.loaderLookup();

            KREUZBERG_EXTRACT_FILE_SYNC = linkFunction(
                "kreuzberg_extract_file_sync",
                FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS)
            );

            KREUZBERG_EXTRACT_FILE_SYNC_WITH_CONFIG = linkFunction(
                "kreuzberg_extract_file_sync_with_config",
                FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS)
            );

            KREUZBERG_EXTRACT_BYTES_SYNC = linkFunction(
                "kreuzberg_extract_bytes_sync",
                FunctionDescriptor.of(
                    ValueLayout.ADDRESS,
                    ValueLayout.ADDRESS,
                    ValueLayout.JAVA_LONG,
                    ValueLayout.ADDRESS
                )
            );

            KREUZBERG_EXTRACT_BYTES_SYNC_WITH_CONFIG = linkFunction(
                "kreuzberg_extract_bytes_sync_with_config",
                FunctionDescriptor.of(
                    ValueLayout.ADDRESS,
                    ValueLayout.ADDRESS,
                    ValueLayout.JAVA_LONG,
                    ValueLayout.ADDRESS,
                    ValueLayout.ADDRESS
                )
            );

            KREUZBERG_BATCH_EXTRACT_FILES_SYNC = linkFunction(
                "kreuzberg_batch_extract_files_sync",
                FunctionDescriptor.of(
                    ValueLayout.ADDRESS,
                    ValueLayout.ADDRESS,
                    ValueLayout.JAVA_LONG,
                    ValueLayout.ADDRESS
                )
            );

            KREUZBERG_BATCH_EXTRACT_BYTES_SYNC = linkFunction(
                "kreuzberg_batch_extract_bytes_sync",
                FunctionDescriptor.of(
                    ValueLayout.ADDRESS,
                    ValueLayout.ADDRESS,
                    ValueLayout.JAVA_LONG,
                    ValueLayout.ADDRESS
                )
            );

            KREUZBERG_LOAD_EXTRACTION_CONFIG_FROM_FILE = linkFunction(
                "kreuzberg_load_extraction_config_from_file",
                FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS)
            );

            KREUZBERG_FREE_STRING = linkFunction(
                "kreuzberg_free_string",
                FunctionDescriptor.ofVoid(ValueLayout.ADDRESS)
            );

            KREUZBERG_FREE_RESULT = linkFunction(
                "kreuzberg_free_result",
                FunctionDescriptor.ofVoid(ValueLayout.ADDRESS)
            );

            KREUZBERG_FREE_BATCH_RESULT = linkFunction(
                "kreuzberg_free_batch_result",
                FunctionDescriptor.ofVoid(ValueLayout.ADDRESS)
            );

            KREUZBERG_CLONE_STRING = linkFunction(
                "kreuzberg_clone_string",
                FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS)
            );

            KREUZBERG_LAST_ERROR = linkFunction(
                "kreuzberg_last_error",
                FunctionDescriptor.of(ValueLayout.ADDRESS)
            );

            KREUZBERG_LAST_ERROR_CODE = linkFunction(
                "kreuzberg_last_error_code",
                FunctionDescriptor.of(ValueLayout.JAVA_INT)
            );

            KREUZBERG_LAST_PANIC_CONTEXT = linkFunction(
                "kreuzberg_last_panic_context",
                FunctionDescriptor.of(ValueLayout.ADDRESS)
            );

            KREUZBERG_VERSION = linkFunction(
                "kreuzberg_version",
                FunctionDescriptor.of(ValueLayout.ADDRESS)
            );

            KREUZBERG_REGISTER_POST_PROCESSOR = linkFunction(
                "kreuzberg_register_post_processor",
                FunctionDescriptor.of(
                    ValueLayout.JAVA_BOOLEAN,
                    ValueLayout.ADDRESS,
                    ValueLayout.ADDRESS,
                    ValueLayout.JAVA_INT
                )
            );

            KREUZBERG_REGISTER_POST_PROCESSOR_WITH_STAGE = linkFunction(
                "kreuzberg_register_post_processor_with_stage",
                FunctionDescriptor.of(
                    ValueLayout.JAVA_BOOLEAN,
                    ValueLayout.ADDRESS,
                    ValueLayout.ADDRESS,
                    ValueLayout.JAVA_INT,
                    ValueLayout.ADDRESS
                )
            );

            KREUZBERG_UNREGISTER_POST_PROCESSOR = linkFunction(
                "kreuzberg_unregister_post_processor",
                FunctionDescriptor.of(ValueLayout.JAVA_BOOLEAN, ValueLayout.ADDRESS)
            );

            KREUZBERG_CLEAR_POST_PROCESSORS = linkFunction(
                "kreuzberg_clear_post_processors",
                FunctionDescriptor.of(ValueLayout.JAVA_BOOLEAN)
            );

            KREUZBERG_LIST_POST_PROCESSORS = linkFunction(
                "kreuzberg_list_post_processors",
                FunctionDescriptor.of(ValueLayout.ADDRESS)
            );

            KREUZBERG_REGISTER_VALIDATOR = linkFunction(
                "kreuzberg_register_validator",
                FunctionDescriptor.of(
                    ValueLayout.JAVA_BOOLEAN,
                    ValueLayout.ADDRESS,
                    ValueLayout.ADDRESS,
                    ValueLayout.JAVA_INT
                )
            );

            KREUZBERG_UNREGISTER_VALIDATOR = linkFunction(
                "kreuzberg_unregister_validator",
                FunctionDescriptor.of(ValueLayout.JAVA_BOOLEAN, ValueLayout.ADDRESS)
            );

            KREUZBERG_CLEAR_VALIDATORS = linkFunction(
                "kreuzberg_clear_validators",
                FunctionDescriptor.of(ValueLayout.JAVA_BOOLEAN)
            );

            KREUZBERG_LIST_VALIDATORS = linkFunction(
                "kreuzberg_list_validators",
                FunctionDescriptor.of(ValueLayout.ADDRESS)
            );

            KREUZBERG_REGISTER_OCR_BACKEND = linkFunction(
                "kreuzberg_register_ocr_backend",
                FunctionDescriptor.of(ValueLayout.JAVA_BOOLEAN, ValueLayout.ADDRESS, ValueLayout.ADDRESS)
            );

            KREUZBERG_REGISTER_OCR_BACKEND_WITH_LANGUAGES = linkFunction(
                "kreuzberg_register_ocr_backend_with_languages",
                FunctionDescriptor.of(
                    ValueLayout.JAVA_BOOLEAN,
                    ValueLayout.ADDRESS,
                    ValueLayout.ADDRESS,
                    ValueLayout.ADDRESS
                )
            );

            KREUZBERG_UNREGISTER_OCR_BACKEND = linkFunction(
                "kreuzberg_unregister_ocr_backend",
                FunctionDescriptor.of(ValueLayout.JAVA_BOOLEAN, ValueLayout.ADDRESS)
            );

            KREUZBERG_LIST_OCR_BACKENDS = linkFunction(
                "kreuzberg_list_ocr_backends",
                FunctionDescriptor.of(ValueLayout.ADDRESS)
            );

            KREUZBERG_CLEAR_OCR_BACKENDS = linkFunction(
                "kreuzberg_clear_ocr_backends",
                FunctionDescriptor.of(ValueLayout.JAVA_BOOLEAN)
            );

            KREUZBERG_LIST_DOCUMENT_EXTRACTORS = linkFunction(
                "kreuzberg_list_document_extractors",
                FunctionDescriptor.of(ValueLayout.ADDRESS)
            );

            KREUZBERG_UNREGISTER_DOCUMENT_EXTRACTOR = linkFunction(
                "kreuzberg_unregister_document_extractor",
                FunctionDescriptor.of(ValueLayout.JAVA_BOOLEAN, ValueLayout.ADDRESS)
            );

            KREUZBERG_CLEAR_DOCUMENT_EXTRACTORS = linkFunction(
                "kreuzberg_clear_document_extractors",
                FunctionDescriptor.of(ValueLayout.JAVA_BOOLEAN)
            );

            KREUZBERG_DETECT_MIME_TYPE = linkFunction(
                "kreuzberg_detect_mime_type",
                FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.JAVA_BOOLEAN)
            );

            KREUZBERG_VALIDATE_MIME_TYPE = linkFunction(
                "kreuzberg_validate_mime_type",
                FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS)
            );

            KREUZBERG_DETECT_MIME_TYPE_FROM_BYTES = linkFunction(
                "kreuzberg_detect_mime_type_from_bytes",
                FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.JAVA_LONG)
            );

            KREUZBERG_GET_EXTENSIONS_FOR_MIME = linkFunction(
                "kreuzberg_get_extensions_for_mime",
                FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS)
            );

            KREUZBERG_CONFIG_DISCOVER = linkFunction(
                "kreuzberg_config_discover",
                FunctionDescriptor.of(ValueLayout.ADDRESS)
            );

            KREUZBERG_LIST_EMBEDDING_PRESETS = linkFunction(
                "kreuzberg_list_embedding_presets",
                FunctionDescriptor.of(ValueLayout.ADDRESS)
            );

            KREUZBERG_GET_EMBEDDING_PRESET = linkFunction(
                "kreuzberg_get_embedding_preset",
                FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS)
            );
        } catch (Exception e) {
            throw new ExceptionInInitializerError(e);
        }
    }

    private KreuzbergFFI() {
    }

    /**
     * Links a C function to a Java MethodHandle.
     *
     * @param name the name of the C function
     * @param descriptor the function descriptor
     * @return a MethodHandle for the function
     */
    private static MethodHandle linkFunction(String name, FunctionDescriptor descriptor) {
        MemorySegment symbol = LOOKUP.find(name)
            .orElseThrow(() -> new UnsatisfiedLinkError("Failed to find symbol: " + name));
        return LINKER.downcallHandle(symbol, descriptor);
    }

    /**
     * Loads the native library from the classpath or system path.
     */
    private static void loadNativeLibrary() {
        String osName = System.getProperty("os.name").toLowerCase(Locale.ROOT);
        String libName;
        String libExt;
        String pdfiumLibName;

        if (osName.contains("mac") || osName.contains("darwin")) {
            libName = "libkreuzberg_ffi";
            pdfiumLibName = "libpdfium";
            libExt = ".dylib";
        } else if (osName.contains("win")) {
            libName = "kreuzberg_ffi";
            pdfiumLibName = "pdfium";
            libExt = ".dll";
        } else {
            libName = "libkreuzberg_ffi";
            pdfiumLibName = "libpdfium";
            libExt = ".so";
        }

        String ffiDir = System.getenv("KREUZBERG_FFI_DIR");
        if (ffiDir != null && !ffiDir.isEmpty()) {
            java.nio.file.Path ffiPath = java.nio.file.Path.of(ffiDir);
            java.nio.file.Path libPath = ffiPath.resolve(libName + libExt);
            java.nio.file.Path pdfiumPath = ffiPath.resolve(pdfiumLibName + libExt);

            if (java.nio.file.Files.exists(libPath)) {
                try {
                    if (java.nio.file.Files.exists(pdfiumPath)) {
                        System.load(pdfiumPath.toAbsolutePath().toString());
                    }
                    System.load(libPath.toAbsolutePath().toString());
                    return;
                } catch (UnsatisfiedLinkError e) {
                    System.err.println("[KreuzbergFFI] Failed to load native libraries from "
                        + libPath + ": " + e.getMessage());
                }
            }
        }

        String resourcePath = "/" + libName + libExt;
        String pdfiumResourcePath = "/" + pdfiumLibName + libExt;
        var resource = KreuzbergFFI.class.getResource(resourcePath);

        if (resource != null) {
            try (java.io.InputStream in = KreuzbergFFI.class.getResourceAsStream(resourcePath)) {
                java.nio.file.Path tempDir = java.nio.file.Files.createTempDirectory("kreuzberg_native");
                tempDir.toFile().deleteOnExit();

                java.nio.file.Path tempPdfium = null;
                var pdfiumResource = KreuzbergFFI.class.getResource(pdfiumResourcePath);
                if (pdfiumResource != null) {
                    var pdfiumStream = KreuzbergFFI.class.getResourceAsStream(pdfiumResourcePath);
                    try (java.io.InputStream pdfiumIn = pdfiumStream) {
                        tempPdfium = tempDir.resolve(pdfiumLibName + libExt);
                        tempPdfium.toFile().deleteOnExit();
                        var replaceExisting = java.nio.file.StandardCopyOption.REPLACE_EXISTING;
                        java.nio.file.Files.copy(pdfiumIn, tempPdfium, replaceExisting);
                    }
                }

                java.nio.file.Path tempLib = tempDir.resolve(libName + libExt);
                tempLib.toFile().deleteOnExit();
                java.nio.file.Files.copy(in, tempLib, java.nio.file.StandardCopyOption.REPLACE_EXISTING);

                if (tempPdfium != null) {
                    System.load(tempPdfium.toAbsolutePath().toString());
                }

                System.load(tempLib.toAbsolutePath().toString());
                return;
            } catch (Exception e) {
                System.err.println("[KreuzbergFFI] Failed to extract and load native libraries "
                    + "from resources: " + e.getMessage());
                e.printStackTrace();
            }
        }

        String projectRoot = System.getProperty("user.dir");
        java.nio.file.Path targetLib = java.nio.file.Path.of(
            projectRoot, "target", "classes", libName + libExt);
        java.nio.file.Path targetPdfium = java.nio.file.Path.of(
            projectRoot, "target", "classes", pdfiumLibName + libExt);

        if (java.nio.file.Files.exists(targetLib)) {
            if (java.nio.file.Files.exists(targetPdfium)) {
                System.load(targetPdfium.toAbsolutePath().toString());
            }
            System.load(targetLib.toAbsolutePath().toString());
            return;
        }

        try {
            System.loadLibrary("pdfium");
        } catch (UnsatisfiedLinkError e) {
            System.err.println("[KreuzbergFFI] Failed to load optional pdfium library: " + e.getMessage());
        }
        System.loadLibrary("kreuzberg_ffi");
    }

    /**
     * Reads a null-terminated C string from native memory.
     *
     * @param address the address of the C string
     * @return the Java String, or null if address is NULL
     */
    static String readCString(MemorySegment address) {
        if (address == null || address.address() == 0) {
            return null;
        }
        return address.reinterpret(Long.MAX_VALUE).getString(0);
    }

    /**
     * Allocates native memory for a C string.
     *
     * @param arena the arena to allocate in
     * @param str the Java string
     * @return a MemorySegment containing the C string
     */
    static MemorySegment allocateCString(Arena arena, String str) {
        return arena.allocateFrom(str);
    }

    /**
     * Gets the last error code from the FFI layer.
     *
     * @return the error code as an integer
     * @throws Throwable if the FFI call fails unexpectedly
     */
    static int getLastErrorCode() throws Throwable {
        return (int) KREUZBERG_LAST_ERROR_CODE.invoke();
    }

    /**
     * Gets the panic context from the FFI layer.
     *
     * @return a PanicContext if a panic occurred, or null if no panic
     * @throws Throwable if the FFI call fails unexpectedly
     */
    static PanicContext getLastPanicContext() throws Throwable {
        MemorySegment result = (MemorySegment) KREUZBERG_LAST_PANIC_CONTEXT.invoke();
        if (result != null && result.address() != 0) {
            String jsonString = readCString(result);
            try {
                KREUZBERG_FREE_STRING.invoke(result);
            } catch (Exception ex) {
                System.err.println("Failed to free panic context: " + ex);
            }
            if (jsonString != null) {
                return PanicContext.fromJson(jsonString);
            }
        }
        return null;
    }
}
