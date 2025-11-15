package dev.kreuzberg;

import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.Linker;
import java.lang.foreign.MemoryLayout;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.StructLayout;
import java.lang.foreign.SymbolLookup;
import java.lang.foreign.ValueLayout;
import java.lang.invoke.MethodHandle;

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

    // Function handles
    static final MethodHandle KREUZBERG_EXTRACT_FILE_SYNC;
    static final MethodHandle KREUZBERG_EXTRACT_FILE_SYNC_WITH_CONFIG;
    static final MethodHandle KREUZBERG_FREE_STRING;
    static final MethodHandle KREUZBERG_FREE_RESULT;
    static final MethodHandle KREUZBERG_LAST_ERROR;
    static final MethodHandle KREUZBERG_VERSION;
    static final MethodHandle KREUZBERG_REGISTER_OCR_BACKEND;

    // Memory layouts
    static final StructLayout C_EXTRACTION_RESULT_LAYOUT = MemoryLayout.structLayout(
        ValueLayout.ADDRESS.withName("content"),
        ValueLayout.ADDRESS.withName("mime_type"),
        ValueLayout.ADDRESS.withName("language"),
        ValueLayout.ADDRESS.withName("date"),
        ValueLayout.ADDRESS.withName("subject"),
        ValueLayout.ADDRESS.withName("tables_json"),
        ValueLayout.ADDRESS.withName("detected_languages_json"),
        ValueLayout.ADDRESS.withName("metadata_json"),
        ValueLayout.JAVA_BOOLEAN.withName("success"),
        MemoryLayout.paddingLayout(7) // Padding to align to 8 bytes
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
    static final long TABLES_JSON_OFFSET = C_EXTRACTION_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("tables_json"));
    static final long DETECTED_LANGUAGES_JSON_OFFSET = C_EXTRACTION_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("detected_languages_json"));
    static final long METADATA_JSON_OFFSET = C_EXTRACTION_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("metadata_json"));
    static final long SUCCESS_OFFSET = C_EXTRACTION_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("success"));

    static {
        try {
            // Load the native library
            loadNativeLibrary();
            LOOKUP = SymbolLookup.loaderLookup();

            // Link to C functions
            KREUZBERG_EXTRACT_FILE_SYNC = linkFunction(
                "kreuzberg_extract_file_sync",
                FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS)
            );

            KREUZBERG_EXTRACT_FILE_SYNC_WITH_CONFIG = linkFunction(
                "kreuzberg_extract_file_sync_with_config",
                FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS)
            );

            KREUZBERG_FREE_STRING = linkFunction(
                "kreuzberg_free_string",
                FunctionDescriptor.ofVoid(ValueLayout.ADDRESS)
            );

            KREUZBERG_FREE_RESULT = linkFunction(
                "kreuzberg_free_result",
                FunctionDescriptor.ofVoid(ValueLayout.ADDRESS)
            );

            KREUZBERG_LAST_ERROR = linkFunction(
                "kreuzberg_last_error",
                FunctionDescriptor.of(ValueLayout.ADDRESS)
            );

            KREUZBERG_VERSION = linkFunction(
                "kreuzberg_version",
                FunctionDescriptor.of(ValueLayout.ADDRESS)
            );

            KREUZBERG_REGISTER_OCR_BACKEND = linkFunction(
                "kreuzberg_register_ocr_backend",
                FunctionDescriptor.of(ValueLayout.JAVA_BOOLEAN, ValueLayout.ADDRESS, ValueLayout.ADDRESS)
            );
        } catch (Exception e) {
            throw new ExceptionInInitializerError(e);
        }
    }

    private KreuzbergFFI() {
        // Private constructor to prevent instantiation
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
        String osName = System.getProperty("os.name").toLowerCase();
        String libName;
        String libExt;

        // Determine library name and extension based on OS
        if (osName.contains("mac") || osName.contains("darwin")) {
            libName = "libkreuzberg_ffi";
            libExt = ".dylib";
        } else if (osName.contains("win")) {
            libName = "kreuzberg_ffi";
            libExt = ".dll";
        } else {
            libName = "libkreuzberg_ffi";
            libExt = ".so";
        }

        // Try to load from classpath first (for packaged JAR)
        String resourcePath = "/" + libName + libExt;
        var resource = KreuzbergFFI.class.getResource(resourcePath);

        if (resource != null) {
            // Library found in classpath, extract and load it
            try {
                // On macOS/Linux, preload libpdfium if it exists (kreuzberg_ffi depends on it)
                if (!osName.contains("win")) {
                    String pdfiumName = osName.contains("mac") || osName.contains("darwin")
                            ? "libpdfium.dylib" : "libpdfium.so";
                    String pdfiumPath = "/" + pdfiumName;
                    java.io.InputStream pdfiumIn = KreuzbergFFI.class.getResourceAsStream(pdfiumPath);
                    if (pdfiumIn != null) {
                        java.nio.file.Path tempPdfium = java.nio.file.Files.createTempFile("libpdfium", libExt);
                        tempPdfium.toFile().deleteOnExit();
                        java.nio.file.Files.copy(pdfiumIn, tempPdfium,
                                java.nio.file.StandardCopyOption.REPLACE_EXISTING);
                        pdfiumIn.close();
                        try {
                            System.load(tempPdfium.toAbsolutePath().toString());
                        } catch (UnsatisfiedLinkError e) {
                            // Ignore if pdfium can't be loaded, main library load will fail if needed
                        }
                    }
                }

                java.io.InputStream in = KreuzbergFFI.class.getResourceAsStream(resourcePath);
                java.nio.file.Path tempLib = java.nio.file.Files.createTempFile(libName, libExt);
                tempLib.toFile().deleteOnExit();
                java.nio.file.Files.copy(in, tempLib, java.nio.file.StandardCopyOption.REPLACE_EXISTING);
                in.close();
                System.load(tempLib.toAbsolutePath().toString());
                return;
            } catch (Exception e) {
                // Fall through to try loading from library path
            }
        }

        // Try to load from build directory (for development/testing)
        String projectRoot = System.getProperty("user.dir");
        java.nio.file.Path targetLib = java.nio.file.Path.of(projectRoot, "target", "classes", libName + libExt);

        if (java.nio.file.Files.exists(targetLib)) {
            // On macOS/Linux, preload libpdfium if it exists (kreuzberg_ffi depends on it)
            if (!osName.contains("win")) {
                String pdfiumName = osName.contains("mac") || osName.contains("darwin")
                        ? "libpdfium.dylib" : "libpdfium.so";
                java.nio.file.Path pdfiumLib = java.nio.file.Path.of(projectRoot, "target", "classes", pdfiumName);
                if (java.nio.file.Files.exists(pdfiumLib)) {
                    try {
                        System.load(pdfiumLib.toAbsolutePath().toString());
                    } catch (UnsatisfiedLinkError e) {
                        // Ignore if pdfium can't be loaded, main library load will fail if truly needed
                    }
                }
            }
            System.load(targetLib.toAbsolutePath().toString());
            return;
        }

        // Fall back to system library path
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
}
