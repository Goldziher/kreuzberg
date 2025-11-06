# frozen_string_literal: true

require_relative 'kreuzberg/setup_lib_path'
Kreuzberg::SetupLibPath.configure

require_relative 'kreuzberg/version'
require 'kreuzberg_rb'

module Kreuzberg
  autoload :Config, 'kreuzberg/config'
  autoload :Result, 'kreuzberg/result'
  autoload :CLI, 'kreuzberg/cli'
  autoload :CLIProxy, 'kreuzberg/cli_proxy'
  autoload :APIProxy, 'kreuzberg/api_proxy'
  autoload :MCPProxy, 'kreuzberg/mcp_proxy'
  autoload :Errors, 'kreuzberg/errors'
  autoload :PostProcessorProtocol, 'kreuzberg/post_processor_protocol'
  autoload :ValidatorProtocol, 'kreuzberg/validator_protocol'
  autoload :OcrBackendProtocol, 'kreuzberg/ocr_backend_protocol'

  class << self
    # Store native methods as private methods
    alias native_extract_file_sync extract_file_sync
    alias native_extract_bytes_sync extract_bytes_sync
    alias native_batch_extract_files_sync batch_extract_files_sync
    alias native_extract_file extract_file
    alias native_extract_bytes extract_bytes
    alias native_batch_extract_files batch_extract_files

    private :native_extract_file_sync, :native_extract_bytes_sync, :native_batch_extract_files_sync
    private :native_extract_file, :native_extract_bytes, :native_batch_extract_files
  end

  module_function

  # Extract content from a file (synchronous).
  #
  # @param path [String] Path to the file
  # @param mime_type [String, nil] Optional MIME type hint
  # @param config [Hash, Config::Extraction, nil] Extraction configuration
  # @return [Result] Extraction result object
  #
  # @example Basic usage
  #   result = Kreuzberg.extract_file_sync("document.pdf")
  #   puts result.content
  #
  # @example With configuration
  #   config = Kreuzberg::Config::Extraction.new(force_ocr: true)
  #   result = Kreuzberg.extract_file_sync("scanned.pdf", config: config)
  #
  def extract_file_sync(path, mime_type: nil, config: nil)
    opts = Kreuzberg.normalize_config(config)
    args = [path.to_s]
    args << mime_type.to_s if mime_type
    hash = native_extract_file_sync(*args, **opts)
    Result.new(hash)
  end

  # Extract content from bytes (synchronous).
  #
  # @param data [String] Binary data to extract
  # @param mime_type [String] MIME type of the data
  # @param config [Hash, Config::Extraction, nil] Extraction configuration
  # @return [Result] Extraction result object
  #
  # @example
  #   data = File.binread("document.pdf")
  #   result = Kreuzberg.extract_bytes_sync(data, "application/pdf")
  #
  def extract_bytes_sync(data, mime_type, config: nil)
    opts = Kreuzberg.normalize_config(config)
    hash = native_extract_bytes_sync(data.to_s, mime_type.to_s, **opts)
    Result.new(hash)
  end

  # Batch extract content from multiple files (synchronous).
  #
  # @param paths [Array<String>] List of file paths
  # @param config [Hash, Config::Extraction, nil] Extraction configuration
  # @return [Array<Result>] Array of extraction result objects
  #
  # @example
  #   paths = ["doc1.pdf", "doc2.docx", "doc3.xlsx"]
  #   results = Kreuzberg.batch_extract_files_sync(paths)
  #   results.each { |r| puts r.content }
  #
  def batch_extract_files_sync(paths, config: nil)
    opts = Kreuzberg.normalize_config(config)
    hashes = native_batch_extract_files_sync(paths.map(&:to_s), **opts)
    hashes.map { |hash| Result.new(hash) }
  end

  # Extract content from a file (asynchronous via Tokio runtime).
  #
  # Note: Ruby doesn't have native async/await. This uses a blocking Tokio runtime.
  # For background processing, use extract_file_sync in a Thread.
  #
  # @param path [String] Path to the file
  # @param mime_type [String, nil] Optional MIME type hint
  # @param config [Hash, Config::Extraction, nil] Extraction configuration
  # @return [Result] Extraction result object
  #
  def extract_file(path, mime_type: nil, config: nil)
    opts = Kreuzberg.normalize_config(config)
    args = [path.to_s]
    args << mime_type.to_s if mime_type
    hash = native_extract_file(*args, **opts)
    Result.new(hash)
  end

  # Extract content from bytes (asynchronous via Tokio runtime).
  #
  # @param data [String] Binary data
  # @param mime_type [String] MIME type
  # @param config [Hash, Config::Extraction, nil] Extraction configuration
  # @return [Result] Extraction result object
  #
  def extract_bytes(data, mime_type, config: nil)
    opts = Kreuzberg.normalize_config(config)
    hash = native_extract_bytes(data.to_s, mime_type.to_s, **opts)
    Result.new(hash)
  end

  # Batch extract content from multiple files (asynchronous via Tokio runtime).
  #
  # @param paths [Array<String>] List of file paths
  # @param config [Hash, Config::Extraction, nil] Extraction configuration
  # @return [Array<Result>] Array of extraction result objects
  #
  def batch_extract_files(paths, config: nil)
    opts = Kreuzberg.normalize_config(config)
    hashes = native_batch_extract_files(paths.map(&:to_s), **opts)
    hashes.map { |hash| Result.new(hash) }
  end

  module_function :clear_cache
  module_function :cache_stats

  # Register a custom post-processor.
  #
  # Post-processors enrich extraction results by adding metadata, transforming content,
  # or performing additional analysis. They are called after extraction completes.
  #
  # @param name [String] Unique name for the post-processor
  # @param processor [Proc, #call] Callable that implements the post-processor protocol
  # @return [void]
  #
  # @example Register a post-processor as a Proc
  #   Kreuzberg.register_post_processor("upcase", ->(result) {
  #     result["content"] = result["content"].upcase
  #     result
  #   })
  #
  # @example Register a post-processor class
  #   class EntityExtractor
  #     def call(result)
  #       entities = extract_entities(result["content"])
  #       result["metadata"]["entities"] = entities
  #       result
  #     end
  #   end
  #
  #   Kreuzberg.register_post_processor("entities", EntityExtractor.new)
  #
  # @see PostProcessorProtocol for detailed protocol requirements
  module_function :register_post_processor

  # Unregister a post-processor by name.
  #
  # @param name [String] Name of the post-processor to unregister
  # @return [void]
  #
  # @example
  #   Kreuzberg.unregister_post_processor("upcase")
  module_function :unregister_post_processor

  # Clear all registered post-processors.
  #
  # @return [void]
  #
  # @example
  #   Kreuzberg.clear_post_processors
  module_function :clear_post_processors

  # Register a custom validator.
  #
  # Validators are called before extraction to validate input files or configurations.
  # If validation fails, the validator should raise a Kreuzberg::Errors::ValidationError.
  #
  # @param name [String] Unique name for the validator
  # @param validator [Proc, #call] Callable that implements the validator protocol
  # @return [void]
  #
  # @example Register a validator as a Proc
  #   Kreuzberg.register_validator("file_size", ->(result) {
  #     if result["content"].length < 10
  #       raise Kreuzberg::Errors::ValidationError.new("Content too short")
  #     end
  #   })
  #
  # @example Register a validator class
  #   class FileSizeValidator
  #     def call(result)
  #       if result["content"].length < 10
  #         raise Kreuzberg::Errors::ValidationError.new("Content too short")
  #       end
  #     end
  #   end
  #
  #   Kreuzberg.register_validator("file_size", FileSizeValidator.new)
  #
  # @see ValidatorProtocol for detailed protocol requirements
  module_function :register_validator

  # Unregister a validator by name.
  #
  # @param name [String] Name of the validator to unregister
  # @return [void]
  #
  # @example
  #   Kreuzberg.unregister_validator("file_size")
  module_function :unregister_validator

  # Clear all registered validators.
  #
  # @return [void]
  #
  # @example
  #   Kreuzberg.clear_validators
  module_function :clear_validators

  # Register a custom OCR backend.
  #
  # OCR backends implement optical character recognition for images and scanned documents.
  # They must implement the OcrBackendProtocol interface.
  #
  # @param name [String] Unique name for the OCR backend
  # @param backend [#process_image, #name] Object implementing the OcrBackendProtocol
  # @return [void]
  #
  # @example Register a custom OCR backend
  #   class CustomOcrBackend
  #     def name
  #       "custom-ocr"
  #     end
  #
  #     def process_image(image_bytes, config)
  #       # Perform OCR on image_bytes
  #       text = my_ocr_engine.recognize(image_bytes)
  #       text
  #     end
  #   end
  #
  #   Kreuzberg.register_ocr_backend("custom-ocr", CustomOcrBackend.new)
  #
  # @see OcrBackendProtocol for detailed protocol requirements
  module_function :register_ocr_backend

  # Batch extract content from multiple byte arrays (synchronous).
  #
  # @param data_array [Array<String>] Array of binary data to extract
  # @param mime_types [Array<String>] Array of MIME types (must match data_array length)
  # @param config [Hash, Config::Extraction, nil] Extraction configuration
  # @return [Array<Result>] Array of extraction result objects
  #
  # @example
  #   data1 = File.binread("doc1.pdf")
  #   data2 = File.binread("doc2.docx")
  #   results = Kreuzberg.batch_extract_bytes_sync([data1, data2], ["application/pdf", "application/vnd.openxmlformats-officedocument.wordprocessingml.document"])
  #   results.each { |r| puts r.content }
  module_function :batch_extract_bytes_sync

  # Batch extract content from multiple byte arrays (asynchronous via Tokio runtime).
  #
  # @param data_array [Array<String>] Array of binary data to extract
  # @param mime_types [Array<String>] Array of MIME types (must match data_array length)
  # @param config [Hash, Config::Extraction, nil] Extraction configuration
  # @return [Array<Result>] Array of extraction result objects
  module_function :batch_extract_bytes

  # Normalize config from Hash or Config object to keyword arguments
  # @api private
  def self.normalize_config(config)
    return {} if config.nil?
    return config if config.is_a?(Hash)

    raise ArgumentError, 'config must be a Hash or respond to :to_h' unless config.respond_to?(:to_h)

    config.to_h
  end
end
