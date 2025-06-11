# OCR Backends

Kreuzberg supports multiple OCR backends for text extraction from images and documents. Each backend has its own configuration options and capabilities.

## Available Backends

- Tesseract OCR
- EasyOCR
- PaddleOCR

## GPU Acceleration

Kreuzberg supports GPU acceleration for OCR backends that support it (currently EasyOCR and PaddleOCR). This can significantly improve performance for large documents or high-resolution images.

### Device Support

The following compute devices are supported:

- CPU (default)
- CUDA (NVIDIA GPUs)
- MPS (Apple Silicon GPUs)

### Configuration

GPU settings can be configured through the OCR backend configuration:

```python
from kreuzberg import ExtractionConfig
from kreuzberg._ocr._easyocr import EasyOCRConfig

# Auto-detect best device
config = ExtractionConfig(
    ocr_backend="easyocr",
    ocr_config=EasyOCRConfig(
        device="auto"  # Will use CUDA if available, then MPS, then CPU
    )
)

# Explicitly request GPU with memory limit
config = ExtractionConfig(
    ocr_backend="easyocr",
    ocr_config=EasyOCRConfig(
        use_gpu=True,
        device="cuda",
        gpu_memory_limit=4.0  # Limit GPU memory to 4GB
    )
)
```

### Memory Management

To prevent out-of-memory errors, you can set a memory limit for GPU operations:

```python
config = EasyOCRConfig(
    use_gpu=True,
    gpu_memory_limit=4.0  # Limit to 4GB
)
```

The memory limit is enforced per process and will automatically fall back to CPU if the limit is exceeded.

### Fallback Behavior

If GPU acceleration is requested but not available, the system will automatically fall back to CPU processing. This ensures that OCR operations continue to work even without GPU support.

## Backend-Specific Configuration

### EasyOCR

```python
from kreuzberg._ocr._easyocr import EasyOCRConfig

config = EasyOCRConfig(
    language=["en"],  # List of languages
    use_gpu=True,  # Enable GPU acceleration
    device="cuda",  # Use CUDA device
    gpu_memory_limit=4.0,  # Limit GPU memory to 4GB
    # ... other EasyOCR specific settings
)
```

### PaddleOCR

```python
from kreuzberg._ocr._paddleocr import PaddleOCRConfig

config = PaddleOCRConfig(
    language=["en"],  # List of languages
    use_gpu=True,  # Enable GPU acceleration
    device="cuda",  # Use CUDA device
    gpu_memory_limit=4.0,  # Limit GPU memory to 4GB
    # ... other PaddleOCR specific settings
)
```

## Performance Considerations

- GPU acceleration is most beneficial for:
  - Large documents
  - High-resolution images
  - Batch processing
  - Real-time applications
- CPU processing may be more efficient for:
  - Small documents
  - Low-resolution images
  - Single-page processing
- Memory usage can be monitored through logging:
  ```python
  import logging
  logging.basicConfig(level=logging.DEBUG)
  ```

## Error Handling

The system provides clear error messages when:
- Requested GPU is not available
- Memory limit is exceeded
- Device initialization fails

Example error handling:
```python
from kreuzberg.exceptions import DeviceNotAvailableError

try:
    result = extractor.process_image(image)
except DeviceNotAvailableError as e:
    print(f"GPU not available: {e}")
    # Fall back to CPU processing
    config.use_gpu = False
    result = extractor.process_image(image)
``` 