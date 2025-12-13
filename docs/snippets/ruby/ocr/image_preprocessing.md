```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  ocr: Kreuzberg::Config::OCR.new(
    tesseract_config: Kreuzberg::Config::Tesseract.new(
      preprocessing: Kreuzberg::Config::ImagePreprocessing.new(
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
