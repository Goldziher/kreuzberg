# frozen_string_literal: true

module Kreuzberg
  # OCR backend protocol interface.
  #
  # This module defines the protocol that all Ruby OCR backends must implement
  # to be registered with the Rust core via the FFI bridge.
  #
  # OCR backends implement optical character recognition for images and scanned documents.
  # They are called when OCR is enabled in the extraction configuration.
  #
  # @example Implementing a custom OCR backend
  #   class CustomOcrBackend
  #     include Kreuzberg::OcrBackendProtocol
  #
  #     def name
  #       "custom-ocr"
  #     end
  #
  #     def process_image(image_bytes, config)
  #       # Perform OCR on image_bytes
  #       # This is a placeholder - integrate with a real OCR engine
  #       text = my_ocr_engine.recognize(image_bytes, language: config["language"])
  #       text
  #     end
  #   end
  #
  #   backend = CustomOcrBackend.new
  #   Kreuzberg.register_ocr_backend(backend.name, backend)
  #
  #   # Use in extraction
  #   result = Kreuzberg.extract_file_sync(
  #     "scanned.pdf",
  #     config: { ocr: { backend: "custom-ocr", language: "eng" } }
  #   )
  #
  # @example Implementing an OCR backend with initialization
  #   class ModelBasedOcr
  #     include Kreuzberg::OcrBackendProtocol
  #
  #     def initialize
  #       @model = nil
  #     end
  #
  #     def name
  #       "model-ocr"
  #     end
  #
  #     def process_image(image_bytes, config)
  #       # Load model on first use (lazy initialization)
  #       @model ||= load_model
  #
  #       # Run OCR
  #       @model.recognize(image_bytes, config)
  #     end
  #
  #     private
  #
  #     def load_model
  #       # Load ML model for OCR
  #       MyOcrModel.load("path/to/model")
  #     end
  #   end
  #
  #   Kreuzberg.register_ocr_backend("model-ocr", ModelBasedOcr.new)
  #
  module OcrBackendProtocol
    # Return the unique name of this OCR backend.
    #
    # This name is used in ExtractionConfig to select the backend:
    #
    #   config = { ocr: { backend: "custom-ocr", language: "eng" } }
    #
    # The name should be a lowercase string with hyphens (e.g., "custom-ocr", "tesseract").
    #
    # @return [String] Unique backend identifier
    #
    # @example
    #   def name
    #     "custom-ocr"
    #   end
    def name
      raise NotImplementedError, "#{self.class} must implement #name"
    end

    # Process image bytes and extract text via OCR.
    #
    # This method receives raw image data (PNG, JPEG, TIFF, etc.) and an OCR configuration
    # hash. It must return the extracted text as a string.
    #
    # The config hash contains OCR settings such as:
    # - "language" [String] - Language code (e.g., "eng", "deu", "fra")
    # - "backend" [String] - Backend name (same as #name)
    # - Additional backend-specific settings
    #
    # @param image_bytes [String] Binary image data (PNG, JPEG, TIFF, etc.)
    # @param config [Hash] OCR configuration with the following keys:
    #   - "language" [String] - Language code for OCR (e.g., "eng", "deu")
    #   - "backend" [String] - Backend name
    #
    # @return [String] Extracted text content
    #
    # @example
    #   def process_image(image_bytes, config)
    #     language = config["language"] || "eng"
    #     text = my_ocr_engine.recognize(image_bytes, language: language)
    #     text
    #   end
    def process_image(image_bytes, config)
      raise NotImplementedError, "#{self.class} must implement #process_image(image_bytes, config)"
    end
  end
end
