package dev.kreuzberg.config;

import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;

/**
 * Wrapper for html-to-markdown conversion options. Stores arbitrary key/value pairs.
 */
public final class HtmlOptions {
  private final Map<String, Object> options;

  private HtmlOptions(Builder builder) {
    this.options = Collections.unmodifiableMap(new LinkedHashMap<>(builder.options));
  }

  public static Builder builder() {
    return new Builder();
  }

  public Map<String, Object> toMap() {
    return options;
  }

  static HtmlOptions fromMap(Map<String, Object> map) {
    if (map == null) {
      return null;
    }
    Builder builder = builder();
    for (Map.Entry<String, Object> entry : map.entrySet()) {
      if ("preprocessing".equals(entry.getKey()) && entry.getValue() instanceof Map) {
        @SuppressWarnings("unchecked")
        Map<String, Object> preprocessing = (Map<String, Object>) entry.getValue();
        HtmlPreprocessingOptions pre = HtmlPreprocessingOptions.fromMap(preprocessing);
        if (pre != null) {
          builder.put("preprocessing", pre.toMap());
        }
      } else {
        builder.put(entry.getKey(), entry.getValue());
      }
    }
    return builder.build();
  }

  public static final class Builder {
    private final Map<String, Object> options = new LinkedHashMap<>();

    private Builder() {
    }

    public Builder put(String key, Object value) {
      if (value != null) {
        options.put(key, value);
      }
      return this;
    }

    public Builder preprocessing(HtmlPreprocessingOptions preprocessing) {
      if (preprocessing != null) {
        options.put("preprocessing", preprocessing.toMap());
      }
      return this;
    }

    public Builder options(Map<String, Object> raw) {
      if (raw != null) {
        options.putAll(raw);
      }
      return this;
    }

    public HtmlOptions build() {
      return new HtmlOptions(this);
    }
  }
}
