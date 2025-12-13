```ruby title="Ruby"
require 'kreuzberg'

result = Kreuzberg.extract_file_sync('document.pdf')

# Iterate over tables
result.tables.each do |table|
  puts "Table with #{table['cells'].length} rows"
  puts table['markdown']  # Markdown representation

  # Access cells
  table['cells'].each do |row|
    puts row
  end
end
```
