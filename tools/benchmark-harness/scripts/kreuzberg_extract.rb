#!/usr/bin/env ruby
# frozen_string_literal: true

# Kreuzberg Ruby extraction wrapper for benchmark harness.
#
# Supports two modes:
# - sync: extract_file - synchronous extraction (default)
# - batch: batch_extract_file - batch extraction for multiple files
#
# Debug output is written to stderr to avoid interfering with JSON output on stdout.

require 'kreuzberg'
require 'json'

DEBUG = ENV.fetch('KREUZBERG_BENCHMARK_DEBUG', 'false') == 'true'

def debug_log(message)
  return unless DEBUG
  warn "[DEBUG] #{Time.now.iso8601(3)} - #{message}"
end

def extract_sync(file_path)
  debug_log "=== SYNC EXTRACTION START ==="
  debug_log "Input: file_path=#{file_path}"
  debug_log "File exists: #{File.exist?(file_path)}"
  debug_log "File size: #{File.size(file_path)} bytes" if File.exist?(file_path)

  start_monotonic = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  start_wall = Time.now
  debug_log "Timing start (monotonic): #{start_monotonic.round(6)}, wall: #{start_wall.iso8601(6)}"

  result = Kreuzberg.extract_file(file_path)

  end_monotonic = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  end_wall = Time.now
  duration_s = end_monotonic - start_monotonic
  duration_ms = duration_s * 1000.0

  debug_log "Timing end (monotonic): #{end_monotonic.round(6)}, wall: #{end_wall.iso8601(6)}"
  debug_log "Duration (seconds): #{duration_s.round(6)}"
  debug_log "Duration (milliseconds): #{duration_ms.round(3)}"
  debug_log "Result class: #{result.class}"
  debug_log "Result has content: #{!result.content.nil?}"
  debug_log "Content length: #{result.content&.length || 'nil'} characters"
  debug_log "Result has metadata: #{!result.metadata.nil?}"
  debug_log "Metadata type: #{result.metadata&.class || 'nil'}"

  payload = {
    content: result.content,
    metadata: result.metadata || {},
    _extraction_time_ms: duration_ms
  }

  debug_log "Output JSON size: #{JSON.generate(payload).bytesize} bytes"
  debug_log "=== SYNC EXTRACTION END ==="

  payload
rescue StandardError => e
  debug_log "ERROR during sync extraction: #{e.class} - #{e.message}"
  debug_log "Backtrace:\n#{e.backtrace.join("\n")}"
  raise
end

def extract_batch(file_paths)
  debug_log "=== BATCH EXTRACTION START ==="
  debug_log "Input: #{file_paths.length} files"
  file_paths.each_with_index do |path, idx|
    debug_log "  [#{idx}] #{path} (exists: #{File.exist?(path)}, size: #{File.exist?(path) ? File.size(path) : 'N/A'} bytes)"
  end

  start_monotonic = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  start_wall = Time.now
  debug_log "Timing start (monotonic): #{start_monotonic.round(6)}, wall: #{start_wall.iso8601(6)}"

  results = Kreuzberg.batch_extract_file(file_paths)

  end_monotonic = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  end_wall = Time.now
  total_duration_s = end_monotonic - start_monotonic
  total_duration_ms = total_duration_s * 1000.0

  debug_log "Timing end (monotonic): #{end_monotonic.round(6)}, wall: #{end_wall.iso8601(6)}"
  debug_log "Total duration (seconds): #{total_duration_s.round(6)}"
  debug_log "Total duration (milliseconds): #{total_duration_ms.round(3)}"
  debug_log "Results count: #{results.length}"

  per_file_duration_ms = file_paths.length.positive? ? total_duration_ms / file_paths.length : 0
  debug_log "Per-file average duration (milliseconds): #{per_file_duration_ms.round(3)}"

  results_with_timing = results.map.with_index do |result, idx|
    debug_log "  Result[#{idx}] - content length: #{result.content&.length || 'nil'}, has metadata: #{!result.metadata.nil?}"
    {
      content: result.content,
      metadata: result.metadata || {},
      _extraction_time_ms: per_file_duration_ms,
      _batch_total_ms: total_duration_ms
    }
  end

  debug_log "=== BATCH EXTRACTION END ==="

  results_with_timing
rescue StandardError => e
  debug_log "ERROR during batch extraction: #{e.class} - #{e.message}"
  debug_log "Backtrace:\n#{e.backtrace.join("\n")}"
  raise
end

def main
  debug_log "Ruby script started"
  debug_log "ARGV: #{ARGV.inspect}"
  debug_log "ARGV length: #{ARGV.length}"

  if ARGV.length < 2
    warn 'Usage: kreuzberg_extract.rb <mode> <file_path> [additional_files...]'
    warn 'Modes: sync, batch'
    warn 'Debug mode: set KREUZBERG_BENCHMARK_DEBUG=true to enable debug logging to stderr'
    exit 1
  end

  mode = ARGV[0]
  file_paths = ARGV[1..]

  debug_log "Mode: #{mode}"
  debug_log "File paths (#{file_paths.length}): #{file_paths.inspect}"

  case mode
  when 'sync'
    if file_paths.length != 1
      warn 'Error: sync mode requires exactly one file'
      exit 1
    end
    debug_log "Executing sync mode with file: #{file_paths[0]}"
    payload = extract_sync(file_paths[0])
    output = JSON.generate(payload)
    debug_log "Output JSON: #{output}"
    puts output

  when 'batch'
    if file_paths.empty?
      warn 'Error: batch mode requires at least one file'
      exit 1
    end
    debug_log "Executing batch mode with #{file_paths.length} files"

    results = extract_batch(file_paths)

    # For single file in batch mode, return single result
    if file_paths.length == 1
      output = JSON.generate(results[0])
      debug_log "Output JSON (single file): #{output}"
      puts output
    else
      # For multiple files, return array
      output = JSON.generate(results)
      debug_log "Output JSON (multiple files): #{output[0..200]}..." if output.length > 200
      puts output
    end

  else
    warn "Error: Unknown mode '#{mode}'. Use sync or batch"
    exit 1
  end

  debug_log "Script completed successfully"
rescue StandardError => e
  debug_log "FATAL ERROR: #{e.class} - #{e.message}"
  debug_log "Backtrace:\n#{e.backtrace.join("\n")}"
  warn "Error extracting with Kreuzberg: #{e.message}"
  exit 1
end

main if __FILE__ == $PROGRAM_NAME
