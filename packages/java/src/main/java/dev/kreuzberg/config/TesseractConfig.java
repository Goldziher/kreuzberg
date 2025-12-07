package dev.kreuzberg.config;

import java.util.HashMap;
import java.util.Map;

/**
 * Subset of Tesseract settings exposed via bindings.
 */
public final class TesseractConfig {
  private final Integer psm;
  private final Boolean enableTableDetection;
  private final String tesseditCharWhitelist;

  private TesseractConfig(Builder builder) {
    this.psm = builder.psm;
    this.enableTableDetection = builder.enableTableDetection;
    this.tesseditCharWhitelist = builder.tesseditCharWhitelist;
  }

  public static Builder builder() {
    return new Builder();
  }

  public Integer getPsm() {
    return psm;
  }

  public Boolean getEnableTableDetection() {
    return enableTableDetection;
  }

  public String getTesseditCharWhitelist() {
    return tesseditCharWhitelist;
  }

  public Map<String, Object> toMap() {
    Map<String, Object> map = new HashMap<>();
    if (psm != null) {
      map.put("psm", psm);
    }
    if (enableTableDetection != null) {
      map.put("enable_table_detection", enableTableDetection);
    }
    if (tesseditCharWhitelist != null) {
      map.put("tessedit_char_whitelist", tesseditCharWhitelist);
    }
    return map;
  }

  @SuppressWarnings("unchecked")
  static TesseractConfig fromMap(Map<String, Object> map) {
    if (map == null) {
      return null;
    }
    Builder builder = builder();
    Object psmValue = map.get("psm");
    if (psmValue instanceof Number) {
      builder.psm(((Number) psmValue).intValue());
    }
    Object tableDetection = map.get("enable_table_detection");
    if (tableDetection instanceof Boolean) {
      builder.enableTableDetection((Boolean) tableDetection);
    }
    Object whitelist = map.get("tessedit_char_whitelist");
    if (whitelist instanceof String) {
      builder.tesseditCharWhitelist((String) whitelist);
    }
    return builder.build();
  }

  public static final class Builder {
    private Integer psm;
    private Boolean enableTableDetection;
    private String tesseditCharWhitelist;

    private Builder() {
    }

    public Builder psm(Integer psm) {
      this.psm = psm;
      return this;
    }

    public Builder enableTableDetection(Boolean enableTableDetection) {
      this.enableTableDetection = enableTableDetection;
      return this;
    }

    public Builder tesseditCharWhitelist(String tesseditCharWhitelist) {
      this.tesseditCharWhitelist = tesseditCharWhitelist;
      return this;
    }

    public TesseractConfig build() {
      return new TesseractConfig(this);
    }
  }
}
