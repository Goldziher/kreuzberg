package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.PropertyNamingStrategies;
import java.util.Collections;
import java.util.List;
import java.util.Map;

final class ResultParser {
    private static final ObjectMapper MAPPER = new ObjectMapper()
        .setPropertyNamingStrategy(PropertyNamingStrategies.SNAKE_CASE)
        .configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);

    private static final TypeReference<List<Table>> TABLE_LIST = new TypeReference<>() { };
    private static final TypeReference<List<String>> STRING_LIST = new TypeReference<>() { };
    private static final TypeReference<List<Chunk>> CHUNK_LIST = new TypeReference<>() { };
    private static final TypeReference<List<ExtractedImage>> IMAGE_LIST = new TypeReference<>() { };
    private static final TypeReference<Map<String, Object>> METADATA_MAP = new TypeReference<>() { };

    private ResultParser() {
    }

    static ExtractionResult parse(
        String content,
        String mimeType,
        String tablesJson,
        String detectedLanguagesJson,
        String metadataJson,
        String chunksJson,
        String imagesJson,
        boolean success
    ) throws KreuzbergException {
        try {
            Map<String, Object> metadata = decode(metadataJson, METADATA_MAP, Collections.emptyMap());
            List<Table> tables = decode(tablesJson, TABLE_LIST, List.of());
            List<String> detectedLanguages = decode(detectedLanguagesJson, STRING_LIST, List.of());
            List<Chunk> chunks = decode(chunksJson, CHUNK_LIST, List.of());
            List<ExtractedImage> images = decode(imagesJson, IMAGE_LIST, List.of());

            return new ExtractionResult(
                content != null ? content : "",
                mimeType != null ? mimeType : "",
                metadata,
                tables,
                detectedLanguages,
                chunks,
                images,
                success
            );
        } catch (Exception e) {
            throw new KreuzbergException("Failed to parse extraction result", e);
        }
    }

    private static <T> T decode(String json, TypeReference<T> type, T fallback) throws Exception {
        if (json == null || json.isBlank()) {
            return fallback;
        }
        return MAPPER.readValue(json, type);
    }

    static String toJson(Map<String, Object> map) throws Exception {
        return MAPPER.writeValueAsString(map);
    }

    static ExtractionResult fromJson(String json) throws KreuzbergException {
        if (json == null || json.isBlank()) {
            throw new KreuzbergException("Result JSON cannot be null or empty");
        }
        try {
            WireExtractionResult wire = MAPPER.readValue(json, WireExtractionResult.class);
            return new ExtractionResult(
                wire.content != null ? wire.content : "",
                wire.mimeType != null ? wire.mimeType : "",
                wire.metadata != null ? wire.metadata : Collections.emptyMap(),
                wire.tables != null ? wire.tables : List.of(),
                wire.detectedLanguages != null ? wire.detectedLanguages : List.of(),
                wire.chunks != null ? wire.chunks : List.of(),
                wire.images != null ? wire.images : List.of(),
                wire.success == null || wire.success
            );
        } catch (Exception e) {
            throw new KreuzbergException("Failed to parse result JSON", e);
        }
    }

    static String toJson(ExtractionResult result) throws Exception {
        WireExtractionResult wire = new WireExtractionResult(
            result.getContent(),
            result.getMimeType(),
            result.getMetadata(),
            result.getTables(),
            result.getDetectedLanguages(),
            result.getChunks(),
            result.getImages(),
            result.isSuccess()
        );
        return MAPPER.writeValueAsString(wire);
    }

    static String toJsonValue(Object value) throws Exception {
        return MAPPER.writeValueAsString(value);
    }

    static List<String> parseStringList(String json) throws Exception {
        return MAPPER.readValue(json, STRING_LIST);
    }

    private static final class WireExtractionResult {
        private final String content;
        private final String mimeType;
        private final Map<String, Object> metadata;
        private final List<Table> tables;
        private final List<String> detectedLanguages;
        private final List<Chunk> chunks;
        private final List<ExtractedImage> images;
        private final Boolean success;

        WireExtractionResult(
            @JsonProperty("content") String content,
            @JsonProperty("mime_type") String mimeType,
            @JsonProperty("metadata") Map<String, Object> metadata,
            @JsonProperty("tables") List<Table> tables,
            @JsonProperty("detected_languages") List<String> detectedLanguages,
            @JsonProperty("chunks") List<Chunk> chunks,
            @JsonProperty("images") List<ExtractedImage> images,
            @JsonProperty("success") Boolean success
        ) {
            this.content = content;
            this.mimeType = mimeType;
            this.metadata = metadata;
            this.tables = tables;
            this.detectedLanguages = detectedLanguages;
            this.chunks = chunks;
            this.images = images;
            this.success = success;
        }
    }
}
