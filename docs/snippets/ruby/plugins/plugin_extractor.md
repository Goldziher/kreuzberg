```ruby title="Ruby"
require 'kreuzberg'

class CustomPostProcessor
  def call(result)
    result['metadata'] ||= {}
    result['metadata']['processed_by'] = 'CustomPostProcessor'
    result
  end
end

class CustomValidator
  def call(result)
    raise Kreuzberg::Errors::ValidationError, 'Empty' if result['content'].empty?
  end
end

processor = CustomPostProcessor.new
validator = CustomValidator.new

Kreuzberg.register_post_processor('custom', processor)
Kreuzberg.register_validator('custom', validator)
```
