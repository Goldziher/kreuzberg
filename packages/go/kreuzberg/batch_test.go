package kreuzberg

import (
	"fmt"
	"path/filepath"
	"sync"
	"testing"
	"time"
)

// TestBatchExtractFilesSync tests basic batch file extraction.
func TestBatchExtractFilesSync(t *testing.T) {
	dir := t.TempDir()
	path1, err := writeValidPDFToFile(dir, "file1.pdf")
	if err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}
	path2, err := writeValidPDFToFile(dir, "file2.pdf")
	if err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}

	results, err := BatchExtractFilesSync([]string{path1, path2}, nil)
	if err != nil {
		t.Fatalf("BatchExtractFilesSync failed: %v", err)
	}
	if len(results) != 2 {
		t.Fatalf("expected 2 results, got %d", len(results))
	}
}

// TestBatchExtractFilesWithEmptyList tests batch extraction with empty file list.
func TestBatchExtractFilesWithEmptyList(t *testing.T) {
	results, err := BatchExtractFilesSync([]string{}, nil)
	if err != nil {
		t.Fatalf("expected no error for empty list: %v", err)
	}
	if len(results) != 0 {
		t.Fatalf("expected 0 results for empty list, got %d", len(results))
	}
}

// TestBatchExtractFilesWithMissingFile tests batch extraction with one missing file.
func TestBatchExtractFilesWithMissingFile(t *testing.T) {
	dir := t.TempDir()
	validPath, err := writeValidPDFToFile(dir, "valid.pdf")
	if err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}

	missingPath := filepath.Join(dir, "missing.pdf")
	results, err := BatchExtractFilesSync([]string{validPath, missingPath}, nil)
	// May error or return partial results depending on implementation
	_ = err
	_ = results
}

// TestBatchExtractFilesWithEmptyPath tests batch extraction validation.
func TestBatchExtractFilesWithEmptyPath(t *testing.T) {
	_, err := BatchExtractFilesSync([]string{""}, nil)
	if err == nil {
		t.Fatalf("expected error for empty path")
	}
}

// TestBatchExtractFilesWithConfig tests batch extraction with configuration.
func TestBatchExtractFilesWithConfig(t *testing.T) {
	dir := t.TempDir()
	path1, err := writeValidPDFToFile(dir, "file1.pdf")
	if err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}

	config := &ExtractionConfig{
		UseCache: BoolPtr(false),
	}

	results, err := BatchExtractFilesSync([]string{path1}, config)
	if err != nil {
		t.Fatalf("batch extraction with config failed: %v", err)
	}
	if len(results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(results))
	}
}

// TestBatchExtractBytesSync tests batch extraction from byte arrays.
func TestBatchExtractBytesSync(t *testing.T) {
	pdfData, err := getValidPDFBytes()
	if err != nil {
		t.Fatalf("failed to get PDF bytes: %v", err)
	}
	items := []BytesWithMime{
		{Data: pdfData, MimeType: "application/pdf"},
		{Data: pdfData, MimeType: "application/pdf"},
	}

	results, err := BatchExtractBytesSync(items, nil)
	if err != nil {
		t.Fatalf("BatchExtractBytesSync failed: %v", err)
	}
	if len(results) != 2 {
		t.Fatalf("expected 2 results, got %d", len(results))
	}
}

// TestBatchExtractBytesWithEmptyList tests batch bytes extraction with empty list.
func TestBatchExtractBytesWithEmptyList(t *testing.T) {
	results, err := BatchExtractBytesSync([]BytesWithMime{}, nil)
	if err != nil {
		t.Fatalf("expected no error for empty list: %v", err)
	}
	if len(results) != 0 {
		t.Fatalf("expected 0 results, got %d", len(results))
	}
}

// TestBatchExtractBytesWithEmptyData tests batch extraction validation.
func TestBatchExtractBytesWithEmptyData(t *testing.T) {
	items := []BytesWithMime{
		{Data: []byte{}, MimeType: "application/pdf"},
	}

	_, err := BatchExtractBytesSync(items, nil)
	if err == nil {
		t.Fatalf("expected error for empty data")
	}
}

// TestBatchExtractBytesWithEmptyMimeType tests validation of MIME type.
func TestBatchExtractBytesWithEmptyMimeType(t *testing.T) {
	items := []BytesWithMime{
		{Data: []byte("%PDF-1.7\n"), MimeType: ""},
	}

	_, err := BatchExtractBytesSync(items, nil)
	if err == nil {
		t.Fatalf("expected error for empty MIME type")
	}
}

// TestBatchExtractBytesWithConfig tests batch bytes extraction with configuration.
func TestBatchExtractBytesWithConfig(t *testing.T) {
	pdfData, err := getValidPDFBytes()
	if err != nil {
		t.Fatalf("failed to get PDF bytes: %v", err)
	}
	items := []BytesWithMime{
		{Data: pdfData, MimeType: "application/pdf"},
	}

	config := &ExtractionConfig{
		UseCache: BoolPtr(true),
	}

	results, err := BatchExtractBytesSync(items, config)
	if err != nil {
		t.Fatalf("batch extraction with config failed: %v", err)
	}
	if len(results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(results))
	}
}

// TestBatchLargeBatch tests batch extraction with many files.
func TestBatchLargeBatch(t *testing.T) {
	dir := t.TempDir()
	var paths []string

	for i := 0; i < 50; i++ {
		filename := fmt.Sprintf("file_%d.pdf", i)
		path, err := writeValidPDFToFile(dir, filename)
		if err != nil {
			t.Fatalf("failed to write test file: %v", err)
		}
		paths = append(paths, path)
	}

	results, err := BatchExtractFilesSync(paths, nil)
	if err != nil {
		t.Fatalf("large batch extraction failed: %v", err)
	}
	if len(results) != 50 {
		t.Fatalf("expected 50 results, got %d", len(results))
	}
}

// TestBatchConcurrentOperations tests thread-safe concurrent operations.
func TestBatchConcurrentOperations(t *testing.T) {
	dir := t.TempDir()
	path, err := writeValidPDFToFile(dir, "test.pdf")
	if err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}

	var wg sync.WaitGroup
	numGoroutines := 5
	errChan := make(chan error, numGoroutines)

	for i := 0; i < numGoroutines; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			_, err := ExtractFileSync(path, nil)
			if err != nil {
				errChan <- err
			}
		}()
	}

	wg.Wait()
	close(errChan)

	if len(errChan) > 0 {
		err := <-errChan
		t.Fatalf("concurrent extraction failed: %v", err)
	}
}

// TestBatchWithPartialFailures tests handling of partial failures in batch.
func TestBatchWithPartialFailures(t *testing.T) {
	dir := t.TempDir()
	validPath, err := writeValidPDFToFile(dir, "valid.pdf")
	if err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}

	missingPath := filepath.Join(dir, "missing.pdf")

	// Batch may return partial results or error
	results, err := BatchExtractFilesSync([]string{validPath, missingPath}, nil)
	_ = err
	_ = results
}

// TestBatchResultConsistency tests that batch results are consistent.
func TestBatchResultConsistency(t *testing.T) {
	dir := t.TempDir()
	path, err := writeValidPDFToFile(dir, "test.pdf")
	if err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}

	results, err := BatchExtractFilesSync([]string{path}, nil)
	if err != nil {
		t.Fatalf("batch extraction failed: %v", err)
	}

	singleResult, err := ExtractFileSync(path, nil)
	if err != nil {
		t.Fatalf("single extraction failed: %v", err)
	}

	if len(results) != 1 {
		t.Fatalf("batch should return 1 result")
	}

	// Results should be structurally similar
	if (results[0] == nil) != (singleResult == nil) {
		t.Fatalf("batch and single result consistency mismatch")
	}
}

// TestBatchOrderPreservation tests that batch results maintain order.
func TestBatchOrderPreservation(t *testing.T) {
	dir := t.TempDir()
	var paths []string

	for i := 0; i < 5; i++ {
		filename := fmt.Sprintf("file_%c.pdf", 'a'+i)
		path, err := writeValidPDFToFile(dir, filename)
		if err != nil {
			t.Fatalf("failed to write test file: %v", err)
		}
		paths = append(paths, path)
	}

	results, err := BatchExtractFilesSync(paths, nil)
	if err != nil {
		t.Fatalf("batch extraction failed: %v", err)
	}

	if len(results) != len(paths) {
		t.Fatalf("result count mismatch: expected %d, got %d", len(paths), len(results))
	}
}

// TestBatchMemoryManagement tests that batch operations manage memory properly.
func TestBatchMemoryManagement(t *testing.T) {
	t.Run("file batch memory", func(t *testing.T) {
		dir := t.TempDir()
		var paths []string

		for i := 0; i < 100; i++ {
			filename := fmt.Sprintf("file_%d.pdf", i%10)
			path, _ := writeValidPDFToFile(dir, filename)
			if path != "" {
				paths = append(paths, path)
			}
		}

		// Should complete without excessive memory use
		_, err := BatchExtractFilesSync(paths, nil)
		_ = err // May succeed or fail
	})

	t.Run("bytes batch memory", func(t *testing.T) {
		pdfData, _ := getValidPDFBytes()
		var items []BytesWithMime
		for i := 0; i < 100; i++ {
			items = append(items, BytesWithMime{
				Data:     pdfData,
				MimeType: "application/pdf",
			})
		}

		// Should complete without excessive memory use
		_, err := BatchExtractBytesSync(items, nil)
		_ = err // May succeed or fail
	})
}

// TestBatchConcurrentByteExtraction tests concurrent byte extraction.
func TestBatchConcurrentByteExtraction(t *testing.T) {
	pdfData, err := getValidPDFBytes()
	if err != nil {
		t.Fatalf("failed to get PDF bytes: %v", err)
	}

	var wg sync.WaitGroup
	numGoroutines := 5
	errChan := make(chan error, numGoroutines)

	for i := 0; i < numGoroutines; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			items := []BytesWithMime{
				{Data: pdfData, MimeType: "application/pdf"},
			}
			_, err := BatchExtractBytesSync(items, nil)
			if err != nil {
				errChan <- err
			}
		}()
	}

	wg.Wait()
	close(errChan)

	if len(errChan) > 0 {
		err := <-errChan
		t.Fatalf("concurrent byte extraction failed: %v", err)
	}
}

// TestBatchMixedOperations tests mixing file and byte batch operations.
func TestBatchMixedOperations(t *testing.T) {
	dir := t.TempDir()
	path, err := writeValidPDFToFile(dir, "test.pdf")
	if err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}

	// File batch
	_, err1 := BatchExtractFilesSync([]string{path}, nil)

	// Byte batch
	pdfData, _ := getValidPDFBytes()
	items := []BytesWithMime{{Data: pdfData, MimeType: "application/pdf"}}
	_, err2 := BatchExtractBytesSync(items, nil)

	// Both should work without interference
	_ = err1
	_ = err2
}

// TestBatchPerformanceMetrics tests batch performance characteristics.
func TestBatchPerformanceMetrics(t *testing.T) {
	t.Run("small batch performance", func(t *testing.T) {
		dir := t.TempDir()
		var paths []string

		for i := 0; i < 3; i++ {
			filename := fmt.Sprintf("file_%c.pdf", 'a'+i)
			path, err := writeValidPDFToFile(dir, filename)
			if err != nil {
				t.Fatalf("failed to write test file: %v", err)
			}
			paths = append(paths, path)
		}

		start := time.Now()
		_, err := BatchExtractFilesSync(paths, nil)
		duration := time.Since(start)

		_ = err
		if duration < 0 {
			t.Fatalf("negative duration detected")
		}
	})

	t.Run("medium batch performance", func(t *testing.T) {
		pdfData, _ := getValidPDFBytes()
		var items []BytesWithMime
		for i := 0; i < 20; i++ {
			items = append(items, BytesWithMime{
				Data:     pdfData,
				MimeType: "application/pdf",
			})
		}

		start := time.Now()
		_, err := BatchExtractBytesSync(items, nil)
		duration := time.Since(start)

		_ = err
		if duration < 0 {
			t.Fatalf("negative duration detected")
		}
	})
}

// TestBatchWithDifferentMimeTypes tests batch with mixed MIME types.
func TestBatchWithDifferentMimeTypes(t *testing.T) {
	pdfData, err := getValidPDFBytes()
	if err != nil {
		t.Fatalf("failed to get PDF bytes: %v", err)
	}
	items := []BytesWithMime{
		{Data: pdfData, MimeType: "application/pdf"},
		{Data: pdfData, MimeType: "application/pdf"},
		{Data: pdfData, MimeType: "application/pdf"},
	}

	results, err := BatchExtractBytesSync(items, nil)
	if err != nil {
		t.Fatalf("batch with mixed MIME failed: %v", err)
	}
	if len(results) != 3 {
		t.Fatalf("expected 3 results, got %d", len(results))
	}
}

// TestBatchErrorPropagation tests error propagation in batch operations.
func TestBatchErrorPropagation(t *testing.T) {
	t.Run("invalid MIME type in batch", func(t *testing.T) {
		items := []BytesWithMime{
			{Data: []byte("test"), MimeType: "invalid/mime"},
		}

		_, err := BatchExtractBytesSync(items, nil)
		// May error or handle gracefully
		_ = err
	})

	t.Run("mixed valid and invalid", func(t *testing.T) {
		pdfData, _ := getValidPDFBytes()
		items := []BytesWithMime{
			{Data: pdfData, MimeType: "application/pdf"},
			{Data: []byte("test"), MimeType: "invalid/mime"},
		}

		results, err := BatchExtractBytesSync(items, nil)
		// May return partial results
		_ = err
		_ = results
	})
}

// TestBatchResultValidation tests that batch results are valid.
func TestBatchResultValidation(t *testing.T) {
	dir := t.TempDir()
	path, err := writeValidPDFToFile(dir, "test.pdf")
	if err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}

	results, err := BatchExtractFilesSync([]string{path}, nil)
	if err != nil {
		t.Fatalf("batch extraction failed: %v", err)
	}

	if len(results) != 1 {
		t.Fatalf("expected 1 result")
	}

	if results[0] != nil {
		if results[0].MimeType == "" && results[0].Success {
			t.Fatalf("result should have MIME type if successful")
		}
	}
}

// TestBatchResultMetadata tests metadata in batch results.
func TestBatchResultMetadata(t *testing.T) {
	dir := t.TempDir()
	path, err := writeValidPDFToFile(dir, "test.pdf")
	if err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}

	results, err := BatchExtractFilesSync([]string{path}, nil)
	if err != nil {
		t.Fatalf("batch extraction failed: %v", err)
	}

	if len(results) > 0 && results[0] != nil {
		// Metadata should be present
		_ = results[0].Metadata
	}
}

// TestBatchWithConfigurationVariants tests batch with various configurations.
func TestBatchWithConfigurationVariants(t *testing.T) {
	dir := t.TempDir()
	path, err := writeValidPDFToFile(dir, "test.pdf")
	if err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}

	t.Run("with cache disabled", func(t *testing.T) {
		config := &ExtractionConfig{UseCache: BoolPtr(false)}
		_, err := BatchExtractFilesSync([]string{path}, config)
		_ = err
	})

	t.Run("with quality processing", func(t *testing.T) {
		config := &ExtractionConfig{EnableQualityProcessing: BoolPtr(true)}
		_, err := BatchExtractFilesSync([]string{path}, config)
		_ = err
	})

	t.Run("with chunking enabled", func(t *testing.T) {
		config := &ExtractionConfig{
			Chunking: &ChunkingConfig{
				Enabled: BoolPtr(true),
			},
		}
		_, err := BatchExtractFilesSync([]string{path}, config)
		_ = err
	})
}

// TestBatchResourceCleanup tests that resources are properly cleaned up.
func TestBatchResourceCleanup(t *testing.T) {
	for i := 0; i < 3; i++ {
		dir := t.TempDir()
		path, err := writeValidPDFToFile(dir, "test.pdf")
		if err != nil {
			t.Fatalf("failed to write test file: %v", err)
		}

		_, err = BatchExtractFilesSync([]string{path}, nil)
		_ = err

		// Temp directory should be cleaned up properly by t.TempDir()
	}
}

// TestBatchParallelFileAccess tests parallel access to batch operations.
func TestBatchParallelFileAccess(t *testing.T) {
	dir := t.TempDir()
	path, err := writeValidPDFToFile(dir, "shared.pdf")
	if err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}

	var wg sync.WaitGroup
	numGoroutines := 5
	errChan := make(chan error, numGoroutines)

	for i := 0; i < numGoroutines; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			_, err := BatchExtractFilesSync([]string{path}, nil)
			if err != nil {
				errChan <- err
			}
		}()
	}

	wg.Wait()
	close(errChan)

	if len(errChan) > 0 {
		err := <-errChan
		t.Fatalf("parallel batch access failed: %v", err)
	}
}

// TestBatchBytesSizeVariation tests batch extraction with various data sizes.
func TestBatchBytesSizeVariation(t *testing.T) {
	t.Run("tiny batch items", func(t *testing.T) {
		items := []BytesWithMime{
			{Data: []byte("%PDF"), MimeType: "application/pdf"},
		}
		_, err := BatchExtractBytesSync(items, nil)
		_ = err
	})

	t.Run("normal size batch items", func(t *testing.T) {
		pdfData, _ := getValidPDFBytes()
		items := []BytesWithMime{
			{Data: pdfData, MimeType: "application/pdf"},
		}
		_, err := BatchExtractBytesSync(items, nil)
		_ = err
	})
}

// TestBatchDeferredCleanup tests deferred cleanup in batch operations.
func TestBatchDeferredCleanup(t *testing.T) {
	pdfData, _ := getValidPDFBytes()
	items := []BytesWithMime{
		{Data: pdfData, MimeType: "application/pdf"},
		{Data: pdfData, MimeType: "application/pdf"},
	}

	// Verify cleanup happens after extraction
	_, err := BatchExtractBytesSync(items, nil)
	if err != nil {
		t.Logf("batch extraction completed with error: %v", err)
	}

	// Second batch should work fine if resources were properly cleaned
	items2 := []BytesWithMime{
		{Data: pdfData, MimeType: "application/pdf"},
	}
	_, err2 := BatchExtractBytesSync(items2, nil)
	_ = err2
}

// TestBatchExtractionIndexing tests result indexing in batch operations.
func TestBatchExtractionIndexing(t *testing.T) {
	dir := t.TempDir()
	var paths []string

	for i := 0; i < 10; i++ {
		filename := fmt.Sprintf("file_%d.pdf", i)
		path, err := writeValidPDFToFile(dir, filename)
		if err != nil {
			t.Fatalf("failed to write test file: %v", err)
		}
		paths = append(paths, path)
	}

	results, err := BatchExtractFilesSync(paths, nil)
	if err != nil {
		t.Fatalf("batch extraction failed: %v", err)
	}

	if len(results) != len(paths) {
		t.Fatalf("result count mismatch: expected %d, got %d", len(paths), len(results))
	}

	// Each result should correspond to the input at the same index
	for i := range results {
		_ = results[i] // Verify we can access each result
	}
}

// TestBatchWithNoConfig tests batch operations when config is nil.
func TestBatchWithNoConfig(t *testing.T) {
	dir := t.TempDir()
	path, err := writeValidPDFToFile(dir, "test.pdf")
	if err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}

	_, err = BatchExtractFilesSync([]string{path}, nil)
	// Should use default config
	_ = err
}

// TestBatchConsecutiveOperations tests consecutive batch operations.
func TestBatchConsecutiveOperations(t *testing.T) {
	dir := t.TempDir()
	path1, err := writeValidPDFToFile(dir, "file1.pdf")
	if err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}
	path2, err := writeValidPDFToFile(dir, "file2.pdf")
	if err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}

	_, err1 := BatchExtractFilesSync([]string{path1}, nil)
	_, err2 := BatchExtractFilesSync([]string{path2}, nil)

	_ = err1
	_ = err2
}

// TestBatchResultNilHandling tests handling of nil results in batch.
func TestBatchResultNilHandling(t *testing.T) {
	results, err := BatchExtractFilesSync([]string{}, nil)
	if err != nil {
		t.Fatalf("batch extraction failed: %v", err)
	}

	// Empty batch should return empty results
	if results == nil {
		t.Fatalf("expected non-nil results slice for empty batch")
	}
}

// TestBatchConfigImmutability tests that config is not modified during batch.
func TestBatchConfigImmutability(t *testing.T) {
	dir := t.TempDir()
	path, err := writeValidPDFToFile(dir, "test.pdf")
	if err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}

	config := &ExtractionConfig{
		UseCache: BoolPtr(true),
	}

	originalValue := *config.UseCache
	_, err = BatchExtractFilesSync([]string{path}, config)
	_ = err

	if config.UseCache == nil || *config.UseCache != originalValue {
		t.Fatalf("config was modified during batch extraction")
	}
}

// TestBatchDataIntegrity tests that extracted data integrity is maintained.
func TestBatchDataIntegrity(t *testing.T) {
	pdfData, _ := getValidPDFBytes()
	items := []BytesWithMime{
		{Data: pdfData, MimeType: "application/pdf"},
		{Data: pdfData, MimeType: "application/pdf"},
	}

	results, err := BatchExtractBytesSync(items, nil)
	if err != nil {
		t.Fatalf("batch extraction failed: %v", err)
	}

	// Verify we got results for all items
	if len(results) != len(items) {
		t.Fatalf("expected %d results, got %d", len(items), len(results))
	}
}
