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
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Objects;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;
import java.util.Optional;

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
@SuppressWarnings("PMD.AvoidCatchingThrowable")
public final class Kreuzberg {
    private static final Linker LINKER = Linker.nativeLinker();
    private static final FunctionDescriptor STRING_CALLBACK = FunctionDescriptor.of(
        ValueLayout.ADDRESS,
        ValueLayout.ADDRESS
    );
    private static final FunctionDescriptor OCR_CALLBACK = FunctionDescriptor.of(
        ValueLayout.ADDRESS,
        ValueLayout.ADDRESS,
        ValueLayout.JAVA_LONG,
        ValueLayout.ADDRESS
    );
    private static final int OCR_BACKEND_ARGUMENT_INDEX = 3;
    private static final Map<String, CallbackHandle> POST_PROCESSOR_CALLBACKS = new ConcurrentHashMap<>();
    private static final Map<String, CallbackHandle> VALIDATOR_CALLBACKS = new ConcurrentHashMap<>();
    private static final Map<String, CallbackHandle> OCR_CALLBACKS = new ConcurrentHashMap<>();

    private Kreuzberg() {
        // Private constructor to prevent instantiation
    }

    public static ExtractionResult extractFile(String path) throws IOException, KreuzbergException {
        return extractFile(Path.of(path), null);
    }

    public static ExtractionResult extractFile(Path path) throws IOException, KreuzbergException {
        return extractFile(path, null);
    }

    public static ExtractionResult extractFile(Path path, ExtractionConfig config)
        throws IOException, KreuzbergException {
        validateFile(path);
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment pathSegment = KreuzbergFFI.allocateCString(arena, path.toString());
            MemorySegment configSegment = config == null ? MemorySegment.NULL : encodeConfig(arena, config);
            MemorySegment resultPtr;

            if (configSegment.address() != 0) {
                resultPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_EXTRACT_FILE_SYNC_WITH_CONFIG.invoke(
                    pathSegment, configSegment);
            } else {
                resultPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_EXTRACT_FILE_SYNC.invoke(pathSegment);
            }

            if (resultPtr == null || resultPtr.address() == 0) {
                throw new KreuzbergException("Extraction failed: " + getLastError());
            }

            return parseAndFreeResult(resultPtr);
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error during extraction", e);
        }
    }

    public static ExtractionResult extractBytes(byte[] data, String mimeType, ExtractionConfig config)
        throws KreuzbergException {
        Objects.requireNonNull(data, "data must not be null");
        if (data.length == 0) {
            throw new KreuzbergException("data cannot be empty");
        }
        if (mimeType == null || mimeType.isBlank()) {
            throw new KreuzbergException("mimeType is required");
        }

        try (Arena arena = Arena.ofConfined()) {
            MemorySegment dataSegment = arena.allocateFrom(ValueLayout.JAVA_BYTE, data);
            MemorySegment mimeSegment = KreuzbergFFI.allocateCString(arena, mimeType);
            MemorySegment configSegment = config == null ? MemorySegment.NULL : encodeConfig(arena, config);

            MemorySegment resultPtr = (MemorySegment) (
                configSegment.address() != 0
                    ? KreuzbergFFI.KREUZBERG_EXTRACT_BYTES_SYNC_WITH_CONFIG.invoke(
                        dataSegment, (long) data.length, mimeSegment, configSegment)
                    : KreuzbergFFI.KREUZBERG_EXTRACT_BYTES_SYNC.invoke(
                        dataSegment, (long) data.length, mimeSegment)
            );

            if (resultPtr == null || resultPtr.address() == 0) {
                throw new KreuzbergException("Extraction failed: " + getLastError());
            }

            return parseAndFreeResult(resultPtr);
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error during extraction", e);
        }
    }

    public static List<ExtractionResult> batchExtractFiles(List<String> paths, ExtractionConfig config)
        throws KreuzbergException {
        Objects.requireNonNull(paths, "paths must not be null");
        if (paths.isEmpty()) {
            return Collections.emptyList();
        }

        try (Arena arena = Arena.ofConfined()) {
            MemorySegment[] cStrings = new MemorySegment[paths.size()];
            for (int i = 0; i < paths.size(); i++) {
                cStrings[i] = KreuzbergFFI.allocateCString(arena, paths.get(i));
            }
            MemorySegment arraySegment = arena.allocate(ValueLayout.ADDRESS, cStrings.length);
            for (int i = 0; i < cStrings.length; i++) {
                arraySegment.setAtIndex(ValueLayout.ADDRESS, i, cStrings[i]);
            }
            MemorySegment configSegment = config == null ? MemorySegment.NULL : encodeConfig(arena, config);

            MemorySegment batchPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_BATCH_EXTRACT_FILES_SYNC.invoke(
                arraySegment, (long) paths.size(), configSegment);

            if (batchPtr == null || batchPtr.address() == 0) {
                throw new KreuzbergException("Batch extraction failed: " + getLastError());
            }

            return parseAndFreeBatch(batchPtr);
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error during batch extraction", e);
        }
    }

    public static List<ExtractionResult> batchExtractBytes(List<BytesWithMime> items, ExtractionConfig config)
        throws KreuzbergException {
        Objects.requireNonNull(items, "items must not be null");
        if (items.isEmpty()) {
            return Collections.emptyList();
        }

        try (Arena arena = Arena.ofConfined()) {
            long structSize = KreuzbergFFI.C_BYTES_WITH_MIME_LAYOUT.byteSize();
            MemorySegment bytesWithMimeArray = arena.allocate(structSize * items.size(), structSize);
            for (int i = 0; i < items.size(); i++) {
                BytesWithMime item = items.get(i);
                MemorySegment element = bytesWithMimeArray.asSlice((long) i * structSize, structSize);
                MemorySegment dataSeg = arena.allocateFrom(ValueLayout.JAVA_BYTE, item.data());
                MemorySegment mimeSeg = KreuzbergFFI.allocateCString(arena, item.mimeType());
                element.set(ValueLayout.ADDRESS, KreuzbergFFI.BYTES_DATA_OFFSET, dataSeg);
                element.set(ValueLayout.JAVA_LONG, KreuzbergFFI.BYTES_LEN_OFFSET, item.data().length);
                element.set(ValueLayout.ADDRESS, KreuzbergFFI.BYTES_MIME_OFFSET, mimeSeg);
            }

            MemorySegment configSegment = config == null ? MemorySegment.NULL : encodeConfig(arena, config);

            MemorySegment batchPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_BATCH_EXTRACT_BYTES_SYNC.invoke(
                bytesWithMimeArray,
                (long) items.size(),
                configSegment
            );

            if (batchPtr == null || batchPtr.address() == 0) {
                throw new KreuzbergException("Batch extraction failed: " + getLastError());
            }

            return parseAndFreeBatch(batchPtr);
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error during batch extraction", e);
        }
    }

    public static ExtractionConfig loadExtractionConfigFromFile(Path path) throws KreuzbergException {
        try {
            validateFile(path);
        } catch (IOException e) {
            throw new KreuzbergException("Invalid configuration file path", e);
        }
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment pathSeg = KreuzbergFFI.allocateCString(arena, path.toString());
            MemorySegment jsonPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_LOAD_EXTRACTION_CONFIG_FROM_FILE.invoke(
                pathSeg);
            if (jsonPtr == null || jsonPtr.address() == 0) {
                throw new KreuzbergException("Failed to load extraction config: " + getLastError());
            }
            try {
                String json = KreuzbergFFI.readCString(jsonPtr);
                return ExtractionConfig.fromJson(json);
            } finally {
                KreuzbergFFI.KREUZBERG_FREE_STRING.invoke(jsonPtr);
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error loading config", e);
        }
    }

    public static ExtractionConfig discoverExtractionConfig() throws KreuzbergException {
        try {
            MemorySegment jsonPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_CONFIG_DISCOVER.invoke();
            if (jsonPtr == null || jsonPtr.address() == 0) {
                String error = getLastError();
                if (error != null && !error.isBlank()) {
                    throw new KreuzbergException("Failed to discover config: " + error);
                }
                // No config found, return null
                return null;
            }
            try {
                String json = KreuzbergFFI.readCString(jsonPtr);
                return ExtractionConfig.fromJson(json);
            } finally {
                KreuzbergFFI.KREUZBERG_FREE_STRING.invoke(jsonPtr);
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error discovering config", e);
        }
    }

    public static CompletableFuture<ExtractionResult> extractFileAsync(Path path, ExtractionConfig config) {
        return CompletableFuture.supplyAsync(() -> {
            try {
                return extractFile(path, config);
            } catch (IOException | KreuzbergException e) {
                throw new RuntimeException(e);
            }
        });
    }

    public static CompletableFuture<ExtractionResult> extractBytesAsync(
        byte[] data,
        String mimeType,
        ExtractionConfig config
    ) {
        return CompletableFuture.supplyAsync(() -> {
            try {
                return extractBytes(data, mimeType, config);
            } catch (KreuzbergException e) {
                throw new RuntimeException(e);
            }
        });
    }

    public static CompletableFuture<List<ExtractionResult>> batchExtractFilesAsync(
        List<String> paths,
        ExtractionConfig config
    ) {
        return CompletableFuture.supplyAsync(() -> {
            try {
                return batchExtractFiles(paths, config);
            } catch (KreuzbergException e) {
                throw new RuntimeException(e);
            }
        });
    }

    public static CompletableFuture<List<ExtractionResult>> batchExtractBytesAsync(
        List<BytesWithMime> items,
        ExtractionConfig config
    ) {
        return CompletableFuture.supplyAsync(() -> {
            try {
                return batchExtractBytes(items, config);
            } catch (KreuzbergException e) {
                throw new RuntimeException(e);
            }
        });
    }

    public static String detectMimeType(String path) throws KreuzbergException {
        return detectMimeType(path, true);
    }

    public static String detectMimeType(String path, boolean checkExists) throws KreuzbergException {
        Objects.requireNonNull(path, "path must not be null");
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment pathSeg = KreuzbergFFI.allocateCString(arena, path);
            MemorySegment mimePtr = (MemorySegment) KreuzbergFFI.KREUZBERG_DETECT_MIME_TYPE.invoke(
                pathSeg,
                checkExists
            );
            if (mimePtr == null || mimePtr.address() == 0) {
                throw new KreuzbergException("Failed to detect MIME type: " + getLastError());
            }
            return KreuzbergFFI.readCString(mimePtr);
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error detecting MIME type", e);
        }
    }

    public static String validateMimeType(String mimeType) throws KreuzbergException {
        Objects.requireNonNull(mimeType, "mimeType must not be null");
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment mimeSeg = KreuzbergFFI.allocateCString(arena, mimeType);
            MemorySegment validatedPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_VALIDATE_MIME_TYPE.invoke(mimeSeg);
            if (validatedPtr == null || validatedPtr.address() == 0) {
                throw new KreuzbergException("Failed to validate MIME type: " + getLastError());
            }
            return KreuzbergFFI.readCString(validatedPtr);
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error validating MIME type", e);
        }
    }

    public static List<String> listEmbeddingPresets() throws KreuzbergException {
        try {
            MemorySegment presetsPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_LIST_EMBEDDING_PRESETS.invoke();
            if (presetsPtr == null || presetsPtr.address() == 0) {
                throw new KreuzbergException("Failed to list embedding presets: " + getLastError());
            }
            try {
                String json = KreuzbergFFI.readCString(presetsPtr);
                return ResultParser.parseStringList(json);
            } finally {
                KreuzbergFFI.KREUZBERG_FREE_STRING.invoke(presetsPtr);
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error listing embedding presets", e);
        }
    }

    public static Optional<EmbeddingPreset> getEmbeddingPreset(String name) throws KreuzbergException {
        Objects.requireNonNull(name, "name must not be null");
        if (name.isBlank()) {
            throw new KreuzbergException("Preset name must not be blank");
        }

        try (Arena arena = Arena.ofConfined()) {
            MemorySegment nameSeg = KreuzbergFFI.allocateCString(arena, name);
            MemorySegment presetPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_GET_EMBEDDING_PRESET.invoke(nameSeg);
            if (presetPtr == null || presetPtr.address() == 0) {
                String error = getLastError();
                if (error.toLowerCase(Locale.ROOT).contains("unknown embedding preset")) {
                    return Optional.empty();
                }
                throw new KreuzbergException("Failed to fetch embedding preset: " + error);
            }
            try {
                String json = KreuzbergFFI.readCString(presetPtr);
                return Optional.ofNullable(ResultParser.parseEmbeddingPreset(json));
            } finally {
                KreuzbergFFI.KREUZBERG_FREE_STRING.invoke(presetPtr);
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error fetching embedding preset", e);
        }
    }

    /**
     * Register a post-processor using its default priority and stage.
     *
     * @param name unique processor name
     * @param processor processor implementation
     * @throws KreuzbergException if registration fails
     */
    public static void registerPostProcessor(String name, PostProcessor processor) throws KreuzbergException {
        registerPostProcessor(name, processor, processor.priority(), processor.processingStage());
    }

    /**
     * Register a post-processor with explicit priority and stage.
     *
     * @param name unique processor name
     * @param processor processor implementation
     * @param priority order within the stage (higher runs first)
     * @param stage processing stage
     * @throws KreuzbergException if registration fails
     */
    @SuppressWarnings("PMD.CloseResource")
    public static void registerPostProcessor(
        String name,
        PostProcessor processor,
        int priority,
        ProcessingStage stage
    ) throws KreuzbergException {
        Objects.requireNonNull(processor, "processor must not be null");
        ProcessingStage effectiveStage = stage == null ? ProcessingStage.MIDDLE : stage;
        String normalizedName = validatePluginName(name, "PostProcessor");

        Arena arena = Arena.ofShared();
        boolean registered = false;
        try {
            MemorySegment callback = createPostProcessorCallback(processor, arena);
            MemorySegment nameSegment = KreuzbergFFI.allocateCString(arena, normalizedName);
            MemorySegment stageSegment = KreuzbergFFI.allocateCString(arena, effectiveStage.wireName());

            boolean success = (boolean) KreuzbergFFI.KREUZBERG_REGISTER_POST_PROCESSOR_WITH_STAGE.invoke(
                nameSegment,
                callback,
                priority,
                stageSegment
            );

            if (!success) {
                throw new KreuzbergException("Failed to register post-processor: " + getLastError());
            }

            POST_PROCESSOR_CALLBACKS.put(normalizedName, new CallbackHandle(arena, callback, processor));
            registered = true;
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error registering post-processor", e);
        } finally {
            if (!registered) {
                closeCallback(new CallbackHandle(arena, MemorySegment.NULL, processor));
            }
        }
    }

    /**
     * Unregister a post-processor by name.
     *
     * @param name processor name
     * @throws KreuzbergException if unregistering fails
     */
    public static void unregisterPostProcessor(String name) throws KreuzbergException {
        String normalizedName = validatePluginName(name, "PostProcessor");
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment nameSeg = KreuzbergFFI.allocateCString(arena, normalizedName);
            boolean success = (boolean) KreuzbergFFI.KREUZBERG_UNREGISTER_POST_PROCESSOR.invoke(nameSeg);
            if (!success) {
                throw new KreuzbergException("Failed to unregister post-processor: " + getLastError());
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error unregistering post-processor", e);
        } finally {
            CallbackHandle handle = POST_PROCESSOR_CALLBACKS.remove(normalizedName);
            closeCallback(handle);
        }
    }

    /**
     * Remove all registered post-processors.
     *
     * @throws KreuzbergException if clearing fails
     */
    public static void clearPostProcessors() throws KreuzbergException {
        try {
            boolean success = (boolean) KreuzbergFFI.KREUZBERG_CLEAR_POST_PROCESSORS.invoke();
            if (!success) {
                throw new KreuzbergException("Failed to clear post-processors: " + getLastError());
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error clearing post-processors", e);
        } finally {
            POST_PROCESSOR_CALLBACKS.values().forEach(Kreuzberg::closeCallback);
            POST_PROCESSOR_CALLBACKS.clear();
        }
    }

    /**
     * List registered post-processor names.
     *
     * @return processor names
     * @throws KreuzbergException if listing fails
     */
    public static List<String> listPostProcessors() throws KreuzbergException {
        try {
            MemorySegment namesPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_LIST_POST_PROCESSORS.invoke();
            if (namesPtr == null || namesPtr.address() == 0) {
                return List.of();
            }
            try {
                String json = KreuzbergFFI.readCString(namesPtr);
                return ResultParser.parseStringList(json);
            } finally {
                KreuzbergFFI.KREUZBERG_FREE_STRING.invoke(namesPtr);
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Failed to list post-processors", e);
        }
    }

    /**
     * Register a validator using its default priority.
     *
     * @param name validator name
     * @param validator validator implementation
     * @throws KreuzbergException if registration fails
     */
    public static void registerValidator(String name, Validator validator) throws KreuzbergException {
        registerValidator(name, validator, validator.priority());
    }

    /**
     * Register a validator with explicit priority.
     *
     * @param name validator name
     * @param validator validator implementation
     * @param priority order (higher runs earlier)
     * @throws KreuzbergException if registration fails
     */
    @SuppressWarnings("PMD.CloseResource")
    public static void registerValidator(String name, Validator validator, int priority) throws KreuzbergException {
        Objects.requireNonNull(validator, "validator must not be null");
        String normalizedName = validatePluginName(name, "Validator");

        Arena arena = Arena.ofShared();
        boolean registered = false;
        try {
            MemorySegment callback = createValidatorCallback(validator, arena);
            MemorySegment nameSegment = KreuzbergFFI.allocateCString(arena, normalizedName);

            boolean success = (boolean) KreuzbergFFI.KREUZBERG_REGISTER_VALIDATOR.invoke(
                nameSegment,
                callback,
                priority
            );

            if (!success) {
                throw new KreuzbergException("Failed to register validator: " + getLastError());
            }

            VALIDATOR_CALLBACKS.put(normalizedName, new CallbackHandle(arena, callback, validator));
            registered = true;
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error registering validator", e);
        } finally {
            if (!registered) {
                closeCallback(new CallbackHandle(arena, MemorySegment.NULL, validator));
            }
        }
    }

    /**
     * Unregister a validator by name.
     *
     * @param name validator name
     * @throws KreuzbergException if unregistering fails
     */
    public static void unregisterValidator(String name) throws KreuzbergException {
        String normalizedName = validatePluginName(name, "Validator");
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment nameSeg = KreuzbergFFI.allocateCString(arena, normalizedName);
            boolean success = (boolean) KreuzbergFFI.KREUZBERG_UNREGISTER_VALIDATOR.invoke(nameSeg);
            if (!success) {
                throw new KreuzbergException("Failed to unregister validator: " + getLastError());
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error unregistering validator", e);
        } finally {
            CallbackHandle handle = VALIDATOR_CALLBACKS.remove(normalizedName);
            closeCallback(handle);
        }
    }

    /**
     * Remove all registered validators.
     *
     * @throws KreuzbergException if clearing fails
     */
    public static void clearValidators() throws KreuzbergException {
        try {
            boolean success = (boolean) KreuzbergFFI.KREUZBERG_CLEAR_VALIDATORS.invoke();
            if (!success) {
                throw new KreuzbergException("Failed to clear validators: " + getLastError());
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error clearing validators", e);
        } finally {
            VALIDATOR_CALLBACKS.values().forEach(Kreuzberg::closeCallback);
            VALIDATOR_CALLBACKS.clear();
        }
    }

    /**
     * List registered validator names.
     *
     * @return validator names
     * @throws KreuzbergException if listing fails
     */
    public static List<String> listValidators() throws KreuzbergException {
        try {
            MemorySegment namesPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_LIST_VALIDATORS.invoke();
            if (namesPtr == null || namesPtr.address() == 0) {
                return List.of();
            }
            try {
                String json = KreuzbergFFI.readCString(namesPtr);
                return ResultParser.parseStringList(json);
            } finally {
                KreuzbergFFI.KREUZBERG_FREE_STRING.invoke(namesPtr);
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Failed to list validators", e);
        }
    }

    /**
     * Register an OCR backend using its supported language set.
     *
     * @param name backend name
     * @param backend backend implementation
     * @throws KreuzbergException if registration fails
     */
    public static void registerOcrBackend(String name, OcrBackend backend) throws KreuzbergException {
        registerOcrBackend(name, backend, backend.supportedLanguages());
    }

    /**
     * Register an OCR backend with optional language filtering.
     *
     * @param name backend name
     * @param backend backend implementation
     * @param supportedLanguages languages supported by the backend (empty for all)
     * @throws KreuzbergException if registration fails
     */
    @SuppressWarnings("PMD.CloseResource")
    public static void registerOcrBackend(
        String name,
        OcrBackend backend,
        List<String> supportedLanguages
    ) throws KreuzbergException {
        Objects.requireNonNull(backend, "backend must not be null");
        String normalizedName = validatePluginName(name, "OCR backend");
        List<String> languages = supportedLanguages == null ? List.of() : supportedLanguages;

        Arena arena = Arena.ofShared();
        boolean registered = false;
        try {
            MemorySegment callback = createOcrCallback(backend, arena);
            MemorySegment nameSeg = KreuzbergFFI.allocateCString(arena, normalizedName);
            MemorySegment languagesSeg = MemorySegment.NULL;
            if (!languages.isEmpty()) {
                String json = ResultParser.toJsonValue(languages);
                languagesSeg = KreuzbergFFI.allocateCString(arena, json);
            }

            boolean success;
            if (languagesSeg.address() == 0) {
                success = (boolean) KreuzbergFFI.KREUZBERG_REGISTER_OCR_BACKEND.invoke(nameSeg, callback);
            } else {
                success = (boolean) KreuzbergFFI.KREUZBERG_REGISTER_OCR_BACKEND_WITH_LANGUAGES.invoke(
                    nameSeg,
                    callback,
                    languagesSeg
                );
            }
            if (!success) {
                throw new KreuzbergException("Failed to register OCR backend: " + getLastError());
            }

            OCR_CALLBACKS.put(normalizedName, new CallbackHandle(arena, callback, backend));
            registered = true;
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error registering OCR backend", e);
        } finally {
            if (!registered) {
                closeCallback(new CallbackHandle(arena, MemorySegment.NULL, backend));
            }
        }
    }

    /**
     * Unregister an OCR backend by name.
     *
     * @param name backend name
     * @throws KreuzbergException if unregistering fails
     */
    public static void unregisterOCRBackend(String name) throws KreuzbergException {
        String normalizedName = validatePluginName(name, "OCR backend");
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment nameSeg = KreuzbergFFI.allocateCString(arena, normalizedName);
            boolean success = (boolean) KreuzbergFFI.KREUZBERG_UNREGISTER_OCR_BACKEND.invoke(nameSeg);
            if (!success) {
                throw new KreuzbergException("Failed to unregister OCR backend: " + getLastError());
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error unregistering OCR backend", e);
        } finally {
            CallbackHandle handle = OCR_CALLBACKS.remove(normalizedName);
            closeCallback(handle);
        }
    }

    /**
     * List registered OCR backend names.
     *
     * @return OCR backend names
     * @throws KreuzbergException if listing fails
     */
    public static List<String> listOCRBackends() throws KreuzbergException {
        try {
            MemorySegment namesPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_LIST_OCR_BACKENDS.invoke();
            if (namesPtr == null || namesPtr.address() == 0) {
                return List.of();
            }
            try {
                String json = KreuzbergFFI.readCString(namesPtr);
                return ResultParser.parseStringList(json);
            } finally {
                KreuzbergFFI.KREUZBERG_FREE_STRING.invoke(namesPtr);
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Failed to list OCR backends", e);
        }
    }

    /**
     * Remove all registered OCR backends.
     *
     * @throws KreuzbergException if clearing fails
     */
    public static void clearOCRBackends() throws KreuzbergException {
        try {
            boolean success = (boolean) KreuzbergFFI.KREUZBERG_CLEAR_OCR_BACKENDS.invoke();
            if (!success) {
                throw new KreuzbergException("Failed to clear OCR backends: " + getLastError());
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error clearing OCR backends", e);
        } finally {
            OCR_CALLBACKS.values().forEach(Kreuzberg::closeCallback);
            OCR_CALLBACKS.clear();
        }
    }

    /**
     * List registered document extractor names.
     *
     * @return extractor names
     * @throws KreuzbergException if listing fails
     */
    public static List<String> listDocumentExtractors() throws KreuzbergException {
        try {
            MemorySegment namesPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_LIST_DOCUMENT_EXTRACTORS.invoke();
            if (namesPtr == null || namesPtr.address() == 0) {
                return List.of();
            }
            try {
                String json = KreuzbergFFI.readCString(namesPtr);
                return ResultParser.parseStringList(json);
            } finally {
                KreuzbergFFI.KREUZBERG_FREE_STRING.invoke(namesPtr);
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Failed to list document extractors", e);
        }
    }

    /**
     * Unregister a document extractor by name.
     *
     * @param name extractor name
     * @throws KreuzbergException if unregistering fails
     */
    public static void unregisterDocumentExtractor(String name) throws KreuzbergException {
        String normalizedName = validatePluginName(name, "Document extractor");
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment nameSeg = KreuzbergFFI.allocateCString(arena, normalizedName);
            boolean success = (boolean) KreuzbergFFI.KREUZBERG_UNREGISTER_DOCUMENT_EXTRACTOR.invoke(nameSeg);
            if (!success) {
                throw new KreuzbergException("Failed to unregister document extractor: " + getLastError());
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error unregistering document extractor", e);
        }
    }

    /**
     * Remove all registered document extractors.
     *
     * @throws KreuzbergException if clearing fails
     */
    public static void clearDocumentExtractors() throws KreuzbergException {
        try {
            boolean success = (boolean) KreuzbergFFI.KREUZBERG_CLEAR_DOCUMENT_EXTRACTORS.invoke();
            if (!success) {
                throw new KreuzbergException("Failed to clear document extractors: " + getLastError());
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error clearing document extractors", e);
        }
    }

    /**
     * Detect MIME type from raw bytes.
     *
     * @param data byte array to analyze
     * @return detected MIME type
     * @throws KreuzbergException if detection fails
     */
    public static String detectMimeType(byte[] data) throws KreuzbergException {
        Objects.requireNonNull(data, "data must not be null");
        if (data.length == 0) {
            throw new KreuzbergException("data cannot be empty");
        }

        try (Arena arena = Arena.ofConfined()) {
            MemorySegment dataSegment = arena.allocateFrom(ValueLayout.JAVA_BYTE, data);
            MemorySegment mimePtr = (MemorySegment) KreuzbergFFI.KREUZBERG_DETECT_MIME_TYPE_FROM_BYTES.invoke(
                dataSegment,
                (long) data.length
            );
            if (mimePtr == null || mimePtr.address() == 0) {
                throw new KreuzbergException("Failed to detect MIME type from bytes: " + getLastError());
            }
            return KreuzbergFFI.readCString(mimePtr);
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error detecting MIME type from bytes", e);
        }
    }

    /**
     * Detect MIME type from a file path.
     *
     * @param path file path
     * @return detected MIME type
     * @throws KreuzbergException if detection fails
     */
    public static String detectMimeTypeFromPath(String path) throws KreuzbergException {
        return detectMimeType(path, true);
    }

    /**
     * Get file extensions for a MIME type.
     *
     * @param mimeType MIME type string
     * @return list of file extensions (without leading dot)
     * @throws KreuzbergException if lookup fails
     */
    public static List<String> getExtensionsForMime(String mimeType) throws KreuzbergException {
        Objects.requireNonNull(mimeType, "mimeType must not be null");
        if (mimeType.isBlank()) {
            throw new KreuzbergException("mimeType must not be blank");
        }

        try (Arena arena = Arena.ofConfined()) {
            MemorySegment mimeSeg = KreuzbergFFI.allocateCString(arena, mimeType);
            MemorySegment extensionsPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_GET_EXTENSIONS_FOR_MIME.invoke(mimeSeg);
            if (extensionsPtr == null || extensionsPtr.address() == 0) {
                throw new KreuzbergException("Failed to get extensions for MIME type: " + getLastError());
            }
            try {
                String json = KreuzbergFFI.readCString(extensionsPtr);
                return ResultParser.parseStringList(json);
            } finally {
                KreuzbergFFI.KREUZBERG_FREE_STRING.invoke(extensionsPtr);
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new KreuzbergException("Unexpected error getting extensions for MIME type", e);
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

    private static String validatePluginName(String name, String kind) throws KreuzbergException {
        if (name == null || name.isBlank()) {
            throw new KreuzbergException(kind + " name must not be blank");
        }
        return name.trim();
    }

    private static MemorySegment createPostProcessorCallback(PostProcessor processor, Arena arena)
        throws KreuzbergException {
        try {
            MethodHandle handle = MethodHandles.lookup().findStatic(
                Kreuzberg.class,
                "postProcessorCallback",
                MethodType.methodType(MemorySegment.class, MemorySegment.class, PostProcessor.class)
            );
            MethodHandle bound = MethodHandles.insertArguments(handle, 1, processor);
            return LINKER.upcallStub(bound, STRING_CALLBACK, arena);
        } catch (NoSuchMethodException | IllegalAccessException e) {
            throw new KreuzbergException("Failed to create post-processor callback", e);
        }
    }

    private static MemorySegment createValidatorCallback(Validator validator, Arena arena) throws KreuzbergException {
        try {
            MethodHandle handle = MethodHandles.lookup().findStatic(
                Kreuzberg.class,
                "validatorCallback",
                MethodType.methodType(MemorySegment.class, MemorySegment.class, Validator.class)
            );
            MethodHandle bound = MethodHandles.insertArguments(handle, 1, validator);
            return LINKER.upcallStub(bound, STRING_CALLBACK, arena);
        } catch (NoSuchMethodException | IllegalAccessException e) {
            throw new KreuzbergException("Failed to create validator callback", e);
        }
    }

    private static MemorySegment createOcrCallback(OcrBackend backend, Arena arena) throws KreuzbergException {
        try {
            MethodHandle handle = MethodHandles.lookup().findStatic(
                Kreuzberg.class,
                "ocrCallback",
                MethodType.methodType(
                    MemorySegment.class,
                    MemorySegment.class,
                    long.class,
                    MemorySegment.class,
                    OcrBackend.class
                )
            );
            MethodHandle bound = MethodHandles.insertArguments(handle, OCR_BACKEND_ARGUMENT_INDEX, backend);
            return LINKER.upcallStub(bound, OCR_CALLBACK, arena);
        } catch (NoSuchMethodException | IllegalAccessException e) {
            throw new KreuzbergException("Failed to create OCR callback", e);
        }
    }

    private static MemorySegment cloneStringWithRust(String value, Arena scratch) throws Exception {
        if (value == null) {
            return MemorySegment.NULL;
        }
        MemorySegment local = KreuzbergFFI.allocateCString(scratch, value);
        try {
            return (MemorySegment) KreuzbergFFI.KREUZBERG_CLONE_STRING.invoke(local);
        } catch (Throwable e) {
            throw new Exception("Failed to clone string in native memory", e);
        }
    }

    @SuppressWarnings("PMD.UnusedPrivateMethod")
    private static MemorySegment postProcessorCallback(MemorySegment jsonPtr, PostProcessor processor) {
        try (Arena scratch = Arena.ofConfined()) {
            String json = KreuzbergFFI.readCString(jsonPtr);
            ExtractionResult result = ResultParser.fromJson(json);
            ExtractionResult updated = processor.process(result);
            if (updated == null) {
                return MemorySegment.NULL;
            }
            String output = ResultParser.toJson(updated);
            return cloneStringWithRust(output, scratch);
        } catch (Throwable e) {
            return MemorySegment.NULL;
        }
    }

    @SuppressWarnings("PMD.UnusedPrivateMethod")
    private static MemorySegment validatorCallback(MemorySegment jsonPtr, Validator validator) {
        try {
            String json = KreuzbergFFI.readCString(jsonPtr);
            ExtractionResult result = ResultParser.fromJson(json);
            validator.validate(result);
            return MemorySegment.NULL;
        } catch (ValidationException e) {
            try (Arena scratch = Arena.ofConfined()) {
                String message = e.getMessage() != null ? e.getMessage() : "Validation failed";
                return cloneStringWithRust(message, scratch);
            } catch (Throwable ex) {
                return MemorySegment.NULL;
            }
        } catch (Throwable e) {
            try (Arena scratch = Arena.ofConfined()) {
                String message = e.getMessage() != null ? e.getMessage() : "Validator failed";
                return cloneStringWithRust(message, scratch);
            } catch (Throwable ex) {
                return MemorySegment.NULL;
            }
        }
    }

    @SuppressWarnings("PMD.UnusedPrivateMethod")
    private static MemorySegment ocrCallback(
        MemorySegment bytesPtr,
        long length,
        MemorySegment configPtr,
        OcrBackend backend
    ) {
        try (Arena scratch = Arena.ofConfined()) {
            MemorySegment slice = bytesPtr.asSlice(0, length);
            byte[] data = slice.toArray(ValueLayout.JAVA_BYTE);
            String configJson = KreuzbergFFI.readCString(configPtr);
            String text = backend.processImage(data, configJson);
            if (text == null) {
                return MemorySegment.NULL;
            }
            return cloneStringWithRust(text, scratch);
        } catch (Throwable e) {
            return MemorySegment.NULL;
        }
    }

    private static void closeCallback(CallbackHandle handle) {
        if (handle != null) {
            try {
                handle.arena().close();
            } catch (Exception ignored) {
                // Ignore errors while closing arenas on cleanup
            }
        }
    }

    private record CallbackHandle(Arena arena, MemorySegment functionPointer, Object target) {
    }

    private static ExtractionResult parseAndFreeResult(MemorySegment resultPtr) throws Throwable {
        try {
            MemorySegment result = resultPtr.reinterpret(KreuzbergFFI.C_EXTRACTION_RESULT_LAYOUT.byteSize());

            String content = KreuzbergFFI.readCString(result.get(ValueLayout.ADDRESS, KreuzbergFFI.CONTENT_OFFSET));
            String mimeType = KreuzbergFFI.readCString(result.get(ValueLayout.ADDRESS, KreuzbergFFI.MIME_TYPE_OFFSET));
            String tablesJson = KreuzbergFFI.readCString(
                result.get(ValueLayout.ADDRESS, KreuzbergFFI.TABLES_OFFSET)
            );
            String detectedLanguagesJson = KreuzbergFFI.readCString(
                result.get(ValueLayout.ADDRESS, KreuzbergFFI.DETECTED_LANGUAGES_OFFSET)
            );
            String metadataJson = KreuzbergFFI.readCString(
                result.get(ValueLayout.ADDRESS, KreuzbergFFI.METADATA_OFFSET)
            );
            String chunksJson = KreuzbergFFI.readCString(
                result.get(ValueLayout.ADDRESS, KreuzbergFFI.CHUNKS_OFFSET)
            );
            String imagesJson = KreuzbergFFI.readCString(
                result.get(ValueLayout.ADDRESS, KreuzbergFFI.IMAGES_OFFSET)
            );
            boolean success = result.get(ValueLayout.JAVA_BOOLEAN, KreuzbergFFI.SUCCESS_OFFSET);

            return ResultParser.parse(
                content,
                mimeType,
                tablesJson,
                detectedLanguagesJson,
                metadataJson,
                chunksJson,
                imagesJson,
                success
            );
        } finally {
            KreuzbergFFI.KREUZBERG_FREE_RESULT.invoke(resultPtr);
        }
    }

    private static List<ExtractionResult> parseAndFreeBatch(MemorySegment batchPtr) throws Throwable {
        try {
            MemorySegment batch = batchPtr.reinterpret(KreuzbergFFI.C_BATCH_RESULT_LAYOUT.byteSize());
            long count = batch.get(ValueLayout.JAVA_LONG, KreuzbergFFI.BATCH_COUNT_OFFSET);
            MemorySegment resultsPtr = batch.get(ValueLayout.ADDRESS, KreuzbergFFI.BATCH_RESULTS_PTR_OFFSET);

            if (resultsPtr == null || resultsPtr.address() == 0 || count <= 0) {
                return Collections.emptyList();
            }

            MemorySegment array = resultsPtr.reinterpret(count * ValueLayout.ADDRESS.byteSize());
            List<ExtractionResult> results = new ArrayList<>((int) count);
            for (long i = 0; i < count; i++) {
                MemorySegment ptr = array.getAtIndex(ValueLayout.ADDRESS, i);
                if (ptr == null || ptr.address() == 0) {
                    results.add(
                        new ExtractionResult(
                            "",
                            "",
                            Collections.emptyMap(),
                            List.of(),
                            List.of(),
                            List.of(),
                            List.of(),
                            false
                        )
                    );
                } else {
                    results.add(parseAndFreeResult(ptr));
                }
            }
            return results;
        } finally {
            KreuzbergFFI.KREUZBERG_FREE_BATCH_RESULT.invoke(batchPtr);
        }
    }

    private static MemorySegment encodeConfig(Arena arena, ExtractionConfig config) throws KreuzbergException {
        try {
            String json = ResultParser.toJson(config.toMap());
            return KreuzbergFFI.allocateCString(arena, json);
        } catch (Throwable e) {
            throw new KreuzbergException("Failed to serialize extraction config", e);
        }
    }

    private static void validateFile(Path path) throws IOException {
        if (!Files.exists(path)) {
            throw new IOException("File not found: " + path);
        }
        if (!Files.isRegularFile(path)) {
            throw new IOException("Not a regular file: " + path);
        }
        if (!Files.isReadable(path)) {
            throw new IOException("File not readable: " + path);
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
