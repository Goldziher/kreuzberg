package dev.kreuzberg.config;

import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * Post-processor configuration.
 *
 * @since 4.0.0
 */
public final class PostProcessorConfig {
  private final boolean enabled;
  private final List<String> enabledProcessors;
  private final List<String> disabledProcessors;

  private PostProcessorConfig(Builder builder) {
    this.enabled = builder.enabled;
    this.enabledProcessors = builder.enabledProcessors != null
        ? Collections.unmodifiableList(new ArrayList<>(builder.enabledProcessors))
        : null;
    this.disabledProcessors = builder.disabledProcessors != null
        ? Collections.unmodifiableList(new ArrayList<>(builder.disabledProcessors))
        : null;
  }

  public static Builder builder() {
    return new Builder();
  }

  public boolean isEnabled() {
    return enabled;
  }

  public List<String> getEnabledProcessors() {
    return enabledProcessors;
  }

  public List<String> getDisabledProcessors() {
    return disabledProcessors;
  }

  public Map<String, Object> toMap() {
    Map<String, Object> map = new HashMap<>();
    map.put("enabled", enabled);
    if (enabledProcessors != null && !enabledProcessors.isEmpty()) {
      map.put("enabled_processors", enabledProcessors);
    }
    if (disabledProcessors != null && !disabledProcessors.isEmpty()) {
      map.put("disabled_processors", disabledProcessors);
    }
    return map;
  }

  public static final class Builder {
    private boolean enabled = true;
    private List<String> enabledProcessors;
    private List<String> disabledProcessors;

    private Builder() {
    }

    public Builder enabled(boolean enabled) {
      this.enabled = enabled;
      return this;
    }

    public Builder enabledProcessors(List<String> enabledProcessors) {
      this.enabledProcessors = enabledProcessors;
      return this;
    }

    public Builder enabledProcessor(String processor) {
      if (this.enabledProcessors == null) {
        this.enabledProcessors = new ArrayList<>();
      }
      this.enabledProcessors.add(processor);
      return this;
    }

    public Builder disabledProcessors(List<String> disabledProcessors) {
      this.disabledProcessors = disabledProcessors;
      return this;
    }

    public Builder disabledProcessor(String processor) {
      if (this.disabledProcessors == null) {
        this.disabledProcessors = new ArrayList<>();
      }
      this.disabledProcessors.add(processor);
      return this;
    }

    public PostProcessorConfig build() {
      return new PostProcessorConfig(this);
    }
  }

  static PostProcessorConfig fromMap(Map<String, Object> map) {
    if (map == null) {
      return null;
    }
    Builder builder = builder();
    if (map.get("enabled") instanceof Boolean) {
      builder.enabled((Boolean) map.get("enabled"));
    }
    if (map.get("enabled_processors") instanceof Iterable) {
      List<String> enabled = new ArrayList<>();
      for (Object value : (Iterable<?>) map.get("enabled_processors")) {
        if (value instanceof String) {
          enabled.add((String) value);
        }
      }
      builder.enabledProcessors(enabled);
    }
    if (map.get("disabled_processors") instanceof Iterable) {
      List<String> disabled = new ArrayList<>();
      for (Object value : (Iterable<?>) map.get("disabled_processors")) {
        if (value instanceof String) {
          disabled.add((String) value);
        }
      }
      builder.disabledProcessors(disabled);
    }
    return builder.build();
  }
}
