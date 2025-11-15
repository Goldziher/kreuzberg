package dev.kreuzberg;

import com.fasterxml.jackson.databind.annotation.JsonDeserialize;
import java.util.ArrayList;
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
    private final String content;
    private final String mimeType;
    private final Optional<String> language;
    private final Optional<String> date;
    private final Optional<String> subject;
    private final List<Table> tables;
    private final List<String> detectedLanguages;
    private final Map<String, Object> metadata;

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
        Map<String, Object> metadata
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
            && Objects.equals(metadata, other.metadata);
    }

    @Override
    public int hashCode() {
        return Objects.hash(content, mimeType, language, date, subject, tables, detectedLanguages, metadata);
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
        return new ExtractionResult(newContent, mimeType, language, date, subject, tables, detectedLanguages, metadata);
    }

    /**
     * Returns a new ExtractionResult with the specified MIME type.
     *
     * @param newMimeType the new MIME type
     * @return a new ExtractionResult with updated MIME type
     */
    public ExtractionResult withMimeType(String newMimeType) {
        return new ExtractionResult(content, newMimeType, language, date, subject, tables, detectedLanguages, metadata);
    }

    /**
     * Returns a new ExtractionResult with the specified language.
     *
     * @param newLanguage the new language (may be null)
     * @return a new ExtractionResult with updated language
     */
    public ExtractionResult withLanguage(String newLanguage) {
        return new ExtractionResult(content, mimeType, Optional.ofNullable(newLanguage), date, subject, tables,
                detectedLanguages, metadata);
    }

    /**
     * Returns a new ExtractionResult with the specified date.
     *
     * @param newDate the new date (may be null)
     * @return a new ExtractionResult with updated date
     */
    public ExtractionResult withDate(String newDate) {
        return new ExtractionResult(content, mimeType, language, Optional.ofNullable(newDate), subject, tables,
                detectedLanguages, metadata);
    }

    /**
     * Returns a new ExtractionResult with the specified subject.
     *
     * @param newSubject the new subject (may be null)
     * @return a new ExtractionResult with updated subject
     */
    public ExtractionResult withSubject(String newSubject) {
        return new ExtractionResult(content, mimeType, language, date, Optional.ofNullable(newSubject), tables,
                detectedLanguages, metadata);
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
            com.fasterxml.jackson.databind.JsonNode node = p.getCodec().readTree(p);

            // Read top-level fields
            String content = node.get("content").asText();
            String mimeType = node.has("mime_type") ? node.get("mime_type").asText() : node.get("mimeType").asText();

            // Read metadata fields (may be nested or flattened)
            String language = null;
            String date = null;
            String subject = null;

            if (node.has("metadata") && node.get("metadata").isObject()) {
                // Rust format: nested metadata object
                com.fasterxml.jackson.databind.JsonNode metadataNode = node.get("metadata");
                if (metadataNode.has("language") && !metadataNode.get("language").isNull()) {
                    language = metadataNode.get("language").asText();
                }
                if (metadataNode.has("date") && !metadataNode.get("date").isNull()) {
                    date = metadataNode.get("date").asText();
                }
                if (metadataNode.has("subject") && !metadataNode.get("subject").isNull()) {
                    subject = metadataNode.get("subject").asText();
                }
            } else {
                // Java format: flattened fields
                if (node.has("language") && !node.get("language").isNull()) {
                    language = node.get("language").asText();
                }
                if (node.has("date") && !node.get("date").isNull()) {
                    date = node.get("date").asText();
                }
                if (node.has("subject") && !node.get("subject").isNull()) {
                    subject = node.get("subject").asText();
                }
            }

            // Read tables
            List<Table> tables = Collections.emptyList();
            if (node.has("tables") && node.get("tables").isArray()) {
                tables = new ArrayList<>();
                for (com.fasterxml.jackson.databind.JsonNode tableNode : node.get("tables")) {
                    // Parse each table - simplified for now
                    tables.add(parseTable(tableNode));
                }
            }

            // Read detected_languages
            List<String> detectedLanguages = Collections.emptyList();
            if (node.has("detected_languages") && node.get("detected_languages").isArray()) {
                detectedLanguages = new ArrayList<>();
                for (com.fasterxml.jackson.databind.JsonNode langNode : node.get("detected_languages")) {
                    detectedLanguages.add(langNode.asText());
                }
            }

            return new ExtractionResult(
                content,
                mimeType,
                Optional.ofNullable(language),
                Optional.ofNullable(date),
                Optional.ofNullable(subject),
                tables,
                detectedLanguages,
                Collections.emptyMap() // metadata map not used in callbacks
            );
        }

        private Table parseTable(com.fasterxml.jackson.databind.JsonNode tableNode) {
            List<List<String>> cells = new ArrayList<>();
            if (tableNode.has("cells") && tableNode.get("cells").isArray()) {
                for (com.fasterxml.jackson.databind.JsonNode rowNode : tableNode.get("cells")) {
                    List<String> row = new ArrayList<>();
                    if (rowNode.isArray()) {
                        for (com.fasterxml.jackson.databind.JsonNode cellNode : rowNode) {
                            row.add(cellNode.asText());
                        }
                    }
                    cells.add(row);
                }
            }
            String markdown = tableNode.has("markdown") ? tableNode.get("markdown").asText() : "";
            int pageNumber = tableNode.has("page_number") ? tableNode.get("page_number").asInt() : 0;

            return new Table(cells, markdown, pageNumber);
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
