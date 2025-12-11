```ruby
require 'kreuzberg'

class PdfMetadataExtractor
  def initialize
    @count = 0
  end

  def call(result)
    return result unless result['mime_type'] == 'application/pdf'
    @count += 1
    result['metadata'] ||= {}
    result['metadata']['pdf_order'] = @count
    result
  end
end

extractor = PdfMetadataExtractor.new
Kreuzberg.register_post_processor('pdf_metadata', extractor)

config = Kreuzberg::Config::Extraction.new(
  postprocessor: { enabled: true }
)

result = Kreuzberg.extract_file_sync('report.pdf', config: config)
puts "Metadata: #{result.metadata.inspect}"
```
