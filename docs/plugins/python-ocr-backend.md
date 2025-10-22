# Python OCR Backend Development

OCR backends extract text from images and scanned documents. This guide covers implementing custom OCR engines in Python.

## Overview

Custom OCR backends allow you to:
- Integrate cloud OCR services (Google Vision, AWS Textract, Azure)
- Use specialized ML models (PyTorch, TensorFlow)
- Implement domain-specific OCR (handwriting, ancient scripts)
- Add custom preprocessing pipelines

## Basic OCR Backend

### Minimal Implementation

```python
from kreuzberg import register_ocr_backend
from kreuzberg.types import OcrResult

class SimpleOCR:
    def process_image(self, image_bytes: bytes, language: str) -> OcrResult:
        """Process image and return OCR result."""
        # Your OCR logic here
        text = self._extract_text(image_bytes, language)

        return OcrResult(
            text=text,
            words=[],  # Optional word-level details
            confidence=0.95,
        )

    def _extract_text(self, image_bytes: bytes, language: str) -> str:
        """Extract text from image."""
        # Implementation
        return "Extracted text"

    def name(self) -> str:
        """Return backend name."""
        return "simple_ocr"

# Register the backend
register_ocr_backend("simple_ocr", SimpleOCR())
```

### Key Requirements

1. **`process_image()` method**: Takes image bytes and language code, returns `OcrResult`
2. **`name()` method**: Returns unique string identifier
3. **Thread-safe**: Must handle concurrent calls
4. **Registration**: Call `register_ocr_backend()` with name and instance

### Using Custom Backend

```python
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        backend="simple_ocr",  # Use your custom backend
        language="eng",
    )
)

result = extract_file_sync("scanned.pdf", config=config)
print(result.content)
```

## Complete Example: Google Cloud Vision

```python
from kreuzberg import register_ocr_backend
from kreuzberg.types import OcrResult, OcrWord, BoundingBox
from google.cloud import vision
from google.cloud.vision_v1 import types
import base64

class GoogleVisionOCR:
    """Google Cloud Vision API OCR backend."""

    def __init__(self, credentials_path: str = None):
        """Initialize with optional credentials path."""
        if credentials_path:
            import os
            os.environ["GOOGLE_APPLICATION_CREDENTIALS"] = credentials_path

        self.client = vision.ImageAnnotatorClient()

    def process_image(self, image_bytes: bytes, language: str) -> OcrResult:
        """Process image using Google Cloud Vision."""
        # Create image object
        image = types.Image(content=image_bytes)

        # Configure language hints
        image_context = types.ImageContext(
            language_hints=[language] if language else []
        )

        # Perform OCR
        response = self.client.text_detection(
            image=image,
            image_context=image_context
        )

        if response.error.message:
            raise RuntimeError(f"Google Vision API error: {response.error.message}")

        # Extract full text
        if not response.text_annotations:
            return OcrResult(text="", words=[], confidence=0.0)

        full_text = response.text_annotations[0].description

        # Extract word-level details
        words = []
        for annotation in response.text_annotations[1:]:  # Skip first (full text)
            # Get bounding box
            vertices = annotation.bounding_poly.vertices
            bbox = BoundingBox(
                x=vertices[0].x,
                y=vertices[0].y,
                width=vertices[2].x - vertices[0].x,
                height=vertices[2].y - vertices[0].y,
            )

            # Create word object
            words.append(OcrWord(
                text=annotation.description,
                confidence=annotation.confidence if hasattr(annotation, 'confidence') else 0.95,
                bbox=bbox,
            ))

        # Calculate average confidence
        avg_confidence = sum(w.confidence for w in words) / len(words) if words else 0.95

        return OcrResult(
            text=full_text,
            words=words,
            confidence=avg_confidence,
        )

    def name(self) -> str:
        return "google_vision"

# Register
register_ocr_backend("google_vision", GoogleVisionOCR(
    credentials_path="/path/to/credentials.json"
))
```

### Usage

```python
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        backend="google_vision",
        language="en",  # Google uses 2-letter codes
    )
)

result = extract_file_sync("scanned_document.pdf", config=config)
print(result.content)
print(f"Confidence: {result.metadata.get('ocr_confidence', 0):.2%}")
```

## Cloud OCR Services

### AWS Textract

```python
from kreuzberg import register_ocr_backend
from kreuzberg.types import OcrResult, OcrWord, BoundingBox
import boto3

class AWSTextractOCR:
    """AWS Textract OCR backend."""

    def __init__(self, region_name: str = "us-east-1"):
        self.client = boto3.client('textract', region_name=region_name)

    def process_image(self, image_bytes: bytes, language: str) -> OcrResult:
        """Process image using AWS Textract."""
        # Call Textract
        response = self.client.detect_document_text(
            Document={'Bytes': image_bytes}
        )

        # Extract text
        blocks = response['Blocks']
        lines = [block['Text'] for block in blocks if block['BlockType'] == 'LINE']
        full_text = '\n'.join(lines)

        # Extract words with positions
        words = []
        for block in blocks:
            if block['BlockType'] == 'WORD':
                # Get bounding box
                bbox_data = block['Geometry']['BoundingBox']
                bbox = BoundingBox(
                    x=int(bbox_data['Left'] * 1000),  # Convert to pixels (approximate)
                    y=int(bbox_data['Top'] * 1000),
                    width=int(bbox_data['Width'] * 1000),
                    height=int(bbox_data['Height'] * 1000),
                )

                words.append(OcrWord(
                    text=block['Text'],
                    confidence=block['Confidence'] / 100.0,
                    bbox=bbox,
                ))

        avg_confidence = sum(w.confidence for w in words) / len(words) if words else 0.0

        return OcrResult(
            text=full_text,
            words=words,
            confidence=avg_confidence,
        )

    def name(self) -> str:
        return "aws_textract"

register_ocr_backend("aws_textract", AWSTextractOCR(region_name="us-west-2"))
```

### Azure Cognitive Services

```python
from kreuzberg import register_ocr_backend
from kreuzberg.types import OcrResult, OcrWord, BoundingBox
from azure.cognitiveservices.vision.computervision import ComputerVisionClient
from azure.cognitiveservices.vision.computervision.models import OperationStatusCodes
from msrest.authentication import CognitiveServicesCredentials
from io import BytesIO
import time

class AzureOCR:
    """Azure Cognitive Services OCR backend."""

    def __init__(self, api_key: str, endpoint: str):
        self.client = ComputerVisionClient(
            endpoint,
            CognitiveServicesCredentials(api_key)
        )

    def process_image(self, image_bytes: bytes, language: str) -> OcrResult:
        """Process image using Azure OCR."""
        # Start OCR operation
        read_response = self.client.read_in_stream(
            BytesIO(image_bytes),
            language=language if language else None,
            raw=True
        )

        # Get operation location
        operation_location = read_response.headers["Operation-Location"]
        operation_id = operation_location.split("/")[-1]

        # Wait for completion
        while True:
            result = self.client.get_read_result(operation_id)
            if result.status not in [OperationStatusCodes.running, OperationStatusCodes.not_started]:
                break
            time.sleep(0.5)

        if result.status != OperationStatusCodes.succeeded:
            raise RuntimeError(f"Azure OCR failed: {result.status}")

        # Extract text
        lines = []
        words = []

        for page in result.analyze_result.read_results:
            for line in page.lines:
                lines.append(line.text)

                for word in line.words:
                    # Bounding box format: [x1, y1, x2, y2, x3, y3, x4, y4]
                    bbox_points = word.bounding_box
                    bbox = BoundingBox(
                        x=int(bbox_points[0]),
                        y=int(bbox_points[1]),
                        width=int(bbox_points[4] - bbox_points[0]),
                        height=int(bbox_points[5] - bbox_points[1]),
                    )

                    words.append(OcrWord(
                        text=word.text,
                        confidence=word.confidence,
                        bbox=bbox,
                    ))

        full_text = '\n'.join(lines)
        avg_confidence = sum(w.confidence for w in words) / len(words) if words else 0.0

        return OcrResult(
            text=full_text,
            words=words,
            confidence=avg_confidence,
        )

    def name(self) -> str:
        return "azure_ocr"

register_ocr_backend("azure_ocr", AzureOCR(
    api_key="your_api_key",
    endpoint="https://your-resource.cognitiveservices.azure.com/"
))
```

## Custom ML Models

### PyTorch OCR Model

```python
from kreuzberg import register_ocr_backend
from kreuzberg.types import OcrResult
import torch
from PIL import Image
from io import BytesIO

class CustomPyTorchOCR:
    """Custom PyTorch model for OCR."""

    def __init__(self, model_path: str, device: str = "cuda"):
        self.device = torch.device(device if torch.cuda.is_available() else "cpu")

        # Load your model
        self.model = torch.load(model_path, map_location=self.device)
        self.model.eval()

        # Load tokenizer/decoder
        self.decoder = self._load_decoder()

    def process_image(self, image_bytes: bytes, language: str) -> OcrResult:
        """Process image using custom PyTorch model."""
        # Load and preprocess image
        image = Image.open(BytesIO(image_bytes)).convert('RGB')
        tensor = self._preprocess(image)

        # Run inference
        with torch.no_grad():
            output = self.model(tensor.to(self.device))

        # Decode output
        text = self._decode(output)
        confidence = self._calculate_confidence(output)

        return OcrResult(
            text=text,
            words=[],  # Could extract word-level if model supports it
            confidence=confidence,
        )

    def _preprocess(self, image: Image.Image) -> torch.Tensor:
        """Preprocess image for model."""
        # Your preprocessing logic
        # Resize, normalize, etc.
        from torchvision import transforms

        transform = transforms.Compose([
            transforms.Resize((32, 128)),
            transforms.ToTensor(),
            transforms.Normalize(mean=[0.485, 0.456, 0.406],
                               std=[0.229, 0.224, 0.225])
        ])
        return transform(image).unsqueeze(0)

    def _decode(self, output: torch.Tensor) -> str:
        """Decode model output to text."""
        # Your decoding logic (CTC, attention, etc.)
        return self.decoder.decode(output)

    def _calculate_confidence(self, output: torch.Tensor) -> float:
        """Calculate confidence from model output."""
        # Implementation depends on model architecture
        return torch.softmax(output, dim=-1).max().item()

    def _load_decoder(self):
        """Load decoder for model output."""
        # Implementation depends on model
        pass

    def name(self) -> str:
        return "custom_pytorch_ocr"

register_ocr_backend("custom_pytorch_ocr", CustomPyTorchOCR(
    model_path="/path/to/model.pt",
    device="cuda"
))
```

### TensorFlow/Keras OCR Model

```python
from kreuzberg import register_ocr_backend
from kreuzberg.types import OcrResult
import tensorflow as tf
import numpy as np
from PIL import Image
from io import BytesIO

class CustomTensorFlowOCR:
    """Custom TensorFlow model for OCR."""

    def __init__(self, model_path: str):
        self.model = tf.keras.models.load_model(model_path)
        self.char_list = self._load_character_list()

    def process_image(self, image_bytes: bytes, language: str) -> OcrResult:
        """Process image using custom TensorFlow model."""
        # Load and preprocess
        image = Image.open(BytesIO(image_bytes)).convert('RGB')
        processed = self._preprocess(image)

        # Run inference
        predictions = self.model.predict(processed, verbose=0)

        # Decode
        text = self._decode_predictions(predictions)
        confidence = float(np.max(predictions))

        return OcrResult(
            text=text,
            words=[],
            confidence=confidence,
        )

    def _preprocess(self, image: Image.Image) -> np.ndarray:
        """Preprocess image for model."""
        img = image.resize((128, 32))
        img_array = np.array(img) / 255.0
        return np.expand_dims(img_array, axis=0)

    def _decode_predictions(self, predictions: np.ndarray) -> str:
        """Decode model predictions to text."""
        # CTC decoding or your custom logic
        decoded = tf.keras.backend.ctc_decode(
            predictions,
            input_length=np.ones(predictions.shape[0]) * predictions.shape[1],
            greedy=True
        )[0][0]

        # Convert indices to characters
        text = ''.join([self.char_list[int(idx)] for idx in decoded[0] if idx != -1])
        return text

    def _load_character_list(self) -> list:
        """Load character list for decoding."""
        # Your character set
        return list("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 ")

    def name(self) -> str:
        return "custom_tensorflow_ocr"

register_ocr_backend("custom_tensorflow_ocr", CustomTensorFlowOCR(
    model_path="/path/to/model.h5"
))
```

## Image Preprocessing

### Custom Preprocessing Pipeline

```python
from kreuzberg import register_ocr_backend
from kreuzberg.types import OcrResult
from PIL import Image, ImageEnhance, ImageFilter
from io import BytesIO
import pytesseract

class PreprocessedTesseractOCR:
    """Tesseract with advanced preprocessing."""

    def __init__(self, enhance_contrast: bool = True,
                 denoise: bool = True,
                 sharpen: bool = False):
        self.enhance_contrast = enhance_contrast
        self.denoise = denoise
        self.sharpen = sharpen

    def process_image(self, image_bytes: bytes, language: str) -> OcrResult:
        """Process with preprocessing."""
        # Load image
        image = Image.open(BytesIO(image_bytes))

        # Apply preprocessing
        processed = self._preprocess(image)

        # Run Tesseract
        text = pytesseract.image_to_string(
            processed,
            lang=language or 'eng'
        )

        # Get confidence
        data = pytesseract.image_to_data(processed, output_type=pytesseract.Output.DICT)
        confidences = [float(c) for c in data['conf'] if c != '-1']
        avg_confidence = sum(confidences) / len(confidences) / 100.0 if confidences else 0.0

        return OcrResult(
            text=text,
            words=[],
            confidence=avg_confidence,
        )

    def _preprocess(self, image: Image.Image) -> Image.Image:
        """Apply preprocessing steps."""
        # Convert to grayscale
        if image.mode != 'L':
            image = image.convert('L')

        # Enhance contrast
        if self.enhance_contrast:
            enhancer = ImageEnhance.Contrast(image)
            image = enhancer.enhance(2.0)

        # Denoise
        if self.denoise:
            image = image.filter(ImageFilter.MedianFilter(size=3))

        # Sharpen
        if self.sharpen:
            image = image.filter(ImageFilter.SHARPEN)

        # Binarize (Otsu's method approximation)
        threshold = self._calculate_threshold(image)
        image = image.point(lambda p: 255 if p > threshold else 0)

        return image

    def _calculate_threshold(self, image: Image.Image) -> int:
        """Calculate Otsu's threshold."""
        histogram = image.histogram()
        total = sum(histogram)

        sum_total = sum(i * histogram[i] for i in range(256))
        sum_background = 0
        weight_background = 0

        max_variance = 0
        threshold = 0

        for i in range(256):
            weight_background += histogram[i]
            if weight_background == 0:
                continue

            weight_foreground = total - weight_background
            if weight_foreground == 0:
                break

            sum_background += i * histogram[i]

            mean_background = sum_background / weight_background
            mean_foreground = (sum_total - sum_background) / weight_foreground

            variance = weight_background * weight_foreground * \
                      (mean_background - mean_foreground) ** 2

            if variance > max_variance:
                max_variance = variance
                threshold = i

        return threshold

    def name(self) -> str:
        return "preprocessed_tesseract"

register_ocr_backend("preprocessed_tesseract", PreprocessedTesseractOCR(
    enhance_contrast=True,
    denoise=True,
    sharpen=False
))
```

## Best Practices

### Performance

1. **Batch processing**: Process multiple images together when possible
2. **Model caching**: Keep models in memory, don't reload per request
3. **GPU utilization**: Use GPU for ML models when available
4. **Async I/O**: Use async HTTP clients for cloud APIs

### Error Handling

```python
from kreuzberg import register_ocr_backend
from kreuzberg.types import OcrResult
import logging

logger = logging.getLogger(__name__)

class RobustOCR:
    """OCR backend with comprehensive error handling."""

    def process_image(self, image_bytes: bytes, language: str) -> OcrResult:
        """Process with error handling."""
        try:
            # Validate input
            if not image_bytes:
                raise ValueError("Empty image bytes")

            # Process
            text = self._extract_text(image_bytes, language)

            if not text:
                logger.warning("OCR returned empty text")
                return OcrResult(text="", words=[], confidence=0.0)

            return OcrResult(text=text, words=[], confidence=0.95)

        except ConnectionError as e:
            # Network errors
            logger.error(f"OCR API connection error: {e}")
            raise

        except ValueError as e:
            # Validation errors
            logger.error(f"OCR validation error: {e}")
            raise

        except Exception as e:
            # Unexpected errors
            logger.error(f"Unexpected OCR error: {e}", exc_info=True)
            raise RuntimeError(f"OCR processing failed: {e}")

    def _extract_text(self, image_bytes: bytes, language: str) -> str:
        """Extract text (implementation)."""
        pass

    def name(self) -> str:
        return "robust_ocr"
```

### Resource Management

```python
class ManagedOCR:
    """OCR backend with resource management."""

    def __init__(self):
        self._client = None

    def process_image(self, image_bytes: bytes, language: str) -> OcrResult:
        """Process with lazy initialization."""
        # Lazy initialize client
        if self._client is None:
            self._initialize()

        # Use client
        return self._process_with_client(image_bytes, language)

    def _initialize(self):
        """Initialize resources."""
        # Load models, create clients, etc.
        pass

    def _process_with_client(self, image_bytes: bytes, language: str) -> OcrResult:
        """Process using initialized client."""
        pass

    def cleanup(self):
        """Cleanup resources."""
        if self._client:
            self._client.close()
            self._client = None

    def name(self) -> str:
        return "managed_ocr"
```

## Testing OCR Backends

```python
import pytest
from kreuzberg.types import OcrResult
from my_ocr import GoogleVisionOCR

def test_ocr_basic_text():
    """Test OCR on simple text image."""
    ocr = GoogleVisionOCR()

    # Load test image
    with open("tests/test_images/simple_text.png", "rb") as f:
        image_bytes = f.read()

    # Process
    result = ocr.process_image(image_bytes, "eng")

    # Assert
    assert isinstance(result, OcrResult)
    assert result.text
    assert result.confidence > 0.0

def test_ocr_empty_image():
    """Test OCR on empty/blank image."""
    ocr = GoogleVisionOCR()

    with open("tests/test_images/blank.png", "rb") as f:
        image_bytes = f.read()

    result = ocr.process_image(image_bytes, "eng")

    # Should return empty result, not error
    assert result.text == ""

def test_ocr_language_support():
    """Test OCR with different languages."""
    ocr = GoogleVisionOCR()

    with open("tests/test_images/german_text.png", "rb") as f:
        image_bytes = f.read()

    result = ocr.process_image(image_bytes, "deu")

    assert result.text
    assert "ü" in result.text or "ä" in result.text or "ö" in result.text
```

## Next Steps

- [Python PostProcessor Development](python-postprocessor.md) - Transform extraction results
- [Plugin Development Overview](overview.md) - Compare plugin types
- [OCR System Concepts](../concepts/ocr.md) - OCR architecture details
