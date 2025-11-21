```ruby
require "kreuzberg"
require "logger"

logger = Logger.new($stdout)
logger.level = Logger::INFO

class LoggingPostProcessor
  def name
    "logging-processor"
  end

  def priority
    5
  end

  def process(result)
    logger = Logger.new($stdout)
    logger.info("[PostProcessor] Processing #{result.mime_type}")
    logger.info("[PostProcessor] Content length: #{result.content.length}")

    if result.content.empty?
      logger.warn("[PostProcessor] Warning: Empty content extracted")
    end

    result
  end
end

class LoggingValidator
  def name
    "logging-validator"
  end

  def priority
    100
  end

  def call(result)
    logger = Logger.new($stdout)
    logger.info("[Validator] Validating extraction result (#{result.content.length} bytes)")

    if result.content.length < 50
      logger.error("[Validator] Error: Content below minimum threshold")
      raise Kreuzberg::ValidationError, "Content too short"
    end
  end
end

# Register plugins with logging
processor = LoggingPostProcessor.new
validator = LoggingValidator.new

Kreuzberg.register_post_processor("logging-processor", processor)
Kreuzberg.register_validator("logging-validator", validator, priority: 100)

logger.info("[Main] Plugins registered with logging enabled")
```
