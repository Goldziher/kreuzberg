```ruby
require 'kreuzberg'

# Clear all post-processors
Kreuzberg.clear_post_processors

# Clear all validators
Kreuzberg.clear_validators

# Note: OCR backends do not have a bulk clear method
# Unregister OCR backends individually using unregister_ocr_backend()
```
