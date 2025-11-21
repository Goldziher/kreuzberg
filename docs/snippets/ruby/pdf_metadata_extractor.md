```ruby
require "kreuzberg"

# Stateful post-processor that extracts and enriches PDF metadata
class PdfMetadataExtractor
  def initialize
    @processed_count = 0
    @start_time = Time.now
  end

  def call(result)
    # Only process PDF files
    return result unless result["mime_type"] == "application/pdf"

    # Increment counter
    @processed_count += 1

    # Extract and enrich PDF-specific metadata
    result["metadata"] ||= {}
    result["metadata"]["pdf_processed"] = true
    result["metadata"]["pdf_processing_order"] = @processed_count

    # Add processing timestamp
    result["metadata"]["pdf_processed_at"] = Time.now.iso8601

    # Log processing (in production, use proper logging)
    elapsed = Time.now - @start_time
    result["metadata"]["pdf_total_elapsed_seconds"] = elapsed

    result
  end

  # Report statistics
  def statistics
    {
      processed_count: @processed_count,
      elapsed_time: Time.now - @start_time
    }
  end
end

# Initialize and register processor
extractor = PdfMetadataExtractor.new
Kreuzberg.register_post_processor("pdf_metadata_extractor", extractor)

# Usage with multiple files
config = Kreuzberg::Config::Extraction.new(
  postprocessor: { enabled: true }
)

files = ["report1.pdf", "report2.pdf", "document.pdf"]
results = files.map do |file|
  Kreuzberg.extract_file(file, config)
end

puts "Processed #{extractor.statistics[:processed_count]} PDFs"
puts "Elapsed time: #{extractor.statistics[:elapsed_time].round(2)}s"
```
