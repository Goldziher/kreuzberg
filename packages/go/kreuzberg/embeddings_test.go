package kreuzberg

import "testing"

func TestListEmbeddingPresets(t *testing.T) {
	presets, err := ListEmbeddingPresets()
	if err != nil {
		t.Fatalf("list embedding presets: %v", err)
	}
	if len(presets) == 0 {
		t.Fatalf("expected at least one preset")
	}
}

func TestGetEmbeddingPreset(t *testing.T) {
	preset, err := GetEmbeddingPreset("balanced")
	if err != nil {
		t.Fatalf("get embedding preset: %v", err)
	}
	if preset == nil {
		t.Fatalf("preset should not be nil")
	}
	if preset.Name == "" || preset.ModelName == "" {
		t.Fatalf("preset fields missing: %+v", preset)
	}

	if _, err := GetEmbeddingPreset("nonexistent"); err == nil {
		t.Fatalf("expected error for unknown preset")
	}
}
