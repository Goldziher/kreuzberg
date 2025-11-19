```ruby
require 'kreuzberg'

files = ['doc1.pdf', 'doc2.docx']

data_list = files.map { |f| File.binread(f) }
mime_types = files.map do |f|
  f.end_with?('.pdf') ? 'application/pdf' :
    'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
end

results = Kreuzberg.batch_extract_bytes_sync(
  data_list,
  mime_types
)

results.each_with_index do |result, i|
  puts "Document #{i + 1}: #{result.content.length} characters"
end
```
