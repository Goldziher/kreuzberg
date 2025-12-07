package dev.kreuzberg.config;

import java.util.HashMap;
import java.util.Map;

/**
 * HTML preprocessing toggles for Markdown conversion.
 */
public final class HtmlPreprocessingOptions {
  private final Boolean enabled;
  private final String preset;
  private final Boolean removeNavigation;
  private final Boolean removeForms;

  private HtmlPreprocessingOptions(Builder builder) {
    this.enabled = builder.enabled;
    this.preset = builder.preset;
    this.removeNavigation = builder.removeNavigation;
    this.removeForms = builder.removeForms;
  }

  public static Builder builder() {
    return new Builder();
  }

  public Map<String, Object> toMap() {
    Map<String, Object> map = new HashMap<>();
    if (enabled != null) {
      map.put("enabled", enabled);
    }
    if (preset != null) {
      map.put("preset", preset);
    }
    if (removeNavigation != null) {
      map.put("remove_navigation", removeNavigation);
    }
    if (removeForms != null) {
      map.put("remove_forms", removeForms);
    }
    return map;
  }

  static HtmlPreprocessingOptions fromMap(Map<String, Object> map) {
    if (map == null) {
      return null;
    }
    Builder builder = builder();
    if (map.get("enabled") instanceof Boolean) {
      builder.enabled((Boolean) map.get("enabled"));
    }
    if (map.get("preset") instanceof String) {
      builder.preset((String) map.get("preset"));
    }
    if (map.get("remove_navigation") instanceof Boolean) {
      builder.removeNavigation((Boolean) map.get("remove_navigation"));
    }
    if (map.get("remove_forms") instanceof Boolean) {
      builder.removeForms((Boolean) map.get("remove_forms"));
    }
    return builder.build();
  }

  public static final class Builder {
    private Boolean enabled;
    private String preset;
    private Boolean removeNavigation;
    private Boolean removeForms;

    private Builder() {
    }

    public Builder enabled(Boolean enabled) {
      this.enabled = enabled;
      return this;
    }

    public Builder preset(String preset) {
      this.preset = preset;
      return this;
    }

    public Builder removeNavigation(Boolean removeNavigation) {
      this.removeNavigation = removeNavigation;
      return this;
    }

    public Builder removeForms(Boolean removeForms) {
      this.removeForms = removeForms;
      return this;
    }

    public HtmlPreprocessingOptions build() {
      return new HtmlPreprocessingOptions(this);
    }
  }
}
