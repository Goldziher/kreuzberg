```ruby
require 'kreuzberg'

class StatefulPlugin
  def initialize
    @lock = Mutex.new
    @count = 0
  end

  def call(result)
    @lock.synchronize { @count += 1 }
    result
  end

  def count
    @lock.synchronize { @count }
  end
end

plugin = StatefulPlugin.new
Kreuzberg.register_post_processor('counter', plugin)

config = Kreuzberg::Config::Extraction.new(
  postprocessor: { enabled: true }
)

Kreuzberg.extract_file_sync('document.pdf', config: config)
puts "Processed: #{plugin.count}"
```
