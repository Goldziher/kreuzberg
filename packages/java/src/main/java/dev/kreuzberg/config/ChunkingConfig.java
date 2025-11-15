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
    private final String embedding;
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

    public String getEmbedding() {
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
        private String embedding;
        private Boolean enabled = true;

        private Builder() { }

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

        public Builder embedding(String embedding) {
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
}
