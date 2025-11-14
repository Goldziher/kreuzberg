# frozen_string_literal: true

module Kreuzberg
  module ExtractionAPI
    def extract_file_sync(path, mime_type: nil, config: nil)
      opts = normalize_config(config)
      hash = if mime_type
               native_extract_file_sync(path.to_s, mime_type.to_s, **opts)
             else
               native_extract_file_sync(path.to_s, **opts)
             end
      result = Result.new(hash)
      record_cache_entry!(result, opts)
      result
    end

    def extract_bytes_sync(data, mime_type, config: nil)
      opts = normalize_config(config)
      hash = native_extract_bytes_sync(data.to_s, mime_type.to_s, **opts)
      result = Result.new(hash)
      record_cache_entry!(result, opts)
      result
    end

    def batch_extract_files_sync(paths, config: nil)
      opts = normalize_config(config)
      hashes = native_batch_extract_files_sync(paths.map(&:to_s), **opts)
      results = hashes.map { |hash| Result.new(hash) }
      record_cache_entry!(results, opts)
      results
    end

    def extract_file(path, mime_type: nil, config: nil)
      opts = normalize_config(config)
      hash = if mime_type
               native_extract_file(path.to_s, mime_type.to_s, **opts)
             else
               native_extract_file(path.to_s, **opts)
             end
      result = Result.new(hash)
      record_cache_entry!(result, opts)
      result
    end

    def extract_bytes(data, mime_type, config: nil)
      opts = normalize_config(config)
      hash = native_extract_bytes(data.to_s, mime_type.to_s, **opts)
      result = Result.new(hash)
      record_cache_entry!(result, opts)
      result
    end

    def batch_extract_files(paths, config: nil)
      opts = normalize_config(config)
      hashes = native_batch_extract_files(paths.map(&:to_s), **opts)
      results = hashes.map { |hash| Result.new(hash) }
      record_cache_entry!(results, opts)
      results
    end

    def batch_extract_bytes_sync(data_array, mime_types, config: nil)
      opts = normalize_config(config)
      hashes = native_batch_extract_bytes_sync(data_array.map(&:to_s), mime_types.map(&:to_s), **opts)
      results = hashes.map { |hash| Result.new(hash) }
      record_cache_entry!(results, opts)
      results
    end

    def batch_extract_bytes(data_array, mime_types, config: nil)
      opts = normalize_config(config)
      hashes = native_batch_extract_bytes(data_array.map(&:to_s), mime_types.map(&:to_s), **opts)
      results = hashes.map { |hash| Result.new(hash) }
      record_cache_entry!(results, opts)
      results
    end

    def normalize_config(config)
      return {} if config.nil?
      return config if config.is_a?(Hash)

      config.to_h
    end
  end
end
