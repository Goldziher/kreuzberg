package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Objects;
import java.util.Optional;

/**
 * Metadata for an individual page, slide, or sheet.
 *
 * Captures per-page information including dimensions, content counts, and visibility state
 * (for presentations).
 *
 * @since 4.0.0
 */
public final class PageInfo {
    private final long number;
    private final String title;
    private final Double width;
    private final Double height;
    private final Boolean visible;

    @JsonCreator
    public PageInfo(
        @JsonProperty("number") long number,
        @JsonProperty("title") String title,
        @JsonProperty("width") Double width,
        @JsonProperty("height") Double height,
        @JsonProperty("visible") Boolean visible
    ) {
        if (number < 1) {
            throw new IllegalArgumentException("page number must be positive");
        }
        this.number = number;
        this.title = title;
        this.width = width;
        this.height = height;
        this.visible = visible;
    }

    /**
     * Get the page number (1-indexed).
     *
     * @return page number
     */
    public long getNumber() {
        return number;
    }

    /**
     * Get the page title (usually for presentations).
     *
     * @return page title, or empty if not available
     */
    public Optional<String> getTitle() {
        return Optional.ofNullable(title);
    }

    /**
     * Get the page width in points (PDF) or pixels (images).
     *
     * @return page width, or empty if not available
     */
    public Optional<Double> getWidth() {
        return Optional.ofNullable(width);
    }

    /**
     * Get the page height in points (PDF) or pixels (images).
     *
     * @return page height, or empty if not available
     */
    public Optional<Double> getHeight() {
        return Optional.ofNullable(height);
    }

    /**
     * Get the visibility state of this page (for presentations).
     *
     * @return true if visible, false if hidden, empty if not applicable
     */
    public Optional<Boolean> getVisible() {
        return Optional.ofNullable(visible);
    }

    @Override
    public boolean equals(Object obj) {
        if (this == obj) {
            return true;
        }
        if (!(obj instanceof PageInfo)) {
            return false;
        }
        PageInfo other = (PageInfo) obj;
        return number == other.number
            && Objects.equals(title, other.title)
            && Objects.equals(width, other.width)
            && Objects.equals(height, other.height)
            && Objects.equals(visible, other.visible);
    }

    @Override
    public int hashCode() {
        return Objects.hash(number, title, width, height, visible);
    }

    @Override
    public String toString() {
        return "PageInfo{"
            + "number=" + number
            + ", title=" + title
            + ", width=" + width
            + ", height=" + height
            + ", visible=" + visible
            + '}';
    }
}
