package dev.kreuzberg.config;

import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * PDF-specific extraction options.
 *
 * @since 4.0.0
 */
public final class PdfConfig {
    private final boolean extractImages;
    private final List<String> passwords;
    private final boolean extractMetadata;

    private PdfConfig(Builder builder) {
        this.extractImages = builder.extractImages;
        this.passwords = builder.passwords != null
                ? Collections.unmodifiableList(new ArrayList<>(builder.passwords))
                : null;
        this.extractMetadata = builder.extractMetadata;
    }

    public static Builder builder() {
        return new Builder();
    }

    public boolean isExtractImages() {
        return extractImages;
    }

    public List<String> getPasswords() {
        return passwords;
    }

    public boolean isExtractMetadata() {
        return extractMetadata;
    }

    public Map<String, Object> toMap() {
        Map<String, Object> map = new HashMap<>();
        map.put("extract_images", extractImages);
        if (passwords != null && !passwords.isEmpty()) {
            map.put("passwords", passwords);
        }
        map.put("extract_metadata", extractMetadata);
        return map;
    }

    public static final class Builder {
        private boolean extractImages = false;
        private List<String> passwords;
        private boolean extractMetadata = true;

        private Builder() { }

        public Builder extractImages(boolean extractImages) {
            this.extractImages = extractImages;
            return this;
        }

        public Builder passwords(List<String> passwords) {
            this.passwords = passwords;
            return this;
        }

        public Builder password(String password) {
            if (this.passwords == null) {
                this.passwords = new ArrayList<>();
            }
            this.passwords.add(password);
            return this;
        }

        public Builder extractMetadata(boolean extractMetadata) {
            this.extractMetadata = extractMetadata;
            return this;
        }

        public PdfConfig build() {
            return new PdfConfig(this);
        }
    }
}
