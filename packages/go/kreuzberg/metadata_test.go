package kreuzberg

import (
	"encoding/json"
	"reflect"
	"testing"
)

func TestMetadataRoundTripPreservesFormatAndAdditionalFields(t *testing.T) {
	input := []byte(`{
		"language": "en",
		"date": "2025-01-01",
		"subject": "Agenda",
		"format_type": "pdf",
		"title": "Doc",
		"page_count": 2,
		"image_preprocessing": {
			"original_dimensions": [1024, 2048],
			"original_dpi": [72.0, 72.0],
			"target_dpi": 300,
			"scale_factor": 1.5,
			"auto_adjusted": true,
			"final_dpi": 300,
			"new_dimensions": [2048, 4096],
			"resample_method": "lanczos",
			"dimension_clamped": false,
			"calculated_dpi": 310,
			"skipped_resize": false
		},
		"json_schema": {"type": "object"},
		"error": {"error_type": "ValidationError", "message": "bad"},
		"custom_meta": {"score": 42}
	}`)

	var meta Metadata
	if err := json.Unmarshal(input, &meta); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}

	if meta.Format.Type != FormatPDF {
		t.Fatalf("expected format pdf, got %s", meta.Format.Type)
	}
	if meta.Format.Pdf == nil || meta.Format.Pdf.PageCount == nil || *meta.Format.Pdf.PageCount != 2 {
		t.Fatalf("expected pdf metadata with page count")
	}
	if meta.Additional == nil || len(meta.Additional) != 1 {
		t.Fatalf("expected additional metadata")
	}

	if _, ok := meta.Additional["custom_meta"]; !ok {
		t.Fatalf("missing custom metadata field")
	}

	encoded, err := json.Marshal(meta)
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}

	var want map[string]any
	if err := json.Unmarshal(input, &want); err != nil {
		t.Fatalf("unmarshal want: %v", err)
	}

	var got map[string]any
	if err := json.Unmarshal(encoded, &got); err != nil {
		t.Fatalf("unmarshal got: %v", err)
	}

	if !reflect.DeepEqual(want, got) {
		t.Fatalf("metadata mismatch: want %#v, got %#v", want, got)
	}
}

func TestMetadataRoundTripHandlesTextFormats(t *testing.T) {
	input := []byte(`{
		"language": "en",
		"format_type": "text",
		"line_count": 10,
		"word_count": 20,
		"character_count": 40,
		"headers": ["Intro"],
		"links": [["https://example.com", "Example"]],
		"custom_score": 0.42
	}`)

	var meta Metadata
	if err := json.Unmarshal(input, &meta); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}

	if meta.Format.Type != FormatText {
		t.Fatalf("expected text format")
	}
	text, ok := meta.TextMetadata()
	if !ok || text.WordCount != 20 {
		t.Fatalf("text metadata not decoded")
	}

	encoded, err := json.Marshal(meta)
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}

	var want map[string]any
	if err := json.Unmarshal(input, &want); err != nil {
		t.Fatalf("want decode: %v", err)
	}
	var got map[string]any
	if err := json.Unmarshal(encoded, &got); err != nil {
		t.Fatalf("got decode: %v", err)
	}

	if !reflect.DeepEqual(want, got) {
		t.Fatalf("metadata mismatch: want %#v, got %#v", want, got)
	}
}
