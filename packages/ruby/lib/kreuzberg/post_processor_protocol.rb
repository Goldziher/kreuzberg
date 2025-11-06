# frozen_string_literal: true

module Kreuzberg
  # PostProcessor protocol interface.
  #
  # This module defines the protocol that all Ruby post-processors must implement
  # to be registered with the Rust core via the FFI bridge.
  #
  # Post-processors enrich extraction results by adding metadata, transforming content,
  # or performing additional analysis. They are called after extraction completes.
  #
  # @example Implementing a simple post-processor
  #   class UpcaseProcessor
  #     include Kreuzberg::PostProcessorProtocol
  #
  #     def call(result)
  #       result["content"] = result["content"].upcase
  #       result
  #     end
  #   end
  #
  #   Kreuzberg.register_post_processor("upcase", UpcaseProcessor.new)
  #
  # @example Implementing a post-processor that adds metadata
  #   class EntityExtractor
  #     include Kreuzberg::PostProcessorProtocol
  #
  #     def call(result)
  #       entities = extract_entities(result["content"])
  #       result["metadata"]["entities"] = entities
  #       result
  #     end
  #
  #     private
  #
  #     def extract_entities(text)
  #       # Extract named entities from text
  #       # This is a placeholder - use a real NER library in production
  #       text.scan(/[A-Z][a-z]+(?:\s[A-Z][a-z]+)*/)
  #     end
  #   end
  #
  #   Kreuzberg.register_post_processor("entities", EntityExtractor.new)
  #
  # @example Using a Proc as a post-processor
  #   Kreuzberg.register_post_processor("word_count", ->(result) {
  #     word_count = result["content"].split.length
  #     result["metadata"]["word_count"] = word_count
  #     result
  #   })
  #
  module PostProcessorProtocol
    # Process and enrich an extraction result.
    #
    # This method is called after extraction completes. It receives the extraction result
    # as a hash and must return the modified hash. The processor can:
    # - Add new keys to result["metadata"]
    # - Transform result["content"]
    # - Add entries to result["tables"]
    # - Modify any other result fields
    #
    # Existing metadata keys will not be overwritten by the FFI bridge, so it's safe
    # to add new keys without worrying about conflicts.
    #
    # @param result [Hash] Extraction result with the following structure:
    #   - "content" [String] - Extracted text content
    #   - "mime_type" [String] - MIME type of the source document
    #   - "metadata" [Hash] - Document metadata (title, author, etc.)
    #   - "tables" [Array<Hash>] - Extracted tables
    #   - "detected_languages" [Array<String>, nil] - Detected language codes
    #   - "chunks" [Array<String>, nil] - Content chunks (if chunking enabled)
    #
    # @return [Hash] Modified extraction result with enriched metadata
    #
    # @example
    #   def call(result)
    #     text = result["content"]
    #     entities = extract_entities(text)
    #     result["metadata"]["entities"] = entities
    #     result
    #   end
    def call(result)
      raise NotImplementedError, "#{self.class} must implement #call(result)"
    end
  end
end
