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
      attr_reader :max_chars, :max_overlap, :preset, :embedding

      def initialize(
        max_chars: 1000,
        max_overlap: 200,
        preset: nil,
        embedding: nil
      )
        @max_chars = max_chars.to_i
        @max_overlap = max_overlap.to_i
        @preset = preset&.to_s
        @embedding = embedding
      end

      def to_h
        {
          max_chars: @max_chars,
          max_overlap: @max_overlap,
          preset: @preset,
          embedding: @embedding
        }.compact
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
        @enabled = !enabled.nil?
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
        @extract_images = !extract_images.nil?
        @passwords = if passwords.is_a?(Array)
                       passwords.map(&:to_s)
                     else
                       (passwords ? [passwords.to_s] : nil)
                     end
        @extract_metadata = !!extract_metadata
      end

      def to_h
        {
          extract_images: @extract_images,
          passwords: @passwords,
          extract_metadata: @extract_metadata
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
    # @example With all options
    #   config = Extraction.new(
    #     use_cache: true,
    #     enable_quality_processing: true,
    #     force_ocr: false,
    #     ocr: Config::OCR.new(language: "deu"),
    #     chunking: Config::Chunking.new(max_chars: 500),
    #     language_detection: Config::LanguageDetection.new(enabled: true),
    #     pdf_options: Config::PDF.new(extract_images: true, passwords: ["secret"])
    #   )
    #
    class Extraction
      attr_reader :use_cache, :enable_quality_processing, :force_ocr,
                  :ocr, :chunking, :language_detection, :pdf_options

      def initialize(
        use_cache: true,
        enable_quality_processing: false,
        force_ocr: false,
        ocr: nil,
        chunking: nil,
        language_detection: nil,
        pdf_options: nil
      )
        @use_cache = !use_cache.nil?
        @enable_quality_processing = !enable_quality_processing.nil?
        @force_ocr = !force_ocr.nil?
        @ocr = normalize_config(ocr, OCR)
        @chunking = normalize_config(chunking, Chunking)
        @language_detection = normalize_config(language_detection, LanguageDetection)
        @pdf_options = normalize_config(pdf_options, PDF)
      end

      def to_h
        {
          use_cache: @use_cache,
          enable_quality_processing: @enable_quality_processing,
          force_ocr: @force_ocr,
          ocr: @ocr&.to_h,
          chunking: @chunking&.to_h,
          language_detection: @language_detection&.to_h,
          pdf_options: @pdf_options&.to_h
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
  end
end
