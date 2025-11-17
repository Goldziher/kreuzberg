package dev.kreuzberg;

import dev.kreuzberg.config.ExtractionConfig;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionException;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for Kreuzberg Java bindings.
 */
class KreuzbergTest {
    private static final int DEFAULT_PRIORITY = 100;
    private static final int MIN_CONTENT_LENGTH = 10;
    private static final int SAMPLE_CONCURRENCY = 3;

    @Test
    void testGetVersion() {
        String version = Kreuzberg.getVersion();
        assertNotNull(version, "Version should not be null");
        assertFalse(version.isEmpty(), "Version should not be empty");
        assertTrue(version.matches("\\d+\\.\\d+\\.\\d+.*"), "Version should match pattern");
    }

    @Test
    void testExtractTextFile(@TempDir Path tempDir) throws IOException, KreuzbergException {
        // Create a test file
        Path testFile = tempDir.resolve("test.txt");
        String content = "Hello, Kreuzberg!";
        Files.writeString(testFile, content);

        // Extract
        ExtractionResult result = Kreuzberg.extractFileSync(testFile);

        // Verify
        assertNotNull(result, "Result should not be null");
        assertNotNull(result.content(), "Content should not be null");
        assertTrue(result.content().contains("Hello"), "Content should contain test text");
        assertNotNull(result.mimeType(), "MIME type should not be null");
    }

    @Test
    void testExtractNonexistentFile() {
        Path nonexistent = Path.of("/nonexistent/file.txt");
        assertThrows(IOException.class, () -> {
            Kreuzberg.extractFileSync(nonexistent);
        }, "Should throw IOException for nonexistent file");
    }

    @Test
    void testExtractionResultToString(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "Test content");

        ExtractionResult result = Kreuzberg.extractFileSync(testFile);
        String str = result.toString();

        assertNotNull(str, "toString should not return null");
        assertTrue(str.contains("ExtractionResult"), "toString should contain class name");
        assertTrue(str.contains("mimeType"), "toString should contain field names");
    }

    @Test
    void testExtractionResultFields(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "Test");

        ExtractionResult result = Kreuzberg.extractFileSync(testFile);

        // Required fields
        assertNotNull(result.content());
        assertNotNull(result.mimeType());

        // Optional fields (just verify they're Optional)
        assertNotNull(result.language());
        assertNotNull(result.date());
        assertNotNull(result.subject());
        assertNotNull(result.getChunks());
        assertNotNull(result.getImages());
    }

    @Test
    void testExtractionResultWithMethods(@TempDir Path tempDir) throws IOException, KreuzbergException {
        // Create test file
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "original");

        ExtractionResult original = Kreuzberg.extractFileSync(testFile);

        // Test withContent
        ExtractionResult withNewContent = original.withContent("modified");
        assertEquals("modified", withNewContent.content());
        assertEquals(original.mimeType(), withNewContent.mimeType()); // Other fields unchanged

        // Test withLanguage
        ExtractionResult withLanguage = original.withLanguage("eng");
        assertTrue(withLanguage.language().isPresent());
        assertEquals("eng", withLanguage.language().get());

        // Test withSubject
        ExtractionResult withSubject = original.withSubject("Test Subject");
        assertTrue(withSubject.subject().isPresent());
        assertEquals("Test Subject", withSubject.subject().get());

        // Test withDate
        ExtractionResult withDate = original.withDate("2024-01-01");
        assertTrue(withDate.date().isPresent());
        assertEquals("2024-01-01", withDate.date().get());
    }

    @Test
    void testLoadConfig(@TempDir Path tempDir) throws Exception {
        Path configFile = tempDir.resolve("kreuzberg.toml");
        Files.writeString(
            configFile,
            "use_cache = false\nmax_concurrent_extractions = " + SAMPLE_CONCURRENCY + "\n"
        );

        ExtractionConfig config = Kreuzberg.loadConfig(configFile);
        assertNotNull(config, "Config should load");
        assertFalse(config.isUseCache(), "use_cache should reflect file");
        assertEquals(Integer.valueOf(SAMPLE_CONCURRENCY), config.getMaxConcurrentExtractions());
        assertEquals(Boolean.FALSE, config.toMap().get("use_cache"));
    }

    @Test
    void testExtractBytesSync() throws KreuzbergException {
        // Create test data
        String content = "Hello, Kreuzberg from bytes!";
        byte[] data = content.getBytes();

        // Extract from bytes
        ExtractionResult result = Kreuzberg.extractBytesSync(data, "text/plain");

        // Verify
        assertNotNull(result, "Result should not be null");
        assertNotNull(result.content(), "Content should not be null");
        assertTrue(result.content().contains("Hello"), "Content should contain test text");
        assertNotNull(result.mimeType(), "MIME type should not be null");
    }

    @Test
    void testExtractBytesSyncWithNullData() {
        assertThrows(IllegalArgumentException.class, () -> {
            Kreuzberg.extractBytesSync(null, "text/plain");
        }, "Should throw IllegalArgumentException for null data");
    }

    @Test
    void testExtractBytesSyncWithEmptyData() {
        assertThrows(IllegalArgumentException.class, () -> {
            Kreuzberg.extractBytesSync(new byte[0], "text/plain");
        }, "Should throw IllegalArgumentException for empty data");
    }

    @Test
    void testExtractBytesSyncWithNullMimeType() {
        byte[] data = "test".getBytes();
        assertThrows(IllegalArgumentException.class, () -> {
            Kreuzberg.extractBytesSync(data, null);
        }, "Should throw IllegalArgumentException for null MIME type");
    }

    @Test
    void testBatchExtractFilesSync(@TempDir Path tempDir) throws IOException, KreuzbergException {
        // Create test files
        Path file1 = tempDir.resolve("test1.txt");
        Path file2 = tempDir.resolve("test2.txt");
        Files.writeString(file1, "Content of file 1");
        Files.writeString(file2, "Content of file 2");

        // Batch extract
        java.util.List<String> filePaths = java.util.List.of(
            file1.toString(),
            file2.toString()
        );
        java.util.List<ExtractionResult> results = Kreuzberg.batchExtractFilesSync(filePaths);

        // Verify
        assertNotNull(results, "Results should not be null");
        assertEquals(2, results.size(), "Should have 2 results");
        assertTrue(results.get(0).content().contains("file 1"), "First result should contain correct content");
        assertTrue(results.get(1).content().contains("file 2"), "Second result should contain correct content");
    }

    @Test
    void testBatchExtractFilesSyncWithEmptyList() {
        assertThrows(IllegalArgumentException.class, () -> {
            Kreuzberg.batchExtractFilesSync(java.util.List.of());
        }, "Should throw IllegalArgumentException for empty list");
    }

    @Test
    void testBatchExtractFilesSyncWithNullList() {
        assertThrows(IllegalArgumentException.class, () -> {
            Kreuzberg.batchExtractFilesSync(null);
        }, "Should throw IllegalArgumentException for null list");
    }

    @Test
    void testBatchExtractBytesSync() throws KreuzbergException {
        // Create test data
        BytesWithMime data1 = new BytesWithMime("Content 1".getBytes(), "text/plain");
        BytesWithMime data2 = new BytesWithMime("Content 2".getBytes(), "text/plain");

        // Batch extract
        java.util.List<BytesWithMime> dataList = java.util.List.of(data1, data2);
        java.util.List<ExtractionResult> results = Kreuzberg.batchExtractBytesSync(dataList);

        // Verify
        assertNotNull(results, "Results should not be null");
        assertEquals(2, results.size(), "Should have 2 results");
        assertTrue(results.get(0).content().contains("Content 1"), "First result should contain correct content");
        assertTrue(results.get(1).content().contains("Content 2"), "Second result should contain correct content");
    }

    @Test
    void testBatchExtractBytesSyncWithEmptyList() {
        assertThrows(IllegalArgumentException.class, () -> {
            Kreuzberg.batchExtractBytesSync(java.util.List.of());
        }, "Should throw IllegalArgumentException for empty list");
    }

    @Test
    void testBatchExtractBytesSyncWithNullList() {
        assertThrows(IllegalArgumentException.class, () -> {
            Kreuzberg.batchExtractBytesSync(null);
        }, "Should throw IllegalArgumentException for null list");
    }

    @Test
    void testExtractFileAsync(@TempDir Path tempDir) throws Exception {
        Path testFile = tempDir.resolve("async-file.txt");
        Files.writeString(testFile, "Async hello world");

        ExtractionResult result = Kreuzberg.extractFileAsync(testFile.toString()).join();
        assertNotNull(result);
        assertTrue(result.content().contains("Async hello world"));
    }

    @Test
    void testExtractFileAsyncError() {
        CompletionException exception = assertThrows(CompletionException.class, () -> {
            Kreuzberg.extractFileAsync("/nonexistent/async/file.txt").join();
        });
        assertTrue(exception.getCause() instanceof IOException,
                "Expected IOException as cause, got: " + exception.getCause());
    }

    @Test
    void testExtractBytesAsync() {
        byte[] data = "Hello async bytes".getBytes();
        ExtractionResult result = Kreuzberg.extractBytesAsync(data, "text/plain").join();
        assertNotNull(result);
        assertTrue(result.content().contains("Hello async bytes"));
    }

    @Test
    void testBatchExtractFilesAsync(@TempDir Path tempDir) throws Exception {
        Path file1 = tempDir.resolve("async1.txt");
        Path file2 = tempDir.resolve("async2.txt");
        Files.writeString(file1, "Async file one");
        Files.writeString(file2, "Async file two");

        CompletableFuture<java.util.List<ExtractionResult>> future = Kreuzberg.batchExtractFilesAsync(
                java.util.List.of(file1.toString(), file2.toString()));
        java.util.List<ExtractionResult> results = future.join();

        assertEquals(2, results.size());
        assertTrue(results.get(0).content().contains("Async file"));
        assertTrue(results.get(1).content().contains("Async file"));
    }

    @Test
    void testBatchExtractBytesAsync() {
        BytesWithMime data1 = new BytesWithMime("Async content 1".getBytes(), "text/plain");
        BytesWithMime data2 = new BytesWithMime("Async content 2".getBytes(), "text/plain");

        java.util.List<ExtractionResult> results = Kreuzberg.batchExtractBytesAsync(
                java.util.List.of(data1, data2)).join();

        assertEquals(2, results.size());
        assertTrue(results.get(0).content().contains("Async content 1"));
        assertTrue(results.get(1).content().contains("Async content 2"));
    }

    @Test
    void testPostProcessorRegistration(@TempDir Path tempDir) throws Exception {
        // Create a test file
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "hello world");

        // Create an uppercase post-processor
        PostProcessor uppercaseProcessor = result -> result.withContent(result.content().toUpperCase());

        // Register the processor
        try (java.lang.foreign.Arena arena = java.lang.foreign.Arena.ofConfined()) {
            Kreuzberg.registerPostProcessor("test-uppercase", uppercaseProcessor, DEFAULT_PRIORITY, arena);

            // Extract and verify the processor was applied
            ExtractionResult result = Kreuzberg.extractFileSync(testFile);
            assertTrue(result.content().contains("HELLO WORLD"), "Content should be uppercase");

            // Unregister
            Kreuzberg.unregisterPostProcessor("test-uppercase");
        }
    }

    @Test
    void testPostProcessorUnregistration(@TempDir Path tempDir) throws Exception {
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "hello");

        PostProcessor uppercaseProcessor = result -> result.withContent(result.content().toUpperCase());

        try (java.lang.foreign.Arena arena = java.lang.foreign.Arena.ofConfined()) {
            // Register
            Kreuzberg.registerPostProcessor("test-unregister", uppercaseProcessor, DEFAULT_PRIORITY, arena);

            // Unregister
            Kreuzberg.unregisterPostProcessor("test-unregister");

            // Extract - should NOT be uppercase now
            ExtractionResult result = Kreuzberg.extractFileSync(testFile);
            assertFalse(result.content().contains("HELLO"),
                    "Content should not be uppercase after unregistration");
        }
    }

    @Test
    void testValidatorRegistration(@TempDir Path tempDir) throws Exception {
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "short");

        // Create a validator that requires minimum content length
        Validator minLengthValidator = result -> {
            if (result.content().length() < MIN_CONTENT_LENGTH) {
                throw new ValidationException("Content too short: " + result.content().length());
            }
        };

        try (java.lang.foreign.Arena arena = java.lang.foreign.Arena.ofConfined()) {
            Kreuzberg.registerValidator("test-min-length", minLengthValidator, DEFAULT_PRIORITY, arena);

            // Extract should fail validation
            ValidationException exception = assertThrows(ValidationException.class, () -> {
                Kreuzberg.extractFileSync(testFile);
            }, "Should throw ValidationException for short content");

            assertTrue(exception.getMessage().contains("too short"),
                    "Exception message should contain 'too short'");

            // Unregister
            Kreuzberg.unregisterValidator("test-min-length");
        }
    }

    @Test
    void testValidatorUnregistration(@TempDir Path tempDir) throws Exception {
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "short");

        Validator minLengthValidator = result -> {
            if (result.content().length() < MIN_CONTENT_LENGTH) {
                throw new ValidationException("Content too short");
            }
        };

        try (java.lang.foreign.Arena arena = java.lang.foreign.Arena.ofConfined()) {
            // Register
            Kreuzberg.registerValidator("test-validator-unreg", minLengthValidator, DEFAULT_PRIORITY, arena);

            // Unregister
            Kreuzberg.unregisterValidator("test-validator-unreg");

            // Extract should succeed now
            ExtractionResult result = Kreuzberg.extractFileSync(testFile);
            assertNotNull(result, "Result should not be null after validator unregistration");
        }
    }

    @Test
    void testPostProcessorWithNullName() {
        PostProcessor processor = result -> result;
        assertThrows(IllegalArgumentException.class, () -> {
            try (java.lang.foreign.Arena arena = java.lang.foreign.Arena.ofConfined()) {
                Kreuzberg.registerPostProcessor(null, processor, DEFAULT_PRIORITY, arena);
            }
        }, "Should throw IllegalArgumentException for null processor name");
    }

    @Test
    void testValidatorWithNullName() {
        Validator validator = result -> { };
        assertThrows(IllegalArgumentException.class, () -> {
            try (java.lang.foreign.Arena arena = java.lang.foreign.Arena.ofConfined()) {
                Kreuzberg.registerValidator(null, validator, DEFAULT_PRIORITY, arena);
            }
        }, "Should throw IllegalArgumentException for null validator name");
    }
}
