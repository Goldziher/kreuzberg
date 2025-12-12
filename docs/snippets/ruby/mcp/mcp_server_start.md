```ruby title="Ruby"
require 'open3'

begin
  Open3.popen3('kreuzberg', 'mcp') do |stdin, stdout, stderr, wait_thr|
    puts stdout.read
    wait_thr.join
  end
rescue => e
  puts "Failed to start MCP server: #{e.message}"
end
```
