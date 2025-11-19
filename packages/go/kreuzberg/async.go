package kreuzberg

import "context"

type asyncResult[T any] struct {
	value T
	err   error
}

func runAsync[T any](ctx context.Context, fn func() (T, error)) (T, error) {
	var zero T
	if ctx == nil {
		ctx = context.Background()
	}

	resultCh := make(chan asyncResult[T], 1)
	go func() {
		value, err := fn()
		resultCh <- asyncResult[T]{value: value, err: err}
	}()

	select {
	case <-ctx.Done():
		return zero, ctx.Err()
	case out := <-resultCh:
		return out.value, out.err
	}
}

// ExtractFile asynchronously extracts a document from disk.
//
// The extraction runs in a goroutine. Context cancellation is best-effort:
// the underlying native call cannot be interrupted yet, but the function
// returns early with ctx.Err() once the context is done.
func ExtractFile(ctx context.Context, path string, config *ExtractionConfig) (*ExtractionResult, error) {
	return runAsync(ctx, func() (*ExtractionResult, error) {
		return ExtractFileSync(path, config)
	})
}

// ExtractBytes asynchronously processes in-memory documents.
func ExtractBytes(ctx context.Context, data []byte, mimeType string, config *ExtractionConfig) (*ExtractionResult, error) {
	return runAsync(ctx, func() (*ExtractionResult, error) {
		return ExtractBytesSync(data, mimeType, config)
	})
}

// BatchExtractFiles asynchronously processes multiple files.
func BatchExtractFiles(ctx context.Context, paths []string, config *ExtractionConfig) ([]*ExtractionResult, error) {
	return runAsync(ctx, func() ([]*ExtractionResult, error) {
		return BatchExtractFilesSync(paths, config)
	})
}

// BatchExtractBytes asynchronously processes multiple in-memory documents.
func BatchExtractBytes(ctx context.Context, items []BytesWithMime, config *ExtractionConfig) ([]*ExtractionResult, error) {
	return runAsync(ctx, func() ([]*ExtractionResult, error) {
		return BatchExtractBytesSync(items, config)
	})
}
