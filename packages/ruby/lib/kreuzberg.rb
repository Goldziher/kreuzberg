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
  autoload :ErrorContext, 'kreuzberg/error_context'
  autoload :PostProcessorProtocol, 'kreuzberg/post_processor_protocol'
  autoload :ValidatorProtocol, 'kreuzberg/validator_protocol'
  autoload :OcrBackendProtocol, 'kreuzberg/ocr_backend_protocol'

  # Alias for API consistency with other language bindings
  ExtractionConfig = Config::Extraction

  @__cache_tracker = { entries: 0, bytes: 0 }

  class << self
    # Store native methods as private methods
    alias native_extract_file_sync extract_file_sync
    alias native_extract_bytes_sync extract_bytes_sync
    alias native_batch_extract_files_sync batch_extract_files_sync
    alias native_extract_file extract_file
    alias native_extract_bytes extract_bytes
    alias native_batch_extract_files batch_extract_files
    alias native_batch_extract_bytes_sync batch_extract_bytes_sync
    alias native_batch_extract_bytes batch_extract_bytes
    alias native_clear_cache clear_cache
    alias native_cache_stats cache_stats

    private :native_extract_file_sync, :native_extract_bytes_sync, :native_batch_extract_files_sync
    private :native_extract_file, :native_extract_bytes, :native_batch_extract_files
    private :native_batch_extract_bytes_sync, :native_batch_extract_bytes
  end

  # Register a Ruby post-processor that conforms to PostProcessorProtocol.
  module_function :register_post_processor

  # Remove a post-processor by name.
  module_function :unregister_post_processor

  # Purge all registered post-processors.
  module_function :clear_post_processors

  # Register a validator that follows ValidatorProtocol.
  module_function :register_validator

  # Remove a validator by name.
  module_function :unregister_validator

  # Purge all validators.
  module_function :clear_validators

  # List all registered validators.
  module_function :list_validators

  # List all registered post-processors.
  module_function :list_post_processors

  # Register an OCR backend instance implementing OcrBackendProtocol.
  module_function :register_ocr_backend

  # Unregister an OCR backend by name.
  module_function :unregister_ocr_backend

  # List all registered OCR backends.
  module_function :list_ocr_backends
end

require_relative 'kreuzberg/cache_api'
require_relative 'kreuzberg/extraction_api'

Kreuzberg.singleton_class.prepend(Kreuzberg::CacheAPI)
Kreuzberg.singleton_class.prepend(Kreuzberg::ExtractionAPI)
