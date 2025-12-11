```ruby
require 'kreuzberg'
require 'benchmark'

config = Kreuzberg::ExtractionConfig.new(use_cache: false)
kreuzberg = Kreuzberg::Client.new(config)
file_path = 'document.pdf'
num_runs = 10

puts "Sync extraction (#{num_runs} runs):"
sync_time = Benchmark.realtime do
  num_runs.times do
    kreuzberg.extract_file(file_path)
  end
end
avg_sync = sync_time / num_runs
puts "  - Total time: #{sync_time.round(3)}s"
puts "  - Average: #{avg_sync.round(3)}s per extraction"

puts "\nAsync extraction (#{num_runs} parallel runs):"
async_time = Benchmark.realtime do
  threads = num_runs.times.map do
    Thread.new { kreuzberg.extract_file(file_path) }
  end
  threads.map(&:join)
end
puts "  - Total time: #{async_time.round(3)}s"
puts "  - Average: #{(async_time / num_runs).round(3)}s per extraction"
puts "  - Speedup: #{(sync_time / async_time).round(1)}x"

cache_config = Kreuzberg::ExtractionConfig.new(use_cache: true)
kreuzberg_cached = Kreuzberg::Client.new(cache_config)

puts "\nFirst extraction (populates cache)..."
first_time = Benchmark.realtime do
  kreuzberg_cached.extract_file(file_path)
end
puts "  - Time: #{first_time.round(3)}s"

puts "Second extraction (from cache)..."
cached_time = Benchmark.realtime do
  kreuzberg_cached.extract_file(file_path)
end
puts "  - Time: #{cached_time.round(3)}s"
puts "  - Cache speedup: #{(first_time / cached_time).round(1)}x"
```
