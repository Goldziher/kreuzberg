package dev.kreuzberg.config;

import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * Token reduction configuration for minimizing output size.
 *
 * @since 4.0.0
 */
public final class TokenReductionConfig {
  private static final List<String> VALID_MODES =
      Arrays.asList("off", "light", "moderate", "aggressive", "maximum");

  private final String mode;
  private final boolean preserveImportantWords;

  private TokenReductionConfig(Builder builder) {
    this.mode = builder.mode;
    this.preserveImportantWords = builder.preserveImportantWords;
  }

  public static Builder builder() {
    return new Builder();
  }

  public String getMode() {
    return mode;
  }

  public boolean isPreserveImportantWords() {
    return preserveImportantWords;
  }

  public Map<String, Object> toMap() {
    Map<String, Object> map = new HashMap<>();
    map.put("mode", mode);
    map.put("preserve_important_words", preserveImportantWords);
    return map;
  }

  public static final class Builder {
    private String mode = "off";
    private boolean preserveImportantWords = true;

    private Builder() {
      // Use defaults
    }

    public Builder mode(String mode) {
      if (!VALID_MODES.contains(mode)) {
        throw new IllegalArgumentException(
            "mode must be one of: " + String.join(", ", VALID_MODES));
      }
      this.mode = mode;
      return this;
    }

    public Builder preserveImportantWords(boolean preserveImportantWords) {
      this.preserveImportantWords = preserveImportantWords;
      return this;
    }

    public TokenReductionConfig build() {
      return new TokenReductionConfig(this);
    }
  }
}
