```ruby
require "kreuzberg"
require "thread"

# Stateful plugin with thread-safe state management using Mutex
class StatefulPlugin
  def initialize
    # Use Mutex to ensure thread-safe access to shared state
    @state_lock = Mutex.new
    @call_count = 0
    @cache = {}
    @mime_types_seen = Set.new
  end

  def call(result)
    # Safely update state across multiple threads
    @state_lock.synchronize do
      @call_count += 1

      # Track MIME types encountered
      mime_type = result["mime_type"]
      @mime_types_seen.add(mime_type)

      # Cache the last result metadata
      @cache["last_mime"] = mime_type
      @cache["last_processed_at"] = Time.now
      @cache["total_calls"] = @call_count
    end

    result
  end

  # Thread-safe method to get statistics
  def statistics
    @state_lock.synchronize do
      {
        call_count: @call_count,
        mime_types_seen: @mime_types_seen.to_a,
        cache: @cache.dup
      }
    end
  end

  # Thread-safe method to reset state
  def reset
    @state_lock.synchronize do
      @call_count = 0
      @cache.clear
      @mime_types_seen.clear
    end
  end
end

# Initialize plugin
plugin = StatefulPlugin.new
Kreuzberg.register_post_processor("stateful_tracker", plugin)

# Simulate concurrent processing
config = Kreuzberg::Config::Extraction.new(
  postprocessor: { enabled: true }
)

# Sequential extractions
%w[document.pdf image.png data.docx].each do |file|
  Kreuzberg.extract_file(file, config)
end

# Check accumulated statistics
stats = plugin.statistics
puts "Processed #{stats[:call_count]} files"
puts "MIME types seen: #{stats[:mime_types_seen].join(', ')}"
puts "Last processed: #{stats[:cache]['last_processed_at']}"
```
