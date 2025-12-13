```ruby title="Ruby"
require 'net/http'
require 'uri'
require 'json'

# Multiple files
form_data = [
  ['files', File.open('doc1.pdf')],
  ['files', File.open('doc2.docx')]
]
request.set_form form_data, 'multipart/form-data'
```
