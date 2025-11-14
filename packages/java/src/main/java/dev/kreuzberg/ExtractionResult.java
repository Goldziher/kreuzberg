package dev.kreuzberg;

import java.util.Objects;
import java.util.Optional;

/**
 * Result of a document extraction operation.
 *
 * <p>Contains the extracted text content and metadata from a document.</p>
 *
 * @param content the extracted text content
 * @param mimeType the detected MIME type of the document
 * @param language the detected language (ISO 639 code), if available
 * @param date the document date, if available
 * @param subject the document subject/description, if available
 */
public record ExtractionResult(
    String content,
    String mimeType,
    Optional<String> language,
    Optional<String> date,
    Optional<String> subject
) {
    /**
     * Creates a new extraction result.
     *
     * @param content the extracted text content (must not be null)
     * @param mimeType the detected MIME type (must not be null)
     * @param language the detected language (may be null)
     * @param date the document date (may be null)
     * @param subject the document subject (may be null)
     * @throws NullPointerException if content or mimeType is null
     */
    public ExtractionResult {
        Objects.requireNonNull(content, "content must not be null");
        Objects.requireNonNull(mimeType, "mimeType must not be null");
        language = Optional.ofNullable(language).flatMap(opt -> opt);
        date = Optional.ofNullable(date).flatMap(opt -> opt);
        subject = Optional.ofNullable(subject).flatMap(opt -> opt);
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
            Optional.ofNullable(subject)
        );
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
            + '}';
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
        return new ExtractionResult(newContent, mimeType, language, date, subject);
    }

    /**
     * Returns a new ExtractionResult with the specified MIME type.
     *
     * @param newMimeType the new MIME type
     * @return a new ExtractionResult with updated MIME type
     */
    public ExtractionResult withMimeType(String newMimeType) {
        return new ExtractionResult(content, newMimeType, language, date, subject);
    }

    /**
     * Returns a new ExtractionResult with the specified language.
     *
     * @param newLanguage the new language (may be null)
     * @return a new ExtractionResult with updated language
     */
    public ExtractionResult withLanguage(String newLanguage) {
        return new ExtractionResult(content, mimeType, Optional.ofNullable(newLanguage), date, subject);
    }

    /**
     * Returns a new ExtractionResult with the specified date.
     *
     * @param newDate the new date (may be null)
     * @return a new ExtractionResult with updated date
     */
    public ExtractionResult withDate(String newDate) {
        return new ExtractionResult(content, mimeType, language, Optional.ofNullable(newDate), subject);
    }

    /**
     * Returns a new ExtractionResult with the specified subject.
     *
     * @param newSubject the new subject (may be null)
     * @return a new ExtractionResult with updated subject
     */
    public ExtractionResult withSubject(String newSubject) {
        return new ExtractionResult(content, mimeType, language, date, Optional.ofNullable(newSubject));
    }
}
