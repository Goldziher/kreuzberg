#!/usr/bin/env ruby
# frozen_string_literal: true

# Kreuzberg Ruby extraction wrapper for benchmark harness.
#
# Supports two modes:
# - sync: extract_file - synchronous extraction (default)
# - batch: batch_extract_file - batch extraction for multiple files

require 'kreuzberg'
require 'json'

def extract_sync(file_path)
  start = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  result = Kreuzberg.extract_file(file_path)
  duration_ms = (Process.clock_gettime(Process::CLOCK_MONOTONIC) - start) * 1000.0

  {
    content: result.content,
    metadata: result.metadata || {},
    _extraction_time_ms: duration_ms
  }
end

def extract_batch(file_paths)
  start = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  results = Kreuzberg.batch_extract_file(file_paths)
  total_duration_ms = (Process.clock_gettime(Process::CLOCK_MONOTONIC) - start) * 1000.0

  # Calculate per-file duration (approximate)
  per_file_duration_ms = file_paths.length.positive? ? total_duration_ms / file_paths.length : 0

  results.map do |result|
    {
      content: result.content,
      metadata: result.metadata || {},
      _extraction_time_ms: per_file_duration_ms,
      _batch_total_ms: total_duration_ms
    }
  end
end

def main
  if ARGV.length < 2
    warn 'Usage: kreuzberg_extract.rb <mode> <file_path> [additional_files...]'
    warn 'Modes: sync, batch'
    exit 1
  end

  mode = ARGV[0]
  file_paths = ARGV[1..]

  case mode
  when 'sync'
    if file_paths.length != 1
      warn 'Error: sync mode requires exactly one file'
      exit 1
    end
    payload = extract_sync(file_paths[0])
    puts JSON.generate(payload)

  when 'batch'
    if file_paths.empty?
      warn 'Error: batch mode requires at least one file'
      exit 1
    end

    results = extract_batch(file_paths)

    # For single file in batch mode, return single result
    if file_paths.length == 1
      puts JSON.generate(results[0])
    else
      # For multiple files, return array
      puts JSON.generate(results)
    end

  else
    warn "Error: Unknown mode '#{mode}'. Use sync or batch"
    exit 1
  end
rescue StandardError => e
  warn "Error extracting with Kreuzberg: #{e.message}"
  exit 1
end

main if __FILE__ == $PROGRAM_NAME
