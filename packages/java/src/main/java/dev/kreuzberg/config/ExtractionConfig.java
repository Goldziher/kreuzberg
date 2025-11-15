package dev.kreuzberg.config;

import java.util.HashMap;
import java.util.Map;

/**
 * Main extraction configuration.
 *
 * @since 4.0.0
 */
public final class ExtractionConfig {
    private final boolean useCache;
    private final boolean enableQualityProcessing;
    private final boolean forceOcr;
    private final OcrConfig ocr;
    private final ChunkingConfig chunking;
    private final LanguageDetectionConfig languageDetection;
    private final PdfConfig pdfOptions;
    private final ImageExtractionConfig imageExtraction;
    private final ImagePreprocessingConfig imagePreprocessing;
    private final PostProcessorConfig postprocessor;
    private final TokenReductionConfig tokenReduction;

    private ExtractionConfig(Builder builder) {
        this.useCache = builder.useCache;
        this.enableQualityProcessing = builder.enableQualityProcessing;
        this.forceOcr = builder.forceOcr;
        this.ocr = builder.ocr;
        this.chunking = builder.chunking;
        this.languageDetection = builder.languageDetection;
        this.pdfOptions = builder.pdfOptions;
        this.imageExtraction = builder.imageExtraction;
        this.imagePreprocessing = builder.imagePreprocessing;
        this.postprocessor = builder.postprocessor;
        this.tokenReduction = builder.tokenReduction;
    }

    public static Builder builder() {
        return new Builder();
    }

    public boolean isUseCache() {
        return useCache;
    }

    public boolean isEnableQualityProcessing() {
        return enableQualityProcessing;
    }

    public boolean isForceOcr() {
        return forceOcr;
    }

    public OcrConfig getOcr() {
        return ocr;
    }

    public ChunkingConfig getChunking() {
        return chunking;
    }

    public LanguageDetectionConfig getLanguageDetection() {
        return languageDetection;
    }

    public PdfConfig getPdfOptions() {
        return pdfOptions;
    }

    public ImageExtractionConfig getImageExtraction() {
        return imageExtraction;
    }

    public ImagePreprocessingConfig getImagePreprocessing() {
        return imagePreprocessing;
    }

    public PostProcessorConfig getPostprocessor() {
        return postprocessor;
    }

    public TokenReductionConfig getTokenReduction() {
        return tokenReduction;
    }

    public Map<String, Object> toMap() {
        Map<String, Object> map = new HashMap<>();
        map.put("use_cache", useCache);
        map.put("enable_quality_processing", enableQualityProcessing);
        map.put("force_ocr", forceOcr);
        if (ocr != null) {
            map.put("ocr", ocr.toMap());
        }
        if (chunking != null) {
            map.put("chunking", chunking.toMap());
        }
        if (languageDetection != null) {
            map.put("language_detection", languageDetection.toMap());
        }
        if (pdfOptions != null) {
            map.put("pdf_options", pdfOptions.toMap());
        }
        if (imageExtraction != null) {
            map.put("image_extraction", imageExtraction.toMap());
        }
        if (imagePreprocessing != null) {
            map.put("image_preprocessing", imagePreprocessing.toMap());
        }
        if (postprocessor != null) {
            map.put("postprocessor", postprocessor.toMap());
        }
        if (tokenReduction != null) {
            map.put("token_reduction", tokenReduction.toMap());
        }
        return map;
    }

    public static final class Builder {
        private boolean useCache = true;
        private boolean enableQualityProcessing = false;
        private boolean forceOcr = false;
        private OcrConfig ocr;
        private ChunkingConfig chunking;
        private LanguageDetectionConfig languageDetection;
        private PdfConfig pdfOptions;
        private ImageExtractionConfig imageExtraction;
        private ImagePreprocessingConfig imagePreprocessing;
        private PostProcessorConfig postprocessor;
        private TokenReductionConfig tokenReduction;

        private Builder() { }

        public Builder useCache(boolean useCache) {
            this.useCache = useCache;
            return this;
        }

        public Builder enableQualityProcessing(boolean enableQualityProcessing) {
            this.enableQualityProcessing = enableQualityProcessing;
            return this;
        }

        public Builder forceOcr(boolean forceOcr) {
            this.forceOcr = forceOcr;
            return this;
        }

        public Builder ocr(OcrConfig ocr) {
            this.ocr = ocr;
            return this;
        }

        public Builder chunking(ChunkingConfig chunking) {
            this.chunking = chunking;
            return this;
        }

        public Builder languageDetection(LanguageDetectionConfig languageDetection) {
            this.languageDetection = languageDetection;
            return this;
        }

        public Builder pdfOptions(PdfConfig pdfOptions) {
            this.pdfOptions = pdfOptions;
            return this;
        }

        public Builder imageExtraction(ImageExtractionConfig imageExtraction) {
            this.imageExtraction = imageExtraction;
            return this;
        }

        public Builder imagePreprocessing(ImagePreprocessingConfig imagePreprocessing) {
            this.imagePreprocessing = imagePreprocessing;
            return this;
        }

        public Builder postprocessor(PostProcessorConfig postprocessor) {
            this.postprocessor = postprocessor;
            return this;
        }

        public Builder tokenReduction(TokenReductionConfig tokenReduction) {
            this.tokenReduction = tokenReduction;
            return this;
        }

        public ExtractionConfig build() {
            return new ExtractionConfig(this);
        }
    }
}
