package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Collections;
import java.util.List;
import java.util.Objects;
import java.util.Optional;

/**
 * Structure describing pages/slides/sheets within an extracted document.
 *
 * Includes total count, unit type, byte offset boundaries for each page,
 * and detailed per-page metadata.
 *
 * @since 4.0.0
 */
public final class PageStructure {
    private final long totalCount;
    private final PageUnitType unitType;
    private final List<PageBoundary> boundaries;
    private final List<PageInfo> pages;

    @JsonCreator
    public PageStructure(
        @JsonProperty("total_count") long totalCount,
        @JsonProperty("unit_type") PageUnitType unitType,
        @JsonProperty("boundaries") List<PageBoundary> boundaries,
        @JsonProperty("pages") List<PageInfo> pages
    ) {
        if (totalCount < 1) {
            throw new IllegalArgumentException("totalCount must be positive");
        }
        this.totalCount = totalCount;
        this.unitType = Objects.requireNonNull(unitType, "unitType must not be null");
        this.boundaries = boundaries != null ? Collections.unmodifiableList(boundaries) : null;
        this.pages = pages != null ? Collections.unmodifiableList(pages) : null;
    }

    /**
     * Get the total number of pages/slides/sheets in the document.
     *
     * @return total page count
     */
    public long getTotalCount() {
        return totalCount;
    }

    /**
     * Get the type of paginated unit (Page, Slide, or Sheet).
     *
     * @return the unit type
     */
    public PageUnitType getUnitType() {
        return unitType;
    }

    /**
     * Get the byte offset boundaries for each page.
     *
     * Maps byte ranges in the extracted content to page numbers.
     * Used for calculating which pages a chunk spans.
     *
     * @return list of page boundaries, or empty if not available
     */
    public Optional<List<PageBoundary>> getBoundaries() {
        return Optional.ofNullable(boundaries);
    }

    /**
     * Get detailed metadata for each page (optional).
     *
     * Only populated when detailed page information is needed.
     *
     * @return list of page info objects, or empty if not available
     */
    public Optional<List<PageInfo>> getPages() {
        return Optional.ofNullable(pages);
    }

    @Override
    public boolean equals(Object obj) {
        if (this == obj) {
            return true;
        }
        if (!(obj instanceof PageStructure)) {
            return false;
        }
        PageStructure other = (PageStructure) obj;
        return totalCount == other.totalCount
            && unitType == other.unitType
            && Objects.equals(boundaries, other.boundaries)
            && Objects.equals(pages, other.pages);
    }

    @Override
    public int hashCode() {
        return Objects.hash(totalCount, unitType, boundaries, pages);
    }

    @Override
    public String toString() {
        return "PageStructure{"
            + "totalCount=" + totalCount
            + ", unitType=" + unitType
            + ", boundaries=" + (boundaries != null ? boundaries.size() + " items" : "null")
            + ", pages=" + (pages != null ? pages.size() + " items" : "null")
            + '}';
    }
}
