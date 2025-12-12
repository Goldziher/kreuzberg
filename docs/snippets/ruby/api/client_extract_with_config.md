```ruby title="Ruby"
require 'net/http'
require 'uri'
require 'json'

# With configuration
form_data_with_config = [
  ['files', File.open('scanned.pdf')],
  ['config', '{"ocr":{"language":"eng"},"force_ocr":true}']
]
request.set_form form_data_with_config, 'multipart/form-data'
```
