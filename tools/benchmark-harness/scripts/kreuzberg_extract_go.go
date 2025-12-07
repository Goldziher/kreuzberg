package main

import (
	"encoding/json"
	"fmt"
	"os"
	"time"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

var debugEnabled = os.Getenv("KREUZBERG_BENCHMARK_DEBUG") != ""

func debug(msg string, args ...interface{}) {
	if debugEnabled {
		fmt.Fprintf(os.Stderr, "[DEBUG] "+msg+"\n", args...)
	}
}

type payload struct {
	Content          string         `json:"content"`
	Metadata         map[string]any `json:"metadata"`
	ExtractionTimeMs float64        `json:"_extraction_time_ms"`
	BatchTotalTimeMs float64        `json:"_batch_total_ms,omitempty"`
}

func main() {
	debug("Kreuzberg Go extraction script started")
	debug("Command-line args: %v", os.Args)
	debug("Working directory: %s", getWorkingDir())
	debug("LD_LIBRARY_PATH: %s", os.Getenv("LD_LIBRARY_PATH"))
	debug("DYLD_LIBRARY_PATH: %s", os.Getenv("DYLD_LIBRARY_PATH"))

	if len(os.Args) < 3 {
		fmt.Fprintln(os.Stderr, "Usage: kreuzberg_extract_go.go <mode> <file_path> [additional_files...]")
		fmt.Fprintln(os.Stderr, "Modes: sync, batch")
		os.Exit(1)
	}

	mode := os.Args[1]
	files := os.Args[2:]

	debug("Mode: %s, Files: %v", mode, files)

	switch mode {
	case "sync":
		if len(files) != 1 {
			fatal(fmt.Errorf("sync mode requires exactly one file"))
		}
		debug("Starting sync extraction for: %s", files[0])
		result, err := extractSync(files[0])
		if err != nil {
			fatal(err)
		}
		debug("Sync extraction completed successfully")
		mustEncode(result)
	case "batch":
		if len(files) == 0 {
			fatal(fmt.Errorf("batch mode requires at least one file"))
		}
		debug("Starting batch extraction for %d files", len(files))
		results, err := extractBatch(files)
		if err != nil {
			fatal(err)
		}
		debug("Batch extraction completed successfully")
		mustEncode(results)
	default:
		fatal(fmt.Errorf("unknown mode %q", mode))
	}
}

func getWorkingDir() string {
	pwd, err := os.Getwd()
	if err != nil {
		return fmt.Sprintf("<error: %v>", err)
	}
	return pwd
}

func extractSync(path string) (*payload, error) {
	start := time.Now()
	debug("ExtractFileSync called with path: %s", path)
	result, err := kreuzberg.ExtractFileSync(path, nil)
	if err != nil {
		debug("ExtractFileSync failed: %v", err)
		return nil, err
	}
	elapsed := time.Since(start).Seconds() * 1000.0
	debug("ExtractFileSync succeeded, elapsed: %.2f ms", elapsed)
	meta, err := metadataMap(result.Metadata)
	if err != nil {
		debug("metadataMap failed: %v", err)
		return nil, err
	}
	return &payload{
		Content:          result.Content,
		Metadata:         meta,
		ExtractionTimeMs: elapsed,
	}, nil
}

func extractBatch(paths []string) (any, error) {
	start := time.Now()
	debug("BatchExtractFilesSync called with %d files", len(paths))
	results, err := kreuzberg.BatchExtractFilesSync(paths, nil)
	if err != nil {
		debug("BatchExtractFilesSync failed: %v", err)
		return nil, err
	}
	totalMs := time.Since(start).Seconds() * 1000.0
	debug("BatchExtractFilesSync succeeded, %d results, total elapsed: %.2f ms", len(results), totalMs)
	if len(paths) == 1 && len(results) == 1 {
		meta, err := metadataMap(results[0].Metadata)
		if err != nil {
			return nil, err
		}
		return &payload{
			Content:          results[0].Content,
			Metadata:         meta,
			ExtractionTimeMs: totalMs,
			BatchTotalTimeMs: totalMs,
		}, nil
	}

	out := make([]*payload, 0, len(results))
	perMs := totalMs / float64(max(len(results), 1))
	for _, item := range results {
		if item == nil {
			continue
		}
		meta, err := metadataMap(item.Metadata)
		if err != nil {
			return nil, err
		}
		out = append(out, &payload{
			Content:          item.Content,
			Metadata:         meta,
			ExtractionTimeMs: perMs,
			BatchTotalTimeMs: totalMs,
		})
	}
	return out, nil
}

func metadataMap(meta kreuzberg.Metadata) (map[string]any, error) {
	bytes, err := json.Marshal(meta)
	if err != nil {
		return nil, err
	}
	var out map[string]any
	if err := json.Unmarshal(bytes, &out); err != nil {
		return nil, err
	}
	return out, nil
}

func mustEncode(value any) {
	debug("Encoding result to JSON")
	enc := json.NewEncoder(os.Stdout)
	enc.SetEscapeHTML(false)
	if err := enc.Encode(value); err != nil {
		debug("JSON encoding failed: %v", err)
		fatal(err)
	}
	debug("JSON output complete")
}

func fatal(err error) {
	fmt.Fprintf(os.Stderr, "Error extracting with Go binding: %v\n", err)
	debug("Exiting with error: %v", err)
	os.Exit(1)
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}
