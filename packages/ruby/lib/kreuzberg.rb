# frozen_string_literal: true

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

  # Clear the extraction cache.
  #
  # @return [void]
  #
  # @example
  #   Kreuzberg.clear_cache
  #
  def clear_cache
    # TODO: Implement cache clearing in Rust FFI
    nil
  end

  # Get cache statistics.
  #
  # @return [Hash] Cache statistics with :total_entries and :total_size_bytes
  #
  # @example
  #   stats = Kreuzberg.cache_stats
  #   puts "Cache entries: #{stats[:total_entries]}"
  #   puts "Cache size: #{stats[:total_size_bytes]} bytes"
  #
  def cache_stats
    # TODO: Implement cache stats in Rust FFI
    { total_entries: 0, total_size_bytes: 0 }
  end

  # Normalize config from Hash or Config object to keyword arguments
  # @api private
  def self.normalize_config(config)
    return {} if config.nil?
    return config if config.is_a?(Hash)

    raise ArgumentError, 'config must be a Hash or respond to :to_h' unless config.respond_to?(:to_h)

    config.to_h
  end
end
