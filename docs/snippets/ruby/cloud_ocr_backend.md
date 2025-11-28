```ruby
require 'kreuzberg'
require 'net/http'

class CloudOcrBackend
  def name
    'cloud-ocr'
  end

  def supported_languages
    %w[eng fra deu]
  end

  def process_image(image_data, language)
    uri = URI('https://api.example.com/ocr')
    req = Net::HTTP::Post.new(uri)
    req['Authorization'] = "Bearer #{ENV['OCR_API_KEY']}"
    req.body = image_data
    res = Net::HTTP.start(uri.hostname, uri.port, use_ssl: true) { |h| h.request(req) }
    raise Kreuzberg::Errors::OCRError, res.message unless res.is_a?(Net::HTTPSuccess)
    { content: JSON.parse(res.body)['text'] }
  rescue StandardError => e
    raise Kreuzberg::Errors::OCRError, e.message
  end
end

Kreuzberg.register_ocr_backend(CloudOcrBackend.new)
config = Kreuzberg::Config::Extraction.new(
  ocr: Kreuzberg::Config::OCR.new(backend: 'cloud-ocr')
)
Kreuzberg.extract_file_sync('doc.pdf', config: config)
```
