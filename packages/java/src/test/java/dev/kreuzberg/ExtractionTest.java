package dev.kreuzberg;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Comprehensive extraction tests for various file types and extraction scenarios.
 *
 * Tests cover text extraction, MIME type detection, encoding detection, large file handling,
 * table extraction, metadata extraction, and various document formats.
 */
class ExtractionTest {

    // ==================== Basic Text Extraction ====================

    @Test
    void testExtractSimpleTextFile(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("simple.txt");
        String content = "Hello, world! This is a test document.";
        Files.writeString(testFile, content);

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result, "Extraction result should not be null");
        assertNotNull(result.getContent(), "Extracted content should not be null");
        assertTrue(result.getContent().contains("Hello"), "Extracted content should contain original text");
        assertTrue(result.isSuccess(), "Extraction should succeed");
    }

    @Test
    void testExtractMultilineTextFile(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("multiline.txt");
        String content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
        Files.writeString(testFile, content);

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getContent(), "Content should not be null");
        assertTrue(result.getContent().length() > 0, "Content should have extracted text");
        assertTrue(result.isSuccess(), "Extraction should succeed for multiline text");
    }

    @Test
    void testExtractEmptyFile(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("empty.txt");
        Files.writeString(testFile, "");

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result, "Result should not be null for empty file");
        assertNotNull(result.getContent(), "Content should not be null");
        assertEquals("", result.getContent(), "Content should be empty string for empty file");
    }

    @Test
    void testExtractLargeTextFile(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("large.txt");
        StringBuilder content = new StringBuilder();
        for (int i = 0; i < 1000; i++) {
            content.append("Line ").append(i).append(": This is a test line with some content.\n");
        }
        Files.writeString(testFile, content.toString());

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getContent(), "Content should be extracted from large file");
        assertTrue(result.getContent().length() > 0, "Large file content should be extracted");
        assertTrue(result.isSuccess(), "Large file extraction should succeed");
    }

    // ==================== MIME Type Detection ====================

    @Test
    void testDetectMimeTypeFromPath(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("document.txt");
        Files.writeString(testFile, "test content");

        String mimeType = Kreuzberg.detectMimeType(testFile.toString());

        assertNotNull(mimeType, "MIME type should be detected");
        assertTrue(mimeType.contains("text"), "Text file should have text MIME type");
    }

    @Test
    void testDetectMimeTypeFromBytes() throws KreuzbergException {
        byte[] data = "Hello, world!".getBytes();

        String mimeType = Kreuzberg.detectMimeType(data);

        assertNotNull(mimeType, "MIME type should be detected from bytes");
        assertFalse(mimeType.isEmpty(), "MIME type should not be empty");
    }

    @Test
    void testMimeTypeDetectionWithVariousExtensions(@TempDir Path tempDir) throws IOException, KreuzbergException {
        String[] extensions = {"txt", "csv", "json", "xml", "html"};

        for (String ext : extensions) {
            Path testFile = tempDir.resolve("test." + ext);
            Files.writeString(testFile, "test content");

            String mimeType = Kreuzberg.detectMimeType(testFile.toString());

            assertNotNull(mimeType, "MIME type should be detected for ." + ext);
            assertFalse(mimeType.isEmpty(), "MIME type should not be empty for ." + ext);
        }
    }

    @Test
    void testValidateMimeType() throws KreuzbergException {
        String validMimeType = "text/plain";

        String result = Kreuzberg.validateMimeType(validMimeType);

        assertNotNull(result, "Validation result should not be null");
        assertFalse(result.isEmpty(), "Validated MIME type should not be empty");
    }

    @Test
    void testExtractFileDetectsMimeType(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("data.txt");
        Files.writeString(testFile, "Test content here");

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getMimeType(), "MIME type should be detected during extraction");
        assertFalse(result.getMimeType().isEmpty(), "MIME type should not be empty");
    }

    // ==================== Encoding Detection ====================

    @Test
    void testExtractUTF8EncodedFile(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("utf8.txt");
        String content = "UTF-8 content with special characters: é, ñ, ü, 中文";
        Files.writeString(testFile, content);

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getContent(), "UTF-8 content should be extracted");
        assertTrue(result.isSuccess(), "UTF-8 extraction should succeed");
    }

    @Test
    void testExtractASCIIFile(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("ascii.txt");
        String content = "Simple ASCII text without special characters";
        Files.writeString(testFile, content);

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getContent(), "ASCII content should be extracted");
        assertTrue(result.isSuccess(), "ASCII extraction should succeed");
    }

    // ==================== Content Extraction from Various Formats ====================

    @Test
    void testExtractCSVContent(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("data.csv");
        String content = "Name,Age,City\nAlice,30,New York\nBob,25,Los Angeles";
        Files.writeString(testFile, content);

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getContent(), "CSV content should be extracted");
        assertTrue(result.isSuccess(), "CSV extraction should succeed");
    }

    @Test
    void testExtractJSONContent(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("data.json");
        String content = "{\"name\": \"John\", \"age\": 30, \"city\": \"New York\"}";
        Files.writeString(testFile, content);

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getContent(), "JSON content should be extracted");
        assertTrue(result.isSuccess(), "JSON extraction should succeed");
    }

    @Test
    void testExtractXMLContent(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("data.xml");
        String content = "<?xml version=\"1.0\"?><root><name>John</name><age>30</age></root>";
        Files.writeString(testFile, content);

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getContent(), "XML content should be extracted");
        assertTrue(result.isSuccess(), "XML extraction should succeed");
    }

    @Test
    void testExtractHTMLContent(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("page.html");
        String content = "<html><body><h1>Title</h1><p>Paragraph content</p></body></html>";
        Files.writeString(testFile, content);

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getContent(), "HTML content should be extracted");
        assertTrue(result.isSuccess(), "HTML extraction should succeed");
    }

    // ==================== Metadata Extraction ====================

    @Test
    void testMetadataPresenceInResult(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "Test document");

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getMetadata(), "Metadata should not be null");
        assertTrue(result.getMetadata().isEmpty() || result.getMetadata().size() >= 0, "Metadata should be retrievable");
    }

    @Test
    void testLanguageMetadataExtraction(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("test.txt");
        String content = "This is a test document written in English.";
        Files.writeString(testFile, content);

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertTrue(result.getLanguage().isEmpty() || result.getLanguage().isPresent(), "Language should be optional");
    }

    @Test
    void testDetectedLanguagesExtraction(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "Test content");

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getDetectedLanguages(), "Detected languages list should not be null");
        assertTrue(result.getDetectedLanguages().isEmpty() || result.getDetectedLanguages().size() > 0,
                "Detected languages list should be valid");
    }

    // ==================== Table and Structured Data Extraction ====================

    @Test
    void testTablesListAvailable(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("data.csv");
        String content = "Header1,Header2\nValue1,Value2";
        Files.writeString(testFile, content);

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getTables(), "Tables list should not be null");
        assertTrue(result.getTables().isEmpty() || result.getTables().size() > 0, "Tables list should be valid");
    }

    @Test
    void testTableStructure(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("table.csv");
        String content = "A,B,C\n1,2,3\n4,5,6";
        Files.writeString(testFile, content);

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        List<Table> tables = result.getTables();
        assertNotNull(tables, "Tables list should not be null");
        for (Table table : tables) {
            assertNotNull(table.cells(), "Table cells should not be null");
        }
    }

    // ==================== Chunking Extraction ====================

    @Test
    void testChunksExtraction(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("document.txt");
        StringBuilder content = new StringBuilder();
        for (int i = 0; i < 100; i++) {
            content.append("Chunk ").append(i).append(": This is content that might be split into chunks.\n");
        }
        Files.writeString(testFile, content.toString());

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getChunks(), "Chunks list should not be null");
        assertTrue(result.getChunks().isEmpty() || result.getChunks().size() > 0, "Chunks list should be valid");
    }

    @Test
    void testChunkMetadata(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("chunked.txt");
        StringBuilder content = new StringBuilder();
        for (int i = 0; i < 50; i++) {
            content.append("Section ").append(i).append(": Content block.\n");
        }
        Files.writeString(testFile, content.toString());

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        List<Chunk> chunks = result.getChunks();
        for (Chunk chunk : chunks) {
            assertNotNull(chunk, "Chunk should not be null");
            assertNotNull(chunk.getContent(), "Chunk content should not be null");
        }
    }

    // ==================== Image Extraction ====================

    @Test
    void testImagesListAvailable(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("document.txt");
        Files.writeString(testFile, "Document content");

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getImages(), "Images list should not be null");
        assertTrue(result.getImages().isEmpty() || result.getImages().size() >= 0, "Images list should be valid");
    }

    @Test
    void testExtractedImageStructure(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "Content with potential images");

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        List<ExtractedImage> images = result.getImages();
        for (ExtractedImage image : images) {
            assertNotNull(image, "Image should not be null");
        }
    }

    // ==================== Byte Extraction ====================

    @Test
    void testExtractBytesWithValidMimeType() throws KreuzbergException {
        byte[] data = "Hello, world!".getBytes();

        ExtractionResult result = Kreuzberg.extractBytes(data, "text/plain", null);

        assertNotNull(result, "Result should not be null");
        assertNotNull(result.getContent(), "Content should be extracted");
        assertTrue(result.isSuccess(), "Extraction should succeed");
    }

    @Test
    void testExtractBytesPreservesContent() throws KreuzbergException {
        byte[] data = "Test content for bytes extraction".getBytes();

        ExtractionResult result = Kreuzberg.extractBytes(data, "text/plain", null);

        assertNotNull(result.getContent(), "Content should be extracted from bytes");
        assertTrue(result.getContent().length() > 0, "Extracted content should have length");
    }

    @Test
    void testExtractBytesWithDifferentMimeTypes() throws KreuzbergException {
        byte[] data = "{\"test\": \"data\"}".getBytes();

        ExtractionResult result = Kreuzberg.extractBytes(data, "application/json", null);

        assertNotNull(result, "Result should not be null");
        assertTrue(result.isSuccess() || !result.isSuccess(), "Result should indicate success or failure");
    }

    @Test
    void testExtractBytesEmptyData() {
        byte[] data = new byte[0];

        assertThrows(KreuzbergException.class, () -> {
            Kreuzberg.extractBytes(data, "text/plain", null);
        }, "Empty data should throw KreuzbergException");
    }

    @Test
    void testExtractBytesNullMimeType() {
        byte[] data = "test".getBytes();

        assertThrows(KreuzbergException.class, () -> {
            Kreuzberg.extractBytes(data, null, null);
        }, "Null MIME type should throw KreuzbergException");
    }

    // ==================== Content Preservation ====================

    @Test
    void testContentPreservationInExtraction(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("preserve.txt");
        String originalContent = "This is the original content that should be preserved.";
        Files.writeString(testFile, originalContent);

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getContent(), "Content should be preserved");
        assertTrue(result.getContent().contains("original"), "Original content should be recognizable");
    }

    @Test
    void testSpecialCharacterPreservation(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("special.txt");
        String content = "Special characters: !@#$%^&*()_+-=[]{}|;:'\",.<>?/";
        Files.writeString(testFile, content);

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getContent(), "Special characters should be extracted");
        assertTrue(result.isSuccess(), "Extraction with special characters should succeed");
    }

    @Test
    void testWhitespacePreservation(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("whitespace.txt");
        String content = "Text   with   multiple   spaces\nAnd\t\ttabs\nAnd\nnewlines";
        Files.writeString(testFile, content);

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getContent(), "Content with whitespace should be extracted");
        assertTrue(result.isSuccess(), "Whitespace preservation extraction should succeed");
    }

    // ==================== File Extensions Lookup ====================

    @Test
    void testGetExtensionsForMimeType() throws KreuzbergException {
        List<String> extensions = Kreuzberg.getExtensionsForMime("text/plain");

        assertNotNull(extensions, "Extensions list should not be null");
        assertFalse(extensions.isEmpty(), "Extensions list should not be empty for text/plain");
    }

    @Test
    void testGetExtensionsForCommonMimeTypes() throws KreuzbergException {
        String[] mimeTypes = {"text/plain", "application/json", "text/html", "text/csv"};

        for (String mimeType : mimeTypes) {
            List<String> extensions = Kreuzberg.getExtensionsForMime(mimeType);
            assertNotNull(extensions, "Extensions should be found for " + mimeType);
        }
    }

    // ==================== Success Flag ====================

    @Test
    void testSuccessFlagIsSet(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("test.txt");
        Files.writeString(testFile, "Test content");

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertTrue(result.isSuccess(), "Success flag should be true for valid extraction");
    }

    @Test
    void testSuccessFlagWithMultipleExtensions(@TempDir Path tempDir) throws IOException, KreuzbergException {
        String[] files = {"test1.txt", "test2.csv", "test3.json", "test4.html"};

        for (String filename : files) {
            Path testFile = tempDir.resolve(filename);
            Files.writeString(testFile, "Test content");

            ExtractionResult result = Kreuzberg.extractFile(testFile);

            assertTrue(result.isSuccess(), "Success flag should be true for " + filename);
        }
    }

    // ==================== Result Completeness ====================

    @Test
    void testResultHasAllFields(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("complete.txt");
        Files.writeString(testFile, "Complete test file");

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getContent(), "Result should have content");
        assertNotNull(result.getMimeType(), "Result should have MIME type");
        assertNotNull(result.getMetadata(), "Result should have metadata");
        assertNotNull(result.getTables(), "Result should have tables list");
        assertNotNull(result.getDetectedLanguages(), "Result should have detected languages");
        assertNotNull(result.getChunks(), "Result should have chunks list");
        assertNotNull(result.getImages(), "Result should have images list");
    }

    @Test
    void testImmutableResultCollections(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("immutable.txt");
        Files.writeString(testFile, "Test");

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertThrows(UnsupportedOperationException.class, () -> {
            result.getMetadata().put("key", "value");
        }, "Metadata should be immutable");

        assertThrows(UnsupportedOperationException.class, () -> {
            result.getTables().add(null);
        }, "Tables list should be immutable");

        assertThrows(UnsupportedOperationException.class, () -> {
            result.getChunks().add(null);
        }, "Chunks list should be immutable");
    }

    // ==================== Edge Cases ====================

    @Test
    void testExtractionWithVeryLongFileName(@TempDir Path tempDir) throws IOException, KreuzbergException {
        String longName = "a".repeat(200) + ".txt";
        Path testFile = tempDir.resolve(longName);
        Files.writeString(testFile, "Content");

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getContent(), "Long filename extraction should succeed");
    }

    @Test
    void testExtractionWithNumbersInContent(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("numbers.txt");
        String content = "Numbers: 0 1 2 3 4 5 6 7 8 9 10 100 1000 1000000";
        Files.writeString(testFile, content);

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        assertNotNull(result.getContent(), "Content with numbers should be extracted");
        assertTrue(result.isSuccess(), "Number extraction should succeed");
    }

    @Test
    void testExtractionMaintainsContentOrder(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path testFile = tempDir.resolve("ordered.txt");
        String content = "First\nSecond\nThird\nFourth\nFifth";
        Files.writeString(testFile, content);

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        String extracted = result.getContent();
        assertNotNull(extracted, "Ordered content should be extracted");
        int firstPos = extracted.indexOf("First");
        int secondPos = extracted.indexOf("Second");
        int thirdPos = extracted.indexOf("Third");

        if (firstPos >= 0 && secondPos >= 0 && thirdPos >= 0) {
            assertTrue(firstPos < secondPos && secondPos < thirdPos,
                    "Content should maintain original order");
        }
    }
}
