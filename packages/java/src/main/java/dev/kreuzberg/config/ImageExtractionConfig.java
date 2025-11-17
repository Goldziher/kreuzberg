package dev.kreuzberg.config;

import java.util.HashMap;
import java.util.Map;

/**
 * Image extraction configuration.
 *
 * @since 4.0.0
 */
public final class ImageExtractionConfig {
  private final boolean extractImages;
  private final int targetDpi;
  private final int maxImageDimension;
  private final boolean autoAdjustDpi;
  private final int minDpi;
  private final int maxDpi;

  private ImageExtractionConfig(Builder builder) {
    this.extractImages = builder.extractImages;
    this.targetDpi = builder.targetDpi;
    this.maxImageDimension = builder.maxImageDimension;
    this.autoAdjustDpi = builder.autoAdjustDpi;
    this.minDpi = builder.minDpi;
    this.maxDpi = builder.maxDpi;
  }

  public static Builder builder() {
    return new Builder();
  }

  public boolean isExtractImages() {
    return extractImages;
  }

  public int getTargetDpi() {
    return targetDpi;
  }

  public int getMaxImageDimension() {
    return maxImageDimension;
  }

  public boolean isAutoAdjustDpi() {
    return autoAdjustDpi;
  }

  public int getMinDpi() {
    return minDpi;
  }

  public int getMaxDpi() {
    return maxDpi;
  }

  public Map<String, Object> toMap() {
    Map<String, Object> map = new HashMap<>();
    map.put("extract_images", extractImages);
    map.put("target_dpi", targetDpi);
    map.put("max_image_dimension", maxImageDimension);
    map.put("auto_adjust_dpi", autoAdjustDpi);
    map.put("min_dpi", minDpi);
    map.put("max_dpi", maxDpi);
    return map;
  }

  public static final class Builder {
    private boolean extractImages = true;
    private int targetDpi = 300;
    private int maxImageDimension = 2000;
    private boolean autoAdjustDpi = true;
    private int minDpi = 150;
    private int maxDpi = 600;

    private Builder() {
      // Use defaults
    }

    public Builder extractImages(boolean extractImages) {
      this.extractImages = extractImages;
      return this;
    }

    public Builder targetDpi(int targetDpi) {
      this.targetDpi = targetDpi;
      return this;
    }

    public Builder maxImageDimension(int maxImageDimension) {
      this.maxImageDimension = maxImageDimension;
      return this;
    }

    public Builder autoAdjustDpi(boolean autoAdjustDpi) {
      this.autoAdjustDpi = autoAdjustDpi;
      return this;
    }

    public Builder minDpi(int minDpi) {
      this.minDpi = minDpi;
      return this;
    }

    public Builder maxDpi(int maxDpi) {
      this.maxDpi = maxDpi;
      return this;
    }

    public ImageExtractionConfig build() {
      return new ImageExtractionConfig(this);
    }
  }
}
