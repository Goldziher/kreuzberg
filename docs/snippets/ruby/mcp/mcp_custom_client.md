```ruby title="Ruby"
require 'json'
require 'open3'

Open3.popen3('kreuzberg', 'mcp') do |stdin, stdout, stderr, wait_thr|
  request = {
    method: 'tools/call',
    params: {
      name: 'extract_file',
      arguments: { path: 'document.pdf', async: true }
    }
  }

  stdin.puts JSON.generate(request)
  stdin.close_write

  response = stdout.gets
  result = JSON.parse(response)
  puts JSON.pretty_generate(result)
end
```
