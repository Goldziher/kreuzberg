# frozen_string_literal: true

module Kreuzberg
  module Config
    # OCR configuration
    #
    # @example
    #   ocr = OCR.new(backend: "tesseract", language: "eng")
    #
    class OCR
      attr_reader :backend, :language, :tesseract_config

      def initialize(
        backend: 'tesseract',
        language: 'eng',
        tesseract_config: nil
      )
        @backend = backend.to_s
        @language = language.to_s
        @tesseract_config = tesseract_config
      end

      def to_h
        {
          backend: @backend,
          language: @language,
          tesseract_config: @tesseract_config
        }.compact
      end
    end

    # Chunking configuration
    #
    # @example
    #   chunking = Chunking.new(max_chars: 1000, max_overlap: 200)
    #
    class Chunking
      attr_reader :max_chars, :max_overlap, :preset, :embedding, :enabled

      def initialize(
        max_chars: nil,
        max_overlap: nil,
        preset: nil,
        embedding: nil,
        chunk_size: nil,
        chunk_overlap: nil,
        enabled: true
      )
        resolved_size = chunk_size || max_chars || 1000
        resolved_overlap = chunk_overlap || max_overlap || 200

        @max_chars = resolved_size.to_i
        @max_overlap = resolved_overlap.to_i
        @preset = preset&.to_s
        @embedding = embedding
        @enabled = enabled.nil? ? nil : !!enabled
      end

      def to_h
        config = {
          max_chars: @max_chars,
          max_overlap: @max_overlap,
          preset: @preset,
          embedding: @embedding
        }.compact
        config[:enabled] = @enabled unless @enabled.nil?
        config
      end
    end

    # Language detection configuration
    #
    # @example
    #   lang = LanguageDetection.new(enabled: true, min_confidence: 0.8)
    #
    class LanguageDetection
      attr_reader :enabled, :min_confidence

      def initialize(enabled: false, min_confidence: 0.5)
        @enabled = enabled ? true : false
        @min_confidence = min_confidence.to_f
      end

      def to_h
        {
          enabled: @enabled,
          min_confidence: @min_confidence
        }
      end
    end

    # PDF-specific options
    #
    # @example
    #   pdf = PDF.new(extract_images: true, passwords: ["secret", "backup"])
    #
    class PDF
      attr_reader :extract_images, :passwords, :extract_metadata

      def initialize(
        extract_images: false,
        passwords: nil,
        extract_metadata: true
      )
        @extract_images = extract_images ? true : false
        @passwords = if passwords.is_a?(Array)
                       passwords.map(&:to_s)
                     else
                       (passwords ? [passwords.to_s] : nil)
                     end
        @extract_metadata = extract_metadata ? true : false
      end

      def to_h
        {
          extract_images: @extract_images,
          passwords: @passwords,
          extract_metadata: @extract_metadata
        }.compact
      end
    end

    # Image extraction configuration
    #
    # @example
    #   image = ImageExtraction.new(extract_images: true, target_dpi: 300)
    #
    # @example With auto-adjust DPI
    #   image = ImageExtraction.new(
    #     extract_images: true,
    #     auto_adjust_dpi: true,
    #     min_dpi: 150,
    #     max_dpi: 600
    #   )
    #
    class ImageExtraction
      attr_reader :extract_images, :target_dpi, :max_image_dimension,
                  :auto_adjust_dpi, :min_dpi, :max_dpi

      def initialize(
        extract_images: true,
        target_dpi: 300,
        max_image_dimension: 2000,
        auto_adjust_dpi: true,
        min_dpi: 150,
        max_dpi: 600
      )
        @extract_images = extract_images ? true : false
        @target_dpi = target_dpi.to_i
        @max_image_dimension = max_image_dimension.to_i
        @auto_adjust_dpi = auto_adjust_dpi ? true : false
        @min_dpi = min_dpi.to_i
        @max_dpi = max_dpi.to_i
      end

      def to_h
        {
          extract_images: @extract_images,
          target_dpi: @target_dpi,
          max_image_dimension: @max_image_dimension,
          auto_adjust_dpi: @auto_adjust_dpi,
          min_dpi: @min_dpi,
          max_dpi: @max_dpi
        }
      end
    end

    # Image preprocessing configuration for OCR
    #
    # @example Basic preprocessing
    #   preprocessing = ImagePreprocessing.new(
    #     binarization_method: "otsu",
    #     denoise: true
    #   )
    #
    # @example Advanced preprocessing
    #   preprocessing = ImagePreprocessing.new(
    #     target_dpi: 600,
    #     auto_rotate: true,
    #     deskew: true,
    #     denoise: true,
    #     contrast_enhance: true,
    #     binarization_method: "sauvola",
    #     invert_colors: false
    #   )
    #
    class ImagePreprocessing
      attr_reader :target_dpi, :auto_rotate, :deskew, :denoise,
                  :contrast_enhance, :binarization_method, :invert_colors

      def initialize(
        target_dpi: 300,
        auto_rotate: true,
        deskew: true,
        denoise: false,
        contrast_enhance: true,
        binarization_method: 'otsu',
        invert_colors: false
      )
        @target_dpi = target_dpi.to_i
        @auto_rotate = auto_rotate ? true : false
        @deskew = deskew ? true : false
        @denoise = denoise ? true : false
        @contrast_enhance = contrast_enhance ? true : false
        @binarization_method = binarization_method.to_s
        @invert_colors = invert_colors ? true : false

        valid_methods = %w[otsu sauvola adaptive]
        return if valid_methods.include?(@binarization_method)

        raise ArgumentError, "binarization_method must be one of: #{valid_methods.join(', ')}"
      end

      def to_h
        {
          target_dpi: @target_dpi,
          auto_rotate: @auto_rotate,
          deskew: @deskew,
          denoise: @denoise,
          contrast_enhance: @contrast_enhance,
          binarization_method: @binarization_method,
          invert_colors: @invert_colors
        }
      end
    end

    # Token reduction configuration
    #
    # @example Disable token reduction
    #   token = TokenReduction.new(mode: "off")
    #
    # @example Light reduction
    #   token = TokenReduction.new(mode: "light", preserve_important_words: true)
    #
    # @example Aggressive reduction
    #   token = TokenReduction.new(mode: "aggressive", preserve_important_words: false)
    #
    class TokenReduction
      attr_reader :mode, :preserve_important_words

      def initialize(mode: 'off', preserve_important_words: true)
        @mode = mode.to_s
        @preserve_important_words = preserve_important_words ? true : false

        valid_modes = %w[off light moderate aggressive maximum]
        return if valid_modes.include?(@mode)

        raise ArgumentError, "mode must be one of: #{valid_modes.join(', ')}"
      end

      def to_h
        {
          mode: @mode,
          preserve_important_words: @preserve_important_words
        }
      end
    end

    # Post-processor configuration
    #
    # @example Enable all post-processors
    #   postprocessor = PostProcessor.new(enabled: true)
    #
    # @example Enable specific processors
    #   postprocessor = PostProcessor.new(
    #     enabled: true,
    #     enabled_processors: ["quality", "formatting"]
    #   )
    #
    # @example Disable specific processors
    #   postprocessor = PostProcessor.new(
    #     enabled: true,
    #     disabled_processors: ["token_reduction"]
    #   )
    #
    class PostProcessor
      attr_reader :enabled, :enabled_processors, :disabled_processors

      def initialize(
        enabled: true,
        enabled_processors: nil,
        disabled_processors: nil
      )
        @enabled = enabled ? true : false
        @enabled_processors = enabled_processors&.map(&:to_s)
        @disabled_processors = disabled_processors&.map(&:to_s)
      end

      def to_h
        {
          enabled: @enabled,
          enabled_processors: @enabled_processors,
          disabled_processors: @disabled_processors
        }.compact
      end
    end

    # Main extraction configuration
    #
    # @example Basic usage
    #   config = Extraction.new(use_cache: true, force_ocr: true)
    #
    # @example With OCR
    #   ocr = Config::OCR.new(backend: "tesseract", language: "eng")
    #   config = Extraction.new(ocr: ocr)
    #
    # @example With image extraction
    #   image = Config::ImageExtraction.new(extract_images: true, target_dpi: 600)
    #   config = Extraction.new(image_extraction: image)
    #
    # @example With preprocessing
    #   preprocessing = Config::ImagePreprocessing.new(
    #     binarization_method: "sauvola",
    #     denoise: true
    #   )
    #   config = Extraction.new(image_preprocessing: preprocessing)
    #
    # @example With post-processing
    #   postprocessor = Config::PostProcessor.new(
    #     enabled: true,
    #     enabled_processors: ["quality"]
    #   )
    #   config = Extraction.new(postprocessor: postprocessor)
    #
    # @example With all options
    #   config = Extraction.new(
    #     use_cache: true,
    #     enable_quality_processing: true,
    #     force_ocr: false,
    #     ocr: Config::OCR.new(language: "deu"),
    #     chunking: Config::Chunking.new(max_chars: 500),
    #     language_detection: Config::LanguageDetection.new(enabled: true),
    #     pdf_options: Config::PDF.new(extract_images: true, passwords: ["secret"]),
    #     image_extraction: Config::ImageExtraction.new(target_dpi: 600),
    #     image_preprocessing: Config::ImagePreprocessing.new(denoise: true),
    #     postprocessor: Config::PostProcessor.new(enabled: true)
    #   )
    #
    class Extraction
      attr_reader :use_cache, :enable_quality_processing, :force_ocr,
                  :ocr, :chunking, :language_detection, :pdf_options,
                  :image_extraction, :image_preprocessing, :postprocessor,
                  :token_reduction

      # Load configuration from a file.
      #
      # Detects the file format from the extension (.toml, .yaml, .yml, .json)
      # and loads the configuration accordingly.
      #
      # @param path [String] Path to the configuration file
      # @return [Kreuzberg::Config::Extraction] Loaded configuration object
      #
      # @example Load from TOML
      #   config = Kreuzberg::Config::Extraction.from_file("config.toml")
      #
      # @example Load from YAML
      #   config = Kreuzberg::Config::Extraction.from_file("config.yaml")
      #
      def self.from_file(path)
        hash = Kreuzberg._config_from_file_native(path)
        new(**hash)
      end

      def initialize(
        use_cache: true,
        enable_quality_processing: false,
        force_ocr: false,
        ocr: nil,
        chunking: nil,
        language_detection: nil,
        pdf_options: nil,
        image_extraction: nil,
        image_preprocessing: nil,
        postprocessor: nil,
        token_reduction: nil
      )
        @use_cache = use_cache ? true : false
        @enable_quality_processing = enable_quality_processing ? true : false
        @force_ocr = force_ocr ? true : false
        @ocr = normalize_config(ocr, OCR)
        @chunking = normalize_config(chunking, Chunking)
        @language_detection = normalize_config(language_detection, LanguageDetection)
        @pdf_options = normalize_config(pdf_options, PDF)
        @image_extraction = normalize_config(image_extraction, ImageExtraction)
        @image_preprocessing = normalize_config(image_preprocessing, ImagePreprocessing)
        @postprocessor = normalize_config(postprocessor, PostProcessor)
        @token_reduction = normalize_config(token_reduction, TokenReduction)
      end

      def to_h
        {
          use_cache: @use_cache,
          enable_quality_processing: @enable_quality_processing,
          force_ocr: @force_ocr,
          ocr: @ocr&.to_h,
          chunking: @chunking&.to_h,
          language_detection: @language_detection&.to_h,
          pdf_options: @pdf_options&.to_h,
          image_extraction: @image_extraction&.to_h,
          image_preprocessing: @image_preprocessing&.to_h,
          postprocessor: @postprocessor&.to_h,
          token_reduction: @token_reduction&.to_h
        }.compact
      end

      private

      def normalize_config(value, klass)
        return nil if value.nil?
        return value if value.is_a?(klass)
        return klass.new(**value) if value.is_a?(Hash)

        raise ArgumentError, "Expected #{klass}, Hash, or nil, got #{value.class}"
      end
    end

    # Backwards compatibility aliases
    Ocr = OCR
  end
end
