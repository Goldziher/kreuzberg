package main

import (
	"encoding/json"
	"fmt"
	"os"
	"time"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

type payload struct {
	Content          string         `json:"content"`
	Metadata         map[string]any `json:"metadata"`
	ExtractionTimeMs float64        `json:"_extraction_time_ms"`
	BatchTotalTimeMs float64        `json:"_batch_total_ms,omitempty"`
}

func main() {
	if len(os.Args) < 3 {
		fmt.Fprintln(os.Stderr, "Usage: kreuzberg_extract_go.go <mode> <file_path> [additional_files...]")
		fmt.Fprintln(os.Stderr, "Modes: sync, batch")
		os.Exit(1)
	}

	mode := os.Args[1]
	files := os.Args[2:]

	switch mode {
	case "sync":
		if len(files) != 1 {
			fatal(fmt.Errorf("sync mode requires exactly one file"))
		}
		result, err := extractSync(files[0])
		if err != nil {
			fatal(err)
		}
		mustEncode(result)
	case "batch":
		if len(files) == 0 {
			fatal(fmt.Errorf("batch mode requires at least one file"))
		}
		results, err := extractBatch(files)
		if err != nil {
			fatal(err)
		}
		mustEncode(results)
	default:
		fatal(fmt.Errorf("unknown mode %q", mode))
	}
}

func extractSync(path string) (*payload, error) {
	start := time.Now()
	result, err := kreuzberg.ExtractFileSync(path, nil)
	if err != nil {
		return nil, err
	}
	elapsed := time.Since(start).Seconds() * 1000.0
	meta, err := metadataMap(result.Metadata)
	if err != nil {
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
	results, err := kreuzberg.BatchExtractFilesSync(paths, nil)
	if err != nil {
		return nil, err
	}
	totalMs := time.Since(start).Seconds() * 1000.0
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
	enc := json.NewEncoder(os.Stdout)
	enc.SetEscapeHTML(false)
	if err := enc.Encode(value); err != nil {
		fatal(err)
	}
}

func fatal(err error) {
	fmt.Fprintf(os.Stderr, "Error extracting with Go binding: %v\n", err)
	os.Exit(1)
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}
