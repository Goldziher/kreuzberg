# frozen_string_literal: true

# Async Patterns for Kreuzberg Ruby Bindings
#
# This example demonstrates async patterns and concurrency approaches for Ruby,
# with comparison to the underlying Rust implementation.

require 'kreuzberg'

# NOTE: Ruby bindings use Tokio runtime with block_on() internally.
# The "async" functions block the Ruby GVL during execution, so there's
# no performance benefit over the _sync variants from Ruby's perspective.

# ============================================================================
# Pattern 1: Synchronous Extraction (Recommended)
# ============================================================================

def basic_sync_extraction
  result = Kreuzberg.extract_file_sync('document.pdf')
  puts "Content: #{result[:content]}"
  puts "MIME type: #{result[:mime_type]}"
end

# ============================================================================
# Pattern 2: "Async" Extraction (Same Performance as Sync)
# ============================================================================

def basic_async_extraction
  # This LOOKS async but actually blocks the Ruby thread
  # Internally uses: runtime.block_on(async { ... })
  result = Kreuzberg.extract_file('document.pdf')
  puts "Content: #{result[:content]}"
end

# ============================================================================
# Pattern 3: Concurrent Processing with Ruby Threads
# ============================================================================

def concurrent_with_threads
  files = ['doc1.pdf', 'doc2.pdf', 'doc3.pdf']

  # Use Ruby threads to achieve parallelism
  # Each thread calls the synchronous API
  threads = files.map do |file|
    Thread.new do
      Kreuzberg.extract_file_sync(file)
    end
  end

  results = threads.map(&:value)
  results.each_with_index do |result, index|
    puts "File #{index + 1}: #{result[:content][0..100]}"
  end
end

# ============================================================================
# Pattern 4: Batch Processing (Preferred for Multiple Files)
# ============================================================================

def batch_processing
  files = ['doc1.pdf', 'doc2.pdf', 'doc3.pdf']

  # The batch API handles concurrency internally via Rust/Tokio
  # This is more efficient than Ruby threads
  results = Kreuzberg.batch_extract_files_sync(files)

  puts "Processed #{results.length} files"
  results.each do |result|
    puts "Content preview: #{result[:content][0..50]}"
  end
end

# ============================================================================
# Pattern 5: Extraction with Configuration
# ============================================================================

def extraction_with_config
  # Configure OCR
  config = {
    ocr: {
      backend: 'tesseract',
      language: 'eng'
    },
    force_ocr: false
  }

  result = Kreuzberg.extract_file_sync('scanned.pdf', **config)
  puts "Extracted with OCR: #{result[:content]}"
end

# ============================================================================
# Pattern 6: Extract from Bytes
# ============================================================================

def extract_from_bytes
  data = File.binread('document.pdf')
  result = Kreuzberg.extract_bytes_sync(data, 'application/pdf')
  puts "Extracted from memory: #{result[:content]}"
end

# ============================================================================
# Pattern 7: Batch Extract from Bytes
# ============================================================================

def batch_extract_from_bytes
  files = ['doc1.pdf', 'doc2.pdf']
  bytes_array = files.map { |f| File.binread(f) }
  mime_types = ['application/pdf', 'application/pdf']

  results = Kreuzberg.batch_extract_bytes_sync(bytes_array, mime_types)
  puts "Processed #{results.length} files from memory"
end

# ============================================================================
# Pattern 8: Error Handling
# ============================================================================

def error_handling
  Kreuzberg.extract_file_sync('nonexistent.pdf')
rescue StandardError => e
  puts "Extraction failed: #{e.message}"
end

# ============================================================================
# Pattern 9: Sequential Processing
# ============================================================================

def sequential_processing
  files = ['doc1.pdf', 'doc2.pdf', 'doc3.pdf']

  files.each do |file|
    result = Kreuzberg.extract_file_sync(file)
    puts "Processed #{file}: #{result[:content][0..50]}"
  end
end

# ============================================================================
# Pattern 10: Background Processing with ActiveJob (Rails)
# ============================================================================

# Example ActiveJob for async processing in Rails
# < ApplicationJob
class DocumentExtractionJob
  # queue_as :default

  def perform(file_path)
    result = Kreuzberg.extract_file_sync(file_path)
    # Store result in database or process further
    puts "Background extraction complete: #{result[:content][0..100]}"
  end
end

# Usage in Rails controller:
# DocumentExtractionJob.perform_later('document.pdf')

# ============================================================================
# Pattern 11: Concurrent Processing with Parallel Gem
# ============================================================================

def concurrent_with_parallel_gem
  require 'parallel'

  files = ['doc1.pdf', 'doc2.pdf', 'doc3.pdf', 'doc4.pdf']

  # Process files in parallel using multiple CPU cores
  results = Parallel.map(files, in_processes: 4) do |file|
    Kreuzberg.extract_file_sync(file)
  end

  results.each do |result|
    puts "Content: #{result[:content][0..50]}"
  end
end

# ============================================================================
# Pattern 12: Timeout Wrapper
# ============================================================================

def extraction_with_timeout(file_path, timeout_seconds = 30)
  require 'timeout'

  Timeout.timeout(timeout_seconds) do
    Kreuzberg.extract_file_sync(file_path)
  end
rescue Timeout::Error
  puts "Extraction timed out after #{timeout_seconds} seconds"
  nil
end

# ============================================================================
# Pattern 13: Custom Ruby PostProcessor Plugin
# ============================================================================

def register_postprocessor
  # Register a Ruby-based post-processor
  uppercase_processor = lambda do |result|
    result[:content] = result[:content].upcase
    result
  end

  Kreuzberg.register_post_processor('uppercase', uppercase_processor, 100)

  # Now all extractions will use the uppercase processor
  result = Kreuzberg.extract_file_sync('document.pdf')
  puts "Uppercase content: #{result[:content]}"

  # Clean up
  Kreuzberg.unregister_post_processor('uppercase')
end

# ============================================================================
# Pattern 14: Custom Ruby Validator Plugin
# ============================================================================

def register_validator
  # Register a Ruby-based validator
  min_length_validator = lambda do |result|
    raise 'Content too short' if result[:content].length < 100
  end

  Kreuzberg.register_validator('min_length', min_length_validator, 100)

  # Validation will run automatically during extraction
  begin
    result = Kreuzberg.extract_file_sync('short_document.pdf')
    puts "Validation passed: #{result[:content]}"
  rescue StandardError => e
    puts "Validation failed: #{e.message}"
  end

  # Clean up
  Kreuzberg.unregister_validator('min_length')
end

# ============================================================================
# Pattern 15: Custom Ruby OCR Backend Plugin
# ============================================================================

# Example OCR backend implementation for custom processing.
class CustomOcrBackend
  def process_image(image_bytes, language)
    # In a real implementation, you would:
    # 1. Call an external OCR service
    # 2. Use an HTTP API
    # 3. Process with a Ruby gem
    "Extracted text from #{image_bytes.length} bytes using #{language}"
  end

  def supports_language?(lang)
    %w[eng deu fra].include?(lang)
  end
end

def register_ocr_backend
  backend = CustomOcrBackend.new
  Kreuzberg.register_ocr_backend('custom', backend)

  # Now you can use the custom backend
  config = {
    ocr: {
      backend: 'custom',
      language: 'eng'
    },
    force_ocr: true
  }

  result = Kreuzberg.extract_file_sync('scanned.pdf', **config)
  puts "Custom OCR result: #{result[:content]}"
end

# ============================================================================
# Main Demonstration
# ============================================================================

def main
  puts '=== Basic Sync Extraction ==='
  basic_sync_extraction

  puts '\n=== Basic Async Extraction (Blocks GVL) ==='
  basic_async_extraction

  puts '\n=== Concurrent with Ruby Threads ==='
  concurrent_with_threads

  puts '\n=== Batch Processing (Preferred) ==='
  batch_processing

  puts '\n=== Extraction with Config ==='
  extraction_with_config

  puts '\n=== Extract from Bytes ==='
  extract_from_bytes

  puts '\n=== Error Handling ==='
  error_handling

  puts '\n=== Sequential Processing ==='
  sequential_processing

  puts '\n=== Extraction with Timeout ==='
  extraction_with_timeout('large_document.pdf', 30)

  puts '\n=== Custom PostProcessor ==='
  register_postprocessor

  puts '\n=== Custom Validator ==='
  register_validator
end

# Run if executed directly
main if __FILE__ == $PROGRAM_NAME

# ============================================================================
# Key Takeaways:
#
# 1. Ruby bindings use Tokio runtime with block_on() internally
# 2. "Async" functions block the Ruby GVL - no concurrency benefit
# 3. Use _sync variants for clarity (same performance)
# 4. Use Ruby threads or Parallel gem for concurrent processing
# 5. Batch API is most efficient for multiple files
# 6. ActiveJob for background processing in Rails
# 7. Ruby plugins (PostProcessor, Validator, OCR) are fully supported
#
# Performance Comparison:
# - Magnus: Blocks GVL, same overhead as sync (~Xms per call)
# - PyO3 (optimized): ~0.17ms overhead, GIL released during await
# - NAPI-RS: ~0ms overhead, automatic Promise conversion
#
# When to Use Ruby Bindings:
# ✅ Rails applications (ActiveJob for background processing)
# ✅ Ruby scripts (existing Ruby codebases)
# ✅ Simple extraction (single-file processing)
# ✅ Batch processing (batch API handles concurrency)
#
# Consider Other Bindings For:
# ❌ High concurrency (use Node.js/NAPI-RS instead)
# ❌ Real-time processing (use Node.js/NAPI-RS instead)
# ❌ I/O-bound workloads (use Python/PyO3 or Node.js/NAPI-RS)
#
# See packages/ruby/ext/kreuzberg_rb/native/README.md for detailed async runtime documentation.
# ============================================================================
