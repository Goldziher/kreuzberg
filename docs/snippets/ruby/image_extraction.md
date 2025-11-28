```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  images: Kreuzberg::Config::ImageExtraction.new(
    extract_images: true,
    target_dpi: 200,
    max_image_dimension: 2048,
    auto_adjust_dpi: true
  )
)
```
