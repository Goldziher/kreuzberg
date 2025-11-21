```ruby
require "kreuzberg"

# Custom post-processor that adds word count to metadata
class WordCountProcessor
  def call(result)
    # Only process if content exists
    return result if result["content"].empty?

    # Calculate word count by splitting on whitespace
    word_count = result["content"].split.length

    # Update metadata with word count
    result["metadata"] ||= {}
    result["metadata"]["word_count"] = word_count

    # Return modified result
    result
  end
end

# Register processor to run early in pipeline
processor = WordCountProcessor.new
Kreuzberg.register_post_processor("word_count", processor, 10)

# Usage in extraction pipeline
config = Kreuzberg::Config::Extraction.new(
  postprocessor: {
    enabled: true
  }
)

result = Kreuzberg.extract_file("document.pdf", config)
puts "Document has #{result['metadata']['word_count']} words"
```
