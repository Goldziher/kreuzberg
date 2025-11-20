```ruby
require 'kreuzberg'

result = Kreuzberg.extract_file_sync('document.pdf')

# Access PDF metadata
if result.metadata['pdf']
  pdf_meta = result.metadata['pdf']
  puts "Pages: #{pdf_meta['page_count']}"
  puts "Author: #{pdf_meta['author']}"
  puts "Title: #{pdf_meta['title']}"
end

# Access HTML metadata
html_result = Kreuzberg.extract_file_sync('page.html')
if html_result.metadata['html']
  html_meta = html_result.metadata['html']
  puts "Title: #{html_meta['title']}"
  puts "Description: #{html_meta['description']}"
  puts "Open Graph Image: #{html_meta['og_image']}"
end
```
