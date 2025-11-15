package dev.kreuzberg;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for Kreuzberg Java bindings.
 */
class KreuzbergTest {
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
}
