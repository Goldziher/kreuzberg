package dev.kreuzberg.e2e;

import static org.junit.jupiter.api.Assertions.*;

import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.KreuzbergException;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

/**
 * Auto-generated plugin API tests for Java binding.
 *
 * <p>E2E tests for plugin registration and management APIs.
 *
 * <p>Tests all plugin types: - Validators - Post-processors - OCR backends - Document extractors
 *
 * <p>Tests all management operations: - Registration - Unregistration - Listing - Clearing
 */
@DisplayName("Plugin APIs E2E Tests")
class PluginAPIsTest {

    @Test
    @DisplayName("List all registered validators")
    void testListValidators() throws KreuzbergException {
        List<String> validators = Kreuzberg.listValidators();
        assertNotNull(validators);
        assertTrue(validators.stream().allMatch(v -> v instanceof String));
    }

    @Test
    @DisplayName("Clear all validators")
    void testClearValidators() throws KreuzbergException {
        Kreuzberg.clearValidators();
        List<String> validators = Kreuzberg.listValidators();
        assertEquals(0, validators.size());
    }

    @Test
    @DisplayName("List all registered post-processors")
    void testListPostProcessors() throws KreuzbergException {
        List<String> processors = Kreuzberg.listPostProcessors();
        assertNotNull(processors);
        assertTrue(processors.stream().allMatch(p -> p instanceof String));
    }

    @Test
    @DisplayName("Clear all post-processors")
    void testClearPostProcessors() throws KreuzbergException {
        Kreuzberg.clearPostProcessors();
        List<String> processors = Kreuzberg.listPostProcessors();
        assertEquals(0, processors.size());
    }

    @Test
    @DisplayName("List all registered OCR backends")
    void testListOCRBackends() throws KreuzbergException {
        List<String> backends = Kreuzberg.listOCRBackends();
        assertNotNull(backends);
        assertTrue(backends.stream().allMatch(b -> b instanceof String));
        // Should include built-in backends
        assertTrue(backends.contains("tesseract"));
    }

    @Test
    @DisplayName("Unregister an OCR backend")
    void testUnregisterOCRBackend() throws KreuzbergException {
        // Should handle nonexistent backend gracefully
        assertDoesNotThrow(() -> Kreuzberg.unregisterOCRBackend("nonexistent-backend-xyz"));
    }

    @Test
    @DisplayName("Clear all OCR backends")
    void testClearOCRBackends() throws KreuzbergException {
        Kreuzberg.clearOCRBackends();
        List<String> backends = Kreuzberg.listOCRBackends();
        assertEquals(0, backends.size());
    }

    @Test
    @DisplayName("List all registered document extractors")
    void testListDocumentExtractors() throws KreuzbergException {
        List<String> extractors = Kreuzberg.listDocumentExtractors();
        assertNotNull(extractors);
        assertTrue(extractors.stream().allMatch(e -> e instanceof String));
        // Should include built-in extractors
        assertTrue(extractors.stream().anyMatch(e -> e.toLowerCase().contains("pdf")));
    }

    @Test
    @DisplayName("Unregister a document extractor")
    void testUnregisterDocumentExtractor() throws KreuzbergException {
        // Should handle nonexistent extractor gracefully
        assertDoesNotThrow(
                () -> Kreuzberg.unregisterDocumentExtractor("nonexistent-extractor-xyz"));
    }

    @Test
    @DisplayName("Clear all document extractors")
    void testClearDocumentExtractors() throws KreuzbergException {
        Kreuzberg.clearDocumentExtractors();
        List<String> extractors = Kreuzberg.listDocumentExtractors();
        assertEquals(0, extractors.size());
    }

    @Test
    @DisplayName("Load configuration from a TOML file")
    void testConfigFromFile(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path configPath = tempDir.resolve("test_config.toml");
        Files.writeString(
                configPath,
                """
[chunking]
max_chars = 100
max_overlap = 20

[language_detection]
enabled = false
""");

        ExtractionConfig config = ExtractionConfig.fromFile(configPath.toString());
        assertNotNull(config.getChunking());
        assertEquals(100, config.getChunking().getMaxChars());
        assertEquals(20, config.getChunking().getMaxOverlap());
        assertNotNull(config.getLanguageDetection());
        assertFalse(config.getLanguageDetection().isEnabled());
    }

    @Test
    @DisplayName("Discover configuration from current or parent directories")
    void testConfigDiscover(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path configPath = tempDir.resolve("kreuzberg.toml");
        Files.writeString(
                configPath,
                """
[chunking]
max_chars = 50
""");

        // Create subdirectory
        Path subDir = tempDir.resolve("subdir");
        Files.createDirectories(subDir);

        String originalDir = System.getProperty("user.dir");
        try {
            System.setProperty("user.dir", subDir.toString());
            ExtractionConfig config = ExtractionConfig.discover();
            assertNotNull(config);
            assertNotNull(config.getChunking());
            assertEquals(50, config.getChunking().getMaxChars());
        } finally {
            System.setProperty("user.dir", originalDir);
        }
    }

    @Test
    @DisplayName("Detect MIME type from bytes")
    void testDetectMimeFromBytes() throws KreuzbergException {
        // PDF magic bytes
        byte[] pdfBytes = "%PDF-1.4\n".getBytes();
        String mimeType = Kreuzberg.detectMimeType(pdfBytes);
        assertTrue(mimeType.toLowerCase().contains("pdf"));
    }

    @Test
    @DisplayName("Detect MIME type from file path")
    void testDetectMimeFromPath(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "Hello, world!");

        String mimeType = Kreuzberg.detectMimeTypeFromPath(testFile.toString());
        assertTrue(mimeType.toLowerCase().contains("text"));
    }

    @Test
    @DisplayName("Get file extensions for a MIME type")
    void testGetExtensionsForMime() throws KreuzbergException {
        List<String> extensions = Kreuzberg.getExtensionsForMime("application/pdf");
        assertNotNull(extensions);
        assertTrue(extensions.contains("pdf"));
    }
}
