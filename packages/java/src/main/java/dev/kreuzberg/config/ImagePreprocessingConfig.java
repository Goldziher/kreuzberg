package dev.kreuzberg.config;

import java.util.HashMap;
import java.util.Map;

/**
 * Image preprocessing configuration for OCR.
 *
 * @since 4.0.0
 */
public final class ImagePreprocessingConfig {
  private final int targetDpi;
  private final boolean autoRotate;
  private final boolean deskew;
  private final boolean denoise;
  private final boolean contrastEnhance;
  private final String binarizationMethod;
  private final boolean invertColors;

  private ImagePreprocessingConfig(Builder builder) {
    this.targetDpi = builder.targetDpi;
    this.autoRotate = builder.autoRotate;
    this.deskew = builder.deskew;
    this.denoise = builder.denoise;
    this.contrastEnhance = builder.contrastEnhance;
    this.binarizationMethod = builder.binarizationMethod;
    this.invertColors = builder.invertColors;
  }

  public static Builder builder() {
    return new Builder();
  }

  public int getTargetDpi() {
    return targetDpi;
  }

  public boolean isAutoRotate() {
    return autoRotate;
  }

  public boolean isDeskew() {
    return deskew;
  }

  public boolean isDenoise() {
    return denoise;
  }

  public boolean isContrastEnhance() {
    return contrastEnhance;
  }

  public String getBinarizationMethod() {
    return binarizationMethod;
  }

  public boolean isInvertColors() {
    return invertColors;
  }

  public Map<String, Object> toMap() {
    Map<String, Object> map = new HashMap<>();
    map.put("target_dpi", targetDpi);
    map.put("auto_rotate", autoRotate);
    map.put("deskew", deskew);
    map.put("denoise", denoise);
    map.put("contrast_enhance", contrastEnhance);
    map.put("binarization_method", binarizationMethod);
    map.put("invert_colors", invertColors);
    return map;
  }

  public static final class Builder {
    private int targetDpi = 300;
    private boolean autoRotate = true;
    private boolean deskew = true;
    private boolean denoise = false;
    private boolean contrastEnhance = true;
    private String binarizationMethod = "otsu";
    private boolean invertColors = false;

    private Builder() {
    }

    public Builder targetDpi(int targetDpi) {
      this.targetDpi = targetDpi;
      return this;
    }

    public Builder autoRotate(boolean autoRotate) {
      this.autoRotate = autoRotate;
      return this;
    }

    public Builder deskew(boolean deskew) {
      this.deskew = deskew;
      return this;
    }

    public Builder denoise(boolean denoise) {
      this.denoise = denoise;
      return this;
    }

    public Builder contrastEnhance(boolean contrastEnhance) {
      this.contrastEnhance = contrastEnhance;
      return this;
    }

    public Builder binarizationMethod(String binarizationMethod) {
      this.binarizationMethod = binarizationMethod;
      return this;
    }

    public Builder invertColors(boolean invertColors) {
      this.invertColors = invertColors;
      return this;
    }

    public ImagePreprocessingConfig build() {
      return new ImagePreprocessingConfig(this);
    }
  }

  static ImagePreprocessingConfig fromMap(Map<String, Object> map) {
    if (map == null) {
      return null;
    }
    Builder builder = builder();
    if (map.get("target_dpi") instanceof Number) {
      builder.targetDpi(((Number) map.get("target_dpi")).intValue());
    }
    if (map.get("auto_rotate") instanceof Boolean) {
      builder.autoRotate((Boolean) map.get("auto_rotate"));
    }
    if (map.get("deskew") instanceof Boolean) {
      builder.deskew((Boolean) map.get("deskew"));
    }
    if (map.get("denoise") instanceof Boolean) {
      builder.denoise((Boolean) map.get("denoise"));
    }
    if (map.get("contrast_enhance") instanceof Boolean) {
      builder.contrastEnhance((Boolean) map.get("contrast_enhance"));
    }
    if (map.get("binarization_method") instanceof String) {
      builder.binarizationMethod((String) map.get("binarization_method"));
    }
    if (map.get("invert_colors") instanceof Boolean) {
      builder.invertColors((Boolean) map.get("invert_colors"));
    }
    return builder.build();
  }
}
