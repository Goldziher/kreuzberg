package dev.kreuzberg.config;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.HashMap;
import java.util.Map;
import java.util.Objects;

/**
 * Configuration for page tracking during extraction.
 *
 * Enables extraction and insertion of page boundaries, and tracking of which pages
 * chunks span across.
 *
 * @since 4.0.0
 */
public final class PageConfig {
    private static final String DEFAULT_MARKER_FORMAT = "\n\n<!-- PAGE {page_num} -->\n\n";

    private final boolean extractPages;
    private final boolean insertPageMarkers;
    private final String markerFormat;

    private PageConfig(Builder builder) {
        this.extractPages = builder.extractPages;
        this.insertPageMarkers = builder.insertPageMarkers;
        this.markerFormat = builder.markerFormat != null ? builder.markerFormat : DEFAULT_MARKER_FORMAT;
    }

    @JsonCreator
    public PageConfig(
        @JsonProperty("extract_pages") boolean extractPages,
        @JsonProperty("insert_page_markers") boolean insertPageMarkers,
        @JsonProperty("marker_format") String markerFormat
    ) {
        this.extractPages = extractPages;
        this.insertPageMarkers = insertPageMarkers;
        this.markerFormat = markerFormat != null ? markerFormat : DEFAULT_MARKER_FORMAT;
    }

    /**
     * Create a new builder for PageConfig.
     *
     * @return a new builder instance
     */
    public static Builder builder() {
        return new Builder();
    }

    /**
     * Whether to extract pages as a separate array in the result.
     *
     * @return true if pages should be extracted separately
     */
    public boolean isExtractPages() {
        return extractPages;
    }

    /**
     * Whether to insert page markers in the main content string.
     *
     * @return true if page markers should be inserted
     */
    public boolean isInsertPageMarkers() {
        return insertPageMarkers;
    }

    /**
     * Get the page marker format string.
     *
     * Uses {page_num} as a placeholder for the page number.
     *
     * @return marker format string
     */
    public String getMarkerFormat() {
        return markerFormat;
    }

    /**
     * Convert to a map representation for serialization.
     *
     * @return map of configuration values
     */
    public Map<String, Object> toMap() {
        Map<String, Object> map = new HashMap<>();
        map.put("extract_pages", extractPages);
        map.put("insert_page_markers", insertPageMarkers);
        map.put("marker_format", markerFormat);
        return map;
    }

    /**
     * Create PageConfig from a map representation.
     *
     * @param map configuration map
     * @return parsed PageConfig
     */
    public static PageConfig fromMap(Map<String, Object> map) {
        if (map == null) {
            return new PageConfig(false, false, DEFAULT_MARKER_FORMAT);
        }
        return new PageConfig(
            asBoolean(map.get("extract_pages"), false),
            asBoolean(map.get("insert_page_markers"), false),
            (String) map.get("marker_format")
        );
    }

    private static boolean asBoolean(Object value, boolean defaultValue) {
        if (value instanceof Boolean) {
            return (Boolean) value;
        }
        if (value instanceof Number) {
            return ((Number) value).intValue() != 0;
        }
        if (value instanceof String) {
            return Boolean.parseBoolean((String) value);
        }
        return defaultValue;
    }

    @Override
    public boolean equals(Object obj) {
        if (this == obj) {
            return true;
        }
        if (!(obj instanceof PageConfig)) {
            return false;
        }
        PageConfig other = (PageConfig) obj;
        return extractPages == other.extractPages
            && insertPageMarkers == other.insertPageMarkers
            && Objects.equals(markerFormat, other.markerFormat);
    }

    @Override
    public int hashCode() {
        return Objects.hash(extractPages, insertPageMarkers, markerFormat);
    }

    @Override
    public String toString() {
        return "PageConfig{"
            + "extractPages=" + extractPages
            + ", insertPageMarkers=" + insertPageMarkers
            + ", markerFormat='" + markerFormat + '\''
            + '}';
    }

    /**
     * Builder for PageConfig with fluent interface.
     */
    public static final class Builder {
        private boolean extractPages = false;
        private boolean insertPageMarkers = false;
        private String markerFormat;

        private Builder() {
        }

        /**
         * Set whether to extract pages as a separate array.
         *
         * @param extractPages whether to extract pages
         * @return this builder for chaining
         */
        public Builder extractPages(boolean extractPages) {
            this.extractPages = extractPages;
            return this;
        }

        /**
         * Set whether to insert page markers in the content.
         *
         * @param insertPageMarkers whether to insert markers
         * @return this builder for chaining
         */
        public Builder insertPageMarkers(boolean insertPageMarkers) {
            this.insertPageMarkers = insertPageMarkers;
            return this;
        }

        /**
         * Set the page marker format string.
         *
         * Use {page_num} as a placeholder for the page number.
         *
         * @param markerFormat marker format string
         * @return this builder for chaining
         */
        public Builder markerFormat(String markerFormat) {
            this.markerFormat = markerFormat;
            return this;
        }

        /**
         * Build the PageConfig instance.
         *
         * @return configured PageConfig
         */
        public PageConfig build() {
            return new PageConfig(this);
        }
    }
}
