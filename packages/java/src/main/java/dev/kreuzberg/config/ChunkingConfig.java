package dev.kreuzberg.config;

import java.util.HashMap;
import java.util.Map;

/**
 * Chunking configuration for splitting extracted text.
 *
 * @since 4.0.0
 */
public final class ChunkingConfig {
  private final int maxChars;
  private final int maxOverlap;
  private final String preset;
  private final Map<String, Object> embedding;
  private final Boolean enabled;

  private ChunkingConfig(Builder builder) {
    this.maxChars = builder.maxChars;
    this.maxOverlap = builder.maxOverlap;
    this.preset = builder.preset;
    this.embedding = builder.embedding;
    this.enabled = builder.enabled;
  }

  public static Builder builder() {
    return new Builder();
  }

  public int getMaxChars() {
    return maxChars;
  }

  public int getMaxOverlap() {
    return maxOverlap;
  }

  public String getPreset() {
    return preset;
  }

  public Map<String, Object> getEmbedding() {
    return embedding;
  }

  public Boolean getEnabled() {
    return enabled;
  }

  public Map<String, Object> toMap() {
    Map<String, Object> map = new HashMap<>();
    map.put("max_chars", maxChars);
    map.put("max_overlap", maxOverlap);
    if (preset != null) {
      map.put("preset", preset);
    }
    if (embedding != null) {
      map.put("embedding", embedding);
    }
    if (enabled != null) {
      map.put("enabled", enabled);
    }
    return map;
  }

  public static final class Builder {
    private int maxChars = 1000;
    private int maxOverlap = 200;
    private String preset;
    private Map<String, Object> embedding;
    private Boolean enabled = true;

    private Builder() {
    }

    public Builder maxChars(int maxChars) {
      this.maxChars = maxChars;
      return this;
    }

    public Builder maxOverlap(int maxOverlap) {
      this.maxOverlap = maxOverlap;
      return this;
    }

    public Builder preset(String preset) {
      this.preset = preset;
      return this;
    }

    public Builder embedding(Map<String, Object> embedding) {
      this.embedding = embedding;
      return this;
    }

    public Builder enabled(Boolean enabled) {
      this.enabled = enabled;
      return this;
    }

    public ChunkingConfig build() {
      return new ChunkingConfig(this);
    }
  }

  static ChunkingConfig fromMap(Map<String, Object> map) {
    if (map == null) {
      return null;
    }
    Builder builder = builder();
    Object maxCharsValue = map.get("max_chars");
    if (maxCharsValue instanceof Number) {
      builder.maxChars(((Number) maxCharsValue).intValue());
    }
    Object maxOverlapValue = map.get("max_overlap");
    if (maxOverlapValue instanceof Number) {
      builder.maxOverlap(((Number) maxOverlapValue).intValue());
    }
    Object presetValue = map.get("preset");
    if (presetValue instanceof String) {
      builder.preset((String) presetValue);
    }
    @SuppressWarnings("unchecked")
    Map<String, Object> embeddingMap = map.get("embedding") instanceof Map
        ? (Map<String, Object>) map.get("embedding")
        : null;
    if (embeddingMap != null && !embeddingMap.isEmpty()) {
      builder.embedding(new HashMap<>(embeddingMap));
    }
    if (map.containsKey("enabled")) {
      Object enabledValue = map.get("enabled");
      if (enabledValue instanceof Boolean) {
        builder.enabled((Boolean) enabledValue);
      }
    }
    return builder.build();
  }
}
