package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * Embedding preset metadata.
 *
 * @param name preset name
 * @param chunkSize chunk size
 * @param overlap overlap between chunks
 * @param modelName embedding model identifier
 * @param dimensions embedding vector dimension
 * @param description human-readable description
 */
public record EmbeddingPreset(
    String name,
    int chunkSize,
    int overlap,
    String modelName,
    int dimensions,
    String description
) {
    @JsonCreator
    public EmbeddingPreset(
        @JsonProperty("name") String name,
        @JsonProperty("chunk_size") int chunkSize,
        @JsonProperty("overlap") int overlap,
        @JsonProperty("model_name") String modelName,
        @JsonProperty("dimensions") int dimensions,
        @JsonProperty("description") String description
    ) {
        this.name = name;
        this.chunkSize = chunkSize;
        this.overlap = overlap;
        this.modelName = modelName;
        this.dimensions = dimensions;
        this.description = description;
    }
}
