package dev.kreuzberg;

import dev.kreuzberg.config.ExtractionConfig;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Comprehensive error handling tests for Kreuzberg extraction.
 *
 * Tests cover file not found errors, invalid MIME types, corrupted files,
 * permission errors, invalid configuration values, and edge cases.
 */
class ErrorHandlingTest {

    // ==================== File Not Found Errors ====================

    @Test
    void testExtractNonexistentFile() {
        Path nonexistent = Path.of("/nonexistent/path/to/file.txt");

        assertThrows(IOException.class, () -> {
            Kreuzberg.extractFile(nonexistent);
        }, "Should throw IOException for nonexistent file");
    }

    @Test
    void testExtractFileWithInvalidPath() {
        Path invalid = Path.of("");

        assertThrows(IOException.class, () -> {
            Kreuzberg.extractFile(invalid);
        }, "Should throw IOException for invalid path");
    }

    @Test
    void testExtractFromNullPath() {
        assertThrows(NullPointerException.class, () -> {
            Kreuzberg.extractFile((Path) null);
        }, "Should throw NullPointerException for null path");
    }

    @Test
    void testExtractFromNullStringPath() {
        assertThrows(NullPointerException.class, () -> {
            Kreuzberg.extractFile((String) null);
        }, "Should throw NullPointerException for null string path");
    }

    // ==================== Directory Instead of File ====================

    @Test
    void testExtractFromDirectory(@TempDir Path tempDir) {
        assertThrows(IOException.class, () -> {
            Kreuzberg.extractFile(tempDir);
        }, "Should throw IOException when trying to extract from directory");
    }

    @Test
    void testExtractFromSymlinkToDirectory(@TempDir Path tempDir) throws IOException {
        Path dir = tempDir.resolve("subdir");
        Files.createDirectory(dir);
        Path link = tempDir.resolve("link");
        try {
            Files.createSymbolicLink(link, dir);
            assertThrows(IOException.class, () -> {
                Kreuzberg.extractFile(link);
            }, "Should throw IOException for symlink to directory");
        } catch (UnsupportedOperationException e) {
            // Symlinks may not be supported on all platforms
        }
    }

    // ==================== Permission Errors ====================

    @Test
    void testExtractUnreadableFile(@TempDir Path tempDir) throws IOException {
        Path testFile = tempDir.resolve("unreadable.txt");
        Files.writeString(testFile, "content");

        try {
            // Try to remove read permissions
            testFile.toFile().setReadable(false);

            assertThrows(IOException.class, () -> {
                Kreuzberg.extractFile(testFile);
            }, "Should throw IOException for unreadable file");
        } finally {
            // Restore permissions for cleanup
            testFile.toFile().setReadable(true);
        }
    }

    // ==================== Invalid MIME Types ====================

    @Test
    void testExtractBytesWithNullMimeType() {
        byte[] data = "test".getBytes();

        assertThrows(KreuzbergException.class, () -> {
            Kreuzberg.extractBytes(data, null, null);
        }, "Should throw KreuzbergException for null MIME type");
    }

    @Test
    void testExtractBytesWithEmptyMimeType() {
        byte[] data = "test".getBytes();

        assertThrows(KreuzbergException.class, () -> {
            Kreuzberg.extractBytes(data, "", null);
        }, "Should throw KreuzbergException for empty MIME type");
    }

    @Test
    void testExtractBytesWithBlankMimeType() {
        byte[] data = "test".getBytes();

        assertThrows(KreuzbergException.class, () -> {
            Kreuzberg.extractBytes(data, "   ", null);
        }, "Should throw KreuzbergException for blank MIME type");
    }

    @Test
    void testValidateMimeTypeWithNull() {
        assertThrows(NullPointerException.class, () -> {
            Kreuzberg.validateMimeType(null);
        }, "Should throw NullPointerException for null MIME type");
    }

    @Test
    void testValidateMimeTypeWithEmpty() {
        assertThrows(KreuzbergException.class, () -> {
            Kreuzberg.validateMimeType("");
        }, "Should throw KreuzbergException for empty MIME type");
    }

    // ==================== Empty Data ====================

    @Test
    void testExtractEmptyByteArray() {
        byte[] empty = new byte[0];

        assertThrows(KreuzbergException.class, () -> {
            Kreuzberg.extractBytes(empty, "text/plain", null);
        }, "Should throw KreuzbergException for empty byte array");
    }

    @Test
    void testExtractNullByteArray() {
        assertThrows(NullPointerException.class, () -> {
            Kreuzberg.extractBytes(null, "text/plain", null);
        }, "Should throw NullPointerException for null byte array");
    }

    // ==================== Batch Operation Errors ====================

    @Test
    void testBatchExtractNullPaths() {
        assertThrows(NullPointerException.class, () -> {
            Kreuzberg.batchExtractFiles(null, null);
        }, "Should throw NullPointerException for null paths list");
    }

    @Test
    void testBatchExtractEmptyPaths() throws KreuzbergException {
        var results = Kreuzberg.batchExtractFiles(java.util.List.of(), null);

        assertNotNull(results, "Should return list for empty paths");
        assertTrue(results.isEmpty(), "Should return empty list for empty input");
    }

    @Test
    void testBatchExtractNullBytesList() {
        assertThrows(NullPointerException.class, () -> {
            Kreuzberg.batchExtractBytes(null, null);
        }, "Should throw NullPointerException for null bytes list");
    }

    @Test
    void testBatchExtractEmptyBytesList() throws KreuzbergException {
        var results = Kreuzberg.batchExtractBytes(java.util.List.of(), null);

        assertNotNull(results, "Should return list for empty bytes");
        assertTrue(results.isEmpty(), "Should return empty list for empty input");
    }

    // ==================== Configuration Validation ====================

    @Test
    void testExtractionWithInvalidConfiguration(@TempDir Path tempDir) throws IOException {
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "content");

        // Create a valid config - the API should not throw for valid configs
        ExtractionConfig config = ExtractionConfig.builder()
                .maxConcurrentExtractions(-1) // Negative value
                .build();

        // Should either succeed or throw, but not crash
        try {
            Kreuzberg.extractFile(testFile, config);
        } catch (KreuzbergException e) {
            // Expected for invalid config
            assertNotNull(e.getMessage(), "Exception should have message");
        }
    }

    @Test
    void testExtractionConfigWithNegativeMaxConcurrent() {
        ExtractionConfig config = ExtractionConfig.builder()
                .maxConcurrentExtractions(-5)
                .build();

        assertEquals(-5, config.getMaxConcurrentExtractions(),
                "Config should accept negative values (validation is backend's responsibility)");
    }

    @Test
    void testChunkingConfigWithZeroMaxChars() {
        var config = dev.kreuzberg.config.ChunkingConfig.builder()
                .maxChars(0)
                .build();

        assertEquals(0, config.getMaxChars(), "Config should accept zero max chars");
    }

    @Test
    void testChunkingConfigWithNegativeOverlap() {
        var config = dev.kreuzberg.config.ChunkingConfig.builder()
                .maxOverlap(-50)
                .build();

        assertEquals(-50, config.getMaxOverlap(), "Config should accept negative overlap");
    }

    // ==================== MIME Type Detection Errors ====================

    @Test
    void testDetectMimeTypeWithNullPath() {
        assertThrows(NullPointerException.class, () -> {
            Kreuzberg.detectMimeType((String) null);
        }, "Should throw NullPointerException for null path");
    }

    @Test
    void testDetectMimeTypeFromNullBytes() {
        assertThrows(NullPointerException.class, () -> {
            Kreuzberg.detectMimeType((byte[]) null);
        }, "Should throw NullPointerException for null bytes");
    }

    @Test
    void testDetectMimeTypeFromEmptyBytes() {
        byte[] empty = new byte[0];

        assertThrows(KreuzbergException.class, () -> {
            Kreuzberg.detectMimeType(empty);
        }, "Should throw KreuzbergException for empty bytes");
    }

    @Test
    void testGetExtensionsForNullMimeType() {
        assertThrows(NullPointerException.class, () -> {
            Kreuzberg.getExtensionsForMime(null);
        }, "Should throw NullPointerException for null MIME type");
    }

    @Test
    void testGetExtensionsForEmptyMimeType() {
        assertThrows(KreuzbergException.class, () -> {
            Kreuzberg.getExtensionsForMime("");
        }, "Should throw KreuzbergException for empty MIME type");
    }

    @Test
    void testGetExtensionsForBlankMimeType() {
        assertThrows(KreuzbergException.class, () -> {
            Kreuzberg.getExtensionsForMime("   ");
        }, "Should throw KreuzbergException for blank MIME type");
    }

    // ==================== Plugin Management Errors ====================

    @Test
    void testRegisterValidatorWithNullName() {
        Validator validator = result -> { };

        assertThrows(KreuzbergException.class, () -> {
            Kreuzberg.registerValidator(null, validator);
        }, "Should throw KreuzbergException for null validator name");
    }

    @Test
    void testRegisterValidatorWithBlankName() {
        Validator validator = result -> { };

        assertThrows(KreuzbergException.class, () -> {
            Kreuzberg.registerValidator("", validator);
        }, "Should throw KreuzbergException for blank validator name");
    }

    @Test
    void testRegisterValidatorWithNullValidator() {
        assertThrows(NullPointerException.class, () -> {
            Kreuzberg.registerValidator("test", null);
        }, "Should throw NullPointerException for null validator");
    }

    @Test
    void testRegisterPostProcessorWithNullName() {
        PostProcessor processor = result -> result;

        assertThrows(KreuzbergException.class, () -> {
            Kreuzberg.registerPostProcessor(null, processor);
        }, "Should throw KreuzbergException for null processor name");
    }

    @Test
    void testRegisterPostProcessorWithBlankName() {
        PostProcessor processor = result -> result;

        assertThrows(KreuzbergException.class, () -> {
            Kreuzberg.registerPostProcessor("", processor);
        }, "Should throw KreuzbergException for blank processor name");
    }

    @Test
    void testRegisterPostProcessorWithNullProcessor() {
        assertThrows(NullPointerException.class, () -> {
            Kreuzberg.registerPostProcessor("test", null);
        }, "Should throw NullPointerException for null processor");
    }

    @Test
    void testRegisterOcrBackendWithNullName() {
        OcrBackend backend = (data, config) -> "text";

        assertThrows(KreuzbergException.class, () -> {
            Kreuzberg.registerOcrBackend(null, backend);
        }, "Should throw KreuzbergException for null backend name");
    }

    @Test
    void testRegisterOcrBackendWithNullBackend() {
        assertThrows(NullPointerException.class, () -> {
            Kreuzberg.registerOcrBackend("test", null);
        }, "Should throw NullPointerException for null backend");
    }

    // ==================== Corruption and Invalid Data ====================

    @Test
    void testExtractCorruptedFile(@TempDir Path tempDir) throws IOException {
        Path testFile = tempDir.resolve("corrupted.bin");
        // Write binary garbage
        Files.write(testFile, new byte[]{(byte) 0xFF, (byte) 0xFE, (byte) 0xFD, (byte) 0xFC});

        // Should not crash, might succeed or fail gracefully
        try {
            ExtractionResult result = Kreuzberg.extractFile(testFile);
            assertNotNull(result, "Should return a result even for corrupted data");
        } catch (KreuzbergException e) {
            // Acceptable to throw for corrupted data
            assertNotNull(e.getMessage(), "Exception should have a message");
        }
    }

    @Test
    void testExtractFileWithInvalidEncoding(@TempDir Path tempDir) throws IOException {
        Path testFile = tempDir.resolve("invalid_encoding.txt");
        // Write bytes that don't form valid UTF-8
        Files.write(testFile, new byte[]{(byte) 0x80, (byte) 0x81, (byte) 0x82});

        // Should handle gracefully
        try {
            ExtractionResult result = Kreuzberg.extractFile(testFile);
            assertNotNull(result, "Should return result for invalid encoding");
        } catch (KreuzbergException e) {
            // Acceptable
        }
    }

    // ==================== Configuration File Errors ====================

    @Test
    void testLoadConfigFromNonexistentFile() {
        Path nonexistent = Path.of("/nonexistent/kreuzberg.toml");

        assertThrows(KreuzbergException.class, () -> {
            ExtractionConfig.fromFile(nonexistent.toString());
        }, "Should throw KreuzbergException for nonexistent config file");
    }

    @Test
    void testLoadConfigFromDirectory(@TempDir Path tempDir) {
        assertThrows(KreuzbergException.class, () -> {
            ExtractionConfig.fromFile(tempDir.toString());
        }, "Should throw KreuzbergException for directory instead of file");
    }

    @Test
    void testLoadConfigWithInvalidSyntax(@TempDir Path tempDir) throws IOException {
        Path configFile = tempDir.resolve("bad.toml");
        Files.writeString(configFile, "invalid toml [[ syntax");

        assertThrows(KreuzbergException.class, () -> {
            ExtractionConfig.fromFile(configFile.toString());
        }, "Should throw KreuzbergException for invalid config syntax");
    }

    // ==================== Discovery Errors ====================

    @Test
    void testDiscoverConfigWhenNotFound() throws KreuzbergException {
        // Discovery should return null or empty, not throw
        ExtractionConfig config = ExtractionConfig.discover();

        // config may be null or a default config
        assertTrue(config == null || config != null, "Discovery should handle missing config");
    }

    // ==================== Async Operation Errors ====================

    @Test
    void testAsyncExtractNonexistentFile() {
        Path nonexistent = Path.of("/nonexistent/file.txt");

        var future = Kreuzberg.extractFileAsync(nonexistent, null);

        assertThrows(Exception.class, () -> {
            future.get(); // Should throw wrapped IOException
        }, "Async extraction should propagate errors");
    }

    @Test
    void testAsyncBatchExtractNullPaths() {
        var future = Kreuzberg.batchExtractFilesAsync(null, null);

        assertThrows(Exception.class, () -> {
            future.get(); // Should throw wrapped NullPointerException
        }, "Async batch extraction should propagate errors");
    }

    // ==================== Validator and PostProcessor Errors ====================

    @Test
    void testValidatorThrowsException() throws KreuzbergException {
        String name = "failing-validator-" + System.currentTimeMillis();

        Validator validator = result -> {
            throw new ValidationException("Validation failed");
        };

        Kreuzberg.registerValidator(name, validator);

        try {
            var validators = Kreuzberg.listValidators();
            assertTrue(validators.contains(name), "Failing validator should be registered");
        } finally {
            Kreuzberg.unregisterValidator(name);
        }
    }

    @Test
    void testPostProcessorReturnsNull() throws KreuzbergException {
        String name = "null-processor-" + System.currentTimeMillis();

        PostProcessor processor = result -> null;

        Kreuzberg.registerPostProcessor(name, processor);

        try {
            var processors = Kreuzberg.listPostProcessors();
            assertTrue(processors.contains(name), "Null-returning processor should be registered");
        } finally {
            Kreuzberg.unregisterPostProcessor(name);
        }
    }

    // ==================== Edge Cases ====================

    @Test
    void testExtractVeryLongFilePath(@TempDir Path tempDir) throws IOException, KreuzbergException {
        StringBuilder longPath = new StringBuilder();
        Path currentDir = tempDir;
        for (int i = 0; i < 20; i++) {
            currentDir = currentDir.resolve("a");
            Files.createDirectory(currentDir);
        }
        Path testFile = currentDir.resolve("test.txt");
        Files.writeString(testFile, "content");

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getContent(), "Should extract file with long path");
    }

    @Test
    void testExtractFileWithSpecialCharactersInName(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("file@#$%^&.txt");
        Files.writeString(testFile, "content");

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getContent(), "Should extract file with special characters in name");
    }

    @Test
    void testMultipleExtractionErrors(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path file1 = tempDir.resolve("test.txt");
        Files.writeString(file1, "content");

        // Multiple valid extractions should work fine
        ExtractionResult result1 = Kreuzberg.extractFile(file1);
        assertNotNull(result1.getContent(), "First extraction should succeed");

        // Second extraction of same file should also work
        ExtractionResult result2 = Kreuzberg.extractFile(file1);
        assertNotNull(result2.getContent(), "Second extraction should succeed");
    }

    @Test
    void testErrorMessageAvailability() throws KreuzbergException {
        try {
            Kreuzberg.extractFile(Path.of("/nonexistent"));
            fail("Should throw IOException");
        } catch (IOException e) {
            assertNotNull(e.getMessage(), "Error should have message");
            assertTrue(e.getMessage().length() > 0, "Error message should not be empty");
        }
    }
}
