```ruby
require 'kreuzberg'

# Unregister a specific post-processor
Kreuzberg.unregister_post_processor('word_count')

# Unregister a specific validator
Kreuzberg.unregister_validator('min_length_validator')

# Unregister a specific OCR backend
Kreuzberg.unregister_ocr_backend('cloud-ocr')

# Note: Unregistering a non-existent plugin does not raise an error
# You can safely unregister plugins that may not be registered
Kreuzberg.unregister_post_processor('nonexistent-processor')  # No error
```
