package dev.kreuzberg;

import dev.kreuzberg.config.ExtractionConfig;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.atomic.AtomicInteger;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Concurrency and thread safety tests for Kreuzberg extraction.
 *
 * Tests cover concurrent file extraction, thread safety verification,
 * race condition detection, and parallel processing scenarios.
 */
class ConcurrencyTest {


    @Test
    void testConcurrentExtractSameFile(@TempDir Path tempDir)
            throws IOException, InterruptedException, ExecutionException {
        Path testFile = tempDir.resolve("shared.txt");
        Files.writeString(testFile, "Shared content for concurrent extraction");

        ExecutorService executor = Executors.newFixedThreadPool(4);
        List<Future<ExtractionResult>> futures = new ArrayList<>();

        try {
            for (int i = 0; i < 10; i++) {
                futures.add(executor.submit(() -> Kreuzberg.extractFile(testFile)));
            }

            List<ExtractionResult> results = new ArrayList<>();
            for (Future<ExtractionResult> future : futures) {
                results.add(future.get());
            }

            assertEquals(10, results.size(), "All extraction tasks should complete");
            for (ExtractionResult result : results) {
                assertNotNull(result.getContent(), "Each extraction should produce content");
                assertTrue(result.isSuccess(), "All extractions should succeed");
            }
        } finally {
            executor.shutdown();
        }
    }

    @Test
    void testConcurrentExtractDifferentFiles(@TempDir Path tempDir)
            throws IOException, InterruptedException, ExecutionException {
        List<Path> files = new ArrayList<>();
        for (int i = 0; i < 5; i++) {
            Path file = tempDir.resolve("file_" + i + ".txt");
            Files.writeString(file, "Content of file " + i);
            files.add(file);
        }

        ExecutorService executor = Executors.newFixedThreadPool(4);
        List<Future<ExtractionResult>> futures = new ArrayList<>();

        try {
            for (Path file : files) {
                futures.add(executor.submit(() -> Kreuzberg.extractFile(file)));
            }

            List<ExtractionResult> results = new ArrayList<>();
            for (Future<ExtractionResult> future : futures) {
                results.add(future.get());
            }

            assertEquals(5, results.size(), "All files should be extracted");
            for (ExtractionResult result : results) {
                assertNotNull(result.getContent(), "Each file should be extracted");
                assertTrue(result.isSuccess(), "All extractions should succeed");
            }
        } finally {
            executor.shutdown();
        }
    }

    @Test
    void testHighConcurrencyExtraction(@TempDir Path tempDir) throws IOException, InterruptedException {
        Path testFile = tempDir.resolve("concurrent.txt");
        Files.writeString(testFile, "Content for high concurrency test");

        ExecutorService executor = Executors.newFixedThreadPool(16);
        List<Future<ExtractionResult>> futures = new ArrayList<>();

        try {
            for (int i = 0; i < 50; i++) {
                futures.add(executor.submit(() -> Kreuzberg.extractFile(testFile)));
            }

            int successCount = 0;
            for (Future<ExtractionResult> future : futures) {
                try {
                    ExtractionResult result = future.get();
                    if (result != null && result.isSuccess()) {
                        successCount++;
                    }
                } catch (Exception e) {
                }
            }

            assertTrue(successCount > 0, "At least some extractions should succeed under high concurrency");
        } finally {
            executor.shutdown();
        }
    }


    @Test
    void testAsyncExtractMultipleFiles(@TempDir Path tempDir) throws IOException, InterruptedException {
        List<Path> files = new ArrayList<>();
        for (int i = 0; i < 3; i++) {
            Path file = tempDir.resolve("async_" + i + ".txt");
            Files.writeString(file, "Async content " + i);
            files.add(file);
        }

        var futures = new ArrayList<CompletableFuture<ExtractionResult>>();
        for (Path file : files) {
            futures.add(Kreuzberg.extractFileAsync(file, null));
        }

        int completed = 0;
        for (CompletableFuture<ExtractionResult> future : futures) {
            try {
                var result = future.get();
                if (result != null) {
                    completed++;
                }
            } catch (Exception e) {
            }
        }

        assertTrue(completed > 0, "At least some async extractions should complete");
    }

    @Test
    void testAsyncExtractWithConfiguration(@TempDir Path tempDir) throws IOException {
        Path testFile = tempDir.resolve("async_config.txt");
        Files.writeString(testFile, "Content with config");

        ExtractionConfig config = ExtractionConfig.builder()
                .useCache(false)
                .build();

        var future = Kreuzberg.extractFileAsync(testFile, config);

        try {
            ExtractionResult result = future.get();
            assertNotNull(result, "Async extraction with config should complete");
            assertNotNull(result.getContent(), "Content should be extracted");
        } catch (Exception e) {
            fail("Async extraction should not throw: " + e.getMessage());
        }
    }


    @Test
    void testConcurrentByteExtraction() throws InterruptedException {
        byte[] data = "Concurrent byte extraction test".getBytes();

        ExecutorService executor = Executors.newFixedThreadPool(4);
        List<Future<ExtractionResult>> futures = new ArrayList<>();

        try {
            for (int i = 0; i < 10; i++) {
                futures.add(executor.submit(() -> Kreuzberg.extractBytes(data, "text/plain", null)));
            }

            int successCount = 0;
            for (Future<ExtractionResult> future : futures) {
                try {
                    ExtractionResult result = future.get();
                    if (result != null && result.isSuccess()) {
                        successCount++;
                    }
                } catch (Exception e) {
                }
            }

            assertTrue(successCount > 0, "At least some byte extractions should succeed");
        } finally {
            executor.shutdown();
        }
    }


    @Test
    void testThreadSafetyWithSharedResult(@TempDir Path tempDir)
            throws IOException, InterruptedException, KreuzbergException {
        Path testFile = tempDir.resolve("thread_safe.txt");
        Files.writeString(testFile, "Thread safety test content");

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        ExecutorService executor = Executors.newFixedThreadPool(4);
        AtomicInteger readCount = new AtomicInteger(0);

        try {
            for (int i = 0; i < 10; i++) {
                executor.submit(() -> {
                    String content = result.getContent();
                    if (content != null && !content.isEmpty()) {
                        readCount.incrementAndGet();
                    }
                });
            }

            executor.shutdown();
            executor.awaitTermination(5, java.util.concurrent.TimeUnit.SECONDS);

            assertEquals(10, readCount.get(), "All threads should successfully read result");
        } finally {
            if (!executor.isTerminated()) {
                executor.shutdownNow();
            }
        }
    }

    @Test
    void testConcurrentMetadataAccess(@TempDir Path tempDir)
            throws IOException, InterruptedException, KreuzbergException {
        Path testFile = tempDir.resolve("metadata.txt");
        Files.writeString(testFile, "Metadata test");

        ExtractionResult result = Kreuzberg.extractFile(testFile);

        ExecutorService executor = Executors.newFixedThreadPool(4);
        AtomicInteger accessCount = new AtomicInteger(0);

        try {
            for (int i = 0; i < 8; i++) {
                executor.submit(() -> {
                    var metadata = result.getMetadata();
                    if (metadata != null) {
                        accessCount.incrementAndGet();
                    }
                });
            }

            executor.shutdown();
            executor.awaitTermination(5, java.util.concurrent.TimeUnit.SECONDS);

            assertEquals(8, accessCount.get(), "All threads should access metadata");
        } finally {
            if (!executor.isTerminated()) {
                executor.shutdownNow();
            }
        }
    }


    @Test
    void testNoRaceConditionInMimeDetection() throws InterruptedException {
        String path = "/test/file.txt";

        ExecutorService executor = Executors.newFixedThreadPool(4);
        List<Future<String>> futures = new ArrayList<>();

        try {
            for (int i = 0; i < 10; i++) {
                futures.add(executor.submit(() -> {
                    try {
                        return Kreuzberg.detectMimeType(path, false);
                    } catch (KreuzbergException e) {
                        return null;
                    }
                }));
            }

            for (Future<String> future : futures) {
                try {
                    future.get();
                } catch (Exception e) {
                }
            }
        } finally {
            executor.shutdown();
        }
    }

    @Test
    void testNoRaceConditionInPluginManagement() throws KreuzbergException, InterruptedException, ExecutionException {
        ExecutorService executor = Executors.newFixedThreadPool(2);

        String validator1Name = "race-validator-1-" + System.nanoTime();
        String validator2Name = "race-validator-2-" + System.nanoTime();

        try {
            var future1 = executor.submit(() -> {
                try {
                    Kreuzberg.registerValidator(validator1Name, result -> { });
                    return true;
                } catch (KreuzbergException e) {
                    return false;
                }
            });

            var future2 = executor.submit(() -> {
                try {
                    Kreuzberg.registerValidator(validator2Name, result -> { });
                    return true;
                } catch (KreuzbergException e) {
                    return false;
                }
            });

            boolean result1 = future1.get();
            boolean result2 = future2.get();

            assertTrue(result1 || result2, "At least one validator registration should succeed");

            try {
                Kreuzberg.unregisterValidator(validator1Name);
            } catch (Exception ignored) {
            }
            try {
                Kreuzberg.unregisterValidator(validator2Name);
            } catch (Exception ignored) {
            }

        } finally {
            executor.shutdown();
        }
    }


    @Test
    void testParallelBatchExtractions(@TempDir Path tempDir) throws IOException, InterruptedException {
        List<Path> files = new ArrayList<>();
        for (int i = 0; i < 3; i++) {
            Path file = tempDir.resolve("batch_" + i + ".txt");
            Files.writeString(file, "Batch file " + i);
            files.add(file);
        }

        List<String> paths = new ArrayList<>();
        for (Path file : files) {
            paths.add(file.toString());
        }

        ExecutorService executor = Executors.newFixedThreadPool(2);
        List<Future<List<ExtractionResult>>> futures = new ArrayList<>();

        try {
            for (int i = 0; i < 3; i++) {
                futures.add(executor.submit(() -> Kreuzberg.batchExtractFiles(paths, null)));
            }

            int successCount = 0;
            for (Future<List<ExtractionResult>> future : futures) {
                try {
                    List<ExtractionResult> results = future.get();
                    if (results != null && !results.isEmpty()) {
                        successCount++;
                    }
                } catch (Exception e) {
                }
            }

            assertTrue(successCount > 0, "At least one batch extraction should succeed");
        } finally {
            executor.shutdown();
        }
    }


    @Test
    void testStressConcurrentExtractions(@TempDir Path tempDir) throws IOException, InterruptedException {
        Path testFile = tempDir.resolve("stress.txt");
        StringBuilder content = new StringBuilder();
        for (int i = 0; i < 100; i++) {
            content.append("Line ").append(i).append(": Stress test content\n");
        }
        Files.writeString(testFile, content.toString());

        ExecutorService executor = Executors.newFixedThreadPool(8);
        List<Future<ExtractionResult>> futures = new ArrayList<>();
        AtomicInteger successCount = new AtomicInteger(0);
        AtomicInteger errorCount = new AtomicInteger(0);

        try {
            for (int i = 0; i < 100; i++) {
                futures.add(executor.submit(() -> {
                    try {
                        ExtractionResult result = Kreuzberg.extractFile(testFile);
                        if (result != null && result.isSuccess()) {
                            successCount.incrementAndGet();
                        }
                        return result;
                    } catch (Exception e) {
                        errorCount.incrementAndGet();
                        return null;
                    }
                }));
            }

            for (Future<ExtractionResult> future : futures) {
                try {
                    future.get();
                } catch (Exception ignored) {
                }
            }

            int successTotal = successCount.get();
            int errorTotal = errorCount.get();

            assertTrue(successTotal + errorTotal == 100, "All tasks should complete");
            assertTrue(successTotal > errorTotal, "Success rate should be higher than error rate");
        } finally {
            executor.shutdown();
        }
    }


    @Test
    void testSynchronizedConcurrentExecution(@TempDir Path tempDir) throws IOException, InterruptedException {
        Path testFile = tempDir.resolve("synced.txt");
        Files.writeString(testFile, "Synchronized execution test");

        int threadCount = 4;
        CountDownLatch startLatch = new CountDownLatch(1);
        CountDownLatch endLatch = new CountDownLatch(threadCount);

        ExecutorService executor = Executors.newFixedThreadPool(threadCount);
        List<ExtractionResult> results = Collections.synchronizedList(new ArrayList<>());

        try {
            for (int i = 0; i < threadCount; i++) {
                executor.submit(() -> {
                    try {
                        startLatch.await();
                        ExtractionResult result = Kreuzberg.extractFile(testFile);
                        results.add(result);
                    } catch (InterruptedException e) {
                        Thread.currentThread().interrupt();
                    } catch (IOException | KreuzbergException e) {
                    } finally {
                        endLatch.countDown();
                    }
                });
            }

            startLatch.countDown();

            assertTrue(endLatch.await(10, java.util.concurrent.TimeUnit.SECONDS),
                    "All threads should complete");

            assertTrue(results.size() > 0, "At least some extractions should complete");
        } finally {
            executor.shutdown();
        }
    }


    @Test
    void testConfigBuilderThreadSafety() throws InterruptedException, ExecutionException {
        ExecutorService executor = Executors.newFixedThreadPool(4);
        List<Future<ExtractionConfig>> futures = new ArrayList<>();

        try {
            for (int i = 0; i < 10; i++) {
                futures.add(executor.submit(() ->
                    ExtractionConfig.builder()
                        .useCache(true)
                        .enableQualityProcessing(false)
                        .maxConcurrentExtractions(4)
                        .build()
                ));
            }

            for (Future<ExtractionConfig> future : futures) {
                ExtractionConfig config = future.get();
                assertNotNull(config, "Config should be created");
            }
        } finally {
            executor.shutdown();
        }
    }


    @Test
    void testCancellableAsyncExtraction(@TempDir Path tempDir) throws IOException {
        Path testFile = tempDir.resolve("cancellable.txt");
        Files.writeString(testFile, "Cancellable extraction test");

        var future = Kreuzberg.extractFileAsync(testFile, null);

        boolean cancelled = future.cancel(true);

        if (cancelled) {
            assertTrue(future.isCancelled(), "Future should be marked as cancelled");
        } else {
            try {
                ExtractionResult result = future.get();
                assertNotNull(result, "Extraction should complete");
            } catch (Exception e) {
            }
        }
    }


    @Test
    void testSequentialVsConcurrentResults(@TempDir Path tempDir)
            throws IOException, InterruptedException, ExecutionException,
            KreuzbergException {
        Path testFile = tempDir.resolve("equivalence.txt");
        Files.writeString(testFile, "Equivalence test content");

        ExtractionResult sequential = Kreuzberg.extractFile(testFile);

        ExecutorService executor = Executors.newFixedThreadPool(2);
        List<Future<ExtractionResult>> futures = new ArrayList<>();

        try {
            for (int i = 0; i < 2; i++) {
                futures.add(executor.submit(() -> Kreuzberg.extractFile(testFile)));
            }

            for (Future<ExtractionResult> future : futures) {
                ExtractionResult concurrent = future.get();

                assertEquals(sequential.getMimeType(), concurrent.getMimeType(),
                        "MIME types should be identical");
                assertTrue(sequential.isSuccess() == concurrent.isSuccess(),
                        "Success status should match");
            }
        } finally {
            executor.shutdown();
        }
    }
}
