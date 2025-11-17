package dev.kreuzberg;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.core.type.TypeReference;
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
import java.util.Map;
import java.util.Objects;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/**
 * High-level Java API for Kreuzberg document intelligence library.
 *
 * <p>
 * Kreuzberg is a powerful document extraction library that supports various file formats including
 * PDFs, Office documents, images, and more.
 * </p>
 *
 * <h2>Basic Usage</h2>
 *
 * <pre>{@code
 * ExtractionResult result = Kreuzberg.extractFileSync("document.pdf");
 * System.out.println(result.content());
 * System.out.println("Type: " + result.mimeType());
 * }</pre>
 *
 * <h2>With Configuration</h2>
 *
 * <pre>{@code
 * ExtractionConfig config = new ExtractionConfig();
 * // Configure as needed
 * ExtractionResult result = Kreuzberg.extractFileSync("document.pdf", null, config);
 * }</pre>
 *
 * <h2>Error Handling</h2>
 *
 * <pre>{@code
 * try {
 *   ExtractionResult result = Kreuzberg.extractFileSync("document.pdf");
 *   // Process result
 * } catch (KreuzbergException e) {
 *   System.err.println("Extraction failed: " + e.getMessage());
 * } catch (IOException e) {
 *   System.err.println("File not found: " + e.getMessage());
 * }
 * }</pre>
 */
public final class Kreuzberg {
  private static final Linker LINKER = Linker.nativeLinker();
  private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper()
      .registerModule(new com.fasterxml.jackson.module.paramnames.ParameterNamesModule()).configure(
          com.fasterxml.jackson.databind.DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);
  private static final ExecutorService ASYNC_EXECUTOR = Executors.newVirtualThreadPerTaskExecutor();
  private static final TypeReference<List<Map<String, Object>>> TABLE_LIST_TYPE =
      new TypeReference<List<Map<String, Object>>>() {
        // Type token for table collections
      };
  private static final TypeReference<List<String>> STRING_LIST_TYPE =
      new TypeReference<List<String>>() {
        // Type token for detected language collections
      };
  private static final TypeReference<Map<String, Object>> STRING_OBJECT_MAP_TYPE =
      new TypeReference<Map<String, Object>>() {
        // Type token for metadata maps
      };

  // Storage for registered processors/validators to prevent GC
  private static final Map<String, PostProcessor> POST_PROCESSORS =
      new java.util.concurrent.ConcurrentHashMap<>();
  private static final Map<String, Validator> VALIDATORS =
      new java.util.concurrent.ConcurrentHashMap<>();
  private static final Map<String, MemorySegment> POST_PROCESSOR_STUBS =
      new java.util.concurrent.ConcurrentHashMap<>();
  private static final Map<String, MemorySegment> VALIDATOR_STUBS =
      new java.util.concurrent.ConcurrentHashMap<>();

  static {
    Runtime.getRuntime().addShutdownHook(new Thread(ASYNC_EXECUTOR::shutdown));
  }

  private Kreuzberg() {
    // Private constructor to prevent instantiation
  }

  /**
   * Extract text and metadata from a file asynchronously.
   *
   * <p>
   * This method schedules the extraction on a dedicated executor backed by virtual threads. The
   * returned {@link CompletableFuture} completes with the extraction result or exceptionally with
   * the same errors thrown by {@link #extractFileSync(String, String, ExtractionConfig)}.
   * </p>
   *
   * @param filePath the file path to extract
   * @return future resolving to the extraction result
   */
  public static CompletableFuture<ExtractionResult> extractFileAsync(String filePath) {
    return extractFileAsync(filePath, null, null);
  }

  /**
   * Extract text and metadata from a file asynchronously with MIME hint.
   *
   * @param filePath the file path to extract
   * @param mimeType optional MIME type hint
   * @return future resolving to the extraction result
   */
  public static CompletableFuture<ExtractionResult> extractFileAsync(String filePath,
      String mimeType) {
    return extractFileAsync(filePath, mimeType, null);
  }

  /**
   * Extract text and metadata from a file asynchronously with configuration.
   *
   * @param filePath the file path to extract
   * @param mimeType optional MIME type hint
   * @param config extraction configuration (uses defaults if null)
   * @return future resolving to the extraction result
   */
  public static CompletableFuture<ExtractionResult> extractFileAsync(String filePath,
      String mimeType, ExtractionConfig config) {
    Objects.requireNonNull(filePath, "filePath must not be null");
    return CompletableFuture.supplyAsync(() -> {
      try {
        return extractFileSync(filePath, mimeType, config);
      } catch (IOException | KreuzbergException e) {
        throw new CompletionException(e);
      }
    }, ASYNC_EXECUTOR);
  }

  /**
   * Extract text and metadata from a file asynchronously (Path overload).
   *
   * @param filePath the path to the file
   * @param mimeType optional MIME type hint
   * @param config extraction configuration (uses defaults if null)
   * @return future resolving to the extraction result
   */
  public static CompletableFuture<ExtractionResult> extractFileAsync(Path filePath, String mimeType,
      ExtractionConfig config) {
    Objects.requireNonNull(filePath, "filePath must not be null");
    return CompletableFuture.supplyAsync(() -> {
      try {
        return extractFileSync(filePath, mimeType, config);
      } catch (IOException | KreuzbergException e) {
        throw new CompletionException(e);
      }
    }, ASYNC_EXECUTOR);
  }

  /**
   * Extract text and metadata from bytes asynchronously.
   *
   * @param data the document bytes
   * @param mimeType the MIME type of the data
   * @return future resolving to the extraction result
   */
  public static CompletableFuture<ExtractionResult> extractBytesAsync(byte[] data,
      String mimeType) {
    return extractBytesAsync(data, mimeType, null);
  }

  /**
   * Extract text and metadata from bytes asynchronously with configuration.
   *
   * @param data the document bytes
   * @param mimeType the MIME type of the data
   * @param config extraction configuration (uses defaults if null)
   * @return future resolving to the extraction result
   */
  public static CompletableFuture<ExtractionResult> extractBytesAsync(byte[] data, String mimeType,
      ExtractionConfig config) {
    return CompletableFuture.supplyAsync(() -> {
      try {
        return extractBytesSync(data, mimeType, config);
      } catch (KreuzbergException e) {
        throw new CompletionException(e);
      }
    }, ASYNC_EXECUTOR);
  }

  /**
   * Batch extract multiple files asynchronously.
   *
   * @param filePaths list of file paths
   * @return future resolving to the list of extraction results
   */
  public static CompletableFuture<List<ExtractionResult>> batchExtractFilesAsync(
      List<String> filePaths) {
    return batchExtractFilesAsync(filePaths, null);
  }

  /**
   * Batch extract multiple files asynchronously with configuration.
   *
   * @param filePaths list of file paths
   * @param config extraction configuration (uses defaults if null)
   * @return future resolving to the list of extraction results
   */
  public static CompletableFuture<List<ExtractionResult>> batchExtractFilesAsync(
      List<String> filePaths, ExtractionConfig config) {
    return CompletableFuture.supplyAsync(() -> {
      try {
        return batchExtractFilesSync(filePaths, config);
      } catch (IOException | KreuzbergException e) {
        throw new CompletionException(e);
      }
    }, ASYNC_EXECUTOR);
  }

  /**
   * Batch extract multiple byte arrays asynchronously.
   *
   * @param dataList list of byte arrays with MIME info
   * @return future resolving to the list of extraction results
   */
  public static CompletableFuture<List<ExtractionResult>> batchExtractBytesAsync(
      List<BytesWithMime> dataList) {
    return batchExtractBytesAsync(dataList, null);
  }

  /**
   * Batch extract multiple byte arrays asynchronously with configuration.
   *
   * @param dataList list of byte arrays with MIME info
   * @param config extraction configuration (uses defaults if null)
   * @return future resolving to the list of extraction results
   */
  public static CompletableFuture<List<ExtractionResult>> batchExtractBytesAsync(
      List<BytesWithMime> dataList, ExtractionConfig config) {
    return CompletableFuture.supplyAsync(() -> {
      try {
        return batchExtractBytesSync(dataList, config);
      } catch (KreuzbergException e) {
        throw new CompletionException(e);
      }
    }, ASYNC_EXECUTOR);
  }

  /**
   * Extract text and metadata from a file.
   *
   * <p>
   * This is the main extraction method used by the e2e tests.
   * </p>
   *
   * @param filePath the path to the file to extract
   * @param config the extraction configuration as a Map (uses defaults if null)
   * @return the extraction result
   * @throws IOException if the file does not exist or cannot be read
   * @throws KreuzbergException if the extraction fails
   */
  public static ExtractionResult extractFile(String filePath, Map<String, Object> config)
      throws IOException, KreuzbergException {
    // Validate file exists
    Path path = Path.of(filePath);
    if (!Files.exists(path)) {
      throw new IOException("File not found: " + filePath);
    }
    if (!Files.isRegularFile(path)) {
      throw new IOException("Not a regular file: " + filePath);
    }
    if (!Files.isReadable(path)) {
      throw new IOException("File not readable: " + filePath);
    }

    try (Arena arena = Arena.ofConfined()) {
      if (config == null || config.isEmpty()) {
        // Call without config
        MemorySegment pathSegment = KreuzbergFFI.allocateCString(arena, filePath);
        MemorySegment resultPtr =
            (MemorySegment) KreuzbergFFI.KREUZBERG_EXTRACT_FILE_SYNC.invoke(pathSegment);
        return processExtractionResult(resultPtr);
      } else {
        // Call with config - convert Map to JSON directly
        String configJson = OBJECT_MAPPER.writeValueAsString(config);
        MemorySegment pathSegment = KreuzbergFFI.allocateCString(arena, filePath);
        MemorySegment configSegment = KreuzbergFFI.allocateCString(arena, configJson);
        MemorySegment resultPtr =
            (MemorySegment) KreuzbergFFI.KREUZBERG_EXTRACT_FILE_SYNC_WITH_CONFIG.invoke(pathSegment,
                configSegment);
        return processExtractionResult(resultPtr);
      }
    } catch (KreuzbergException e) {
      throw e;
    } catch (IOException e) {
      throw e;
    } catch (Throwable e) {
      throw new KreuzbergException("Unexpected error during extraction", e);
    }
  }

  /**
   * Extract text and metadata from a file (synchronous).
   *
   * @param filePath the path to the file to extract
   * @return the extraction result
   * @throws IOException if the file does not exist or cannot be read
   * @throws KreuzbergException if the extraction fails
   */
  public static ExtractionResult extractFileSync(String filePath)
      throws IOException, KreuzbergException {
    return extractFileSync(filePath, null, null);
  }

  /**
   * Extract text and metadata from a file (synchronous).
   *
   * @param filePath the path to the file to extract
   * @return the extraction result
   * @throws IOException if the file does not exist or cannot be read
   * @throws KreuzbergException if the extraction fails
   */
  public static ExtractionResult extractFileSync(Path filePath)
      throws IOException, KreuzbergException {
    return extractFileSync(filePath, null, null);
  }

  /**
   * Extract text and metadata from a file with optional MIME type hint (synchronous).
   *
   * @param filePath the path to the file to extract
   * @param mimeType optional MIME type hint (auto-detected if null)
   * @return the extraction result
   * @throws IOException if the file does not exist or cannot be read
   * @throws KreuzbergException if the extraction fails
   */
  public static ExtractionResult extractFileSync(String filePath, String mimeType)
      throws IOException, KreuzbergException {
    return extractFileSync(filePath, mimeType, null);
  }

  /**
   * Extract text and metadata from a file with optional MIME type hint (synchronous).
   *
   * @param filePath the path to the file to extract
   * @param mimeType optional MIME type hint (auto-detected if null)
   * @return the extraction result
   * @throws IOException if the file does not exist or cannot be read
   * @throws KreuzbergException if the extraction fails
   */
  public static ExtractionResult extractFileSync(Path filePath, String mimeType)
      throws IOException, KreuzbergException {
    return extractFileSync(filePath, mimeType, null);
  }

  /**
   * Extract text and metadata from a file with custom configuration (synchronous).
   *
   * @param filePath the path to the file to extract
   * @param mimeType optional MIME type hint (auto-detected if null)
   * @param config the extraction configuration (uses defaults if null)
   * @return the extraction result
   * @throws IOException if the file does not exist or cannot be read
   * @throws KreuzbergException if the extraction fails
   */
  public static ExtractionResult extractFileSync(String filePath, String mimeType,
      ExtractionConfig config) throws IOException, KreuzbergException {
    return extractFileSync(Path.of(filePath), mimeType, config);
  }

  /**
   * Extract text and metadata from a file with custom configuration (synchronous).
   *
   * @param filePath the path to the file to extract
   * @param mimeType optional MIME type hint (auto-detected if null)
   * @param config the extraction configuration (uses defaults if null)
   * @return the extraction result
   * @throws IOException if the file does not exist or cannot be read
   * @throws KreuzbergException if the extraction fails
   */
  public static ExtractionResult extractFileSync(Path filePath, String mimeType,
      ExtractionConfig config) throws IOException, KreuzbergException {
    // Validate file exists
    if (!Files.exists(filePath)) {
      throw new IOException("File not found: " + filePath);
    }
    if (!Files.isRegularFile(filePath)) {
      throw new IOException("Not a regular file: " + filePath);
    }
    if (!Files.isReadable(filePath)) {
      throw new IOException("File not readable: " + filePath);
    }

    try (Arena arena = Arena.ofConfined()) {
      if (config == null) {
        // Call without config
        MemorySegment pathSegment = KreuzbergFFI.allocateCString(arena, filePath.toString());
        MemorySegment resultPtr =
            (MemorySegment) KreuzbergFFI.KREUZBERG_EXTRACT_FILE_SYNC.invoke(pathSegment);
        return processExtractionResult(resultPtr);
      } else {
        // Call with config
        String configJson = toJson(config.toMap());
        MemorySegment pathSegment = KreuzbergFFI.allocateCString(arena, filePath.toString());
        MemorySegment configSegment = KreuzbergFFI.allocateCString(arena, configJson);
        MemorySegment resultPtr =
            (MemorySegment) KreuzbergFFI.KREUZBERG_EXTRACT_FILE_SYNC_WITH_CONFIG.invoke(pathSegment,
                configSegment);
        return processExtractionResult(resultPtr);
      }
    } catch (KreuzbergException e) {
      throw e;
    } catch (Throwable e) {
      throw new KreuzbergException("Unexpected error during extraction", e);
    }
  }

  /**
   * Extract text and metadata from a byte array (synchronous).
   *
   * @param data the document data as a byte array
   * @param mimeType the MIME type of the data (e.g., "application/pdf")
   * @return the extraction result
   * @throws KreuzbergException if the extraction fails
   */
  public static ExtractionResult extractBytesSync(byte[] data, String mimeType)
      throws KreuzbergException {
    return extractBytesSync(data, mimeType, null);
  }

  /**
   * Extract text and metadata from a byte array with custom configuration (synchronous).
   *
   * @param data the document data as a byte array
   * @param mimeType the MIME type of the data (e.g., "application/pdf")
   * @param config the extraction configuration (uses defaults if null)
   * @return the extraction result
   * @throws KreuzbergException if the extraction fails
   */
  public static ExtractionResult extractBytesSync(byte[] data, String mimeType,
      ExtractionConfig config) throws KreuzbergException {
    if (data == null || data.length == 0) {
      throw new IllegalArgumentException("Data cannot be null or empty");
    }
    if (mimeType == null || mimeType.isEmpty()) {
      throw new IllegalArgumentException("MIME type cannot be null or empty");
    }

    try (Arena arena = Arena.ofConfined()) {
      // Allocate native memory for byte array
      MemorySegment dataSegment = arena.allocate(data.length);
      MemorySegment.copy(data, 0, dataSegment, ValueLayout.JAVA_BYTE, 0, data.length);

      if (config == null) {
        // Call without config
        MemorySegment mimeSegment = KreuzbergFFI.allocateCString(arena, mimeType);
        MemorySegment resultPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_EXTRACT_BYTES_SYNC
            .invoke(dataSegment, (long) data.length, mimeSegment);
        return processExtractionResult(resultPtr);
      } else {
        // Call with config
        String configJson = toJson(config.toMap());
        MemorySegment mimeSegment = KreuzbergFFI.allocateCString(arena, mimeType);
        MemorySegment configSegment = KreuzbergFFI.allocateCString(arena, configJson);
        MemorySegment resultPtr =
            (MemorySegment) KreuzbergFFI.KREUZBERG_EXTRACT_BYTES_SYNC_WITH_CONFIG
                .invoke(dataSegment, (long) data.length, mimeSegment, configSegment);
        return processExtractionResult(resultPtr);
      }
    } catch (KreuzbergException e) {
      throw e;
    } catch (Throwable e) {
      throw new KreuzbergException("Unexpected error during extraction", e);
    }
  }

  /**
   * Extract text and metadata from multiple files in batch (synchronous).
   *
   * @param filePaths list of file paths to extract
   * @return list of extraction results (one per file)
   * @throws IOException if any file does not exist or cannot be read
   * @throws KreuzbergException if the extraction fails
   */
  public static List<ExtractionResult> batchExtractFilesSync(List<String> filePaths)
      throws IOException, KreuzbergException {
    return batchExtractFilesSync(filePaths, null);
  }

  /**
   * Extract text and metadata from multiple files in batch with custom configuration (synchronous).
   *
   * @param filePaths list of file paths to extract
   * @param config the extraction configuration (uses defaults if null)
   * @return list of extraction results (one per file)
   * @throws IOException if any file does not exist or cannot be read
   * @throws KreuzbergException if the extraction fails
   */
  public static List<ExtractionResult> batchExtractFilesSync(List<String> filePaths,
      ExtractionConfig config) throws IOException, KreuzbergException {
    if (filePaths == null || filePaths.isEmpty()) {
      throw new IllegalArgumentException("File paths list cannot be null or empty");
    }

    // Validate all files exist and are readable
    for (String filePath : filePaths) {
      Path path = Path.of(filePath);
      if (!Files.exists(path)) {
        throw new IOException("File not found: " + filePath);
      }
      if (!Files.isRegularFile(path)) {
        throw new IOException("Not a regular file: " + filePath);
      }
      if (!Files.isReadable(path)) {
        throw new IOException("File not readable: " + filePath);
      }
    }

    try (Arena arena = Arena.ofConfined()) {
      // Allocate array of C string pointers
      int count = filePaths.size();
      MemorySegment pathsArray = arena.allocate(ValueLayout.ADDRESS, count);

      // Convert each file path to C string and store pointer
      for (int i = 0; i < count; i++) {
        MemorySegment pathSegment = KreuzbergFFI.allocateCString(arena, filePaths.get(i));
        pathsArray.setAtIndex(ValueLayout.ADDRESS, i, pathSegment);
      }

      MemorySegment batchResultPtr;
      if (config == null) {
        // Call without config
        batchResultPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_BATCH_EXTRACT_FILES_SYNC
            .invoke(pathsArray, (long) count, MemorySegment.NULL);
      } else {
        // Call with config
        String configJson = toJson(config.toMap());
        MemorySegment configSegment = KreuzbergFFI.allocateCString(arena, configJson);
        batchResultPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_BATCH_EXTRACT_FILES_SYNC
            .invoke(pathsArray, (long) count, configSegment);
      }

      return processBatchResult(batchResultPtr);
    } catch (KreuzbergException | IOException e) {
      throw e;
    } catch (Throwable e) {
      throw new KreuzbergException("Unexpected error during batch extraction", e);
    }
  }

  /**
   * Extract text and metadata from multiple byte arrays in batch (synchronous).
   *
   * @param dataList list of byte arrays with MIME types to extract
   * @return list of extraction results (one per byte array)
   * @throws KreuzbergException if the extraction fails
   */
  public static List<ExtractionResult> batchExtractBytesSync(List<BytesWithMime> dataList)
      throws KreuzbergException {
    return batchExtractBytesSync(dataList, null);
  }

  /**
   * Extract text and metadata from multiple byte arrays in batch with custom configuration
   * (synchronous).
   *
   * @param dataList list of byte arrays with MIME types to extract
   * @param config the extraction configuration (uses defaults if null)
   * @return list of extraction results (one per byte array)
   * @throws KreuzbergException if the extraction fails
   */
  public static List<ExtractionResult> batchExtractBytesSync(List<BytesWithMime> dataList,
      ExtractionConfig config) throws KreuzbergException {
    if (dataList == null || dataList.isEmpty()) {
      throw new IllegalArgumentException("Data list cannot be null or empty");
    }

    try (Arena arena = Arena.ofConfined()) {
      int count = dataList.size();

      // Allocate array of CBytesWithMime structures
      long structSize = ValueLayout.ADDRESS.byteSize() + ValueLayout.JAVA_LONG.byteSize()
          + ValueLayout.ADDRESS.byteSize();
      MemorySegment bytesArray = arena.allocate(structSize * count);

      // Fill the array with data
      for (int i = 0; i < count; i++) {
        BytesWithMime item = dataList.get(i);
        long offset = i * structSize;

        // Allocate native memory for byte array
        MemorySegment dataSegment = arena.allocate(item.data().length);
        MemorySegment.copy(item.data(), 0, dataSegment, ValueLayout.JAVA_BYTE, 0,
            item.data().length);

        // Allocate MIME type C string
        MemorySegment mimeSegment = KreuzbergFFI.allocateCString(arena, item.mimeType());

        // Fill CBytesWithMime structure
        bytesArray.set(ValueLayout.ADDRESS, offset, dataSegment);
        bytesArray.set(ValueLayout.JAVA_LONG, offset + ValueLayout.ADDRESS.byteSize(),
            (long) item.data().length);
        bytesArray.set(ValueLayout.ADDRESS,
            offset + ValueLayout.ADDRESS.byteSize() + ValueLayout.JAVA_LONG.byteSize(),
            mimeSegment);
      }

      MemorySegment batchResultPtr;
      if (config == null) {
        // Call without config
        batchResultPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_BATCH_EXTRACT_BYTES_SYNC
            .invoke(bytesArray, (long) count, MemorySegment.NULL);
      } else {
        // Call with config
        String configJson = toJson(config.toMap());
        MemorySegment configSegment = KreuzbergFFI.allocateCString(arena, configJson);
        batchResultPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_BATCH_EXTRACT_BYTES_SYNC
            .invoke(bytesArray, (long) count, configSegment);
      }

      return processBatchResult(batchResultPtr);
    } catch (KreuzbergException e) {
      throw e;
    } catch (Throwable e) {
      throw new KreuzbergException("Unexpected error during batch extraction", e);
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
   * Register a custom OCR backend for use in extraction.
   *
   * <p>
   * The registered backend will be available for use via the backend name in OcrConfig. The Arena
   * parameter controls the lifecycle of the callback - the backend remains registered as long as
   * the Arena is alive.
   * </p>
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
      FunctionDescriptor callbackDescriptor = FunctionDescriptor.of(ValueLayout.ADDRESS, // return:
                                                                                         // *mut
                                                                                         // c_char
                                                                                         // (text
                                                                                         // result)
          ValueLayout.ADDRESS, // image_bytes: *const u8
          ValueLayout.JAVA_LONG, // image_length: usize
          ValueLayout.ADDRESS // config_json: *const c_char
      );

      MethodHandle backendMethod = MethodHandles.lookup().bind(backend, "processImage", MethodType
          .methodType(MemorySegment.class, MemorySegment.class, long.class, MemorySegment.class));

      MemorySegment callbackPointer = LINKER.upcallStub(backendMethod, callbackDescriptor, arena);

      // Register with Rust
      boolean success = (boolean) KreuzbergFFI.KREUZBERG_REGISTER_OCR_BACKEND.invoke(nameSegment,
          callbackPointer);

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
   * Register a custom PostProcessor for enriching extraction results.
   *
   * <p>
   * PostProcessors are applied after the core extraction completes. They can transform content, add
   * metadata, or perform additional analysis.
   * </p>
   *
   * @param name the processor name (must be unique)
   * @param processor the PostProcessor implementation
   * @param priority the execution priority (higher values run first)
   * @param arena the Arena that manages the callback lifecycle
   * @throws IllegalArgumentException if name is null or empty
   * @throws KreuzbergException if registration fails
   *
   * @since 4.0.0
   */
  public static void registerPostProcessor(String name, PostProcessor processor, int priority,
      Arena arena) throws KreuzbergException {
    if (name == null || name.isEmpty()) {
      throw new IllegalArgumentException("Processor name cannot be null or empty");
    }
    if (processor == null) {
      throw new IllegalArgumentException("Processor cannot be null");
    }
    if (arena == null) {
      throw new IllegalArgumentException("Arena cannot be null");
    }

    try {
      // Convert processor name to C string
      MemorySegment nameSegment = KreuzbergFFI.allocateCString(arena, name);

      // Create upcall stub for the processor
      // The callback signature: (result_json: *const c_char) -> *mut c_char
      FunctionDescriptor callbackDescriptor = FunctionDescriptor.of(ValueLayout.ADDRESS, // return:
                                                                                         // *mut
                                                                                         // c_char
                                                                                         // (modified
                                                                                         // result
                                                                                         // JSON)
          ValueLayout.ADDRESS // result_json: *const c_char
      );

      // Create a callback lambda that handles JSON serialization
      PostProcessorCallback callback = (MemorySegment resultJsonPtr) -> {
        try {
          // Deserialize input JSON to ExtractionResult
          String resultJson = KreuzbergFFI.readCString(resultJsonPtr);
          ExtractionResult result = OBJECT_MAPPER.readValue(resultJson, ExtractionResult.class);

          // Call the Java PostProcessor
          ExtractionResult processed = processor.process(result);

          // Serialize the processed result back to JSON
          String processedJson = OBJECT_MAPPER.writeValueAsString(processed);

          // Allocate C string for return value (using global arena since callback runs on FFI
          // thread)
          return Arena.global().allocateFrom(processedJson);
        } catch (Exception e) {
          // Return NULL on error
          return MemorySegment.NULL;
        }
      };

      MethodHandle processorMethod =
          MethodHandles.lookup().findVirtual(PostProcessorCallback.class, "apply",
              MethodType.methodType(MemorySegment.class, MemorySegment.class)).bindTo(callback);

      MemorySegment callbackPointer = LINKER.upcallStub(processorMethod, callbackDescriptor, arena);

      // Store processor and stub to prevent GC
      POST_PROCESSORS.put(name, processor);
      POST_PROCESSOR_STUBS.put(name, callbackPointer);

      // Register with Rust
      boolean success = (boolean) KreuzbergFFI.KREUZBERG_REGISTER_POST_PROCESSOR.invoke(nameSegment,
          callbackPointer, priority);

      if (!success) {
        POST_PROCESSORS.remove(name);
        POST_PROCESSOR_STUBS.remove(name);
        String error = getLastError();
        throw new KreuzbergException("Failed to register PostProcessor: " + error);
      }
    } catch (KreuzbergException e) {
      throw e;
    } catch (Throwable e) {
      throw new KreuzbergException("Unexpected error during PostProcessor registration", e);
    }
  }

  /**
   * Unregister a PostProcessor.
   *
   * @param name the processor name
   * @throws KreuzbergException if unregistration fails
   *
   * @since 4.0.0
   */
  public static void unregisterPostProcessor(String name) throws KreuzbergException {
    if (name == null || name.isEmpty()) {
      throw new IllegalArgumentException("Processor name cannot be null or empty");
    }

    try {
      try (Arena arena = Arena.ofConfined()) {
        MemorySegment nameSegment = KreuzbergFFI.allocateCString(arena, name);
        boolean success =
            (boolean) KreuzbergFFI.KREUZBERG_UNREGISTER_POST_PROCESSOR.invoke(nameSegment);

        if (!success) {
          String error = getLastError();
          throw new KreuzbergException("Failed to unregister PostProcessor: " + error);
        }
      }

      // Remove from storage
      POST_PROCESSORS.remove(name);
      POST_PROCESSOR_STUBS.remove(name);
    } catch (KreuzbergException e) {
      throw e;
    } catch (Throwable e) {
      throw new KreuzbergException("Unexpected error during PostProcessor unregistration", e);
    }
  }

  /**
   * Register a custom Validator for validating extraction results.
   *
   * <p>
   * Validators check extraction results for quality, completeness, or other criteria. If validation
   * fails, they throw a ValidationException.
   * </p>
   *
   * @param name the validator name (must be unique)
   * @param validator the Validator implementation
   * @param priority the execution priority (lower values run first)
   * @param arena the Arena that manages the callback lifecycle
   * @throws IllegalArgumentException if name is null or empty
   * @throws KreuzbergException if registration fails
   *
   * @since 4.0.0
   */
  public static void registerValidator(String name, Validator validator, int priority, Arena arena)
      throws KreuzbergException {
    if (name == null || name.isEmpty()) {
      throw new IllegalArgumentException("Validator name cannot be null or empty");
    }
    if (validator == null) {
      throw new IllegalArgumentException("Validator cannot be null");
    }
    if (arena == null) {
      throw new IllegalArgumentException("Arena cannot be null");
    }

    try {
      // Convert validator name to C string
      MemorySegment nameSegment = KreuzbergFFI.allocateCString(arena, name);

      // Create upcall stub for the validator
      // The callback signature: (result_json: *const c_char) -> *mut c_char (error message or NULL)
      FunctionDescriptor callbackDescriptor = FunctionDescriptor.of(ValueLayout.ADDRESS, // return:
                                                                                         // *mut
                                                                                         // c_char
                                                                                         // (error
                                                                                         // message
                                                                                         // or NULL)
          ValueLayout.ADDRESS // result_json: *const c_char
      );

      // Create a callback lambda that handles JSON serialization
      ValidatorCallback callback = (MemorySegment resultJsonPtr) -> {
        try {
          // Deserialize input JSON to ExtractionResult
          String resultJson = KreuzbergFFI.readCString(resultJsonPtr);
          ExtractionResult result = OBJECT_MAPPER.readValue(resultJson, ExtractionResult.class);

          // Call the Java Validator
          validator.validate(result);

          // Return NULL on success (no error)
          return MemorySegment.NULL;
        } catch (ValidationException e) {
          // Return error message (using global arena since callback runs on FFI thread)
          return Arena.global().allocateFrom(e.getMessage());
        } catch (Exception e) {
          // Return generic error message (using global arena since callback runs on FFI thread)
          return Arena.global().allocateFrom("Validation failed: " + e.getMessage());
        }
      };

      MethodHandle validatorMethod =
          MethodHandles.lookup().findVirtual(ValidatorCallback.class, "apply",
              MethodType.methodType(MemorySegment.class, MemorySegment.class)).bindTo(callback);

      MemorySegment callbackPointer = LINKER.upcallStub(validatorMethod, callbackDescriptor, arena);

      // Store validator and stub to prevent GC
      VALIDATORS.put(name, validator);
      VALIDATOR_STUBS.put(name, callbackPointer);

      // Register with Rust
      boolean success = (boolean) KreuzbergFFI.KREUZBERG_REGISTER_VALIDATOR.invoke(nameSegment,
          callbackPointer, priority);

      if (!success) {
        VALIDATORS.remove(name);
        VALIDATOR_STUBS.remove(name);
        String error = getLastError();
        throw new KreuzbergException("Failed to register Validator: " + error);
      }
    } catch (KreuzbergException e) {
      throw e;
    } catch (Throwable e) {
      throw new KreuzbergException("Unexpected error during Validator registration", e);
    }
  }

  /**
   * Unregister a Validator.
   *
   * @param name the validator name
   * @throws KreuzbergException if unregistration fails
   *
   * @since 4.0.0
   */
  public static void unregisterValidator(String name) throws KreuzbergException {
    if (name == null || name.isEmpty()) {
      throw new IllegalArgumentException("Validator name cannot be null or empty");
    }

    try {
      try (Arena arena = Arena.ofConfined()) {
        MemorySegment nameSegment = KreuzbergFFI.allocateCString(arena, name);
        boolean success = (boolean) KreuzbergFFI.KREUZBERG_UNREGISTER_VALIDATOR.invoke(nameSegment);

        if (!success) {
          String error = getLastError();
          throw new KreuzbergException("Failed to unregister Validator: " + error);
        }
      }

      // Remove from storage
      VALIDATORS.remove(name);
      VALIDATOR_STUBS.remove(name);
    } catch (KreuzbergException e) {
      throw e;
    } catch (Throwable e) {
      throw new KreuzbergException("Unexpected error during Validator unregistration", e);
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
   * Reads an ExtractionResult from a C extraction result memory segment.
   *
   * @param result the memory segment containing the C extraction result
   * @return the extraction result
   */
  private static ExtractionResult readExtractionResultFromMemory(MemorySegment result) {
    // Read string fields
    MemorySegment contentPtr = result.get(ValueLayout.ADDRESS, KreuzbergFFI.CONTENT_OFFSET);
    MemorySegment mimeTypePtr = result.get(ValueLayout.ADDRESS, KreuzbergFFI.MIME_TYPE_OFFSET);
    MemorySegment languagePtr = result.get(ValueLayout.ADDRESS, KreuzbergFFI.LANGUAGE_OFFSET);
    MemorySegment datePtr = result.get(ValueLayout.ADDRESS, KreuzbergFFI.DATE_OFFSET);
    MemorySegment subjectPtr = result.get(ValueLayout.ADDRESS, KreuzbergFFI.SUBJECT_OFFSET);
    MemorySegment tablesJsonPtr = result.get(ValueLayout.ADDRESS, KreuzbergFFI.TABLES_JSON_OFFSET);
    MemorySegment detectedLanguagesJsonPtr =
        result.get(ValueLayout.ADDRESS, KreuzbergFFI.DETECTED_LANGUAGES_JSON_OFFSET);
    MemorySegment metadataJsonPtr =
        result.get(ValueLayout.ADDRESS, KreuzbergFFI.METADATA_JSON_OFFSET);

    String content = KreuzbergFFI.readCString(contentPtr);
    String mimeType = KreuzbergFFI.readCString(mimeTypePtr);
    String language = KreuzbergFFI.readCString(languagePtr);
    String date = KreuzbergFFI.readCString(datePtr);
    String subject = KreuzbergFFI.readCString(subjectPtr);

    // Parse JSON fields
    List<Table> tables = parseTables(KreuzbergFFI.readCString(tablesJsonPtr));
    List<String> detectedLanguages =
        parseDetectedLanguages(KreuzbergFFI.readCString(detectedLanguagesJsonPtr));
    Map<String, Object> metadata = parseMetadata(KreuzbergFFI.readCString(metadataJsonPtr));

    // Create ExtractionResult with all fields
    return new ExtractionResult(content, mimeType,
        language != null ? java.util.Optional.of(language) : java.util.Optional.empty(),
        date != null ? java.util.Optional.of(date) : java.util.Optional.empty(),
        subject != null ? java.util.Optional.of(subject) : java.util.Optional.empty(), tables,
        detectedLanguages, metadata);
  }

  /**
   * Processes the batch extraction result from the native library.
   *
   * @param batchResultPtr the pointer to the C batch extraction result
   * @return list of extraction results
   * @throws KreuzbergException if extraction failed
   * @throws Throwable if an unexpected error occurs
   */
  private static List<ExtractionResult> processBatchResult(MemorySegment batchResultPtr)
      throws KreuzbergException, Throwable {
    // Check for null (error)
    if (batchResultPtr == null || batchResultPtr.address() == 0) {
      String error = getLastError();
      throw new KreuzbergException("Batch extraction failed: " + error);
    }

    try {
      // Read batch result fields
      MemorySegment batchResult =
          batchResultPtr.reinterpret(KreuzbergFFI.C_BATCH_RESULT_LAYOUT.byteSize());

      boolean success =
          batchResult.get(ValueLayout.JAVA_BOOLEAN, KreuzbergFFI.BATCH_SUCCESS_OFFSET);
      if (!success) {
        String error = getLastError();
        throw new KreuzbergException("Batch extraction failed: " + error);
      }

      long count = batchResult.get(ValueLayout.JAVA_LONG, KreuzbergFFI.BATCH_COUNT_OFFSET);
      MemorySegment resultsPtr =
          batchResult.get(ValueLayout.ADDRESS, KreuzbergFFI.BATCH_RESULTS_OFFSET);

      // Reinterpret resultsPtr as an array of pointers
      long arraySize = count * ValueLayout.ADDRESS.byteSize();
      MemorySegment resultsArray = resultsPtr.reinterpret(arraySize);

      // Process each result
      List<ExtractionResult> results = new ArrayList<>((int) count);
      for (int i = 0; i < count; i++) {
        // Get pointer to individual result
        MemorySegment resultPtr = resultsArray.getAtIndex(ValueLayout.ADDRESS, i);

        // Process the individual result (without freeing it, as batch cleanup handles that)
        MemorySegment result =
            resultPtr.reinterpret(KreuzbergFFI.C_EXTRACTION_RESULT_LAYOUT.byteSize());

        boolean resultSuccess = result.get(ValueLayout.JAVA_BOOLEAN, KreuzbergFFI.SUCCESS_OFFSET);
        if (!resultSuccess) {
          String error = getLastError();
          throw new KreuzbergException("Extraction failed for item " + i + ": " + error);
        }

        results.add(readExtractionResultFromMemory(result));
      }

      return results;
    } finally {
      // Free the batch result (this frees all individual results too)
      KreuzbergFFI.KREUZBERG_FREE_BATCH_RESULT.invoke(batchResultPtr);
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
      throwAppropriateException(error);
    }

    try {
      // Read result fields
      MemorySegment result =
          resultPtr.reinterpret(KreuzbergFFI.C_EXTRACTION_RESULT_LAYOUT.byteSize());

      boolean success = result.get(ValueLayout.JAVA_BOOLEAN, KreuzbergFFI.SUCCESS_OFFSET);
      if (!success) {
        String error = getLastError();
        throwAppropriateException(error);
      }

      return readExtractionResultFromMemory(result);
    } finally {
      // Free the result
      KreuzbergFFI.KREUZBERG_FREE_RESULT.invoke(resultPtr);
    }
  }

  /**
   * Throws the appropriate exception type based on error message content.
   *
   * @param error the error message
   * @throws ValidationException if error is a validation error
   * @throws KreuzbergException for other errors
   */
  private static void throwAppropriateException(String error) throws KreuzbergException {
    if (error != null && error.contains("Validation error")) {
      // Extract the validation message (remove "Validation error: " prefix)
      String validationMsg = error.replaceFirst(".*Validation error:\\s*", "");
      throw new ValidationException(validationMsg);
    }
    throw new KreuzbergException("Extraction failed: " + error);
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
    return str.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n").replace("\r", "\\r")
        .replace("\t", "\\t");
  }

  /**
   * Parses tables from JSON string.
   *
   * @param json the JSON string
   * @return list of tables, or empty list if JSON is null or invalid
   */
  private static List<Table> parseTables(String json) {
    if (json == null || json.isEmpty()) {
      return Collections.emptyList();
    }

    try {
      List<Map<String, Object>> rawTables = OBJECT_MAPPER.readValue(json, TABLE_LIST_TYPE);

      List<Table> tables = new ArrayList<>(rawTables.size());
      for (Map<String, Object> rawTable : rawTables) {
        @SuppressWarnings("unchecked")
        List<List<String>> cells = (List<List<String>>) rawTable.get("cells");
        String markdown = (String) rawTable.get("markdown");
        Number pageNumber = (Number) rawTable.get("page_number");

        if (cells != null && markdown != null && pageNumber != null) {
          tables.add(new Table(cells, markdown, pageNumber.intValue()));
        }
      }
      return tables;
    } catch (Exception e) {
      // Log error and return empty list
      return Collections.emptyList();
    }
  }

  /**
   * Parses detected languages from JSON string.
   *
   * @param json the JSON string
   * @return list of language codes, or empty list if JSON is null or invalid
   */
  private static List<String> parseDetectedLanguages(String json) {
    if (json == null || json.isEmpty()) {
      return Collections.emptyList();
    }

    try {
      return OBJECT_MAPPER.readValue(json, STRING_LIST_TYPE);
    } catch (Exception e) {
      // Log error and return empty list
      return Collections.emptyList();
    }
  }

  /**
   * Parses metadata from JSON string.
   *
   * @param json the JSON string
   * @return metadata map, or empty map if JSON is null or invalid
   */
  @SuppressWarnings("unchecked")
  private static Map<String, Object> parseMetadata(String json) {
    if (json == null || json.isEmpty()) {
      return Collections.emptyMap();
    }

    try {
      return OBJECT_MAPPER.readValue(json, STRING_OBJECT_MAP_TYPE);
    } catch (Exception e) {
      // Log error and return empty map
      return Collections.emptyMap();
    }
  }

  /**
   * Functional interface for PostProcessor callbacks.
   */
  @FunctionalInterface
  private interface PostProcessorCallback {
    MemorySegment apply(MemorySegment resultJsonPtr);
  }

  /**
   * Functional interface for Validator callbacks.
   */
  @FunctionalInterface
  private interface ValidatorCallback {
    MemorySegment apply(MemorySegment resultJsonPtr);
  }
}
