package com.kreuzberg.e2e;

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
 * E2E tests for plugin/config/utility APIs.
 *
 * <p>Generated from plugin API fixtures.
 * To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang java
 *
 * @since 4.0.0
 */
@DisplayName("Plugin API Tests")
class PluginAPIsTest {



    @Test
    @DisplayName("Load configuration from a TOML file")
    void configFromFile(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path configPath = tempDir.resolve("test_config.toml");
        Files.writeString(configPath, """
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
    @DisplayName("Clear all document extractors and verify list is empty")
    void extractorsClear() throws KreuzbergException {
        Kreuzberg.clearDocumentExtractors();
        List<String> result = Kreuzberg.listDocumentExtractors();
        assertEquals(0, result.size());
    }

    @Test
    @DisplayName("List all registered document extractors")
    void extractorsList() throws KreuzbergException {
        List<String> result = Kreuzberg.listDocumentExtractors();
        assertNotNull(result);
        assertTrue(result.stream().allMatch(item -> item instanceof String));
    }

    @Test
    @DisplayName("Unregister nonexistent document extractor gracefully")
    void extractorsUnregister() throws KreuzbergException {
        assertDoesNotThrow(() -> Kreuzberg.unregisterDocumentExtractor("nonexistent-extractor-xyz"));
    }


    @Test
    @DisplayName("Detect MIME type from file bytes")
    void mimeDetectBytes() throws KreuzbergException {
        byte[] testBytes = "%PDF-1.4\n".getBytes();
        String result = Kreuzberg.detectMimeType(testBytes);
        assertTrue(result.toLowerCase().contains("pdf"));
    }

    @Test
    @DisplayName("Detect MIME type from file path")
    void mimeDetectPath(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "Hello, world!");

        String result = Kreuzberg.detectMimeTypeFromPath(testFile.toString());
        assertTrue(result.toLowerCase().contains("text"));
    }

    @Test
    @DisplayName("Get file extensions for a MIME type")
    void mimeGetExtensions() throws KreuzbergException {
        List<String> result = Kreuzberg.getExtensionsForMime("application/pdf");
        assertNotNull(result);
        assertTrue(result.contains("pdf"));
    }


    @Test
    @DisplayName("Clear all OCR backends and verify list is empty")
    void ocrBackendsClear() throws KreuzbergException {
        Kreuzberg.clearOCRBackends();
        List<String> result = Kreuzberg.listOCRBackends();
        assertEquals(0, result.size());
    }

    @Test
    @DisplayName("List all registered OCR backends")
    void ocrBackendsList() throws KreuzbergException {
        List<String> result = Kreuzberg.listOCRBackends();
        assertNotNull(result);
        assertTrue(result.stream().allMatch(item -> item instanceof String));
    }

    @Test
    @DisplayName("Unregister nonexistent OCR backend gracefully")
    void ocrBackendsUnregister() throws KreuzbergException {
        assertDoesNotThrow(() -> Kreuzberg.unregisterOCRBackend("nonexistent-backend-xyz"));
    }


    @Test
    @DisplayName("Clear all post-processors and verify list is empty")
    void postProcessorsClear() throws KreuzbergException {
        Kreuzberg.clearPostProcessors();
        List<String> result = Kreuzberg.listPostProcessors();
        assertEquals(0, result.size());
    }

    @Test
    @DisplayName("List all registered post-processors")
    void postProcessorsList() throws KreuzbergException {
        List<String> result = Kreuzberg.listPostProcessors();
        assertNotNull(result);
        assertTrue(result.stream().allMatch(item -> item instanceof String));
    }


    @Test
    @DisplayName("Clear all validators and verify list is empty")
    void validatorsClear() throws KreuzbergException {
        Kreuzberg.clearValidators();
        List<String> result = Kreuzberg.listValidators();
        assertEquals(0, result.size());
    }

    @Test
    @DisplayName("List all registered validators")
    void validatorsList() throws KreuzbergException {
        List<String> result = Kreuzberg.listValidators();
        assertNotNull(result);
        assertTrue(result.stream().allMatch(item -> item instanceof String));
    }

}
