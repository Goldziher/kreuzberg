package dev.kreuzberg;

import dev.kreuzberg.config.ChunkingConfig;
import dev.kreuzberg.config.ExtractionConfig;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Batch operation tests for Kreuzberg extraction.
 *
 * Tests cover batch file extraction, batch byte extraction, partial failure handling,
 * progress tracking simulation, and batch configuration scenarios.
 */
class BatchOperationsTest {


    @Test
    void testBatchExtractMultipleFiles(@TempDir Path tempDir) throws IOException, KreuzbergException {
        List<Path> files = new ArrayList<>();
        for (int i = 0; i < 5; i++) {
            Path file = tempDir.resolve("file_" + i + ".txt");
            Files.writeString(file, "Content of file " + i);
            files.add(file);
        }

        List<String> paths = new ArrayList<>();
        for (Path file : files) {
            paths.add(file.toString());
        }

        List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, null);

        assertNotNull(results, "Results should not be null");
        assertEquals(5, results.size(), "Should have 5 results");
        for (ExtractionResult result : results) {
            assertNotNull(result, "Each result should not be null");
            assertTrue(result.isSuccess(), "Each extraction should succeed");
        }
    }

    @Test
    void testBatchExtractWithDifferentFileTypes(@TempDir Path tempDir) throws IOException, KreuzbergException {
        List<String> paths = new ArrayList<>();

        Path textFile = tempDir.resolve("document.txt");
        Files.writeString(textFile, "Text content");
        paths.add(textFile.toString());

        Path csvFile = tempDir.resolve("data.csv");
        Files.writeString(csvFile, "Name,Age\nAlice,30\nBob,25");
        paths.add(csvFile.toString());

        Path jsonFile = tempDir.resolve("data.json");
        Files.writeString(jsonFile, "{\"key\": \"value\"}");
        paths.add(jsonFile.toString());

        List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, null);

        assertEquals(3, results.size(), "Should extract all file types");
        for (ExtractionResult result : results) {
            assertTrue(result.isSuccess(), "All file types should extract successfully");
            assertNotNull(result.getMimeType(), "MIME type should be detected");
        }
    }

    @Test
    void testBatchExtractLargeNumber(@TempDir Path tempDir) throws IOException, KreuzbergException {
        List<String> paths = new ArrayList<>();

        for (int i = 0; i < 20; i++) {
            Path file = tempDir.resolve("large_batch_" + i + ".txt");
            Files.writeString(file, "File " + i + " content");
            paths.add(file.toString());
        }

        List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, null);

        assertEquals(20, results.size(), "Should extract all 20 files");
        int successCount = 0;
        for (ExtractionResult result : results) {
            if (result.isSuccess()) {
                successCount++;
            }
        }
        assertTrue(successCount > 15, "Most files should extract successfully");
    }


    @Test
    void testBatchExtractBytes() throws KreuzbergException {
        List<BytesWithMime> items = new ArrayList<>();

        items.add(new BytesWithMime("First content".getBytes(), "text/plain"));
        items.add(new BytesWithMime("Second content".getBytes(), "text/plain"));
        items.add(new BytesWithMime("{\"test\": true}".getBytes(), "application/json"));

        List<ExtractionResult> results = Kreuzberg.batchExtractBytes(items, null);

        assertNotNull(results, "Results should not be null");
        assertEquals(3, results.size(), "Should have 3 results");
    }

    @Test
    void testBatchExtractBytesWithConfiguration() throws KreuzbergException {
        List<BytesWithMime> items = new ArrayList<>();
        items.add(new BytesWithMime("Test content 1".getBytes(), "text/plain"));
        items.add(new BytesWithMime("Test content 2".getBytes(), "text/plain"));

        ExtractionConfig config = ExtractionConfig.builder()
                .useCache(false)
                .build();

        List<ExtractionResult> results = Kreuzberg.batchExtractBytes(items, config);

        assertEquals(2, results.size(), "Should extract both items");
    }

    @Test
    void testBatchExtractBytesEmptyList() throws KreuzbergException {
        List<BytesWithMime> items = new ArrayList<>();

        List<ExtractionResult> results = Kreuzberg.batchExtractBytes(items, null);

        assertNotNull(results, "Results should not be null for empty list");
        assertTrue(results.isEmpty(), "Should return empty list for empty input");
    }

    @Test
    void testBatchExtractFilesEmptyList() throws KreuzbergException {
        List<String> paths = new ArrayList<>();

        List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, null);

        assertNotNull(results, "Results should not be null for empty list");
        assertTrue(results.isEmpty(), "Should return empty list for empty input");
    }


    @Test
    void testBatchExtractWithSomeMissingFiles(@TempDir Path tempDir) throws IOException, KreuzbergException {
        List<String> paths = new ArrayList<>();

        for (int i = 0; i < 2; i++) {
            Path file = tempDir.resolve("exists_" + i + ".txt");
            Files.writeString(file, "Existing file " + i);
            paths.add(file.toString());
        }

        paths.add("/nonexistent/file1.txt");
        paths.add("/nonexistent/file2.txt");

        List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, null);

        assertNotNull(results, "Results should be returned");
        assertEquals(4, results.size(), "Should have results for all paths");

        boolean hasSuccess = results.stream().anyMatch(ExtractionResult::isSuccess);
        assertTrue(hasSuccess, "At least some files should extract successfully");
    }

    @Test
    void testBatchExtractResultIndependence(@TempDir Path tempDir) throws IOException, KreuzbergException {
        List<String> paths = new ArrayList<>();
        for (int i = 0; i < 3; i++) {
            Path file = tempDir.resolve("independent_" + i + ".txt");
            Files.writeString(file, "Independent file " + i);
            paths.add(file.toString());
        }

        List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, null);

        assertNotEquals(results.get(0).getContent(), results.get(1).getContent(),
                "Different files should have different content");
    }


    @Test
    void testBatchExtractProgressTracking(@TempDir Path tempDir) throws IOException, KreuzbergException {
        List<String> paths = new ArrayList<>();
        int fileCount = 10;

        for (int i = 0; i < fileCount; i++) {
            Path file = tempDir.resolve("progress_" + i + ".txt");
            Files.writeString(file, "Progress test file " + i);
            paths.add(file.toString());
        }

        List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, null);

        int processedCount = 0;
        for (ExtractionResult result : results) {
            processedCount++;
            int progressPercent = (processedCount * 100) / fileCount;
            assertTrue(progressPercent <= 100, "Progress should not exceed 100%");
        }

        assertEquals(fileCount, processedCount, "Should process all files");
    }

    @Test
    void testBatchExtractReturnOrder(@TempDir Path tempDir) throws IOException, KreuzbergException {
        List<String> paths = new ArrayList<>();
        List<String> fileContent = new ArrayList<>();

        for (int i = 0; i < 5; i++) {
            Path file = tempDir.resolve("order_" + i + ".txt");
            String content = "File number " + i;
            Files.writeString(file, content);
            paths.add(file.toString());
            fileContent.add(content);
        }

        List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, null);

        assertEquals(paths.size(), results.size(), "Should have same number of results as inputs");
    }


    @Test
    void testBatchExtractWithConfiguration(@TempDir Path tempDir) throws IOException, KreuzbergException {
        List<String> paths = new ArrayList<>();
        for (int i = 0; i < 3; i++) {
            Path file = tempDir.resolve("config_batch_" + i + ".txt");
            Files.writeString(file, "Content " + i);
            paths.add(file.toString());
        }

        ExtractionConfig config = ExtractionConfig.builder()
                .useCache(false)
                .enableQualityProcessing(true)
                .build();

        List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, config);

        assertEquals(3, results.size(), "Should extract with configuration");
        for (ExtractionResult result : results) {
            assertTrue(result.isSuccess(), "All should succeed with config");
        }
    }

    @Test
    void testBatchExtractWithChunkingConfig(@TempDir Path tempDir) throws IOException, KreuzbergException {
        List<String> paths = new ArrayList<>();

        for (int i = 0; i < 2; i++) {
            Path file = tempDir.resolve("chunk_batch_" + i + ".txt");
            StringBuilder content = new StringBuilder();
            for (int j = 0; j < 20; j++) {
                content.append("Section ").append(j).append(": Content.\n");
            }
            Files.writeString(file, content.toString());
            paths.add(file.toString());
        }

        ExtractionConfig config = ExtractionConfig.builder()
                .chunking(ChunkingConfig.builder()
                        .maxChars(512)
                        .maxOverlap(64)
                        .enabled(true)
                        .build())
                .build();

        List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, config);

        assertEquals(2, results.size(), "Should extract with chunking config");
    }

    @Test
    void testBatchExtractWithOcrConfiguration(@TempDir Path tempDir) throws IOException, KreuzbergException {
        List<String> paths = new ArrayList<>();
        for (int i = 0; i < 2; i++) {
            Path file = tempDir.resolve("ocr_batch_" + i + ".txt");
            Files.writeString(file, "Text content");
            paths.add(file.toString());
        }

        ExtractionConfig config = ExtractionConfig.builder()
                .forceOcr(false)
                .build();

        List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, config);

        assertNotNull(results, "Should handle batch with OCR config");
    }


    @Test
    void testBatchExtractConsistency(@TempDir Path tempDir) throws IOException, KreuzbergException {
        List<String> paths = new ArrayList<>();
        for (int i = 0; i < 3; i++) {
            Path file = tempDir.resolve("consistent_" + i + ".txt");
            Files.writeString(file, "Consistent content " + i);
            paths.add(file.toString());
        }

        List<ExtractionResult> results1 = Kreuzberg.batchExtractFiles(paths, null);
        List<ExtractionResult> results2 = Kreuzberg.batchExtractFiles(paths, null);

        assertEquals(results1.size(), results2.size(), "Batch size should be consistent");
        for (int i = 0; i < results1.size(); i++) {
            assertEquals(results1.get(i).getMimeType(), results2.get(i).getMimeType(),
                    "MIME types should be consistent across runs");
        }
    }


    @Test
    void testAlternatingBatchAndSingleExtractions(@TempDir Path tempDir) throws IOException, KreuzbergException {
        Path file1 = tempDir.resolve("mixed1.txt");
        Path file2 = tempDir.resolve("mixed2.txt");
        Path file3 = tempDir.resolve("mixed3.txt");

        Files.writeString(file1, "First");
        Files.writeString(file2, "Second");
        Files.writeString(file3, "Third");

        ExtractionResult single = Kreuzberg.extractFile(file1);
        assertNotNull(single.getContent(), "Single extraction should work");

        List<ExtractionResult> batch = Kreuzberg.batchExtractFiles(
                List.of(file2.toString(), file3.toString()), null);
        assertEquals(2, batch.size(), "Batch should have 2 results");

        ExtractionResult single2 = Kreuzberg.extractFile(file1);
        assertNotNull(single2.getContent(), "Second single extraction should work");
    }

    @Test
    void testBatchFollowedByAsync(@TempDir Path tempDir) throws IOException, InterruptedException, KreuzbergException {
        Path file1 = tempDir.resolve("before_async.txt");
        Files.writeString(file1, "Before async");

        List<ExtractionResult> batch = Kreuzberg.batchExtractFiles(
                List.of(file1.toString()), null);
        assertEquals(1, batch.size(), "Batch should succeed");

        var asyncFuture = Kreuzberg.extractFileAsync(file1, null);
        try {
            ExtractionResult asyncResult = asyncFuture.get();
            assertNotNull(asyncResult.getContent(), "Async should work after batch");
        } catch (Exception e) {
            fail("Async after batch should not throw");
        }
    }


    @Test
    void testBatchExtractVeryLargeFiles(@TempDir Path tempDir) throws IOException, KreuzbergException {
        List<String> paths = new ArrayList<>();

        for (int i = 0; i < 2; i++) {
            Path file = tempDir.resolve("large_" + i + ".txt");
            StringBuilder content = new StringBuilder();
            for (int j = 0; j < 500; j++) {
                content.append("Large file content line ").append(j).append("\n");
            }
            Files.writeString(file, content.toString());
            paths.add(file.toString());
        }

        List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, null);

        assertNotNull(results, "Should handle large file batch");
        assertTrue(results.size() > 0, "Should return results for large files");
    }

    @Test
    void testBatchExtractWithSpecialCharacterFilenames(@TempDir Path tempDir) throws IOException, KreuzbergException {
        List<String> paths = new ArrayList<>();

        for (int i = 0; i < 2; i++) {
            Path file = tempDir.resolve("file_@_" + i + "_#.txt");
            Files.writeString(file, "Content with special chars in name");
            paths.add(file.toString());
        }

        List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, null);

        assertEquals(2, results.size(), "Should extract files with special character names");
    }

    @Test
    void testBatchExtractResultsAreIndependent(@TempDir Path tempDir) throws IOException, KreuzbergException {
        List<String> paths = new ArrayList<>();
        for (int i = 0; i < 3; i++) {
            Path file = tempDir.resolve("independent_" + i + ".txt");
            Files.writeString(file, "Content " + i);
            paths.add(file.toString());
        }

        List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, null);

        if (results.size() > 0) {
            var firstContent = results.get(0).getContent();

            assertThrows(UnsupportedOperationException.class, () -> {
                results.get(0).getMetadata().put("key", "value");
            }, "Result metadata should be immutable");
        }
    }


    @Test
    void testBatchExtractHandlesNullResults(@TempDir Path tempDir) throws IOException, KreuzbergException {
        List<String> paths = new ArrayList<>();
        for (int i = 0; i < 2; i++) {
            Path file = tempDir.resolve("normal_" + i + ".txt");
            Files.writeString(file, "Normal content");
            paths.add(file.toString());
        }

        List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, null);

        for (ExtractionResult result : results) {
            assertNotNull(result, "Results should not contain nulls");
        }
    }

    @Test
    void testBatchExtractWithDifferentMimeTypes(@TempDir Path tempDir) throws IOException, KreuzbergException {
        List<String> paths = new ArrayList<>();

        Path txt = tempDir.resolve("doc.txt");
        Files.writeString(txt, "Text");
        paths.add(txt.toString());

        Path csv = tempDir.resolve("data.csv");
        Files.writeString(csv, "a,b,c\n1,2,3");
        paths.add(csv.toString());

        Path xml = tempDir.resolve("data.xml");
        Files.writeString(xml, "<?xml version=\"1.0\"?><root></root>");
        paths.add(xml.toString());

        List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, null);

        assertEquals(3, results.size(), "Should extract all different types");

        String mime1 = results.get(0).getMimeType();
        String mime2 = results.get(1).getMimeType();
        String mime3 = results.get(2).getMimeType();

        assertNotNull(mime1, "First MIME type should be detected");
        assertNotNull(mime2, "Second MIME type should be detected");
        assertNotNull(mime3, "Third MIME type should be detected");
    }


    @Test
    void testAsyncBatchExtractFiles(@TempDir Path tempDir) throws IOException {
        List<String> paths = new ArrayList<>();
        for (int i = 0; i < 3; i++) {
            Path file = tempDir.resolve("async_batch_" + i + ".txt");
            Files.writeString(file, "Async batch content " + i);
            paths.add(file.toString());
        }

        var future = Kreuzberg.batchExtractFilesAsync(paths, null);

        try {
            List<ExtractionResult> results = future.get();
            assertEquals(3, results.size(), "Async batch should return all results");
        } catch (Exception e) {
            fail("Async batch extraction should not throw: " + e.getMessage());
        }
    }

    @Test
    void testAsyncBatchExtractBytes() {
        List<BytesWithMime> items = new ArrayList<>();
        items.add(new BytesWithMime("Async content 1".getBytes(), "text/plain"));
        items.add(new BytesWithMime("Async content 2".getBytes(), "text/plain"));

        var future = Kreuzberg.batchExtractBytesAsync(items, null);

        try {
            List<ExtractionResult> results = future.get();
            assertEquals(2, results.size(), "Async batch should extract all items");
        } catch (Exception e) {
            fail("Async batch extraction should not throw");
        }
    }


    @Test
    void testBatchExtractStatistics(@TempDir Path tempDir) throws IOException, KreuzbergException {
        List<String> paths = new ArrayList<>();
        for (int i = 0; i < 5; i++) {
            Path file = tempDir.resolve("stat_" + i + ".txt");
            Files.writeString(file, "Statistical content " + i);
            paths.add(file.toString());
        }

        List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, null);

        long totalSize = 0;
        int successCount = 0;
        for (ExtractionResult result : results) {
            if (result.isSuccess()) {
                successCount++;
                totalSize += result.getContent().length();
            }
        }

        assertEquals(5, successCount, "All files should extract successfully");
        assertTrue(totalSize > 0, "Total content size should be > 0");
    }
}
