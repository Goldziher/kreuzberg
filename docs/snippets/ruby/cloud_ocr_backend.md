```ruby
require 'kreuzberg'
require 'net/http'
require 'json'

# Custom cloud-based OCR backend
class CloudOcrBackend
  def name
    'cloud-ocr'
  end

  def supported_languages
    ['eng', 'fra', 'deu', 'spa', 'jpn', 'chi_sim']
  end

  def process_image(image_data, language)
    # Send image to cloud OCR service
    uri = URI('https://api.example.com/ocr')
    request = Net::HTTP::Post.new(uri)
    request['Content-Type'] = 'image/jpeg'
    request['Accept-Language'] = language
    request['Authorization'] = "Bearer #{ENV['OCR_API_KEY']}"
    request.body = image_data

    response = Net::HTTP.start(uri.hostname, uri.port, use_ssl: true) do |http|
      http.request(request)
    end

    unless response.is_a?(Net::HTTPSuccess)
      raise Kreuzberg::Errors::OCRError,
            "Cloud OCR failed: #{response.code} #{response.message}"
    end

    # Parse JSON response
    data = JSON.parse(response.body)

    # Return in expected format
    {
      content: data['text'],
      mime_type: 'text/plain',
      metadata: {
        backend: 'cloud-ocr',
        language: language,
        confidence: data['confidence'],
        processing_time_ms: data['processing_time']
      },
      tables: data['tables'] || []
    }
  rescue JSON::ParserError => e
    raise Kreuzberg::Errors::OCRError,
          "Failed to parse cloud OCR response: #{e.message}"
  rescue StandardError => e
    raise Kreuzberg::Errors::OCRError,
          "Cloud OCR error: #{e.message}"
  end
end

# Register the OCR backend
backend = CloudOcrBackend.new
Kreuzberg.register_ocr_backend(backend)

# Use with extraction
config = {
  ocr: {
    backend: 'cloud-ocr',
    language: 'eng'
  }
}

result = Kreuzberg.extract_file_sync('scanned_document.pdf', config)
puts "OCR Confidence: #{result[:metadata][:confidence]}"
puts result[:content]
```
