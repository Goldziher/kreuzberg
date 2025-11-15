package dev.kreuzberg.config;

import java.util.HashMap;
import java.util.Map;

/**
 * OCR configuration options.
 *
 * <p>Configures OCR backend and language settings for text extraction from images.
 *
 * @since 4.0.0
 */
public final class OcrConfig {
    private final String backend;
    private final String language;
    private final String tesseractConfig;

    private OcrConfig(Builder builder) {
        this.backend = builder.backend;
        this.language = builder.language;
        this.tesseractConfig = builder.tesseractConfig;
    }

    /**
     * Creates a new builder for OCR configuration.
     *
     * @return a new builder instance
     */
    public static Builder builder() {
        return new Builder();
    }

    /**
     * Gets the OCR backend name.
     *
     * @return the backend name (e.g., "tesseract")
     */
    public String getBackend() {
        return backend;
    }

    /**
     * Gets the OCR language code.
     *
     * @return the language code (e.g., "eng", "deu")
     */
    public String getLanguage() {
        return language;
    }

    /**
     * Gets the Tesseract-specific configuration string.
     *
     * @return the tesseract config, or null if not set
     */
    public String getTesseractConfig() {
        return tesseractConfig;
    }

    /**
     * Converts this configuration to a map for FFI.
     *
     * @return a map representation
     */
    public Map<String, Object> toMap() {
        Map<String, Object> map = new HashMap<>();
        if (backend != null) {
            map.put("backend", backend);
        }
        if (language != null) {
            map.put("language", language);
        }
        if (tesseractConfig != null) {
            map.put("tesseract_config", tesseractConfig);
        }
        return map;
    }

    /**
     * Builder for {@link OcrConfig}.
     */
    public static final class Builder {
        private String backend = "tesseract";
        private String language = "eng";
        private String tesseractConfig;

        private Builder() { }

        /**
         * Sets the OCR backend.
         *
         * @param backend the backend name
         * @return this builder
         */
        public Builder backend(String backend) {
            this.backend = backend;
            return this;
        }

        /**
         * Sets the OCR language.
         *
         * @param language the language code
         * @return this builder
         */
        public Builder language(String language) {
            this.language = language;
            return this;
        }

        /**
         * Sets Tesseract-specific configuration.
         *
         * @param tesseractConfig the tesseract config string
         * @return this builder
         */
        public Builder tesseractConfig(String tesseractConfig) {
            this.tesseractConfig = tesseractConfig;
            return this;
        }

        /**
         * Builds the OCR configuration.
         *
         * @return the built configuration
         */
        public OcrConfig build() {
            return new OcrConfig(this);
        }
    }
}
