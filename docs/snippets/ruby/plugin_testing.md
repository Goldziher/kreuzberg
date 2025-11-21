```ruby
require "kreuzberg"
require "rspec"

describe "Plugin Testing" do
  describe "PostProcessor" do
    it "should add metadata to extraction result" do
      processor = lambda do |result|
        result.metadata["processed"] = true
        result.metadata["processed_at"] = Time.now.iso8601
        result
      end

      Kreuzberg.register_post_processor("test-processor", processor)

      # Create a mock result
      require "kreuzberg/result"
      mock_result = Kreuzberg::Result.new(
        content: "Test content",
        mime_type: "text/plain",
        metadata: { "custom" => "value" },
        tables: [],
        detected_languages: [],
        chunks: nil,
        images: nil
      )

      # In production, the processor would be called by the extraction pipeline
      # For unit testing, we can call it directly:
      processed = processor.call(mock_result)

      expect(processed.metadata["processed"]).to be true
      expect(processed.metadata["custom"]).to eq("value")

      Kreuzberg.unregister_post_processor("test-processor")
    end
  end

  describe "Validator" do
    it "should validate content length" do
      validator = lambda do |result|
        raise Kreuzberg::ValidationError, "Content too short" if result.content.length < 10
      end

      Kreuzberg.register_validator("length-validator", validator)

      # Create a mock result
      require "kreuzberg/result"
      short_result = Kreuzberg::Result.new(
        content: "Short",
        mime_type: "text/plain",
        metadata: {},
        tables: [],
        detected_languages: [],
        chunks: nil,
        images: nil
      )

      expect { validator.call(short_result) }.to raise_error(Kreuzberg::ValidationError, /Content too short/)

      Kreuzberg.unregister_validator("length-validator")
    end

    it "should pass validation for valid content" do
      validator = lambda do |result|
        raise Kreuzberg::ValidationError, "Content too short" if result.content.length < 10
      end

      valid_result = Kreuzberg::Result.new(
        content: "This is a valid long content",
        mime_type: "text/plain",
        metadata: {},
        tables: [],
        detected_languages: [],
        chunks: nil,
        images: nil
      )

      expect { validator.call(valid_result) }.not_to raise_error
    end

    it "should support protocol-based validators" do
      class CustomValidator
        def call(result)
          raise Kreuzberg::ValidationError, "Empty content" if result.content.empty?
        end
      end

      validator = CustomValidator.new
      Kreuzberg.register_validator("custom-validator", validator)

      valid_result = Kreuzberg::Result.new(
        content: "Content",
        mime_type: "text/plain",
        metadata: {},
        tables: [],
        detected_languages: [],
        chunks: nil,
        images: nil
      )

      expect { validator.call(valid_result) }.not_to raise_error

      Kreuzberg.unregister_validator("custom-validator")
    end
  end
end
```
