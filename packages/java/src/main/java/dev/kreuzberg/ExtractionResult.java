package dev.kreuzberg;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.annotation.JsonDeserialize;
import java.util.ArrayList;
import java.util.Base64;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;

/**
 * Result of a document extraction operation.
 *
 * <p>Contains the extracted text content and metadata from a document.</p>
 */
@JsonDeserialize(using = ExtractionResult.Deserializer.class)
@com.fasterxml.jackson.databind.annotation.JsonSerialize(using = ExtractionResult.Serializer.class)
public final class ExtractionResult {
    private static final Base64.Decoder BASE64_DECODER = Base64.getDecoder();
    private static final TypeReference<Map<String, Object>> METADATA_TYPE = new TypeReference<>() { };

    private final String content;
    private final String mimeType;
    private final Optional<String> language;
    private final Optional<String> date;
    private final Optional<String> subject;
    private final List<Table> tables;
    private final List<String> detectedLanguages;
    private final Map<String, Object> metadata;
    private final List<Chunk> chunks;
    private final List<ExtractedImage> images;

    /**
     * Creates a new extraction result.
     *
     * @param content the extracted text content (must not be null)
     * @param mimeType the detected MIME type (must not be null)
     * @param language the detected language (may be null)
     * @param date the document date (may be null)
     * @param subject the document subject (may be null)
     * @param tables the extracted tables (may be null)
     * @param detectedLanguages the detected languages (may be null)
     * @param metadata the extraction metadata (may be null)
     * @throws NullPointerException if content or mimeType is null
     */
    public ExtractionResult(
        String content,
        String mimeType,
        Optional<String> language,
        Optional<String> date,
        Optional<String> subject,
        List<Table> tables,
        List<String> detectedLanguages,
        Map<String, Object> metadata,
        List<Chunk> chunks,
        List<ExtractedImage> images
    ) {
        this.content = Objects.requireNonNull(content, "content must not be null");
        this.mimeType = Objects.requireNonNull(mimeType, "mimeType must not be null");
        this.language = Optional.ofNullable(language).flatMap(opt -> opt);
        this.date = Optional.ofNullable(date).flatMap(opt -> opt);
        this.subject = Optional.ofNullable(subject).flatMap(opt -> opt);
        this.tables = tables != null ? Collections.unmodifiableList(tables) : Collections.emptyList();
        this.detectedLanguages = detectedLanguages != null
            ? Collections.unmodifiableList(detectedLanguages)
            : Collections.emptyList();
        this.metadata = metadata != null
            ? Collections.unmodifiableMap(new HashMap<>(metadata))
            : Collections.emptyMap();
        if (chunks != null && !chunks.isEmpty()) {
            this.chunks = Collections.unmodifiableList(new ArrayList<>(chunks));
        } else {
            this.chunks = Collections.emptyList();
        }
        if (images != null && !images.isEmpty()) {
            this.images = Collections.unmodifiableList(new ArrayList<>(images));
        } else {
            this.images = Collections.emptyList();
        }
    }

    /**
     * Creates an extraction result from raw values.
     *
     * @param content the extracted text content
     * @param mimeType the detected MIME type
     * @param language the detected language (may be null)
     * @param date the document date (may be null)
     * @param subject the document subject (may be null)
     * @return a new ExtractionResult
     */
    static ExtractionResult of(
        String content,
        String mimeType,
        String language,
        String date,
        String subject
    ) {
        return new ExtractionResult(
            content,
            mimeType,
            Optional.ofNullable(language),
            Optional.ofNullable(date),
            Optional.ofNullable(subject),
            null,
            null,
            null,
            null,
            null
        );
    }

    /**
     * Returns the extracted text content.
     *
     * @return the content
     */
    public String getContent() {
        return content;
    }

    /**
     * Returns the detected MIME type.
     *
     * @return the MIME type
     */
    public String getMimeType() {
        return mimeType;
    }

    /**
     * Returns the detected language.
     *
     * @return the language, if available
     */
    public Optional<String> getLanguage() {
        return language;
    }

    /**
     * Returns the document date.
     *
     * @return the date, if available
     */
    public Optional<String> getDate() {
        return date;
    }

    /**
     * Returns the document subject.
     *
     * @return the subject, if available
     */
    public Optional<String> getSubject() {
        return subject;
    }

    /**
     * Returns the extracted tables.
     *
     * @return an unmodifiable list of tables
     */
    public List<Table> getTables() {
        return tables;
    }

    /**
     * Returns the detected languages.
     *
     * @return an unmodifiable list of detected language codes
     */
    public List<String> getDetectedLanguages() {
        return detectedLanguages;
    }

    /**
     * Returns the extraction metadata.
     *
     * @return an unmodifiable map of metadata
     */
    public Map<String, Object> getMetadata() {
        return metadata;
    }

    /**
     * Returns the extracted chunks.
     *
     * @return an unmodifiable list of chunks (may be empty)
     */
    public List<Chunk> getChunks() {
        return chunks;
    }

    /**
     * Returns the extracted images.
     *
     * @return an unmodifiable list of images (may be empty)
     */
    public List<ExtractedImage> getImages() {
        return images;
    }

    /**
     * Returns the content (for compatibility with record-style access).
     *
     * @return the content
     */
    public String content() {
        return content;
    }

    /**
     * Returns the MIME type (for compatibility with record-style access).
     *
     * @return the MIME type
     */
    public String mimeType() {
        return mimeType;
    }

    /**
     * Returns the language (for compatibility with record-style access).
     *
     * @return the language, if available
     */
    public Optional<String> language() {
        return language;
    }

    /**
     * Returns the date (for compatibility with record-style access).
     *
     * @return the date, if available
     */
    public Optional<String> date() {
        return date;
    }

    /**
     * Returns the subject (for compatibility with record-style access).
     *
     * @return the subject, if available
     */
    public Optional<String> subject() {
        return subject;
    }

    /**
     * Returns the chunks (record-style accessor).
     *
     * @return chunk list
     */
    public List<Chunk> chunks() {
        return chunks;
    }

    /**
     * Returns the images (record-style accessor).
     *
     * @return image list
     */
    public List<ExtractedImage> images() {
        return images;
    }

    @Override
    public String toString() {
        final int contentPreviewLength = 100;
        return "ExtractionResult{"
            + "content='" + truncate(content, contentPreviewLength) + "',"
            + " mimeType='" + mimeType + "',"
            + " language=" + language
            + ", date=" + date
            + ", subject=" + subject
            + ", tables=" + tables.size()
            + ", detectedLanguages=" + detectedLanguages
            + ", chunks=" + chunks.size()
            + ", images=" + images.size()
            + '}';
    }

    @Override
    public boolean equals(Object obj) {
        if (this == obj) {
            return true;
        }
        if (!(obj instanceof ExtractionResult)) {
            return false;
        }
        ExtractionResult other = (ExtractionResult) obj;
        return Objects.equals(content, other.content)
            && Objects.equals(mimeType, other.mimeType)
            && Objects.equals(language, other.language)
            && Objects.equals(date, other.date)
            && Objects.equals(subject, other.subject)
            && Objects.equals(tables, other.tables)
            && Objects.equals(detectedLanguages, other.detectedLanguages)
            && Objects.equals(metadata, other.metadata)
            && Objects.equals(chunks, other.chunks)
            && Objects.equals(images, other.images);
    }

    @Override
    public int hashCode() {
        return Objects.hash(
            content,
            mimeType,
            language,
            date,
            subject,
            tables,
            detectedLanguages,
            metadata,
            chunks,
            images
        );
    }

    private static String truncate(String str, int maxLength) {
        if (str == null) {
            return "null";
        }
        if (str.length() <= maxLength) {
            return str;
        }
        return str.substring(0, maxLength) + "...";
    }

    /**
     * Returns a new ExtractionResult with the specified content.
     *
     * @param newContent the new content
     * @return a new ExtractionResult with updated content
     */
    public ExtractionResult withContent(String newContent) {
        return new ExtractionResult(
            newContent,
            mimeType,
            language,
            date,
            subject,
            tables,
            detectedLanguages,
            metadata,
            chunks,
            images
        );
    }

    /**
     * Returns a new ExtractionResult with the specified MIME type.
     *
     * @param newMimeType the new MIME type
     * @return a new ExtractionResult with updated MIME type
     */
    public ExtractionResult withMimeType(String newMimeType) {
        return new ExtractionResult(
            content,
            newMimeType,
            language,
            date,
            subject,
            tables,
            detectedLanguages,
            metadata,
            chunks,
            images
        );
    }

    /**
     * Returns a new ExtractionResult with the specified language.
     *
     * @param newLanguage the new language (may be null)
     * @return a new ExtractionResult with updated language
     */
    public ExtractionResult withLanguage(String newLanguage) {
        return new ExtractionResult(content, mimeType, Optional.ofNullable(newLanguage), date, subject, tables,
                detectedLanguages, metadata, chunks, images);
    }

    /**
     * Returns a new ExtractionResult with the specified date.
     *
     * @param newDate the new date (may be null)
     * @return a new ExtractionResult with updated date
     */
    public ExtractionResult withDate(String newDate) {
        return new ExtractionResult(content, mimeType, language, Optional.ofNullable(newDate), subject, tables,
                detectedLanguages, metadata, chunks, images);
    }

    /**
     * Returns a new ExtractionResult with the specified subject.
     *
     * @param newSubject the new subject (may be null)
     * @return a new ExtractionResult with updated subject
     */
    public ExtractionResult withSubject(String newSubject) {
        return new ExtractionResult(content, mimeType, language, date, Optional.ofNullable(newSubject), tables,
                detectedLanguages, metadata, chunks, images);
    }

    static ExtractionResult fromJsonNode(JsonNode node, ObjectMapper mapper) {
        if (node == null || node.isNull()) {
            throw new IllegalArgumentException("ExtractionResult JSON node must not be null");
        }

        String content = node.has("content") ? node.get("content").asText() : "";
        String mimeType = node.has("mime_type")
            ? node.get("mime_type").asText()
            : node.path("mimeType").asText("");

        JsonNode metadataNode = node.get("metadata");
        Map<String, Object> parsedMetadata = metadataNode != null && !metadataNode.isNull()
            ? mapper.convertValue(metadataNode, METADATA_TYPE)
            : Collections.emptyMap();

        Optional<String> parsedLanguage = readMetadataString(metadataNode, node, "language");
        Optional<String> parsedDate = readMetadataString(metadataNode, node, "date");
        Optional<String> parsedSubject = readMetadataString(metadataNode, node, "subject");

        List<Table> parsedTables = parseTablesNode(node.get("tables"));
        List<String> parsedLanguages = parseDetectedLanguagesNode(node.get("detected_languages"));
        List<Chunk> parsedChunks = parseChunksNode(node.get("chunks"), mapper);
        List<ExtractedImage> parsedImages = parseImagesNode(node.get("images"), mapper);

        return new ExtractionResult(
            content,
            mimeType,
            parsedLanguage,
            parsedDate,
            parsedSubject,
            parsedTables,
            parsedLanguages,
            parsedMetadata,
            parsedChunks,
            parsedImages
        );
    }

    static List<Chunk> parseChunksJson(String json, ObjectMapper mapper) {
        if (json == null || json.isEmpty()) {
            return Collections.emptyList();
        }
        try {
            JsonNode node = mapper.readTree(json);
            return parseChunksNode(node, mapper);
        } catch (Exception e) {
            return Collections.emptyList();
        }
    }

    static List<ExtractedImage> parseImagesJson(String json, ObjectMapper mapper) {
        if (json == null || json.isEmpty()) {
            return Collections.emptyList();
        }
        try {
            JsonNode node = mapper.readTree(json);
            return parseImagesNode(node, mapper);
        } catch (Exception e) {
            return Collections.emptyList();
        }
    }

    private static Optional<String> readMetadataString(JsonNode metadataNode, JsonNode rootNode, String field) {
        if (metadataNode != null && metadataNode.has(field) && !metadataNode.get(field).isNull()) {
            return Optional.of(metadataNode.get(field).asText());
        }
        if (rootNode != null && rootNode.has(field) && !rootNode.get(field).isNull()) {
            return Optional.of(rootNode.get(field).asText());
        }
        return Optional.empty();
    }

    private static List<Table> parseTablesNode(JsonNode tablesNode) {
        if (tablesNode == null || !tablesNode.isArray()) {
            return Collections.emptyList();
        }
        List<Table> parsed = new ArrayList<>();
        for (JsonNode tableNode : tablesNode) {
            parsed.add(parseTableNode(tableNode));
        }
        return parsed;
    }

    private static Table parseTableNode(JsonNode tableNode) {
        List<List<String>> cells = new ArrayList<>();
        if (tableNode != null && tableNode.has("cells") && tableNode.get("cells").isArray()) {
            for (JsonNode rowNode : tableNode.get("cells")) {
                List<String> row = new ArrayList<>();
                if (rowNode.isArray()) {
                    for (JsonNode cellNode : rowNode) {
                        row.add(cellNode.asText());
                    }
                }
                cells.add(row);
            }
        }
        String markdown = tableNode != null && tableNode.has("markdown") ? tableNode.get("markdown").asText("") : "";
        int pageNumber = tableNode != null && tableNode.has("page_number") ? tableNode.get("page_number").asInt(1) : 1;
        return new Table(cells, markdown, pageNumber);
    }

    private static List<String> parseDetectedLanguagesNode(JsonNode languagesNode) {
        if (languagesNode == null || !languagesNode.isArray()) {
            return Collections.emptyList();
        }
        List<String> languages = new ArrayList<>();
        for (JsonNode langNode : languagesNode) {
            languages.add(langNode.asText());
        }
        return languages;
    }

    private static List<Chunk> parseChunksNode(JsonNode chunksNode, ObjectMapper mapper) {
        if (chunksNode == null || !chunksNode.isArray()) {
            return Collections.emptyList();
        }
        List<Chunk> parsed = new ArrayList<>();
        for (JsonNode chunkNode : chunksNode) {
            String chunkContent = chunkNode.has("content") ? chunkNode.get("content").asText() : null;
            JsonNode metadataNode = chunkNode.get("metadata");
            if (chunkContent == null || metadataNode == null || !metadataNode.isObject()) {
                continue;
            }
            ChunkMetadata metadata = parseChunkMetadataNode(metadataNode);
            List<Double> embedding = parseEmbeddingNode(chunkNode.get("embedding"));
            parsed.add(new Chunk(chunkContent, embedding, metadata));
        }
        return parsed;
    }

    private static ChunkMetadata parseChunkMetadataNode(JsonNode node) {
        int charStart = node.path("char_start").asInt();
        int charEnd = node.path("char_end").asInt();
        Integer tokenCount = node.hasNonNull("token_count") ? node.get("token_count").asInt() : null;
        int chunkIndex = node.path("chunk_index").asInt();
        int totalChunks = node.path("total_chunks").asInt();
        return new ChunkMetadata(charStart, charEnd, tokenCount, chunkIndex, totalChunks);
    }

    private static List<Double> parseEmbeddingNode(JsonNode embeddingNode) {
        if (embeddingNode == null || !embeddingNode.isArray()) {
            return null;
        }
        List<Double> embedding = new ArrayList<>();
        for (JsonNode valueNode : embeddingNode) {
            if (valueNode.isNumber()) {
                embedding.add(valueNode.asDouble());
            }
        }
        return embedding.isEmpty() ? null : embedding;
    }

    private static List<ExtractedImage> parseImagesNode(JsonNode imagesNode, ObjectMapper mapper) {
        if (imagesNode == null || !imagesNode.isArray()) {
            return Collections.emptyList();
        }
        List<ExtractedImage> images = new ArrayList<>();
        for (JsonNode imageNode : imagesNode) {
            ExtractedImage image = parseImageNode(imageNode, mapper);
            if (image != null) {
                images.add(image);
            }
        }
        return images;
    }

    private static ExtractedImage parseImageNode(JsonNode node, ObjectMapper mapper) {
        if (node == null) {
            return null;
        }
        JsonNode dataNode = node.get("data");
        String format = textOrNull(node, "format");
        if (dataNode == null || dataNode.isNull() || format == null) {
            return null;
        }
        byte[] data = BASE64_DECODER.decode(dataNode.asText(""));
        int imageIndex = node.path("image_index").asInt();
        Integer pageNumber = node.hasNonNull("page_number") ? node.get("page_number").asInt() : null;
        Integer width = node.hasNonNull("width") ? node.get("width").asInt() : null;
        Integer height = node.hasNonNull("height") ? node.get("height").asInt() : null;
        String colorspace = textOrNull(node, "colorspace");
        Integer bitsPerComponent = node.hasNonNull("bits_per_component")
            ? node.get("bits_per_component").asInt()
            : null;
        boolean isMask = node.path("is_mask").asBoolean(false);
        String description = textOrNull(node, "description");

        ExtractionResult ocrResult = null;
        JsonNode ocrNode = node.get("ocr_result");
        if (ocrNode != null && !ocrNode.isNull()) {
            try {
                ocrResult = fromJsonNode(ocrNode, mapper);
            } catch (Exception ignored) {
                // Ignore OCR parse failures to avoid interrupting entire extraction
            }
        }

        return new ExtractedImage(
            data,
            format,
            imageIndex,
            pageNumber,
            width,
            height,
            colorspace,
            bitsPerComponent,
            isMask,
            description,
            ocrResult
        );
    }

    private static String textOrNull(JsonNode node, String field) {
        if (node != null && node.has(field) && !node.get(field).isNull()) {
            return node.get(field).asText();
        }
        return null;
    }

    /**
     * Custom deserializer for ExtractionResult that handles Rust FFI JSON format.
     *
     * <p>Rust serializes ExtractionResult with snake_case fields (mime_type) and nested metadata,
     * while Java uses camelCase (mimeType) with flattened fields.</p>
     */
    static class Deserializer extends com.fasterxml.jackson.databind.JsonDeserializer<ExtractionResult> {
        @Override
        public ExtractionResult deserialize(
                com.fasterxml.jackson.core.JsonParser p,
                com.fasterxml.jackson.databind.DeserializationContext ctxt
        ) throws java.io.IOException {
            com.fasterxml.jackson.core.ObjectCodec codec = p.getCodec();
            ObjectMapper mapper = codec instanceof ObjectMapper
                ? (ObjectMapper) codec
                : new ObjectMapper();
            JsonNode node = mapper.readTree(p);
            return ExtractionResult.fromJsonNode(node, mapper);
        }
    }

    /**
     * Custom serializer for ExtractionResult that produces Rust FFI JSON format.
     *
     * <p>Java uses camelCase (mimeType) with flattened fields, but Rust expects
     * snake_case (mime_type) with nested metadata.</p>
     */
    static class Serializer extends com.fasterxml.jackson.databind.JsonSerializer<ExtractionResult> {
        @Override
        public void serialize(
                ExtractionResult value,
                com.fasterxml.jackson.core.JsonGenerator gen,
                com.fasterxml.jackson.databind.SerializerProvider serializers
        ) throws java.io.IOException {
            gen.writeStartObject();

            // Write top-level fields
            gen.writeStringField("content", value.content);
            gen.writeStringField("mime_type", value.mimeType);

            // Write nested metadata object
            gen.writeObjectFieldStart("metadata");
            if (value.language.isPresent()) {
                gen.writeStringField("language", value.language.get());
            }
            if (value.date.isPresent()) {
                gen.writeStringField("date", value.date.get());
            }
            if (value.subject.isPresent()) {
                gen.writeStringField("subject", value.subject.get());
            }
            gen.writeEndObject();

            // Write tables array
            gen.writeArrayFieldStart("tables");
            for (Table table : value.tables) {
                gen.writeStartObject();
                gen.writeArrayFieldStart("cells");
                for (List<String> row : table.cells()) {
                    gen.writeStartArray();
                    for (String cell : row) {
                        gen.writeString(cell);
                    }
                    gen.writeEndArray();
                }
                gen.writeEndArray();
                gen.writeStringField("markdown", table.markdown());
                gen.writeNumberField("page_number", table.pageNumber());
                gen.writeEndObject();
            }
            gen.writeEndArray();

            // Write detected_languages if not empty
            if (!value.detectedLanguages.isEmpty()) {
                gen.writeArrayFieldStart("detected_languages");
                for (String lang : value.detectedLanguages) {
                    gen.writeString(lang);
                }
                gen.writeEndArray();
            }

            gen.writeEndObject();
        }
    }
}
