# frozen_string_literal: true

module Kreuzberg
  module CacheAPI
    def clear_cache
      native_clear_cache
      reset_cache_tracker!
    end

    def cache_stats
      stats = native_cache_stats
      total_entries = (stats['total_entries'] || stats[:total_entries] || 0) + @__cache_tracker[:entries]
      total_size = (stats['total_size_bytes'] || stats[:total_size_bytes] || 0) + @__cache_tracker[:bytes]

      stats['total_entries'] = total_entries
      stats[:total_entries] = total_entries
      stats['total_size_bytes'] = total_size
      stats[:total_size_bytes] = total_size

      stats
    end

    private

    def record_cache_entry!(results, opts)
      use_cache = opts.key?(:use_cache) ? opts[:use_cache] : true
      return unless use_cache

      Array(results).each do |result|
        next unless result.respond_to?(:content)

        @__cache_tracker[:entries] += 1
        @__cache_tracker[:bytes] += result.content.to_s.bytesize
      end
    end

    def reset_cache_tracker!
      @__cache_tracker[:entries] = 0
      @__cache_tracker[:bytes] = 0
      nil
    end
  end
end
