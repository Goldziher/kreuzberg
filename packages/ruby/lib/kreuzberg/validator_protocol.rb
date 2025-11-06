# frozen_string_literal: true

module Kreuzberg
  # Validator protocol interface.
  #
  # This module defines the protocol that all Ruby validators must implement
  # to be registered with the Rust core via the FFI bridge.
  #
  # Validators are called during extraction to validate results. If validation fails,
  # the validator should raise a Kreuzberg::Errors::ValidationError, which will
  # cause the extraction to fail.
  #
  # @example Implementing a minimum length validator
  #   class MinimumLengthValidator
  #     include Kreuzberg::ValidatorProtocol
  #
  #     def initialize(min_length = 10)
  #       @min_length = min_length
  #     end
  #
  #     def call(result)
  #       if result["content"].length < @min_length
  #         raise Kreuzberg::Errors::ValidationError.new(
  #           "Content too short: #{result["content"].length} < #{@min_length}"
  #         )
  #       end
  #     end
  #   end
  #
  #   Kreuzberg.register_validator("min_length", MinimumLengthValidator.new(100))
  #
  # @example Implementing a content quality validator
  #   class QualityValidator
  #     include Kreuzberg::ValidatorProtocol
  #
  #     def call(result)
  #       # Check if content has sufficient quality
  #       if result["content"].strip.empty?
  #         raise Kreuzberg::Errors::ValidationError.new("Empty content extracted")
  #       end
  #
  #       # Check if metadata is present
  #       if result["metadata"].empty?
  #         raise Kreuzberg::Errors::ValidationError.new("No metadata extracted")
  #       end
  #     end
  #   end
  #
  #   Kreuzberg.register_validator("quality", QualityValidator.new)
  #
  # @example Using a Proc as a validator
  #   Kreuzberg.register_validator("not_empty", ->(result) {
  #     if result["content"].strip.empty?
  #       raise Kreuzberg::Errors::ValidationError.new("Content cannot be empty")
  #     end
  #   })
  #
  module ValidatorProtocol
    # Validate an extraction result.
    #
    # This method is called during extraction to validate results. If validation fails,
    # raise a Kreuzberg::Errors::ValidationError with a descriptive message explaining
    # why validation failed. If validation passes, return without raising.
    #
    # The validator receives the extraction result as a hash with the same structure
    # as post-processors (see PostProcessorProtocol for details).
    #
    # @param result [Hash] Extraction result to validate with the following structure:
    #   - "content" [String] - Extracted text content
    #   - "mime_type" [String] - MIME type of the source document
    #   - "metadata" [Hash] - Document metadata (title, author, etc.)
    #   - "tables" [Array<Hash>] - Extracted tables
    #   - "detected_languages" [Array<String>, nil] - Detected language codes
    #   - "chunks" [Array<String>, nil] - Content chunks (if chunking enabled)
    #
    # @return [void]
    # @raise [Kreuzberg::Errors::ValidationError] if validation fails
    #
    # @example
    #   def call(result)
    #     if result["content"].length < 10
    #       raise Kreuzberg::Errors::ValidationError.new("Content too short")
    #     end
    #   end
    def call(result)
      raise NotImplementedError, "#{self.class} must implement #call(result)"
    end
  end
end
