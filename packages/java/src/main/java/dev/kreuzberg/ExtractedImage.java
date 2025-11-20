package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Arrays;
import java.util.Objects;
import java.util.Optional;

/**
 * Represents an image artifact extracted from a document page.
 */
public final class ExtractedImage {
    private final byte[] data;
    private final String format;
    private final int imageIndex;
    private final Integer pageNumber;
    private final Integer width;
    private final Integer height;
    private final String colorspace;
    private final Integer bitsPerComponent;
    private final boolean mask;
    private final String description;
    private final ExtractionResult ocrResult;

    @JsonCreator
    public ExtractedImage(
        @JsonProperty("data") byte[] data,
        @JsonProperty("format") String format,
        @JsonProperty("image_index") int imageIndex,
        @JsonProperty("page_number") Integer pageNumber,
        @JsonProperty("width") Integer width,
        @JsonProperty("height") Integer height,
        @JsonProperty("colorspace") String colorspace,
        @JsonProperty("bits_per_component") Integer bitsPerComponent,
        @JsonProperty("is_mask") boolean mask,
        @JsonProperty("description") String description,
        @JsonProperty("ocr_result") ExtractionResult ocrResult
    ) {
        this.data = Objects.requireNonNull(data, "data must not be null").clone();
        this.format = Objects.requireNonNull(format, "format must not be null");
        this.imageIndex = imageIndex;
        this.pageNumber = pageNumber;
        this.width = width;
        this.height = height;
        this.colorspace = colorspace;
        this.bitsPerComponent = bitsPerComponent;
        this.mask = mask;
        this.description = description;
        this.ocrResult = ocrResult;
    }

    public byte[] getData() {
        return data.clone();
    }

    public String getFormat() {
        return format;
    }

    public int getImageIndex() {
        return imageIndex;
    }

    public Optional<Integer> getPageNumber() {
        return Optional.ofNullable(pageNumber);
    }

    public Optional<Integer> getWidth() {
        return Optional.ofNullable(width);
    }

    public Optional<Integer> getHeight() {
        return Optional.ofNullable(height);
    }

    public Optional<String> getColorspace() {
        return Optional.ofNullable(colorspace);
    }

    public Optional<Integer> getBitsPerComponent() {
        return Optional.ofNullable(bitsPerComponent);
    }

    public boolean isMask() {
        return mask;
    }

    public Optional<String> getDescription() {
        return Optional.ofNullable(description);
    }

    public Optional<ExtractionResult> getOcrResult() {
        return Optional.ofNullable(ocrResult);
    }

    @Override
    public boolean equals(Object obj) {
        if (this == obj) {
            return true;
        }
        if (!(obj instanceof ExtractedImage)) {
            return false;
        }
        ExtractedImage other = (ExtractedImage) obj;
        return imageIndex == other.imageIndex
            && mask == other.mask
            && Arrays.equals(data, other.data)
            && Objects.equals(format, other.format)
            && Objects.equals(pageNumber, other.pageNumber)
            && Objects.equals(width, other.width)
            && Objects.equals(height, other.height)
            && Objects.equals(colorspace, other.colorspace)
            && Objects.equals(bitsPerComponent, other.bitsPerComponent)
            && Objects.equals(description, other.description)
            && Objects.equals(ocrResult, other.ocrResult);
    }

    @Override
    public int hashCode() {
        int result = Objects.hash(
            format,
            imageIndex,
            pageNumber,
            width,
            height,
            colorspace,
            bitsPerComponent,
            mask,
            description,
            ocrResult
        );
        result = 31 * result + Arrays.hashCode(data);
        return result;
    }

    @Override
    public String toString() {
        return "ExtractedImage{"
            + "format='" + format + '\''
            + ", imageIndex=" + imageIndex
            + ", pageNumber=" + pageNumber
            + ", width=" + width
            + ", height=" + height
            + ", mask=" + mask
            + '}';
    }
}
