package kreuzberg

import (
	"context"
	"errors"
	"testing"
	"time"
)

func TestRunAsyncReturnsResult(t *testing.T) {
	ctx := context.Background()
	value, err := runAsync(ctx, func() (int, error) {
		return 42, nil
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if value != 42 {
		t.Fatalf("unexpected value: %d", value)
	}
}

func TestRunAsyncRespectsContextCancellation(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	_, err := runAsync[int](ctx, func() (int, error) {
		time.Sleep(10 * time.Millisecond)
		return 0, nil
	})
	if !errors.Is(err, context.Canceled) {
		t.Fatalf("expected context.Canceled, got %v", err)
	}
}
