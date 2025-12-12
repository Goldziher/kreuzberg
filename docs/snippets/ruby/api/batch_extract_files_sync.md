```ruby title="Ruby"
require 'kreuzberg'

files = ['doc1.pdf', 'doc2.docx', 'doc3.pptx']

results = Kreuzberg.batch_extract_files_sync(files)

results.each_with_index do |result, i|
  puts "File #{i + 1}: #{result.content.length} characters"
end
```
