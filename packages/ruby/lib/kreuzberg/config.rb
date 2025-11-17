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
        @tesseract_config = normalize_tesseract_config(tesseract_config)
      end

      def to_h
        {
          backend: @backend,
          language: @language,
          tesseract_config: @tesseract_config&.to_h
        }.compact
      end

      private

      def normalize_tesseract_config(value)
        return nil if value.nil?
        return value if value.is_a?(Tesseract)
        return Tesseract.new(**value.transform_keys(&:to_sym)) if value.is_a?(Hash)

        raise ArgumentError, "Expected #{Tesseract}, Hash, or nil, got #{value.class}"
      end
    end

    class Tesseract
      attr_reader :options

      def initialize(**options)
        @options = options.transform_keys(&:to_sym)
        normalize_nested_preprocessing!
      end

      def to_h
        @options.dup
      end

      private

      def normalize_nested_preprocessing!
        preprocessing = @options[:preprocessing]
        return if preprocessing.nil?
        return if preprocessing.is_a?(ImagePreprocessing)
        return @options[:preprocessing] = ImagePreprocessing.new(**preprocessing.transform_keys(&:to_sym)) if
          preprocessing.is_a?(Hash)

        raise ArgumentError, "preprocessing must be #{ImagePreprocessing} or Hash"
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
        @embedding = normalize_embedding(embedding)
        @enabled = boolean_or_nil(enabled)
      end

      def to_h
        config = {
          max_chars: @max_chars,
          max_overlap: @max_overlap,
          preset: @preset,
          embedding: @embedding&.to_h
        }.compact
        # @type var config: Hash[Symbol, untyped]
        config[:enabled] = @enabled unless @enabled.nil?
        config
      end

      private

      def normalize_embedding(value)
        return nil if value.nil?
        return value if value.is_a?(Embedding)
        return Embedding.new(**value.transform_keys(&:to_sym)) if value.is_a?(Hash)

        raise ArgumentError, "Expected #{Embedding}, Hash, or nil, got #{value.class}"
      end

      def boolean_or_nil(value)
        return nil if value.nil?

        value ? true : false
      end
    end

    class Embedding
      attr_reader :model, :normalize, :batch_size, :show_download_progress, :cache_dir

      def initialize(
        model: { type: :preset, name: 'balanced' },
        normalize: true,
        batch_size: 32,
        show_download_progress: false,
        cache_dir: nil
      )
        @model = normalize_model(model)
        @normalize = boolean_or_nil(normalize)
        @batch_size = batch_size&.to_i
        @show_download_progress = boolean_or_nil(show_download_progress)
        @cache_dir = cache_dir&.to_s
      end

      def to_h
        {
          model: @model,
          normalize: @normalize,
          batch_size: @batch_size,
          show_download_progress: @show_download_progress,
          cache_dir: @cache_dir
        }.compact
      end

      private

      def normalize_model(model)
        normalized = if model.respond_to?(:to_h)
                       model.to_h
                     else
                       model
                     end
        raise ArgumentError, 'model must be a Hash describing the embedding model' unless normalized.is_a?(Hash)

        normalized.transform_keys(&:to_sym)
      end

      def boolean_or_nil(value)
        return nil if value.nil?

        value ? true : false
      end
    end

    # Language detection configuration
    #
    # @example
    #   lang = LanguageDetection.new(enabled: true, min_confidence: 0.8)
    #
    class LanguageDetection
      attr_reader :enabled, :min_confidence, :detect_multiple

      def initialize(enabled: false, min_confidence: 0.5, detect_multiple: false)
        @enabled = enabled ? true : false
        @min_confidence = min_confidence.to_f
        @detect_multiple = detect_multiple ? true : false
      end

      def to_h
        {
          enabled: @enabled,
          min_confidence: @min_confidence,
          detect_multiple: @detect_multiple
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

    class HtmlPreprocessing
      attr_reader :enabled, :preset, :remove_navigation, :remove_forms

      def initialize(enabled: nil, preset: nil, remove_navigation: nil, remove_forms: nil)
        @enabled = boolean_or_nil(enabled)
        @preset = preset&.to_sym
        @remove_navigation = boolean_or_nil(remove_navigation)
        @remove_forms = boolean_or_nil(remove_forms)
      end

      def to_h
        {
          enabled: @enabled,
          preset: @preset,
          remove_navigation: @remove_navigation,
          remove_forms: @remove_forms
        }.compact
      end

      private

      def boolean_or_nil(value)
        return nil if value.nil?

        value ? true : false
      end
    end

    class HtmlOptions
      attr_reader :options

      def initialize(**options)
        normalized = options.transform_keys(&:to_sym)
        symbol_keys = %i[
          heading_style
          code_block_style
          highlight_style
          list_indent_type
          newline_style
          whitespace_mode
        ]
        symbol_keys.each do |key|
          normalized[key] = normalized[key]&.to_sym if normalized.key?(key)
        end
        if normalized[:preprocessing].is_a?(Hash)
          normalized[:preprocessing] = HtmlPreprocessing.new(**normalized[:preprocessing])
        end
        @options = normalized
      end

      def to_h
        @options.transform_values { |value| value.respond_to?(:to_h) ? value.to_h : value }
      end
    end

    class KeywordYakeParams
      attr_reader :window_size

      def initialize(window_size: 2)
        @window_size = window_size.to_i
      end

      def to_h
        { window_size: @window_size }
      end
    end

    class KeywordRakeParams
      attr_reader :min_word_length, :max_words_per_phrase

      def initialize(min_word_length: 1, max_words_per_phrase: 3)
        @min_word_length = min_word_length.to_i
        @max_words_per_phrase = max_words_per_phrase.to_i
      end

      def to_h
        {
          min_word_length: @min_word_length,
          max_words_per_phrase: @max_words_per_phrase
        }
      end
    end

    class Keywords
      attr_reader :algorithm, :max_keywords, :min_score, :ngram_range,
                  :language, :yake_params, :rake_params

      def initialize(
        algorithm: nil,
        max_keywords: nil,
        min_score: nil,
        ngram_range: nil,
        language: nil,
        yake_params: nil,
        rake_params: nil
      )
        @algorithm = algorithm&.to_s
        @max_keywords = max_keywords&.to_i
        @min_score = min_score&.to_f
        @ngram_range = ngram_range&.map(&:to_i)
        @language = language&.to_s
        @yake_params = normalize_nested(yake_params, KeywordYakeParams)
        @rake_params = normalize_nested(rake_params, KeywordRakeParams)
      end

      def to_h
        {
          algorithm: @algorithm,
          max_keywords: @max_keywords,
          min_score: @min_score,
          ngram_range: @ngram_range,
          language: @language,
          yake_params: @yake_params&.to_h,
          rake_params: @rake_params&.to_h
        }.compact
      end

      private

      def normalize_nested(value, klass)
        return nil if value.nil?
        return value if value.is_a?(klass)
        return klass.new(**value.transform_keys(&:to_sym)) if value.is_a?(Hash)

        raise ArgumentError, "Expected #{klass}, Hash, or nil, got #{value.class}"
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
                  :token_reduction, :keywords, :html_options,
                  :max_concurrent_extractions

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
        # Convert string keys to symbols for keyword arguments
        new(**hash.transform_keys(&:to_sym))
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
        token_reduction: nil,
        keywords: nil,
        html_options: nil,
        max_concurrent_extractions: nil
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
        @keywords = normalize_config(keywords, Keywords)
        @html_options = normalize_config(html_options, HtmlOptions)
        @max_concurrent_extractions = max_concurrent_extractions&.to_i
      end

      # rubocop:disable Metrics/PerceivedComplexity
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
          token_reduction: @token_reduction&.to_h,
          keywords: @keywords&.to_h,
          html_options: @html_options&.to_h,
          max_concurrent_extractions: @max_concurrent_extractions
        }.compact
      end
      # rubocop:enable Metrics/PerceivedComplexity

      private

      def normalize_config(value, klass)
        return nil if value.nil?
        return value if value.is_a?(klass)
        # Convert string keys to symbols for keyword arguments
        return klass.new(**value.transform_keys(&:to_sym)) if value.is_a?(Hash)

        raise ArgumentError, "Expected #{klass}, Hash, or nil, got #{value.class}"
      end
    end

    # Backwards compatibility aliases
    Ocr = OCR
  end
end
