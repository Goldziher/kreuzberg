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
import java.net.URL;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.Enumeration;
import java.util.List;
import java.util.Locale;
import java.util.jar.JarEntry;
import java.util.jar.JarFile;

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
    private static final long C_STRING_MAX_SIZE = 1048576L;
    private static final String NATIVES_RESOURCE_ROOT = "/natives";
    private static final Object NATIVE_EXTRACT_LOCK = new Object();
    private static String cachedExtractKey;
    private static Path cachedExtractDir;

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
        ValueLayout.ADDRESS.withName("page_structure_json"),
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
    static final long PAGE_STRUCTURE_OFFSET = C_EXTRACTION_RESULT_LAYOUT.byteOffset(
        MemoryLayout.PathElement.groupElement("page_structure_json"));
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
        String osArch = System.getProperty("os.arch").toLowerCase(Locale.ROOT);
        String libName;
        String libExt;
        String pdfiumLibName;
        String ortExt;

        if (osName.contains("mac") || osName.contains("darwin")) {
            libName = "libkreuzberg_ffi";
            pdfiumLibName = "libpdfium";
            libExt = ".dylib";
            ortExt = ".dylib";
        } else if (osName.contains("win")) {
            libName = "kreuzberg_ffi";
            pdfiumLibName = "pdfium";
            libExt = ".dll";
            ortExt = ".dll";
        } else {
            libName = "libkreuzberg_ffi";
            pdfiumLibName = "libpdfium";
            libExt = ".so";
            ortExt = ".so";
        }

        String nativesRid = resolveNativesRid(osName, osArch);
        String nativesDir = NATIVES_RESOURCE_ROOT + "/" + nativesRid;

        String ffiDir = System.getenv("KREUZBERG_FFI_DIR");
        if (ffiDir != null && !ffiDir.isEmpty()) {
            Path ffiPath = Path.of(ffiDir);
            Path libPath = ffiPath.resolve(libName + libExt);
            Path pdfiumPath = ffiPath.resolve(pdfiumLibName + libExt);

            if (Files.exists(libPath)) {
                try {
                    if (Files.exists(pdfiumPath)) {
                        System.load(pdfiumPath.toAbsolutePath().toString());
                    }
                    loadOnnxRuntimeRequiredFromDirectory(ffiPath, ortExt);
                    System.load(libPath.toAbsolutePath().toString());
                    return;
                } catch (UnsatisfiedLinkError e) {
                    System.err.println("[KreuzbergFFI] Failed to load native libraries from "
                        + libPath + ": " + e.getMessage());
                }
            }
        }

        Path extracted = tryExtractAndLoadFromResources(nativesDir, libName, pdfiumLibName, libExt);
        if (extracted != null) {
            return;
        }

        String projectRoot = System.getProperty("user.dir");
        Path targetLib = Path.of(
            projectRoot, "target", "classes", libName + libExt);
        Path targetPdfium = Path.of(
            projectRoot, "target", "classes", pdfiumLibName + libExt);

        if (Files.exists(targetLib)) {
            if (Files.exists(targetPdfium)) {
                System.load(targetPdfium.toAbsolutePath().toString());
            }
            System.load(targetLib.toAbsolutePath().toString());
            return;
        }

        try {
            System.loadLibrary("pdfium");
        } catch (UnsatisfiedLinkError e) {
            if (isDebugEnabled()) {
                System.err.println("[KreuzbergFFI] Failed to load optional pdfium library: " + e.getMessage());
            }
        }
        try {
            System.loadLibrary("kreuzberg_ffi");
        } catch (UnsatisfiedLinkError e) {
            String msg = "Failed to load Kreuzberg native library. Expected resource: "
                + nativesDir + "/" + libName + libExt + " (RID: " + nativesRid + "). "
                + "Set KREUZBERG_FFI_DIR to a directory containing " + libName + libExt
                + " and " + pdfiumLibName + libExt + ", or ensure the libraries are on the system library path.";
            UnsatisfiedLinkError out = new UnsatisfiedLinkError(msg + " Original error: " + e.getMessage());
            out.initCause(e);
            throw out;
        }
    }

    private static Path tryExtractAndLoadFromResources(
        String nativesDir,
        String ffiLibraryBaseName,
        String pdfiumLibraryBaseName,
        String libExt
    ) {
        String ffiResourcePath = nativesDir + "/" + ffiLibraryBaseName + libExt;
        URL ffiResource = KreuzbergFFI.class.getResource(ffiResourcePath);
        if (ffiResource == null) {
            return null;
        }

        try {
            Path tempDir = extractOrReuseNativeDirectory(nativesDir);
            List<Path> extractedFiles = listExtractedFiles(tempDir);
            if (extractedFiles.isEmpty()) {
                return null;
            }

            Path pdfiumPath = tempDir.resolve(pdfiumLibraryBaseName + libExt);
            if (Files.exists(pdfiumPath)) {
                System.load(pdfiumPath.toAbsolutePath().toString());
            }

            loadOnnxRuntimeRequired(extractedFiles, libExt);

            Path ffiPath = tempDir.resolve(ffiLibraryBaseName + libExt);
            if (!Files.exists(ffiPath)) {
                throw new UnsatisfiedLinkError("Missing extracted FFI library: " + ffiPath);
            }
            System.load(ffiPath.toAbsolutePath().toString());
            return ffiPath;
        } catch (Exception e) {
            System.err.println("[KreuzbergFFI] Failed to extract and load native libraries from resources: "
                + e.getMessage());
            if (isDebugEnabled()) {
                e.printStackTrace();
            }
            return null;
        }
    }

    private static Path extractOrReuseNativeDirectory(String nativesDir) throws Exception {
        URL location = KreuzbergFFI.class.getProtectionDomain().getCodeSource().getLocation();
        if (location == null) {
            throw new IllegalStateException("Missing code source location for Kreuzberg JAR");
        }

        Path codePath = Path.of(location.toURI());
        String key = codePath.toAbsolutePath() + "::" + nativesDir;

        synchronized (NATIVE_EXTRACT_LOCK) {
            Path existing = cachedExtractDir;
            if (existing != null && key.equals(cachedExtractKey)) {
                return existing;
            }
            Path tempDir = Files.createTempDirectory("kreuzberg_native");
            tempDir.toFile().deleteOnExit();
            List<Path> extracted = extractNativeDirectory(codePath, nativesDir, tempDir);
            if (extracted.isEmpty()) {
                throw new IllegalStateException("No native files extracted from resources dir: " + nativesDir);
            }
            cachedExtractKey = key;
            cachedExtractDir = tempDir;
            return tempDir;
        }
    }

    private static List<Path> listExtractedFiles(Path dir) throws Exception {
        if (!Files.exists(dir) || !Files.isDirectory(dir)) {
            return List.of();
        }
        List<Path> out = new ArrayList<>();
        try (var walk = Files.walk(dir)) {
            for (Path p : (Iterable<Path>) walk::iterator) {
                if (Files.isRegularFile(p)) {
                    out.add(p);
                }
            }
        }
        return out;
    }

    private static List<Path> extractNativeDirectory(Path codePath, String nativesDir, Path destDir) throws Exception {
        if (!Files.exists(destDir) || !Files.isDirectory(destDir)) {
            throw new IllegalArgumentException("Destination directory does not exist: " + destDir);
        }

        String prefix = nativesDir.startsWith("/") ? nativesDir.substring(1) : nativesDir;
        if (!prefix.endsWith("/")) {
            prefix = prefix + "/";
        }

        if (Files.isDirectory(codePath)) {
            Path nativesPath = codePath.resolve(prefix);
            if (!Files.exists(nativesPath) || !Files.isDirectory(nativesPath)) {
                return List.of();
            }
            return copyDirectory(nativesPath, destDir);
        }

        List<Path> extracted = new ArrayList<>();
        try (JarFile jar = new JarFile(codePath.toFile())) {
            Enumeration<JarEntry> entries = jar.entries();
            while (entries.hasMoreElements()) {
                JarEntry entry = entries.nextElement();
                String name = entry.getName();
                if (!name.startsWith(prefix) || entry.isDirectory()) {
                    continue;
                }
                String relative = name.substring(prefix.length());
                Path out = safeResolve(destDir, relative);
                Files.createDirectories(out.getParent());
                try (var in = jar.getInputStream(entry)) {
                    Files.copy(in, out, StandardCopyOption.REPLACE_EXISTING);
                }
                out.toFile().deleteOnExit();
                extracted.add(out);
            }
        }
        return extracted;
    }

    private static List<Path> copyDirectory(Path srcDir, Path destDir) throws Exception {
        List<Path> copied = new ArrayList<>();
        try (var paths = Files.walk(srcDir)) {
            for (Path src : (Iterable<Path>) paths::iterator) {
                if (Files.isDirectory(src)) {
                    continue;
                }
                Path relative = srcDir.relativize(src);
                Path out = safeResolve(destDir, relative.toString());
                Files.createDirectories(out.getParent());
                Files.copy(src, out, StandardCopyOption.REPLACE_EXISTING);
                out.toFile().deleteOnExit();
                copied.add(out);
            }
        }
        return copied;
    }

    private static Path safeResolve(Path destDir, String relative) throws Exception {
        Path normalizedDest = destDir.toAbsolutePath().normalize();
        Path out = normalizedDest.resolve(relative).normalize();
        if (!out.startsWith(normalizedDest)) {
            throw new SecurityException("Blocked extracting native file outside destination directory: " + relative);
        }
        return out;
    }

    private static void loadOnnxRuntimeRequiredFromDirectory(Path dir, String ortExt) {
        if (!Files.exists(dir) || !Files.isDirectory(dir)) {
            throw new UnsatisfiedLinkError("KREUZBERG_FFI_DIR does not exist or is not a directory: " + dir);
        }
        try {
            List<Path> files = listExtractedFiles(dir);
            loadOnnxRuntimeRequired(files, ortExt);
        } catch (UnsatisfiedLinkError e) {
            throw e;
        } catch (Exception e) {
            UnsatisfiedLinkError out = new UnsatisfiedLinkError("Failed to scan ORT libraries in " + dir);
            out.initCause(e);
            throw out;
        }
    }

    private static void loadOnnxRuntimeRequired(List<Path> extractedFiles, String ortExt) {
        Path core = findOnnxRuntimeCoreLibrary(extractedFiles, ortExt);
        if (core == null) {
            String expected = switch (ortExt) {
                case ".dll" -> "onnxruntime.dll (and optionally onnxruntime*.dll)";
                case ".dylib" -> "libonnxruntime*.dylib";
                default -> "libonnxruntime.so*";
            };
            throw new UnsatisfiedLinkError("ONNX Runtime is required (embeddings enabled) but was not found. "
                + "Expected " + expected + " alongside the bundled natives or in KREUZBERG_FFI_DIR.");
        }

        try {
            System.load(core.toAbsolutePath().toString());
        } catch (UnsatisfiedLinkError e) {
            UnsatisfiedLinkError out = new UnsatisfiedLinkError("Failed to load ONNX Runtime core library: " + core);
            out.initCause(e);
            throw out;
        }

        List<Path> candidates = extractedFiles.stream()
            .filter(path -> {
                String name = path.getFileName().toString().toLowerCase(Locale.ROOT);
                return name.contains("onnxruntime") && !path.equals(core);
            })
            .sorted(Comparator.comparing(path -> path.getFileName().toString()))
            .toList();

        for (Path candidate : candidates) {
            try {
                System.load(candidate.toAbsolutePath().toString());
            } catch (UnsatisfiedLinkError e) {
                UnsatisfiedLinkError out = new UnsatisfiedLinkError("Failed to load ONNX Runtime dependency: "
                    + candidate);
                out.initCause(e);
                throw out;
            }
        }
    }

    private static Path findOnnxRuntimeCoreLibrary(List<Path> files, String ortExt) {
        List<Path> candidates = files.stream()
            .filter(Files::isRegularFile)
            .filter(path -> {
                String name = path.getFileName().toString().toLowerCase(Locale.ROOT);
                if (!name.endsWith(ortExt)) {
                    return false;
                }
                if (".dll".equals(ortExt)) {
                    return "onnxruntime.dll".equals(name);
                }
                if (!name.contains("onnxruntime")) {
                    return false;
                }
                if (name.contains("providers") || name.contains("provider")) {
                    return false;
                }
                if (".so".equals(ortExt)) {
                    return name.startsWith("libonnxruntime.so");
                }
                return ".dylib".equals(ortExt) && name.startsWith("libonnxruntime");
            })
            .sorted(Comparator.comparing(path -> path.getFileName().toString().length()))
            .toList();

        if (candidates.isEmpty()) {
            return null;
        }
        return candidates.getFirst();
    }

    private static boolean isDebugEnabled() {
        String env = System.getenv("KREUZBERG_JAVA_DEBUG");
        return env != null && "true".equalsIgnoreCase(env);
    }

    private static String resolveNativesRid(String osName, String osArch) {
        String arch;
        if (osArch.contains("aarch64") || osArch.contains("arm64")) {
            arch = "arm64";
        } else if (osArch.contains("x86_64") || osArch.contains("amd64")) {
            arch = "x86_64";
        } else {
            arch = osArch.replaceAll("[^a-z0-9_]+", "");
        }

        String os;
        if (osName.contains("mac") || osName.contains("darwin")) {
            os = "macos";
        } else if (osName.contains("win")) {
            os = "windows";
        } else {
            os = "linux";
        }

        return os + "-" + arch;
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
        return address.reinterpret(C_STRING_MAX_SIZE).getString(0);
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
