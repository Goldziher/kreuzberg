package dev.kreuzberg;

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
}
