```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  pdf_options: Kreuzberg::Config::PDF.new(
    extract_images: true,
    extract_metadata: true,
    passwords: ['password1', 'password2']
  )
)
```
