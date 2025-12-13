```ruby title="Ruby"
require 'kreuzberg'
require 'logger'

logger = Logger.new($stdout)

class LoggingPostProcessor
  def call(result)
    puts "Processing: #{result['mime_type']}"
    puts "Content: #{result['content'].length} bytes"
    result
  end
end

class LoggingValidator
  def call(result)
    puts "Validating: #{result['content'].length} bytes"
    raise Kreuzberg::Errors::ValidationError, 'Too short' if result['content'].length < 50
  end
end

processor = LoggingPostProcessor.new
validator = LoggingValidator.new

Kreuzberg.register_post_processor('logging-proc', processor)
Kreuzberg.register_validator('logging-val', validator)

logger.info('Plugins registered')
```
