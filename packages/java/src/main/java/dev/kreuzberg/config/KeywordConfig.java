package dev.kreuzberg.config;

import java.util.HashMap;
import java.util.Map;

/**
 * Keyword extraction configuration wrapper.
 */
public final class KeywordConfig {
  private final String algorithm;
  private final Integer maxKeywords;
  private final Double minScore;
  private final int[] ngramRange;
  private final String language;
  private final YakeParams yakeParams;
  private final RakeParams rakeParams;

  private KeywordConfig(Builder builder) {
    this.algorithm = builder.algorithm;
    this.maxKeywords = builder.maxKeywords;
    this.minScore = builder.minScore;
    this.ngramRange = builder.ngramRange;
    this.language = builder.language;
    this.yakeParams = builder.yakeParams;
    this.rakeParams = builder.rakeParams;
  }

  public static Builder builder() {
    return new Builder();
  }

  public Map<String, Object> toMap() {
    Map<String, Object> map = new HashMap<>();
    if (algorithm != null) {
      map.put("algorithm", algorithm);
    }
    if (maxKeywords != null) {
      map.put("max_keywords", maxKeywords);
    }
    if (minScore != null) {
      map.put("min_score", minScore);
    }
    if (ngramRange != null && ngramRange.length == 2) {
      map.put("ngram_range", new int[]{ngramRange[0], ngramRange[1]});
    }
    if (language != null) {
      map.put("language", language);
    }
    if (yakeParams != null) {
      map.put("yake_params", yakeParams.toMap());
    }
    if (rakeParams != null) {
      map.put("rake_params", rakeParams.toMap());
    }
    return map;
  }

  static KeywordConfig fromMap(Map<String, Object> map) {
    if (map == null) {
      return null;
    }
    Builder builder = builder();
    if (map.get("algorithm") instanceof String) {
      builder.algorithm((String) map.get("algorithm"));
    }
    if (map.get("max_keywords") instanceof Number) {
      builder.maxKeywords(((Number) map.get("max_keywords")).intValue());
    }
    if (map.get("min_score") instanceof Number) {
      builder.minScore(((Number) map.get("min_score")).doubleValue());
    }
    if (map.get("ngram_range") instanceof Iterable) {
      Iterable<?> iterable = (Iterable<?>) map.get("ngram_range");
      int[] range = new int[2];
      int idx = 0;
      for (Object value : iterable) {
        if (value instanceof Number && idx < 2) {
          range[idx] = ((Number) value).intValue();
          idx++;
        }
      }
      builder.ngramRange(range);
    }
    if (map.get("language") instanceof String) {
      builder.language((String) map.get("language"));
    }
    if (map.get("yake_params") instanceof Map) {
      @SuppressWarnings("unchecked")
      Map<String, Object> yakeMap = (Map<String, Object>) map.get("yake_params");
      builder.yakeParams(YakeParams.fromMap(yakeMap));
    }
    if (map.get("rake_params") instanceof Map) {
      @SuppressWarnings("unchecked")
      Map<String, Object> rakeMap = (Map<String, Object>) map.get("rake_params");
      builder.rakeParams(RakeParams.fromMap(rakeMap));
    }
    return builder.build();
  }

  public static final class Builder {
    private String algorithm;
    private Integer maxKeywords;
    private Double minScore;
    private int[] ngramRange;
    private String language;
    private YakeParams yakeParams;
    private RakeParams rakeParams;

    private Builder() {
    }

    public Builder algorithm(String algorithm) {
      this.algorithm = algorithm;
      return this;
    }

    public Builder maxKeywords(Integer maxKeywords) {
      this.maxKeywords = maxKeywords;
      return this;
    }

    public Builder minScore(Double minScore) {
      this.minScore = minScore;
      return this;
    }

    public Builder ngramRange(int... ngramRange) {
      if (ngramRange != null && ngramRange.length == 2) {
        this.ngramRange = new int[]{ngramRange[0], ngramRange[1]};
      }
      return this;
    }

    public Builder language(String language) {
      this.language = language;
      return this;
    }

    public Builder yakeParams(YakeParams yakeParams) {
      this.yakeParams = yakeParams;
      return this;
    }

    public Builder rakeParams(RakeParams rakeParams) {
      this.rakeParams = rakeParams;
      return this;
    }

    public KeywordConfig build() {
      return new KeywordConfig(this);
    }
  }

  public static final class YakeParams {
    private final Integer windowSize;

    private YakeParams(Builder builder) {
      this.windowSize = builder.windowSize;
    }

    public Map<String, Object> toMap() {
      Map<String, Object> map = new HashMap<>();
      if (windowSize != null) {
        map.put("window_size", windowSize);
      }
      return map;
    }

    static YakeParams fromMap(Map<String, Object> map) {
      if (map == null) {
        return null;
      }
      Builder builder = builder();
      if (map.get("window_size") instanceof Number) {
        builder.windowSize(((Number) map.get("window_size")).intValue());
      }
      return builder.build();
    }

    public static Builder builder() {
      return new Builder();
    }

    public static final class Builder {
      private Integer windowSize;

      private Builder() { }

      public Builder windowSize(Integer windowSize) {
        this.windowSize = windowSize;
        return this;
      }

      public YakeParams build() {
        return new YakeParams(this);
      }
    }
  }

  public static final class RakeParams {
    private final Integer minWordLength;
    private final Integer maxWordsPerPhrase;

    private RakeParams(Builder builder) {
      this.minWordLength = builder.minWordLength;
      this.maxWordsPerPhrase = builder.maxWordsPerPhrase;
    }

    public Map<String, Object> toMap() {
      Map<String, Object> map = new HashMap<>();
      if (minWordLength != null) {
        map.put("min_word_length", minWordLength);
      }
      if (maxWordsPerPhrase != null) {
        map.put("max_words_per_phrase", maxWordsPerPhrase);
      }
      return map;
    }

    static RakeParams fromMap(Map<String, Object> map) {
      if (map == null) {
        return null;
      }
      Builder builder = builder();
      if (map.get("min_word_length") instanceof Number) {
        builder.minWordLength(((Number) map.get("min_word_length")).intValue());
      }
      if (map.get("max_words_per_phrase") instanceof Number) {
        builder.maxWordsPerPhrase(((Number) map.get("max_words_per_phrase")).intValue());
      }
      return builder.build();
    }

    public static Builder builder() {
      return new Builder();
    }

    public static final class Builder {
      private Integer minWordLength;
      private Integer maxWordsPerPhrase;

      private Builder() { }

      public Builder minWordLength(Integer minWordLength) {
        this.minWordLength = minWordLength;
        return this;
      }

      public Builder maxWordsPerPhrase(Integer maxWordsPerPhrase) {
        this.maxWordsPerPhrase = maxWordsPerPhrase;
        return this;
      }

      public RakeParams build() {
        return new RakeParams(this);
      }
    }
  }
}
