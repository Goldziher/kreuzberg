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

  private LanguageDetectionConfig(Builder builder) {
    this.enabled = builder.enabled;
    this.minConfidence = builder.minConfidence;
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

  public Map<String, Object> toMap() {
    Map<String, Object> map = new HashMap<>();
    map.put("enabled", enabled);
    map.put("min_confidence", minConfidence);
    return map;
  }

  public static final class Builder {
    private boolean enabled = false;
    private double minConfidence = 0.5;

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

    public LanguageDetectionConfig build() {
      return new LanguageDetectionConfig(this);
    }
  }
}
