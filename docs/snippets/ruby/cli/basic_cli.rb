```ruby
require 'json'
require 'open3'

def extract_with_cli(file_path, output_format = 'text')
  stdout, stderr, status = Open3.capture3(
    'kreuzberg', 'extract', file_path, '--format', output_format
  )

  unless status.success?
    warn "Error: #{stderr}"
    exit 1
  end

  return JSON.parse(stdout) if output_format == 'json'
  stdout
end

document = 'document.pdf'

text_output = extract_with_cli(document, 'text')
puts "Extracted: #{text_output.length} characters"

json_output = extract_with_cli(document, 'json')
puts "Format: #{json_output['format']}"
```
