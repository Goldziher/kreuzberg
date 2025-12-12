package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * Byte offset boundary for a page within the extracted content.
 *
 * Maps byte ranges in the extracted content string (UTF-8 boundaries) to page numbers,
 * enabling page range calculation for chunks.
 *
 * @param byteStart byte offset where this page starts in the content string (UTF-8 boundary, inclusive)
 * @param byteEnd byte offset where this page ends in the content string (UTF-8 boundary, exclusive)
 * @param pageNumber page number (1-indexed)
 * @since 4.0.0
 */
public record PageBoundary(
    @JsonProperty("byte_start") long byteStart,
    @JsonProperty("byte_end") long byteEnd,
    @JsonProperty("page_number") long pageNumber
) {
    @JsonCreator
    public PageBoundary {
        if (byteStart < 0) {
            throw new IllegalArgumentException("byteStart must be non-negative");
        }
        if (byteEnd < byteStart) {
            throw new IllegalArgumentException("byteEnd must be >= byteStart");
        }
        if (pageNumber < 1) {
            throw new IllegalArgumentException("pageNumber must be positive");
        }
    }
}
