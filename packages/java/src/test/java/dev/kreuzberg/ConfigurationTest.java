package dev.kreuzberg;

import dev.kreuzberg.config.ChunkingConfig;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.LanguageDetectionConfig;
import dev.kreuzberg.config.OcrConfig;
import dev.kreuzberg.config.PdfConfig;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Comprehensive configuration tests for extraction settings.
 *
 * Tests cover ExtractionConfig builder, OCR configuration, PDF configuration,
 * language detection configuration, chunking configuration, and configuration
 * serialization/deserialization.
 */
class ConfigurationTest {

    // ==================== ExtractionConfig Builder ====================

    @Test
    void testExtractionConfigBuilderDefaults() {
        ExtractionConfig config = ExtractionConfig.builder().build();

        assertNotNull(config, "Built config should not be null");
        assertTrue(config.isUseCache(), "Cache should be enabled by default");
        assertFalse(config.isEnableQualityProcessing(), "Quality processing should be disabled by default");
        assertFalse(config.isForceOcr(), "Force OCR should be disabled by default");
    }

    @Test
    void testExtractionConfigCacheSetting() {
        ExtractionConfig config = ExtractionConfig.builder()
                .useCache(false)
                .build();

        assertFalse(config.isUseCache(), "Cache setting should be respected");
    }

    @Test
    void testExtractionConfigQualityProcessing() {
        ExtractionConfig config = ExtractionConfig.builder()
                .enableQualityProcessing(true)
                .build();

        assertTrue(config.isEnableQualityProcessing(), "Quality processing setting should be respected");
    }

    @Test
    void testExtractionConfigForceOcr() {
        ExtractionConfig config = ExtractionConfig.builder()
                .forceOcr(true)
                .build();

        assertTrue(config.isForceOcr(), "Force OCR setting should be respected");
    }

    @Test
    void testExtractionConfigMultipleSettings() {
        ExtractionConfig config = ExtractionConfig.builder()
                .useCache(false)
                .enableQualityProcessing(true)
                .forceOcr(true)
                .build();

        assertFalse(config.isUseCache(), "Cache setting should be applied");
        assertTrue(config.isEnableQualityProcessing(), "Quality processing should be applied");
        assertTrue(config.isForceOcr(), "Force OCR should be applied");
    }

    // ==================== OCR Configuration ====================

    @Test
    void testOcrConfigBuilder() {
        OcrConfig config = OcrConfig.builder().build();

        assertNotNull(config, "OCR config should not be null");
    }

    @Test
    void testOcrConfigBackendSetting() {
        OcrConfig config = OcrConfig.builder()
                .backend("tesseract")
                .build();

        assertEquals("tesseract", config.getBackend(), "Backend should be set correctly");
    }

    @Test
    void testOcrConfigLanguageSetting() {
        OcrConfig config = OcrConfig.builder()
                .language("eng")
                .build();

        assertEquals("eng", config.getLanguage(), "Language should be set correctly");
    }

    @Test
    void testOcrConfigMultipleLanguages() {
        OcrConfig config = OcrConfig.builder()
                .language("eng+deu")
                .build();

        assertEquals("eng+deu", config.getLanguage(), "Multiple languages should be supported");
    }

    @Test
    void testOcrConfigIntegration() {
        ExtractionConfig config = ExtractionConfig.builder()
                .ocr(OcrConfig.builder()
                        .backend("tesseract")
                        .language("eng")
                        .build())
                .build();

        assertNotNull(config.getOcr(), "OCR config should be integrated");
        assertEquals("tesseract", config.getOcr().getBackend(), "OCR backend should be preserved");
    }

    // ==================== PDF Configuration ====================

    @Test
    void testPdfConfigBuilder() {
        PdfConfig config = PdfConfig.builder().build();

        assertNotNull(config, "PDF config should not be null");
    }

    @Test
    void testPdfConfigWithExtractionMethod() {
        PdfConfig config = PdfConfig.builder().build();

        assertNotNull(config, "PDF config should be created");
    }

    @Test
    void testPdfConfigIntegration() {
        ExtractionConfig config = ExtractionConfig.builder()
                .pdfOptions(PdfConfig.builder().build())
                .build();

        assertNotNull(config.getPdfOptions(), "PDF options should be integrated");
    }

    // ==================== Language Detection Configuration ====================

    @Test
    void testLanguageDetectionConfigBuilder() {
        LanguageDetectionConfig config = LanguageDetectionConfig.builder().build();

        assertNotNull(config, "Language detection config should not be null");
    }

    @Test
    void testLanguageDetectionIntegration() {
        ExtractionConfig config = ExtractionConfig.builder()
                .languageDetection(LanguageDetectionConfig.builder().build())
                .build();

        assertNotNull(config.getLanguageDetection(), "Language detection config should be integrated");
    }

    // ==================== Chunking Configuration ====================

    @Test
    void testChunkingConfigBuilder() {
        ChunkingConfig config = ChunkingConfig.builder().build();

        assertNotNull(config, "Chunking config should not be null");
    }

    @Test
    void testChunkingConfigMaxChars() {
        ChunkingConfig config = ChunkingConfig.builder()
                .maxChars(1024)
                .build();

        assertEquals(1024, config.getMaxChars(), "Max chars should be set correctly");
    }

    @Test
    void testChunkingConfigMaxOverlap() {
        ChunkingConfig config = ChunkingConfig.builder()
                .maxOverlap(100)
                .build();

        assertEquals(100, config.getMaxOverlap(), "Max overlap should be set correctly");
    }

    @Test
    void testChunkingConfigPreset() {
        ChunkingConfig config = ChunkingConfig.builder()
                .preset("default")
                .build();

        assertEquals("default", config.getPreset(), "Preset should be set correctly");
    }

    @Test
    void testChunkingConfigEnabled() {
        ChunkingConfig config = ChunkingConfig.builder()
                .enabled(true)
                .build();

        assertTrue(config.getEnabled(), "Enabled flag should be set correctly");
    }

    @Test
    void testChunkingConfigIntegration() {
        ExtractionConfig config = ExtractionConfig.builder()
                .chunking(ChunkingConfig.builder()
                        .maxChars(2048)
                        .maxOverlap(256)
                        .build())
                .build();

        assertNotNull(config.getChunking(), "Chunking config should be integrated");
        assertEquals(2048, config.getChunking().getMaxChars(), "Chunking settings should be preserved");
    }

    @Test
    void testChunkingConfigMultipleSettings() {
        ChunkingConfig config = ChunkingConfig.builder()
                .maxChars(1500)
                .maxOverlap(150)
                .preset("semantic")
                .enabled(true)
                .build();

        assertEquals(1500, config.getMaxChars(), "Max chars should be set");
        assertEquals(150, config.getMaxOverlap(), "Max overlap should be set");
        assertEquals("semantic", config.getPreset(), "Preset should be set");
        assertTrue(config.getEnabled(), "Enabled should be set");
    }

    // ==================== Configuration Serialization ====================

    @Test
    void testConfigToMap() {
        ExtractionConfig config = ExtractionConfig.builder()
                .useCache(false)
                .enableQualityProcessing(true)
                .build();

        Map<String, Object> map = config.toMap();

        assertNotNull(map, "Map should not be null");
        assertFalse((boolean) map.get("use_cache"), "Cache setting should be in map");
        assertTrue((boolean) map.get("enable_quality_processing"), "Quality processing should be in map");
    }

    @Test
    void testConfigToMapWithSubconfigs() {
        ExtractionConfig config = ExtractionConfig.builder()
                .ocr(OcrConfig.builder().backend("tesseract").build())
                .chunking(ChunkingConfig.builder().maxChars(512).build())
                .build();

        Map<String, Object> map = config.toMap();

        assertNotNull(map, "Map should not be null");
        assertTrue(map.containsKey("ocr"), "OCR config should be in map");
        assertTrue(map.containsKey("chunking"), "Chunking config should be in map");
    }

    // ==================== Max Concurrent Extractions ====================

    @Test
    void testMaxConcurrentExtractionsSetting() {
        ExtractionConfig config = ExtractionConfig.builder()
                .maxConcurrentExtractions(4)
                .build();

        assertEquals(4, config.getMaxConcurrentExtractions(), "Max concurrent extractions should be set");
    }

    @Test
    void testMaxConcurrentExtractionsWithOtherSettings() {
        ExtractionConfig config = ExtractionConfig.builder()
                .useCache(false)
                .maxConcurrentExtractions(8)
                .build();

        assertEquals(8, config.getMaxConcurrentExtractions(), "Max concurrent should be preserved");
        assertFalse(config.isUseCache(), "Other settings should be preserved");
    }

    // ==================== Configuration with File Operations ====================

    @Test
    void testExtractionWithConfiguration(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "Test content");

        ExtractionConfig config = ExtractionConfig.builder()
                .useCache(true)
                .build();

        ExtractionResult result = Kreuzberg.extractFile(testFile, config);

        assertNotNull(result, "Result should not be null with config");
        assertNotNull(result.getContent(), "Content should be extracted with config");
    }

    @Test
    void testExtractionWithOcrConfiguration(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("image_text.txt");
        Files.writeString(testFile, "Text file content");

        ExtractionConfig config = ExtractionConfig.builder()
                .forceOcr(false)
                .ocr(OcrConfig.builder()
                        .language("eng")
                        .build())
                .build();

        ExtractionResult result = Kreuzberg.extractFile(testFile, config);

        assertNotNull(result, "Result should not be null with OCR config");
    }

    @Test
    void testExtractionWithChunkingConfiguration(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("document.txt");
        StringBuilder content = new StringBuilder();
        for (int i = 0; i < 20; i++) {
            content.append("Section ").append(i).append(": Content.\n");
        }
        Files.writeString(testFile, content.toString());

        ExtractionConfig config = ExtractionConfig.builder()
                .chunking(ChunkingConfig.builder()
                        .maxChars(256)
                        .maxOverlap(50)
                        .enabled(true)
                        .build())
                .build();

        ExtractionResult result = Kreuzberg.extractFile(testFile, config);

        assertNotNull(result, "Result should not be null with chunking config");
    }

    // ==================== Configuration Immutability ====================

    @Test
    void testConfigBuilderImmutability() {
        var builder = ExtractionConfig.builder();
        ExtractionConfig config1 = builder.useCache(true).build();
        ExtractionConfig config2 = builder.useCache(false).build();

        // Both configs should reflect their respective builds
        assertTrue(config1.isUseCache(), "First config should have cache enabled");
        assertFalse(config2.isUseCache(), "Second config should have cache disabled");
    }

    // ==================== Configuration Defaults ====================

    @Test
    void testAllDefaults() {
        ExtractionConfig config = ExtractionConfig.builder().build();

        assertTrue(config.isUseCache(), "Cache should be true by default");
        assertFalse(config.isEnableQualityProcessing(), "Quality processing should be false by default");
        assertFalse(config.isForceOcr(), "Force OCR should be false by default");
        assertNull(config.getMaxConcurrentExtractions(), "Max concurrent should be null by default");
    }

    @Test
    void testConfigWithNullSubconfigs() {
        ExtractionConfig config = ExtractionConfig.builder()
                .ocr(null)
                .chunking(null)
                .languageDetection(null)
                .build();

        assertNull(config.getOcr(), "OCR should be null when not set");
        assertNull(config.getChunking(), "Chunking should be null when not set");
        assertNull(config.getLanguageDetection(), "Language detection should be null when not set");
    }

    // ==================== Byte Extraction with Configuration ====================

    @Test
    void testBytesExtractionWithConfiguration() throws KreuzbergException {
        byte[] data = "Test content for bytes".getBytes();

        ExtractionConfig config = ExtractionConfig.builder()
                .useCache(false)
                .build();

        ExtractionResult result = Kreuzberg.extractBytes(data, "text/plain", config);

        assertNotNull(result, "Result should not be null");
        assertNotNull(result.getContent(), "Content should be extracted");
    }

    @Test
    void testBytesExtractionWithOcrConfig() throws KreuzbergException {
        byte[] data = "Test data".getBytes();

        ExtractionConfig config = ExtractionConfig.builder()
                .forceOcr(true)
                .ocr(OcrConfig.builder().language("eng").build())
                .build();

        ExtractionResult result = Kreuzberg.extractBytes(data, "text/plain", config);

        assertNotNull(result, "Result with OCR config should not be null");
    }

    // ==================== Configuration Chaining ====================

    @Test
    void testComplexConfigurationChaining() {
        ExtractionConfig config = ExtractionConfig.builder()
                .useCache(false)
                .enableQualityProcessing(true)
                .forceOcr(false)
                .ocr(OcrConfig.builder()
                        .backend("tesseract")
                        .language("eng")
                        .build())
                .chunking(ChunkingConfig.builder()
                        .maxChars(1024)
                        .maxOverlap(128)
                        .enabled(true)
                        .build())
                .languageDetection(LanguageDetectionConfig.builder().build())
                .pdfOptions(PdfConfig.builder().build())
                .maxConcurrentExtractions(4)
                .build();

        assertFalse(config.isUseCache(), "Cache should be disabled");
        assertTrue(config.isEnableQualityProcessing(), "Quality processing should be enabled");
        assertFalse(config.isForceOcr(), "Force OCR should be disabled");
        assertNotNull(config.getOcr(), "OCR should be configured");
        assertNotNull(config.getChunking(), "Chunking should be configured");
        assertEquals(4, config.getMaxConcurrentExtractions(), "Max concurrent should be set");
    }

    @Test
    void testConfigurationIndependence() {
        ExtractionConfig config1 = ExtractionConfig.builder()
                .useCache(true)
                .build();

        ExtractionConfig config2 = ExtractionConfig.builder()
                .useCache(false)
                .build();

        assertTrue(config1.isUseCache(), "Config1 should have cache enabled");
        assertFalse(config2.isUseCache(), "Config2 should have cache disabled");
    }

    // ==================== Map Conversion Completeness ====================

    @Test
    void testMapConversionIncludesAllSettings() {
        ExtractionConfig config = ExtractionConfig.builder()
                .useCache(false)
                .enableQualityProcessing(true)
                .forceOcr(true)
                .maxConcurrentExtractions(5)
                .build();

        Map<String, Object> map = config.toMap();

        assertTrue(map.containsKey("use_cache"), "use_cache should be in map");
        assertTrue(map.containsKey("enable_quality_processing"), "enable_quality_processing should be in map");
        assertTrue(map.containsKey("force_ocr"), "force_ocr should be in map");
        assertTrue(map.containsKey("max_concurrent_extractions"), "max_concurrent_extractions should be in map");
    }

    @Test
    void testOcrConfigToMap() {
        OcrConfig config = OcrConfig.builder()
                .backend("tesseract")
                .language("eng")
                .build();

        Map<String, Object> map = config.toMap();

        assertNotNull(map, "OCR config map should not be null");
    }

    @Test
    void testChunkingConfigToMap() {
        ChunkingConfig config = ChunkingConfig.builder()
                .maxChars(2048)
                .maxOverlap(256)
                .enabled(true)
                .build();

        Map<String, Object> map = config.toMap();

        assertNotNull(map, "Chunking config map should not be null");
    }
}
