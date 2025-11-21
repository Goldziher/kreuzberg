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
        assertNotNull(result.getContent(), "Content should not be null");
        assertTrue(result.getContent().contains("Hello"), "Content should contain test text");
        assertNotNull(result.getMimeType(), "MIME type should not be null");
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
        assertNotNull(result.getContent());
        assertNotNull(result.getMimeType());

        // Optional fields (just verify they're Optional)
        assertNotNull(result.getLanguage());
        assertNotNull(result.getDate());
        assertNotNull(result.getSubject());
    }

    @Test
    void testListValidators() throws KreuzbergException {
        var validators = Kreuzberg.listValidators();
        assertNotNull(validators, "Validators list should not be null");
    }

    @Test
    void testListPostProcessors() throws KreuzbergException {
        var postProcessors = Kreuzberg.listPostProcessors();
        assertNotNull(postProcessors, "PostProcessors list should not be null");
    }

    @Test
    void testValidatorLifecycle() throws KreuzbergException {
        String name = "test-validator-" + System.currentTimeMillis();

        Validator validator = result -> {
            if (result.getContent().isEmpty()) {
                throw new ValidationException("Content cannot be empty");
            }
        };

        Kreuzberg.registerValidator(name, validator);
        var validators = Kreuzberg.listValidators();
        assertTrue(validators.contains(name), "Validator should be in the list after registration");

        Kreuzberg.unregisterValidator(name);
        validators = Kreuzberg.listValidators();
        assertFalse(validators.contains(name), "Validator should not be in the list after unregistration");
    }

    @Test
    void testPostProcessorLifecycle() throws KreuzbergException {
        String name = "test-processor-" + System.currentTimeMillis();

        PostProcessor processor = result -> {
            return new ExtractionResult(
                result.getContent().toUpperCase(),
                result.getMimeType(),
                result.getMetadata(),
                result.getTables(),
                result.getDetectedLanguages(),
                result.getChunks(),
                result.getImages(),
                result.isSuccess()
            );
        };

        Kreuzberg.registerPostProcessor(name, processor);
        var processors = Kreuzberg.listPostProcessors();
        assertTrue(processors.contains(name), "PostProcessor should be in the list after registration");

        Kreuzberg.unregisterPostProcessor(name);
        processors = Kreuzberg.listPostProcessors();
        assertFalse(processors.contains(name), "PostProcessor should not be in the list after unregistration");
    }

    @Test
    void testClearValidators() throws KreuzbergException {
        String name = "test-validator-clear-" + System.currentTimeMillis();

        Validator validator = result -> { };

        Kreuzberg.registerValidator(name, validator);
        Kreuzberg.clearValidators();
        var validators = Kreuzberg.listValidators();
        assertFalse(validators.contains(name), "Validators should be cleared");
    }

    @Test
    void testClearPostProcessors() throws KreuzbergException {
        String name = "test-processor-clear-" + System.currentTimeMillis();

        PostProcessor processor = result -> result;

        Kreuzberg.registerPostProcessor(name, processor);
        Kreuzberg.clearPostProcessors();
        var processors = Kreuzberg.listPostProcessors();
        assertFalse(processors.contains(name), "PostProcessors should be cleared");
    }

    @Test
    void testListOCRBackends() throws KreuzbergException {
        var backends = Kreuzberg.listOCRBackends();
        assertNotNull(backends, "OCR backends list should not be null");
    }

    @Test
    void testUnregisterOCRBackend() throws KreuzbergException {
        assertDoesNotThrow(() -> Kreuzberg.unregisterOCRBackend("nonexistent"),
            "Unregistering nonexistent OCR backend should not throw");
    }
}
