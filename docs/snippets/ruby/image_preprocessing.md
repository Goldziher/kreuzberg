```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  ocr: Kreuzberg::OcrConfig.new(
    tesseract_config: Kreuzberg::TesseractConfig.new(
      preprocessing: Kreuzberg::ImagePreprocessingConfig.new(
        target_dpi: 300,
        denoise: true,
        deskew: true,
        contrast_enhance: true,
        binarization_method: 'otsu'
      )
    )
  )
)
```
