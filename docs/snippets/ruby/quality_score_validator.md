```ruby
require "kreuzberg"

# Custom validator checking document quality score
class QualityScoreValidator
  def initialize(min_score: 0.5)
    @min_score = min_score
  end

  def call(result)
    metadata = result["metadata"] || {}
    quality_score = metadata["quality_score"] || 0.0

    if quality_score < @min_score
      raise Kreuzberg::Errors::ValidationError,
            format("Quality score too low: %.2f < %.2f", quality_score, @min_score)
    end
  end
end

# Register with default minimum score of 0.5
validator = QualityScoreValidator.new(min_score: 0.5)
Kreuzberg.register_validator("quality_score_check", validator)

# Usage with quality processing enabled
config = Kreuzberg::Config::Extraction.new(
  enable_quality_processing: true
)

begin
  result = Kreuzberg.extract_file("document.pdf", config)
  puts "Document quality verified: #{result['metadata']['quality_score']}"
rescue Kreuzberg::Errors::ValidationError => e
  puts "Quality check failed: #{e.message}"
end
```
