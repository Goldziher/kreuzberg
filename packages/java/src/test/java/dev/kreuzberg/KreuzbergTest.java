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
        ExtractionResult result = Kreuzberg.extractFile(testFile);

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
            Kreuzberg.extractFile(nonexistent);
        }, "Should throw IOException for nonexistent file");
    }

    @Test
    void testExtractionResultToString(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "Test content");

        ExtractionResult result = Kreuzberg.extractFile(testFile);
        String str = result.toString();

        assertNotNull(str, "toString should not return null");
        assertTrue(str.contains("ExtractionResult"), "toString should contain class name");
        assertTrue(str.contains("mimeType"), "toString should contain field names");
    }

    @Test
    void testExtractionResultFields(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "Test");

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        // Required fields
        assertNotNull(result.content());
        assertNotNull(result.mimeType());

        // Optional fields (just verify they're Optional)
        assertNotNull(result.language());
        assertNotNull(result.date());
        assertNotNull(result.subject());
    }

    @Test
    void testPostProcessor(@TempDir Path tempDir) throws IOException, KreuzbergException {
        // Create test file
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "hello world");

        // Create uppercase post-processor
        PostProcessor uppercaseProcessor = result ->
            result.withContent(result.content().toUpperCase());

        // Extract with post-processor
        ExtractionResult result = Kreuzberg.extractFile(testFile, uppercaseProcessor);

        // Verify content was transformed
        assertTrue(result.content().contains("HELLO"), "Content should be uppercased");
        assertFalse(result.content().contains("hello"), "Original content should be gone");
    }

    @Test
    void testMultiplePostProcessors(@TempDir Path tempDir) throws IOException, KreuzbergException {
        // Create test file
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "test");

        // Create chained processors
        PostProcessor uppercase = result -> result.withContent(result.content().toUpperCase());
        PostProcessor addPrefix = result -> result.withContent("PREFIX: " + result.content());

        // Extract with multiple processors
        ExtractionResult result = Kreuzberg.extractFile(testFile, uppercase, addPrefix);

        // Verify both transformations applied
        assertTrue(result.content().startsWith("PREFIX: "), "Should have prefix");
        assertTrue(result.content().contains("TEST"), "Should be uppercased");
    }

    @Test
    void testValidator(@TempDir Path tempDir) throws IOException, KreuzbergException {
        // Create test file with sufficient content
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "This is a long enough content");

        // Create minimum length validator
        Validator minLengthValidator = result -> {
            if (result.content().length() < 10) {
                throw new ValidationException("Content too short: " + result.content().length());
            }
        };

        // Should pass validation
        assertDoesNotThrow(() -> {
            ExtractionResult result = Kreuzberg.extractFileWithValidation(
                testFile,
                null,
                java.util.List.of(minLengthValidator)
            );
            assertNotNull(result);
        });
    }

    @Test
    void testValidatorFailure(@TempDir Path tempDir) throws IOException {
        // Create test file with short content
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "short");

        // Create minimum length validator
        Validator minLengthValidator = result -> {
            if (result.content().length() < 100) {
                throw new ValidationException("Content too short: " + result.content().length());
            }
        };

        // Should fail validation
        ValidationException exception = assertThrows(ValidationException.class, () -> {
            Kreuzberg.extractFileWithValidation(
                testFile,
                null,
                java.util.List.of(minLengthValidator)
            );
        });

        assertTrue(exception.getMessage().contains("too short"), "Error message should explain failure");
    }

    @Test
    void testPostProcessorAndValidator(@TempDir Path tempDir) throws IOException, KreuzbergException {
        // Create test file
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "content");

        // Create processor and validator
        PostProcessor addMetadata = result -> result.withSubject("Processed");
        Validator requireSubject = result -> {
            if (result.subject().isEmpty()) {
                throw new ValidationException("Subject is required");
            }
        };

        // Extract with both processor and validator
        ExtractionResult result = Kreuzberg.extractFileWithValidation(
            testFile,
            java.util.List.of(addMetadata),
            java.util.List.of(requireSubject)
        );

        // Verify processor ran and validator passed
        assertTrue(result.subject().isPresent(), "Subject should be present");
        assertEquals("Processed", result.subject().get(), "Subject should match");
    }

    @Test
    void testExtractionResultWithMethods(@TempDir Path tempDir) throws IOException, KreuzbergException {
        // Create test file
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "original");

        ExtractionResult original = Kreuzberg.extractFile(testFile);

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
