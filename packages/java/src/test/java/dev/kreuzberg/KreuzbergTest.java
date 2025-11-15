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
}
