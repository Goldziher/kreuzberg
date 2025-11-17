package dev.kreuzberg.config;

import java.util.HashMap;
import java.util.Map;

/**
 * Language detection configuration.
 *
 * @since 4.0.0
 */
public final class LanguageDetectionConfig {
  private final boolean enabled;
  private final double minConfidence;
  private final boolean detectMultiple;

  private LanguageDetectionConfig(Builder builder) {
    this.enabled = builder.enabled;
    this.minConfidence = builder.minConfidence;
    this.detectMultiple = builder.detectMultiple;
  }

  public static Builder builder() {
    return new Builder();
  }

  public boolean isEnabled() {
    return enabled;
  }

  public double getMinConfidence() {
    return minConfidence;
  }

  public boolean isDetectMultiple() {
    return detectMultiple;
  }

  public Map<String, Object> toMap() {
    Map<String, Object> map = new HashMap<>();
    map.put("enabled", enabled);
    map.put("min_confidence", minConfidence);
    map.put("detect_multiple", detectMultiple);
    return map;
  }

  public static final class Builder {
    private boolean enabled = false;
    private double minConfidence = 0.5;
    private boolean detectMultiple = false;

    private Builder() {
      // Use defaults
    }

    public Builder enabled(boolean enabled) {
      this.enabled = enabled;
      return this;
    }

    public Builder minConfidence(double minConfidence) {
      this.minConfidence = minConfidence;
      return this;
    }

    public Builder detectMultiple(boolean detectMultiple) {
      this.detectMultiple = detectMultiple;
      return this;
    }

    public LanguageDetectionConfig build() {
      return new LanguageDetectionConfig(this);
    }
  }

  static LanguageDetectionConfig fromMap(Map<String, Object> map) {
    if (map == null) {
      return null;
    }
    Builder builder = builder();
    Object enabledValue = map.get("enabled");
    if (enabledValue instanceof Boolean) {
      builder.enabled((Boolean) enabledValue);
    }
    Object minConfidenceValue = map.get("min_confidence");
    if (minConfidenceValue instanceof Number) {
      builder.minConfidence(((Number) minConfidenceValue).doubleValue());
    }
    Object detectMultipleValue = map.get("detect_multiple");
    if (detectMultipleValue instanceof Boolean) {
      builder.detectMultiple((Boolean) detectMultipleValue);
    }
    return builder.build();
  }
}
