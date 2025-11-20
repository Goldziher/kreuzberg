package dev.kreuzberg;

import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;

/**
 * Result of a document extraction operation.
 *
 * <p>Includes extracted content, tables, metadata, detected languages, text chunks, images, and success flag.</p>
 */
public final class ExtractionResult {
    private final String content;
    private final String mimeType;
    private final Map<String, Object> metadata;
    private final List<Table> tables;
    private final List<String> detectedLanguages;
    private final List<Chunk> chunks;
    private final List<ExtractedImage> images;
    private final boolean success;
    private final Optional<String> language;
    private final Optional<String> date;
    private final Optional<String> subject;

    ExtractionResult(
        String content,
        String mimeType,
        Map<String, Object> metadata,
        List<Table> tables,
        List<String> detectedLanguages,
        List<Chunk> chunks,
        List<ExtractedImage> images,
        boolean success
    ) {
        this.content = Objects.requireNonNull(content, "content must not be null");
        this.mimeType = Objects.requireNonNull(mimeType, "mimeType must not be null");
        this.metadata = Collections.unmodifiableMap(metadata != null ? metadata : Collections.emptyMap());
        this.tables = Collections.unmodifiableList(tables != null ? tables : Collections.emptyList());
        if (detectedLanguages != null) {
            this.detectedLanguages = Collections.unmodifiableList(detectedLanguages);
        } else {
            this.detectedLanguages = List.of();
        }
        this.chunks = Collections.unmodifiableList(chunks != null ? chunks : List.of());
        this.images = Collections.unmodifiableList(images != null ? images : List.of());
        this.success = success;
        this.language = Optional.ofNullable((String) this.metadata.get("language"));
        this.date = Optional.ofNullable((String) this.metadata.get("date"));
        this.subject = Optional.ofNullable((String) this.metadata.get("subject"));
    }

    public String getContent() {
        return content;
    }

    public String getMimeType() {
        return mimeType;
    }

    public Map<String, Object> getMetadata() {
        return metadata;
    }

    public List<Table> getTables() {
        return tables;
    }

    public List<String> getDetectedLanguages() {
        return detectedLanguages;
    }

    public List<Chunk> getChunks() {
        return chunks;
    }

    public List<ExtractedImage> getImages() {
        return images;
    }

    public boolean isSuccess() {
        return success;
    }

    public Optional<String> getLanguage() {
        return language;
    }

    public Optional<String> getDate() {
        return date;
    }

    public Optional<String> getSubject() {
        return subject;
    }

    @Override
    public String toString() {
        return "ExtractionResult{"
            + "contentLength=" + content.length()
            + ", mimeType='" + mimeType + '\''
            + ", tables=" + tables.size()
            + ", detectedLanguages=" + detectedLanguages
            + ", chunks=" + chunks.size()
            + ", images=" + images.size()
            + ", success=" + success
            + '}';
    }
}
