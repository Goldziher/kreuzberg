package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Type of paginated unit in a document.
 *
 * @since 4.0.0
 */
public enum PageUnitType {
    /**
     * Standard document pages (PDF, DOCX, images).
     */
    PAGE("Page"),

    /**
     * Presentation slides (PPTX, ODP).
     */
    SLIDE("Slide"),

    /**
     * Spreadsheet sheets (XLSX, ODS).
     */
    SHEET("Sheet");

    private final String wireValue;

    PageUnitType(String wireValue) {
        this.wireValue = wireValue;
    }

    /**
     * Get the wire format value for this unit type.
     *
     * @return wire value used in serialization
     */
    @JsonValue
    public String wireValue() {
        return wireValue;
    }

    /**
     * Parse a PageUnitType from its wire value.
     *
     * @param wireValue the wire format value
     * @return the corresponding PageUnitType
     * @throws IllegalArgumentException if the value is not recognized
     */
    public static PageUnitType fromWireValue(String wireValue) {
        for (PageUnitType type : values()) {
            if (type.wireValue.equals(wireValue)) {
                return type;
            }
        }
        throw new IllegalArgumentException("Unknown PageUnitType: " + wireValue);
    }
}
