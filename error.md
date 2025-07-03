1m 42s
Run uv run pre-commit run --show-diff-on-failure --color=always --all-files
python tests naming......................................................Passed
trim trailing whitespace.................................................Failed

- hook id: trailing-whitespace
- exit code: 1
- files were modified by this hook

Fixing tests/test_source_files/german-text.txt
Fixing tests/language_detection_test.py
Fixing tests/test_source_files/french-text.txt
Fixing kreuzberg/\_language_detection.py
Fixing tests/test_source_files/spanish-text.txt
Fixing docs/examples/extraction-examples.md
Fixing kreuzberg/\_extractors/\_pdf.py

fix end of files.........................................................Failed

- hook id: end-of-file-fixer
- exit code: 1
- files were modified by this hook

Fixing tests/test_source_files/german-text.txt
Fixing tests/language_detection_test.py
Fixing tests/test_source_files/french-text.txt
Fixing kreuzberg/\_language_detection.py
Fixing tests/test_source_files/spanish-text.txt

check toml...............................................................Passed
check for case conflicts.................................................Passed
detect private key.......................................................Passed
Validate pyproject.toml..................................................Passed
mdformat.................................................................Failed

- hook id: mdformat
- files were modified by this hook
  markdownlint-fix.........................................................Passed
  blacken-docs.............................................................Failed
- hook id: blacken-docs
- exit code: 1
- files were modified by this hook

docs/examples/extraction-examples.md: Rewriting...

prettier.................................................................Passed
pyproject-fmt............................................................Failed

- hook id: pyproject-fmt
- exit code: 1
- files were modified by this hook

--- pyproject.toml

+++ pyproject.toml

@@ -69,13 +69,13 @@

optional-dependencies.gmft = \[
"gmft>=0.4.2",
\]
+optional-dependencies.language-detection = \[

- "fast-langdetect>=0.2.0",
  +\]
  optional-dependencies.paddleocr = \[
  "paddleocr>=3.1.0",
  "paddlepaddle>=3.1.0",
  "setuptools>=80.9.0",
  -\]
  -optional-dependencies.language-detection = \[

- "fast-langdetect>=0.2.0",
  \]
  urls.homepage = "<https://github.com/Goldziher/kreuzberg>"

@@ -83,6 +83,8 @@

dev = \[
"covdefaults>=2.3.0",
"mypy>=1.16.1",

- "numpy>=1.24.0",

- "pandas>=2.0.0",
  "pre-commit>=4.2.0",
  "pytest>=8.4.1",
  "pytest-cov>=6.2.1",
  @@ -91,8 +93,6 @@

  "ruff>=0.12.1",
  "trio>=0.30.0",
  "uv-bump",

- "pandas>=2.0.0",

- "numpy>=1.24.0",
  \]
  doc = \[
  "mkdocs>=1.6.1",

ruff (legacy alias)......................................................Failed

- hook id: ruff
- exit code: 1
- files were modified by this hook

kreuzberg/\_language_detection.py:28:12: BLE001 Do not catch blind exception: `Exception`
|
26 | results = detect_langs(text)
27 | return \[r.lang for r in results[:top_n]\]
28 | except Exception:
| ^^^^^^^^^ BLE001
29 | return []
|

tests/language_detection_test.py:53:9: SIM117 Use a single `with` statement with multiple contexts instead of nested `with` statements
|
51 | def test_detect_languages_missing_dependency(self) -> None:
52 | """Test that MissingDependencyError is raised when fast-langdetect is not available."""
53 | / with patch("kreuzberg.\_language_detection.detect_langs", None):
54 | | with pytest.raises(MissingDependencyError, match="fast-langdetect is required"):
| |**\*\*\*\***\*\*\*\***\*\*\*\***\*\*\*\***\*\*\*\***\*\*\*\***\*\*\*\***\_\_\_\_**\*\*\*\***\*\*\*\***\*\*\*\***\*\*\*\***\*\*\*\***\*\*\*\***\*\*\*\***^ SIM117
55 | detect_languages("Some text")
|
= help: Combine `with` statements

tests/language_detection_test.py:91:63: ANN001 Missing type annotation for function argument `tmp_path`
|
90 | @pytest.mark.anyio
91 | async def test_extract_file_with_language_detection(self, tmp_path) -> None:
| ^^^^^^^^ ANN001
92 | """Test that language detection works with extract_file."""
93 | from kreuzberg import ExtractionConfig, extract_file
|

tests/language_detection_test.py:108:66: ANN001 Missing type annotation for function argument `tmp_path`
|
107 | @pytest.mark.anyio
108 | async def test_extract_file_without_language_detection(self, tmp_path) -> None:
| ^^^^^^^^ ANN001
109 | """Test that language detection is not performed when disabled."""
110 | from kreuzberg import ExtractionConfig, extract_file
|

tests/language_detection_test.py:123:58: ANN001 Missing type annotation for function argument `tmp_path`
|
122 | @pytest.mark.anyio
123 | async def test_extract_file_missing_dependency(self, tmp_path) -> None:
| ^^^^^^^^ ANN001
124 | """Test that MissingDependencyError is raised when language detection is enabled but library is missing."""
125 | from kreuzberg import ExtractionConfig, extract_file
|

tests/language_detection_test.py:137:62: ANN001 Missing type annotation for function argument `tmp_path`
|
135 | await extract_file(test_file, config=config)
136 |
137 | def test_extract_file_sync_with_language_detection(self, tmp_path) -> None:
| ^^^^^^^^ ANN001
138 | """Test that language detection works with extract_file_sync."""
139 | from kreuzberg import ExtractionConfig, extract_file_sync
|

tests/language_detection_test.py:157:66: ANN001 Missing type annotation for function argument `tmp_path`
|
156 | @pytest.mark.anyio
157 | async def test_image_extractor_with_detected_languages(self, tmp_path) -> None:
| ^^^^^^^^ ANN001
158 | """Test that ImageExtractor uses detected languages for OCR."""
159 | from kreuzberg import ExtractionConfig
|

tests/language_detection_test.py:193:64: ANN001 Missing type annotation for function argument `tmp_path`
|
192 | @pytest.mark.anyio
193 | async def test_pdf_extractor_with_detected_languages(self, tmp_path) -> None:
| ^^^^^^^^ ANN001
194 | """Test that PDFExtractor uses detected languages for OCR."""
195 | from kreuzberg import ExtractionConfig
|

Found 83 errors (75 fixed, 8 remaining).

ruff format..............................................................Failed

- hook id: ruff-format
- files were modified by this hook

3 files reformatted, 54 files left unchanged

codespell................................................................Passed
pydoclint................................................................Failed

- hook id: pydoclint
- exit code: 1

Skipping files that match this pattern: .git|.tox
kreuzberg/**init**.py
kreuzberg/\_chunker.py
kreuzberg/\_constants.py
kreuzberg/\_extractors/**init**.py
kreuzberg/\_extractors/\_base.py
kreuzberg/\_extractors/\_html.py
kreuzberg/\_extractors/\_image.py
kreuzberg/\_extractors/\_pandoc.py
kreuzberg/\_extractors/\_pdf.py
kreuzberg/\_extractors/\_presentation.py
kreuzberg/\_extractors/\_spread_sheet.py
kreuzberg/\_gmft.py
kreuzberg/\_language_detection.py
kreuzberg/\_mime_types.py
kreuzberg/\_ocr/**init**.py
kreuzberg/\_ocr/\_base.py
kreuzberg/\_ocr/\_easyocr.py
kreuzberg/\_ocr/\_paddleocr.py
kreuzberg/\_ocr/\_tesseract.py
kreuzberg/\_playa.py
kreuzberg/\_registry.py
kreuzberg/\_types.py
kreuzberg/\_utils/**init**.py
kreuzberg/\_utils/\_device.py
kreuzberg/\_utils/\_string.py
kreuzberg/\_utils/\_sync.py
kreuzberg/\_utils/\_tmp.py
kreuzberg/exceptions.py
kreuzberg/extraction.py
tests/**init**.py
tests/chunker_test.py
tests/conftest.py
tests/exceptions_test.py
tests/extraction_test.py
tests/extractors/**init**.py
tests/extractors/html_test.py
tests/extractors/image_test.py
tests/extractors/pandoc_metadata_test.py
tests/extractors/pandoc_test.py
tests/extractors/pdf_test.py
tests/extractors/presentation_test.py
tests/extractors/spreed_sheet_test.py
tests/gmft_test.py
tests/hooks_test.py
tests/language_detection_test.py
tests/mime_types_test.py
tests/ocr/**init**.py
tests/ocr/device_integration_test.py
tests/ocr/easyocr_test.py
tests/ocr/paddleocr_test.py
tests/ocr/tesseract_test.py
tests/playa_test.py
tests/registry_test.py
tests/utils/**init**.py
tests/utils/device_test.py
tests/utils/string_test.py
tests/utils/tmp_test.py

kreuzberg/extraction.py
70: DOC501: Function `extract_bytes` has raise statements, but the docstring does not have a "Raises" section
70: DOC503: Function `extract_bytes` exceptions in the "Raises" section in the docstring do not match those in the function body. Raised exceptions in the docstring: []. Raised exceptions in the body: ['MissingDependencyError'].
107: DOC501: Function `extract_file` has raise statements, but the docstring does not have a "Raises" section
107: DOC503: Function `extract_file` exceptions in the "Raises" section in the docstring do not match those in the function body. Raised exceptions in the docstring: []. Raised exceptions in the body: ['MissingDependencyError'].

mypy.....................................................................Failed

- hook id: mypy
- exit code: 1

tests/language_detection_test.py:92: error: Function is missing a type annotation for one or more arguments [no-untyped-def]
tests/language_detection_test.py:109: error: Function is missing a type annotation for one or more arguments [no-untyped-def]
tests/language_detection_test.py:124: error: Function is missing a type annotation for one or more arguments [no-untyped-def]
tests/language_detection_test.py:138: error: Function is missing a type annotation for one or more arguments [no-untyped-def]
tests/language_detection_test.py:158: error: Function is missing a type annotation for one or more arguments [no-untyped-def]
tests/language_detection_test.py:169: error: "ExtractionConfig" has no attribute "detected_languages"; maybe "auto_detect_language"? [attr-defined]
tests/language_detection_test.py:188: error: Function is missing a type annotation for one or more arguments [no-untyped-def]
tests/language_detection_test.py:203: error: "ExtractionConfig" has no attribute "detected_languages"; maybe "auto_detect_language"? [attr-defined]
Found 8 errors in 1 file (checked 57 source files)

pre-commit hook(s) made changes.
If you are seeing this message in CI, reproduce locally with: `pre-commit run --all-files`.
To run `pre-commit` as part of git workflow, use `pre-commit install`.
All changes made by hooks:
diff --git a/docs/examples/extraction-examples.md b/docs/examples/extraction-examples.md
index e434f2c..5c43be5 100644
--- a/docs/examples/extraction-examples.md
+++ b/docs/examples/extraction-examples.md
@@ -63,12 +63,12 @@ from kreuzberg import extract_file, ExtractionConfig
async def extract_with_language_detection(): # Enable automatic language detection
config = ExtractionConfig(auto_detect_language=True)

-

- # Extract from a German document

  ```
  result = await extract_file("german_document.pdf", config=config)
  print(f"Detected languages: {result.detected_languages}")
  print(f"Content: {result.content[:100]}...")
  ```

-

- # Extract from a multilingual document

  ```
   result = await extract_file("multilingual.pdf", config=config)
   print(f"Detected languages: {result.detected_languages}")  # e.g., ['en', 'de']
  ```

  @@ -76,11 +76,8 @@ async def extract_with_language_detection():

  async def extract_with_fallback(): # Combine automatic detection with manual configuration

- config = ExtractionConfig(

- ```
     auto_detect_language=True,
  ```

- ```
     ocr_config=TesseractConfig(language="eng")  # Fallback to English
  ```

- )

-

- config = ExtractionConfig(auto_detect_language=True, ocr_config=TesseractConfig(language="eng")) # Fallback to English

- ```
   result = await extract_file("document.pdf", config=config)
   if result.detected_languages:
       print(f"Using detected languages: {result.detected_languages}")
  ```

  diff --git a/docs/user-guide/ocr-configuration.md b/docs/user-guide/ocr-configuration.md
  index 7649996..c93301d 100644
  --- a/docs/user-guide/ocr-configuration.md
  +++ b/docs/user-guide/ocr-configuration.md
  @@ -79,9 +79,9 @@ print(f"Detected languages: {result.detected_languages}")

#### How It Works

1. **Text Extraction**: Kreuzberg first extracts text from your document
   -2. **Language Detection**: If `auto_detect_language=True`, it analyzes the extracted text to identify the language(s)
   -3. **OCR Configuration**: The detected language(s) are automatically configured for the OCR backend
   -4. **Result Storage**: Detected languages are stored in `result.detected_languages`
   +1. **Language Detection**: If `auto_detect_language=True`, it analyzes the extracted text to identify the language(s)
   +1. **OCR Configuration**: The detected language(s) are automatically configured for the OCR backend
   +1. **Result Storage**: Detected languages are stored in `result.detected_languages`

#### Supported Languages

diff --git a/kreuzberg/\_extractors/\_image.py b/kreuzberg/\_extractors/\_image.py
index d481810..55a1996 100644
--- a/kreuzberg/\_extractors/\_image.py
+++ b/kreuzberg/\_extractors/\_image.py
@@ -57,16 +57,16 @@ class ImageExtractor(Extractor):

```
     # Use detected_languages if available
     ocr_config = self.config.get_config_dict().copy()
```

- ```
     detected_languages = getattr(self.config, 'detected_languages', None)
  ```

- ```
     if not detected_languages and hasattr(self.config, 'auto_detect_language') and self.config.auto_detect_language:
  ```

- ```
     detected_languages = getattr(self.config, "detected_languages", None)
  ```

- ```
     if not detected_languages and hasattr(self.config, "auto_detect_language") and self.config.auto_detect_language:
         # Try to get from ExtractionResult if present (for future extensibility)
         pass  # ExtractionResult not available here, so only config is checked
     if detected_languages:
         # For Tesseract and PaddleOCR, use 'language'; for EasyOCR, use 'language_list'
  ```

- ```
         if 'language' in ocr_config:
  ```

- ```
             ocr_config['language'] = '+'.join(detected_languages)
  ```

- ```
         if 'language_list' in ocr_config:
  ```

- ```
             ocr_config['language_list'] = detected_languages
  ```

- ```
         if "language" in ocr_config:
  ```

- ```
             ocr_config["language"] = "+".join(detected_languages)
  ```

- ```
         if "language_list" in ocr_config:
  ```

- ```
             ocr_config["language_list"] = detected_languages
       return await get_ocr_backend(self.config.ocr_backend).process_file(path, **ocr_config)

   def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
  ```

  diff --git a/kreuzberg/\_extractors/\_pdf.py b/kreuzberg/\_extractors/\_pdf.py
  index 68ad6ce..ad31595 100644
  --- a/kreuzberg/\_extractors/\_pdf.py
  +++ b/kreuzberg/\_extractors/\_pdf.py
  @@ -136,17 +136,17 @@ class PDFExtractor(Extractor):
  """
  images = await self.\_convert_pdf_to_images(input_file)
  backend = get_ocr_backend(ocr_backend)

-

- ```
      # Use detected_languages if available
      ocr_config = self.config.get_config_dict().copy()
  ```

- ```
     detected_languages = getattr(self.config, 'detected_languages', None)
  ```

- ```
     detected_languages = getattr(self.config, "detected_languages", None)
     if detected_languages:
         # For Tesseract and PaddleOCR, use 'language'; for EasyOCR, use 'language_list'
  ```

- ```
         if 'language' in ocr_config:
  ```

- ```
             ocr_config['language'] = '+'.join(detected_languages)
  ```

- ```
         if 'language_list' in ocr_config:
  ```

- ```
             ocr_config['language_list'] = detected_languages
  ```

-

- ```
         if "language" in ocr_config:
  ```

- ```
             ocr_config["language"] = "+".join(detected_languages)
  ```

- ```
         if "language_list" in ocr_config:
  ```

- ```
             ocr_config["language_list"] = detected_languages
  ```

- ```
      ocr_results = await run_taskgroup_batched(
          *[backend.process_image(image, **ocr_config) for image in images],
          batch_size=cpu_count(),
  ```

diff --git a/kreuzberg/\_language_detection.py b/kreuzberg/\_language_detection.py
index 2752383..284a3f1 100644
--- a/kreuzberg/\_language_detection.py
+++ b/kreuzberg/\_language_detection.py
@@ -1,18 +1,16 @@
-"""
-Language detection utilities for Kreuzberg.
-"""
-from typing import Optional, List
+"""Language detection utilities for Kreuzberg."""

- from functools import lru_cache

  from kreuzberg.exceptions import MissingDependencyError

  try:
  from fast_langdetect import detect_langs
  -except ImportError as e:
  +except ImportError:
  detect_langs = None

  -def \_require_fast_langdetect():
  +def_require_fast_langdetect() -> None:
  if detect_langs is None:
  raise MissingDependencyError(
  "fast-langdetect is required for language detection. Install with: pip install 'kreuzberg[language-detection]'"
  @@ -20,9 +18,8 @@ def_require_fast_langdetect():

  @lru_cache(maxsize=128)
  -def detect_languages(text: str, top_n: int = 3) -> List\[str\]:

- """

- Detects the most probable languages in the given text using fast-langdetect.
  +def detect_languages(text: str, top_n: int = 3) -> list\[str\]:

- """Detects the most probable languages in the given text using fast-langdetect.
  Returns a list of language codes (e.g., ['en', 'de']).
  """
  \_require_fast_langdetect()
  @@ -35,4 +32,4 @@ def detect_languages(text: str, top_n: int = 3) -> List\[str\]:

def is_language_detection_available() -> bool:
"""Returns True if fast-langdetect is available, False otherwise."""

- return detect_langs is not None
  \\ No newline at end of file
- return detect_langs is not None
  diff --git a/kreuzberg/extraction.py b/kreuzberg/extraction.py
  index 2f99e36..a8aad9e 100644
  --- a/kreuzberg/extraction.py
  +++ b/kreuzberg/extraction.py
  @@ -7,6 +7,7 @@ import anyio

from kreuzberg import ExtractionResult
from kreuzberg.\_chunker import get_chunker
+from kreuzberg.\_language_detection import detect_languages, is_language_detection_available
from kreuzberg.\_mime_types import (
validate_mime_type,
)
@@ -14,7 +15,6 @@ from kreuzberg.\_registry import ExtractorRegistry
from kreuzberg.\_types import ExtractionConfig
from kreuzberg.\_utils.\_string import safe_decode
from kreuzberg.\_utils.\_sync import run_maybe_async, run_maybe_sync
-from kreuzberg.\_language_detection import detect_languages, is_language_detection_available

if TYPE_CHECKING:
from collections.abc import Sequence
@@ -96,6 +96,7 @@ async def extract_bytes(content: bytes, mime_type: str, config: ExtractionConfig
result.detected_languages = detect_languages(result.content)
else:
from kreuzberg.exceptions import MissingDependencyError

- ```
           raise MissingDependencyError(
               "Language detection requested but fast-langdetect is not installed. Install with: pip install 'kreuzberg[language-detection]'"
           )
  ```

  @@ -130,6 +131,7 @@ async def extract_file(
  result.detected_languages = detect_languages(result.content)
  else:
  from kreuzberg.exceptions import MissingDependencyError

- ```
           raise MissingDependencyError(
               "Language detection requested but fast-langdetect is not installed. Install with: pip install 'kreuzberg[language-detection]'"
           )
  ```

  diff --git a/pyproject.toml b/pyproject.toml
  index c806675..1505f77 100644
  --- a/pyproject.toml
  +++ b/pyproject.toml
  @@ -69,20 +69,22 @@ optional-dependencies.easyocr = \[
  optional-dependencies.gmft = \[
  "gmft>=0.4.2",
  \]
  +optional-dependencies.language-detection = \[

- "fast-langdetect>=0.2.0",
  +\]
  optional-dependencies.paddleocr = \[
  "paddleocr>=3.1.0",
  "paddlepaddle>=3.1.0",
  "setuptools>=80.9.0",
  \]
  -optional-dependencies.language-detection = \[

- "fast-langdetect>=0.2.0",
  -\]
  urls.homepage = "<https://github.com/Goldziher/kreuzberg>"

[dependency-groups]
dev = \[
"covdefaults>=2.3.0",
"mypy>=1.16.1",

- "numpy>=1.24.0",

- "pandas>=2.0.0",
  "pre-commit>=4.2.0",
  "pytest>=8.4.1",
  "pytest-cov>=6.2.1",
  @@ -91,8 +93,6 @@ dev = \[
  "ruff>=0.12.1",
  "trio>=0.30.0",
  "uv-bump",

- "pandas>=2.0.0",

- "numpy>=1.24.0",
  \]
  doc = \[
  "mkdocs>=1.6.1",
  diff --git a/tests/language_detection_test.py b/tests/language_detection_test.py
  index 3a24263..6b77bb4 100644
  --- a/tests/language_detection_test.py
  +++ b/tests/language_detection_test.py
  @@ -1,90 +1,87 @@
  """
  Tests for language detection functionality.
  """

- from **future** import annotations

  from unittest.mock import Mock, patch
  -from typing import TYPE_CHECKING

  import pytest

  from kreuzberg.\_language_detection import detect_languages, is_language_detection_available
  from kreuzberg.exceptions import MissingDependencyError

  -if TYPE_CHECKING:

- from pytest_mock import MockerFixture

- class TestLanguageDetection:
  """Test language detection functionality."""

  ```
  def test_is_language_detection_available_with_fast_langdetect(self) -> None:
      """Test that language detection availability is correctly detected when fast-langdetect is available."""
  ```

- ```
     with patch('kreuzberg._language_detection.detect_langs') as mock_detect_langs:
  ```

- ```
         mock_detect_langs.return_value = [Mock(lang='en')]
  ```

- ```
     with patch("kreuzberg._language_detection.detect_langs") as mock_detect_langs:
  ```

- ```
         mock_detect_langs.return_value = [Mock(lang="en")]
         assert is_language_detection_available() is True
  ```

  def test_is_language_detection_available_without_fast_langdetect(self) -> None:
  """Test that language detection availability is correctly detected when fast-langdetect is not available."""

- ```
     with patch('kreuzberg._language_detection.detect_langs', None):
  ```

- ```
     with patch("kreuzberg._language_detection.detect_langs", None):
         assert is_language_detection_available() is False
  ```

  def test_detect_languages_success(self) -> None:
  """Test successful language detection."""

- ```
     mock_result = [Mock(lang='en'), Mock(lang='de'), Mock(lang='fr')]
  ```

- ```
     with patch('kreuzberg._language_detection.detect_langs') as mock_detect_langs:
  ```

- ```
     mock_result = [Mock(lang="en"), Mock(lang="de"), Mock(lang="fr")]
  ```

- ```
     with patch("kreuzberg._language_detection.detect_langs") as mock_detect_langs:
         mock_detect_langs.return_value = mock_result
         result = detect_languages("This is English text with some German words.")
  ```

- ```
         assert result == ['en', 'de', 'fr']
  ```

- ```
         assert result == ["en", "de", "fr"]
  ```

  def test_detect_languages_with_top_n_limit(self) -> None:
  """Test language detection with top_n parameter."""

- ```
     mock_result = [Mock(lang='en'), Mock(lang='de'), Mock(lang='fr'), Mock(lang='es')]
  ```

- ```
     with patch('kreuzberg._language_detection.detect_langs') as mock_detect_langs:
  ```

- ```
     mock_result = [Mock(lang="en"), Mock(lang="de"), Mock(lang="fr"), Mock(lang="es")]
  ```

- ```
     with patch("kreuzberg._language_detection.detect_langs") as mock_detect_langs:
         mock_detect_langs.return_value = mock_result
         result = detect_languages("This is English text with some German words.", top_n=2)
  ```

- ```
         assert result == ['en', 'de']
  ```

- ```
         assert result == ["en", "de"]
  ```

  def test_detect_languages_exception_handling(self) -> None:
  """Test that exceptions in language detection are handled gracefully."""

- ```
     with patch('kreuzberg._language_detection.detect_langs') as mock_detect_langs:
  ```

- ```
     with patch("kreuzberg._language_detection.detect_langs") as mock_detect_langs:
         mock_detect_langs.side_effect = Exception("Detection failed")
         result = detect_languages("Some text")
         assert result == []
  ```

  def test_detect_languages_missing_dependency(self) -> None:
  """Test that MissingDependencyError is raised when fast-langdetect is not available."""

- ```
     with patch('kreuzberg._language_detection.detect_langs', None):
  ```

- ```
     with patch("kreuzberg._language_detection.detect_langs", None):
         with pytest.raises(MissingDependencyError, match="fast-langdetect is required"):
             detect_languages("Some text")
  ```

  def test_detect_languages_caching(self) -> None:
  """Test that language detection results are cached."""

- ```
     mock_result = [Mock(lang='en')]
  ```

- ```
     with patch('kreuzberg._language_detection.detect_langs') as mock_detect_langs:
  ```

- ```
     mock_result = [Mock(lang="en")]
  ```

- ```
     with patch("kreuzberg._language_detection.detect_langs") as mock_detect_langs:
         mock_detect_langs.return_value = mock_result
  ```

-

- ```
          # First call
          result1 = detect_languages("This is English text.")
          # Second call with same text
          result2 = detect_languages("This is English text.")
  ```

-

- ```
          assert result1 == result2
          # Should only be called once due to caching
          assert mock_detect_langs.call_count == 1

  def test_detect_languages_different_texts_not_cached(self) -> None:
      """Test that different texts are not cached together."""
  ```

- ```
     mock_result1 = [Mock(lang='en')]
  ```

- ```
     mock_result2 = [Mock(lang='de')]
  ```

- ```
     with patch('kreuzberg._language_detection.detect_langs') as mock_detect_langs:
  ```

- ```
     mock_result1 = [Mock(lang="en")]
  ```

- ```
     mock_result2 = [Mock(lang="de")]
  ```

- ```
     with patch("kreuzberg._language_detection.detect_langs") as mock_detect_langs:
         mock_detect_langs.side_effect = [mock_result1, mock_result2]
  ```

-

- ```
          result1 = detect_languages("This is English text.")
          result2 = detect_languages("Das ist deutscher Text.")
  ```

-

- ```
         assert result1 == ['en']
  ```

- ```
         assert result2 == ['de']
  ```

-

- ```
         assert result1 == ["en"]
  ```

- ```
         assert result2 == ["de"]
         assert mock_detect_langs.call_count == 2
  ```

@@ -94,64 +91,64 @@ class TestLanguageDetectionIntegration:
@pytest.mark.anyio
async def test_extract_file_with_language_detection(self, tmp_path) -> None:
"""Test that language detection works with extract_file."""

- ```
     from kreuzberg import extract_file, ExtractionConfig
  ```

-

- ```
     from kreuzberg import ExtractionConfig, extract_file
  ```

- ```
      # Create a test file with English text
      test_file = tmp_path / "test.txt"
      test_file.write_text("This is English text for testing language detection.")
  ```

-

- ```
     with patch('kreuzberg._language_detection.detect_langs') as mock_detect_langs:
  ```

- ```
         mock_detect_langs.return_value = [Mock(lang='en')]
  ```

-

-

- ```
     with patch("kreuzberg._language_detection.detect_langs") as mock_detect_langs:
  ```

- ```
         mock_detect_langs.return_value = [Mock(lang="en")]
  ```

- ```
          config = ExtractionConfig(auto_detect_language=True)
          result = await extract_file(test_file, config=config)
  ```

-

- ```
         assert result.detected_languages == ['en']
  ```

-

- ```
         assert result.detected_languages == ["en"]
  ```

  @pytest.mark.anyio
  async def test_extract_file_without_language_detection(self, tmp_path) -> None:
  """Test that language detection is not performed when disabled."""

- ```
     from kreuzberg import extract_file, ExtractionConfig
  ```

-

- ```
     from kreuzberg import ExtractionConfig, extract_file
  ```

- ```
      test_file = tmp_path / "test.txt"
      test_file.write_text("This is English text.")
  ```

-

- ```
     with patch('kreuzberg._language_detection.detect_langs') as mock_detect_langs:
  ```

-

- ```
     with patch("kreuzberg._language_detection.detect_langs") as mock_detect_langs:
         config = ExtractionConfig(auto_detect_language=False)
         result = await extract_file(test_file, config=config)
  ```

-

- ```
          assert result.detected_languages is None
          mock_detect_langs.assert_not_called()

  @pytest.mark.anyio
  async def test_extract_file_missing_dependency(self, tmp_path) -> None:
      """Test that MissingDependencyError is raised when language detection is enabled but library is missing."""
  ```

- ```
     from kreuzberg import extract_file, ExtractionConfig
  ```

- ```
     from kreuzberg import ExtractionConfig, extract_file
     from kreuzberg.exceptions import MissingDependencyError
  ```

-

- ```
      test_file = tmp_path / "test.txt"
      test_file.write_text("This is English text.")
  ```

-

- ```
     with patch('kreuzberg._language_detection.is_language_detection_available', return_value=False):
  ```

-

- ```
     with patch("kreuzberg._language_detection.is_language_detection_available", return_value=False):
         config = ExtractionConfig(auto_detect_language=True)
  ```

-

- ```
          with pytest.raises(MissingDependencyError, match="fast-langdetect is not installed"):
              await extract_file(test_file, config=config)

  def test_extract_file_sync_with_language_detection(self, tmp_path) -> None:
      """Test that language detection works with extract_file_sync."""
  ```

- ```
     from kreuzberg import extract_file_sync, ExtractionConfig
  ```

-

- ```
     from kreuzberg import ExtractionConfig, extract_file_sync
  ```

- ```
      test_file = tmp_path / "test.txt"
      test_file.write_text("This is English text for testing language detection.")
  ```

-

- ```
     with patch('kreuzberg._language_detection.detect_langs') as mock_detect_langs:
  ```

- ```
         mock_detect_langs.return_value = [Mock(lang='en')]
  ```

-

-

- ```
     with patch("kreuzberg._language_detection.detect_langs") as mock_detect_langs:
  ```

- ```
         mock_detect_langs.return_value = [Mock(lang="en")]
  ```

- ```
          config = ExtractionConfig(auto_detect_language=True)
          result = extract_file_sync(test_file, config=config)
  ```

-

- ```
         assert result.detected_languages == ['en']
  ```

-

- ```
         assert result.detected_languages == ["en"]
  ```

class TestOCRBackendIntegration:
@@ -162,73 +159,64 @@ class TestOCRBackendIntegration:
"""Test that ImageExtractor uses detected languages for OCR."""
from kreuzberg import ExtractionConfig
from kreuzberg.\_extractors.\_image import ImageExtractor

-

- ```
      # Create a mock image file
      image_file = tmp_path / "test.png"
      image_file.write_bytes(b"fake image data")
  ```

-

- ```
     config = ExtractionConfig(
  ```

- ```
         ocr_backend="tesseract",
  ```

- ```
         auto_detect_language=True
  ```

- ```
     )
  ```

-

- ```
     config = ExtractionConfig(ocr_backend="tesseract", auto_detect_language=True)
     # Simulate detected languages
  ```

- ```
     config.detected_languages = ['en', 'de']
  ```

-

- ```
     config.detected_languages = ["en", "de"]
  ```

- ```
      extractor = ImageExtractor(mime_type="image/png", config=config)
  ```

-

- ```
     with patch('kreuzberg._ocr.get_ocr_backend') as mock_get_backend:
  ```

-

- ```
     with patch("kreuzberg._ocr.get_ocr_backend") as mock_get_backend:
         mock_backend = Mock()
         mock_backend.process_file.return_value = Mock(
  ```

- ```
             content="Extracted text",
  ```

- ```
             mime_type="text/plain",
  ```

- ```
             metadata={},
  ```

- ```
             chunks=[]
  ```

- ```
             content="Extracted text", mime_type="text/plain", metadata={}, chunks=[]
         )
         mock_get_backend.return_value = mock_backend
  ```

-

- ```
          await extractor.extract_path_async(image_file)
  ```

-

- ```
          # Verify that the backend was called with the correct language configuration
          mock_backend.process_file.assert_called_once()
          call_args = mock_backend.process_file.call_args[1]
  ```

- ```
         assert call_args['language'] == 'en+de'
  ```

- ```
         assert call_args["language"] == "en+de"
  ```

  @pytest.mark.anyio
  async def test_pdf_extractor_with_detected_languages(self, tmp_path) -> None:
  """Test that PDFExtractor uses detected languages for OCR."""
  from kreuzberg import ExtractionConfig
  from kreuzberg.\_extractors.\_pdf import PDFExtractor

-

- ```
      # Create a mock PDF file
      pdf_file = tmp_path / "test.pdf"
      pdf_file.write_bytes(b"fake pdf data")
  ```

-

- ```
      config = ExtractionConfig(
          ocr_backend="tesseract",
          auto_detect_language=True,
  ```

- ```
         force_ocr=True  # Force OCR to test the OCR path
  ```

- ```
         force_ocr=True,  # Force OCR to test the OCR path
     )
     # Simulate detected languages
  ```

- ```
     config.detected_languages = ['en', 'fr']
  ```

-

- ```
     config.detected_languages = ["en", "fr"]
  ```

- ```
      extractor = PDFExtractor(mime_type="application/pdf", config=config)
  ```

-

- ```
     with patch('kreuzberg._ocr.get_ocr_backend') as mock_get_backend:
  ```

-

- ```
     with patch("kreuzberg._ocr.get_ocr_backend") as mock_get_backend:
         mock_backend = Mock()
         mock_backend.process_image.return_value = Mock(
  ```

- ```
             content="Extracted text",
  ```

- ```
             mime_type="text/plain",
  ```

- ```
             metadata={},
  ```

- ```
             chunks=[]
  ```

- ```
             content="Extracted text", mime_type="text/plain", metadata={}, chunks=[]
         )
         mock_get_backend.return_value = mock_backend
  ```

-

- ```
         with patch('kreuzberg._extractors._pdf.PDFExtractor._convert_pdf_to_images') as mock_convert:
  ```

-

- ```
         with patch("kreuzberg._extractors._pdf.PDFExtractor._convert_pdf_to_images") as mock_convert:
             mock_convert.return_value = [Mock()]  # Mock image
  ```

-

- ```
              await extractor.extract_path_async(pdf_file)
  ```

-

- ```
              # Verify that the backend was called with the correct language configuration
              mock_backend.process_image.assert_called_once()
              call_args = mock_backend.process_image.call_args[1]
  ```

- ```
             assert call_args['language'] == 'en+fr'
  ```

\\ No newline at end of file

- ```
             assert call_args["language"] == "en+fr"
  ```

diff --git a/tests/test_source_files/french-text.txt b/tests/test_source_files/french-text.txt
index 66866f9..06ab324 100644
--- a/tests/test_source_files/french-text.txt
+++ b/tests/test_source_files/french-text.txt
@@ -1,2 +1,2 @@
Ceci est un texte français pour tester la détection de langue.
-Il contient des mots et des phrases françaises pour vérifier la fonctionnalité.
\\ No newline at end of file
+Il contient des mots et des phrases françaises pour vérifier la fonctionnalité.
diff --git a/tests/test_source_files/german-text.txt b/tests/test_source_files/german-text.txt
index 70a31bc..0314c86 100644
--- a/tests/test_source_files/german-text.txt
+++ b/tests/test_source_files/german-text.txt
@@ -1,2 +1,2 @@
-Dies ist ein deutscher Text für Tests der Spracherkennung.
-Er enthält deutsche Wörter und Sätze, um die Funktionalität zu überprüfen.
\\ No newline at end of file
+Dies ist ein deutscher Text für Tests der Spracherkennung.
+Er enthält deutsche Wörter und Sätze, um die Funktionalität zu überprüfen.
diff --git a/tests/test_source_files/spanish-text.txt b/tests/test_source_files/spanish-text.txt
index f23e39d..0b08e75 100644
--- a/tests/test_source_files/spanish-text.txt
+++ b/tests/test_source_files/spanish-text.txt
@@ -1,2 +1,2 @@
Este es un texto en español para probar la detección de idioma.
-Contiene palabras y frases en español para verificar la funcionalidad.
\\ No newline at end of file
+Contiene palabras y frases en español para verificar la funcionalidad.
diff --git a/uv.lock b/uv.lock
index b49ce42..0b475f4 100644
--- a/uv.lock
+++ b/uv.lock
@@ -901,6 +901,104 @@ wheels = \[
{ url = "<https://files.pythonhosted.org/packages/36/f4/c6e662dade71f56cd2f3735141b265c3c79293c109549c1e6933b0651ffc/exceptiongroup-1.3.0-py3-none-any.whl>", hash = "sha256:4d111e6e0c13d0644cad6ddaa7ed0261a0b36971f6d23e7ec9b4b9097da78a10", size = 16674, upload-time = "2025-05-10T17:42:49.33Z" },
\]

+\[[package]\]
+name = "fast-langdetect"
+version = "0.3.2"
+source = { registry = "<https://pypi.org/simple>" }
+dependencies = \[

- { name = "fasttext-predict" },

- { name = "requests" },

- { name = "robust-downloader" },
  +\]
  +sdist = { url = "<https://files.pythonhosted.org/packages/73/e4/e69fc0a833fb91f6392f46c688f89e484855a8905e9999f3e0d6e8e75759/fast_langdetect-0.3.2.tar.gz>", hash = "sha256:21b1f98f738545e9d8f39a080e5276861751af178f6548c896419d1ae20a89ac", size = 792453, upload-time = "2025-03-29T06:51:25.599Z" }
  +wheels = \[

- { url = "<https://files.pythonhosted.org/packages/30/71/0e6da751ef4a9afae2903f76fb8e43c612e3d288a7b01a74ccc20e81b68f/fast_langdetect-0.3.2-py3-none-any.whl>", hash = "sha256:40843fcaf0406e9d3892d9f2e0d05dce8b72794b4c14613804bc1d9707951ba0", size = 788098, upload-time = "2025-03-29T06:51:23.773Z" },
  +\]

- +\[[package]\]
  +name = "fasttext-predict"
  +version = "0.9.2.4"
  +source = { registry = "<https://pypi.org/simple>" }
  +sdist = { url = "<https://files.pythonhosted.org/packages/fc/0e/9defbb9385bcb1104cc1d686a14f7d9fafe5fe43f220cccb00f33d91bb47/fasttext_predict-0.9.2.4.tar.gz>", hash = "sha256:18a6fb0d74c7df9280db1f96cb75d990bfd004fa9d669493ea3dd3d54f84dbc7", size = 16332, upload-time = "2024-11-23T17:24:44.801Z" }
  +wheels = \[

- { url = "<https://files.pythonhosted.org/packages/fc/ee/2350a58c071f873a454aae6bf60900fc3ddb024da3478407ac2057cbc757/fasttext_predict-0.9.2.4-cp310-cp310-macosx_10_9_x86_64.whl>", hash = "sha256:ba432f33228928df5f2af6dfa50560cd77f9859914cffd652303fb02ba100456", size = 103885, upload-time = "2024-11-23T17:22:42.533Z" },

- { url = "<https://files.pythonhosted.org/packages/fd/68/e2f8a82c02b6c4333d454a1b0464942d3dae92e4657c08411035c99fe074/fasttext_predict-0.9.2.4-cp310-cp310-macosx_11_0_arm64.whl>", hash = "sha256:6a8e8f17eb894d450168d2590e23d809e845bd4fad5e39b5708dacb2fdb9b2c7", size = 96415, upload-time = "2024-11-23T17:22:44.452Z" },

- { url = "<https://files.pythonhosted.org/packages/a0/77/0c045793c56b9d143c44fab50f05506c47585532cc5a8f1668bf7b899ddf/fasttext_predict-0.9.2.4-cp310-cp310-manylinux_2_17_aarch64.manylinux2014_aarch64.whl>", hash = "sha256:19565fdf0bb9427831cfc75fca736ab9d71ba7ce02e3ea951e5839beb66560b6", size = 281643, upload-time = "2024-11-23T17:22:46.342Z" },

- { url = "<https://files.pythonhosted.org/packages/5b/ce/c735a67b858bbdb915f3de6d12bc0ad47f0bf0dfce8fc4d42b2ce65e1226/fasttext_predict-0.9.2.4-cp310-cp310-manylinux_2_17_i686.manylinux2014_i686.whl>", hash = "sha256:cb6986815506e3261c0b3f6227dce49eeb4fd3422dab9cd37e2db2fb3691c68b", size = 306088, upload-time = "2024-11-23T17:22:47.571Z" },

- { url = "<https://files.pythonhosted.org/packages/52/a1/b5838f96b6b10f9d4166fd5a5bdc2c32fc42500c236c6318512c5ede99a9/fasttext_predict-0.9.2.4-cp310-cp310-manylinux_2_17_x86_64.manylinux2014_x86_64.whl>", hash = "sha256:229dfdf8943dd76231206c7c9179e3f99d45879e5b654626ee7b73b7fa495d53", size = 294031, upload-time = "2024-11-23T17:22:49.617Z" },

- { url = "<https://files.pythonhosted.org/packages/76/0c/c655919969568ffc7667185595ae56c9cb35a3c4ec3c351654eebea75de5/fasttext_predict-0.9.2.4-cp310-cp310-manylinux_2_31_armv7l.whl>", hash = "sha256:397016ebfa9ec06d6dba09c29e295eea583ea3f45fa4592cc832b257dc84522e", size = 234108, upload-time = "2024-11-23T17:22:51.525Z" },

- { url = "<https://files.pythonhosted.org/packages/f1/f0/b88ea4e4549d49b3202e0b4312f9bc1a42742618aaf2696d63508f861282/fasttext_predict-0.9.2.4-cp310-cp310-musllinux_1_2_aarch64.whl>", hash = "sha256:fc93f9f8f7e982eb635bc860688be04f355fab3d76a243037e26862646f50430", size = 1209246, upload-time = "2024-11-23T17:22:53.656Z" },

- { url = "<https://files.pythonhosted.org/packages/59/dc/1a916fe673f67066f6bb25b5372c282db8924a231662e250646e3ce90f93/fasttext_predict-0.9.2.4-cp310-cp310-musllinux_1_2_armv7l.whl>", hash = "sha256:f4be96ac0b01a3cda82be90e7f6afdafab98919995825c27babd2749a8319be9", size = 1098972, upload-time = "2024-11-23T17:22:55.792Z" },

- { url = "<https://files.pythonhosted.org/packages/b5/9e/ef3aba387a9efdeefce2d9537ee9404b4b8bafe3f1209c8efa6a6cb8022d/fasttext_predict-0.9.2.4-cp310-cp310-musllinux_1_2_i686.whl>", hash = "sha256:f505f737f9493d22ee0c54af7c7eb7828624d5089a1e85072bdb1bd7d3f8f82e", size = 1385514, upload-time = "2024-11-23T17:22:58.022Z" },

- { url = "<https://files.pythonhosted.org/packages/29/d8/b930d3eda35da0ad66335bfb154cb063cfc071dc9b7affe64ae0d90ac04c/fasttext_predict-0.9.2.4-cp310-cp310-musllinux_1_2_x86_64.whl>", hash = "sha256:9ce69f28862dd551d43e27aa0a8de924b6b34412bff998c23c3d4abd70813183", size = 1275740, upload-time = "2024-11-23T17:23:00.376Z" },

- { url = "<https://files.pythonhosted.org/packages/da/13/2611784710956acc1195bcc1ad476fb4d115a30a64175e8064bb83bc30ec/fasttext_predict-0.9.2.4-cp310-cp310-win32.whl>", hash = "sha256:864b6bb543275aee74360eee1d2cc23a440f09991e97efcdcf0b9a5af00f9aa9", size = 90247, upload-time = "2024-11-23T17:23:01.534Z" },

- { url = "<https://files.pythonhosted.org/packages/6d/33/df75b2a1e207eda91efe35766e09dba41ef735e390b156c9c3adc0014e68/fasttext_predict-0.9.2.4-cp310-cp310-win_amd64.whl>", hash = "sha256:7e72abe12c13fd12f8bb137b1f7561096fbd3bb24905a27d9e93a4921ee68dc6", size = 103099, upload-time = "2024-11-23T17:23:02.526Z" },

- { url = "<https://files.pythonhosted.org/packages/c7/12/5c1ddcc721c569132f6340498527b421dcb523470a0aee1b39fcb76c9fe3/fasttext_predict-0.9.2.4-cp311-cp311-macosx_10_9_x86_64.whl>", hash = "sha256:147996c86aa0928c7118f85d18b6a77c458db9ca236db26d44ee5ceaab0c0b6b", size = 105258, upload-time = "2024-11-23T17:23:04.364Z" },

- { url = "<https://files.pythonhosted.org/packages/9f/69/6efd7db47f95a5e2e6e71f69ab3271f5002e99bb88c8f1639c109609cf12/fasttext_predict-0.9.2.4-cp311-cp311-macosx_11_0_arm64.whl>", hash = "sha256:5342f7363709e22524a31750c21e4b735b6666749a167fc03cc3bbf18ea8eccd", size = 97636, upload-time = "2024-11-23T17:23:06.166Z" },

- { url = "<https://files.pythonhosted.org/packages/a7/67/953ca1707fdb2c4bfc5b495b78f98116b45e7b5c39a76875c8b6dcf81ce4/fasttext_predict-0.9.2.4-cp311-cp311-manylinux_2_17_aarch64.manylinux2014_aarch64.whl>", hash = "sha256:6cbecd3908909339316f61db38030ce43890c25bddb06c955191458af13ccfc5", size = 284910, upload-time = "2024-11-23T17:23:07.296Z" },

- { url = "<https://files.pythonhosted.org/packages/ef/c7/b60cf7e58baab5c798f616b2fa0692b8f78d6fc6279574fcfbd7c5235edb/fasttext_predict-0.9.2.4-cp311-cp311-manylinux_2_17_i686.manylinux2014_i686.whl>", hash = "sha256:9de4fcfb54bec35be6b0dffcdc5ace1a3a07f79ee3e8d33d13b82cc4116c5f2f", size = 308768, upload-time = "2024-11-23T17:23:08.5Z" },

- { url = "<https://files.pythonhosted.org/packages/ee/4d/fe2d0619494700f2a85db4cf8050977e1215f484ac8596187301655dc516/fasttext_predict-0.9.2.4-cp311-cp311-manylinux_2_17_x86_64.manylinux2014_x86_64.whl>", hash = "sha256:5af82e09227d993befc00271407b9d3c8aae81d34b35f96208223faf609f4b0c", size = 298342, upload-time = "2024-11-23T17:23:10.408Z" },

- { url = "<https://files.pythonhosted.org/packages/ea/06/f31dc802a2c9acab454eae47902674d92e381a97a363034d359961a955ce/fasttext_predict-0.9.2.4-cp311-cp311-manylinux_2_31_armv7l.whl>", hash = "sha256:337ee60179f32e8b0efa822e59316de15709c7684e7854021b4f6af82b7767ac", size = 236374, upload-time = "2024-11-23T17:23:12.435Z" },

- { url = "<https://files.pythonhosted.org/packages/33/91/a4252c22f2fda855298ef683981d9986f89f9d36fd40ef4c2868cd84e4fd/fasttext_predict-0.9.2.4-cp311-cp311-musllinux_1_2_aarch64.whl>", hash = "sha256:aa9da0c52e65a45dbc87df67015ec1d2712f04de47733e197176550521feea87", size = 1212640, upload-time = "2024-11-23T17:23:13.804Z" },

- { url = "<https://files.pythonhosted.org/packages/ca/3e/47e9e844d15e2b7f64e1a2f1e5897b5c1d59b23ebff64494000259610d2c/fasttext_predict-0.9.2.4-cp311-cp311-musllinux_1_2_armv7l.whl>", hash = "sha256:495efde8afb622266c0e4de41978a6db731a0a685e1db032e7d22937850c9b44", size = 1100895, upload-time = "2024-11-23T17:23:16.109Z" },

- { url = "<https://files.pythonhosted.org/packages/8a/f1/d49bfb81cd3b544021698f6feab1dca2fcca432097978828ad8322eab50b/fasttext_predict-0.9.2.4-cp311-cp311-musllinux_1_2_i686.whl>", hash = "sha256:e5726ba34d79a143b69426e29905eb4d3f4ee8aee94927b3bea3dd566712986b", size = 1385913, upload-time = "2024-11-23T17:23:17.806Z" },

- { url = "<https://files.pythonhosted.org/packages/ed/4a/37bd8d31e46116fcb203b4899aeec32a3adbacf8b09ba3f5d9e3f864b7e4/fasttext_predict-0.9.2.4-cp311-cp311-musllinux_1_2_x86_64.whl>", hash = "sha256:5ac2f35830705c61dd848314c4c077a393608c181725dc353a69361821aa69a8", size = 1279522, upload-time = "2024-11-23T17:23:20.205Z" },

- { url = "<https://files.pythonhosted.org/packages/64/4c/3ec10626e0b25527d10ed258ac1d7c56e5f88100d5c7abba093f918deff6/fasttext_predict-0.9.2.4-cp311-cp311-win32.whl>", hash = "sha256:7b2f8a5cf5f2c451777dbb7ea4957c7919a57ce29a4157a0a381933c9ea6fa70", size = 91396, upload-time = "2024-11-23T17:23:22.194Z" },

- { url = "<https://files.pythonhosted.org/packages/34/b0/456578e7269dace3d7a80a34b30c7757aea6aa34601853c58e5ad186d3d6/fasttext_predict-0.9.2.4-cp311-cp311-win_amd64.whl>", hash = "sha256:83a3c00fdb73a304bc529bc0ae0e225bc2cb956fcfb8e1c7a882b2a1aaa97e19", size = 104390, upload-time = "2024-11-23T17:23:23.332Z" },

- { url = "<https://files.pythonhosted.org/packages/fb/fa/612bf85ce8928120843279ae256f4fffbb9758af81536ddf25f9136b1759/fasttext_predict-0.9.2.4-cp312-cp312-macosx_10_13_x86_64.whl>", hash = "sha256:dcf8661da4f515551523470a745df246121f7e19736fcf3f48f04287963e6279", size = 104836, upload-time = "2024-11-23T17:23:25.219Z" },

- { url = "<https://files.pythonhosted.org/packages/7a/04/106b6fe3f980d6a4f41bfb3106be22d42f87b1e8beb2959361ee4ee08960/fasttext_predict-0.9.2.4-cp312-cp312-macosx_11_0_arm64.whl>", hash = "sha256:99dbfcc3f353da2639fd04fc574a65ff4195b018311f790583147cdc6eb122f4", size = 97377, upload-time = "2024-11-23T17:23:26.319Z" },

- { url = "<https://files.pythonhosted.org/packages/57/b9/b4962c92bd93dd234ea1d1cab643a86d948dab3f269e34a554a004ed6524/fasttext_predict-0.9.2.4-cp312-cp312-manylinux_2_17_aarch64.manylinux2014_aarch64.whl>", hash = "sha256:427e99ba963b2c744ed7233304037a83b7adece97de6f361cfd356aa43cb87f3", size = 283102, upload-time = "2024-11-23T17:23:27.497Z" },

- { url = "<https://files.pythonhosted.org/packages/1d/18/92203820cf00b9a34f40f10456e4ed3019010a9b13a87e11d8b98cd98933/fasttext_predict-0.9.2.4-cp312-cp312-manylinux_2_17_i686.manylinux2014_i686.whl>", hash = "sha256:8b9480cc75a906571a8e5fc717b91b4783f1820aaa5ed36a304d689280de8602", size = 307416, upload-time = "2024-11-23T17:23:28.68Z" },

- { url = "<https://files.pythonhosted.org/packages/06/8d/334cd9acb84e569d37617444661ca7b59d1bc1a83abe42aa845d23fb1273/fasttext_predict-0.9.2.4-cp312-cp312-manylinux_2_17_x86_64.manylinux2014_x86_64.whl>", hash = "sha256:11ef7af2a4431c76d2226e47334e86b9c4a78a98f6cb68b1ce9a1fc20e04c904", size = 296055, upload-time = "2024-11-23T17:23:29.934Z" },

- { url = "<https://files.pythonhosted.org/packages/08/0b/2c83cc67eb5a29f182c8ea425e4b026db0593712edb8eaaf082501ca349f/fasttext_predict-0.9.2.4-cp312-cp312-manylinux_2_31_armv7l.whl>", hash = "sha256:ecb0b854596ba847742597b35c2d0134fcf3a59214d09351d01535854078d56b", size = 237279, upload-time = "2024-11-23T17:23:31.358Z" },

- { url = "<https://files.pythonhosted.org/packages/14/81/0f1b3bda499ffeb7109fe51d9321dc74100db5a4801e3f9a9efe2348922d/fasttext_predict-0.9.2.4-cp312-cp312-musllinux_1_2_aarch64.whl>", hash = "sha256:fbbcfefac10f625d95fc42f28d76cc5bf0c12875f147b5a79108a2669e64a2dc", size = 1214253, upload-time = "2024-11-23T17:23:33.529Z" },

- { url = "<https://files.pythonhosted.org/packages/d1/e6/b1a177a990c29b043a9658f9f4ec7234576ad31939362f9760c237f91d6d/fasttext_predict-0.9.2.4-cp312-cp312-musllinux_1_2_armv7l.whl>", hash = "sha256:a8cb78a00c04b7eb7da18b4805f8557b36911dc4375c947d8938897d2e131841", size = 1099909, upload-time = "2024-11-23T17:23:34.983Z" },

- { url = "<https://files.pythonhosted.org/packages/09/a0/7f23c7c4398f399552f39144849868991da543b66b9bfa8f49a6550fdd46/fasttext_predict-0.9.2.4-cp312-cp312-musllinux_1_2_i686.whl>", hash = "sha256:299ae56ad53e1381c65030143da7bcae12546fd32bc019215592ec1ee40fd19e", size = 1384102, upload-time = "2024-11-23T17:23:37.237Z" },

- { url = "<https://files.pythonhosted.org/packages/e4/2c/568cf15fd48e4cefd0e605af62da5f5f51db3b012f8441d201d0a1173eb1/fasttext_predict-0.9.2.4-cp312-cp312-musllinux_1_2_x86_64.whl>", hash = "sha256:091938062002fe30d214f6e493a3a1e6180d401212d37eea23c29f4b55f3f347", size = 1281283, upload-time = "2024-11-23T17:23:39.676Z" },

- { url = "<https://files.pythonhosted.org/packages/e7/68/0967ec3d5333c23fae1f1bdb851fa896f8f6068ef0ca3a8afee1aa2ee57d/fasttext_predict-0.9.2.4-cp312-cp312-win32.whl>", hash = "sha256:981b8d9734623f8f9a8003970f765e14b1d91ee82c59c35e8eba6b76368fa95e", size = 91089, upload-time = "2024-11-23T17:23:41.082Z" },

- { url = "<https://files.pythonhosted.org/packages/a7/c5/11c1f50b47f492d562974878ec34b6a0b84699f8b05e1cc3a75c65349784/fasttext_predict-0.9.2.4-cp312-cp312-win_amd64.whl>", hash = "sha256:bd3c33971c241577b0767e55d97acfda790f77378f9d5ee7872b6ee4bd63130b", size = 104889, upload-time = "2024-11-23T17:23:42.193Z" },

- { url = "<https://files.pythonhosted.org/packages/89/fc/5cd65224c33e33d6faec3fa1047162dc266ed2213016139d936bd36fb7c3/fasttext_predict-0.9.2.4-cp313-cp313-macosx_10_13_x86_64.whl>", hash = "sha256:ddb85e62c95e4e02d417c782e3434ef65554df19e3522f5230f6be15a9373c05", size = 104916, upload-time = "2024-11-23T17:23:43.367Z" },

- { url = "<https://files.pythonhosted.org/packages/d9/53/8d542773e32c9d98dd8c680e390fe7e6d4fc92ab3439dc1bb8e70c46c7ad/fasttext_predict-0.9.2.4-cp313-cp313-macosx_11_0_arm64.whl>", hash = "sha256:102129d45cf98dda871e83ae662f71d999b9ef6ff26bc842ffc1520a1f82930c", size = 97502, upload-time = "2024-11-23T17:23:44.447Z" },

- { url = "<https://files.pythonhosted.org/packages/50/99/049fd6b01937705889bd9a00c31e5c55f0ae4b7704007b2ef7a82bf2b867/fasttext_predict-0.9.2.4-cp313-cp313-manylinux_2_17_aarch64.manylinux2014_aarch64.whl>", hash = "sha256:05ba6a0fbf8cb2141b8ca2bc461db97af8ac31a62341e4696a75048b9de39e10", size = 282951, upload-time = "2024-11-23T17:23:46.31Z" },

- { url = "<https://files.pythonhosted.org/packages/83/cb/79b71709edbb53c3c5f8a8b60fe2d3bc98d28a8e75367c89afedf3307aa9/fasttext_predict-0.9.2.4-cp313-cp313-manylinux_2_17_i686.manylinux2014_i686.whl>", hash = "sha256:0c7a779215571296ecfcf86545cb30ec3f1c6f43cbcd69f83cc4f67049375ea1", size = 307377, upload-time = "2024-11-23T17:23:47.685Z" },

- { url = "<https://files.pythonhosted.org/packages/7c/4a/b15b7be003e76613173cc77d9c6cce4bf086073079354e0177deaa768f59/fasttext_predict-0.9.2.4-cp313-cp313-manylinux_2_17_x86_64.manylinux2014_x86_64.whl>", hash = "sha256:ddd2f03f3f206585543f5274b1dbc5f651bae141a1b14c9d5225c2a12e5075c2", size = 295746, upload-time = "2024-11-23T17:23:49.024Z" },

- { url = "<https://files.pythonhosted.org/packages/e3/d3/f030cd45bdd4b052fcf23e730fdf0804e024b0cad43d7c7f8704faaec2f5/fasttext_predict-0.9.2.4-cp313-cp313-manylinux_2_31_armv7l.whl>", hash = "sha256:748f9edc3222a1fb7a61331c4e06d3b7f2390ae493f91f09d372a00b81762a8d", size = 236939, upload-time = "2024-11-23T17:23:50.306Z" },

- { url = "<https://files.pythonhosted.org/packages/a2/01/6f2985afd58fdc5f4ecd058d5d9427d03081d468960982df97316c03f6bb/fasttext_predict-0.9.2.4-cp313-cp313-musllinux_1_2_aarch64.whl>", hash = "sha256:1aee47a40757cd24272b34eaf9ceeea86577fd0761b0fd0e41599c6549abdf04", size = 1214189, upload-time = "2024-11-23T17:23:51.647Z" },

- { url = "<https://files.pythonhosted.org/packages/75/07/931bcdd4e2406e45e54d57e056c2e0766616a5280a18fbf6ef078aa439ab/fasttext_predict-0.9.2.4-cp313-cp313-musllinux_1_2_armv7l.whl>", hash = "sha256:6ff0f152391ee03ffc18495322100c01735224f7843533a7c4ff33c8853d7be1", size = 1099889, upload-time = "2024-11-23T17:23:53.127Z" },

- { url = "<https://files.pythonhosted.org/packages/a2/eb/6521b4bbf387252a96a6dc0f54986f078a93db0a9d4ba77258dcf1fa8be7/fasttext_predict-0.9.2.4-cp313-cp313-musllinux_1_2_i686.whl>", hash = "sha256:4d92f5265318b41d6e68659fd459babbff692484e492c5013995b90a56b517c9", size = 1383959, upload-time = "2024-11-23T17:23:54.521Z" },

- { url = "<https://files.pythonhosted.org/packages/b7/6b/d56606761afb3a3912c52971f0f804e2e9065f049c412b96c47d6fca6218/fasttext_predict-0.9.2.4-cp313-cp313-musllinux_1_2_x86_64.whl>", hash = "sha256:3a7720cce1b8689d88df76cac1425e84f9911c69a4e40a5309d7d3435e1bb97c", size = 1281097, upload-time = "2024-11-23T17:23:55.9Z" },

- { url = "<https://files.pythonhosted.org/packages/91/83/55bb4a37bb3b3a428941f4e1323c345a662254f576f8860b3098d9742510/fasttext_predict-0.9.2.4-cp313-cp313-win32.whl>", hash = "sha256:d16acfced7871ed0cd55b476f0dbdddc7a5da1ffc9745a3c5674846cf1555886", size = 91137, upload-time = "2024-11-23T17:23:57.886Z" },

- { url = "<https://files.pythonhosted.org/packages/9c/1d/c1ccc8790ce54200c84164d99282f088dddb9760aeefc8860856aafa40b4/fasttext_predict-0.9.2.4-cp313-cp313-win_amd64.whl>", hash = "sha256:96a23328729ce62a851f8953582e576ca075ee78d637df4a78a2b3609784849e", size = 104896, upload-time = "2024-11-23T17:23:59.028Z" },

- { url = "<https://files.pythonhosted.org/packages/a4/c9/a1ccc749c59e2480767645ecc03bd842a7fa5b2b780d69ac370e6f8298d2/fasttext_predict-0.9.2.4-cp313-cp313t-macosx_10_13_x86_64.whl>", hash = "sha256:b1357d0d9d8568db84668b57e7c6880b9c46f757e8954ad37634402d36f09dba", size = 109401, upload-time = "2024-11-23T17:24:00.191Z" },

- { url = "<https://files.pythonhosted.org/packages/90/1f/33182b76eb0524155e8ff93e7939feaf5325385e5ff2a154f383d9a02317/fasttext_predict-0.9.2.4-cp313-cp313t-macosx_11_0_arm64.whl>", hash = "sha256:9604c464c5d86c7eba34b040080be7012e246ef512b819e428b7deb817290dae", size = 102131, upload-time = "2024-11-23T17:24:02.052Z" },

- { url = "<https://files.pythonhosted.org/packages/2b/df/1886daea373382e573f28ce49e3fc8fb6b0ee0c84e2b0becf5b254cd93fb/fasttext_predict-0.9.2.4-cp313-cp313t-manylinux_2_17_aarch64.manylinux2014_aarch64.whl>", hash = "sha256:cc6da186c2e4497cbfaba9c5424e58c7b72728b25d980829eb96daccd7cface1", size = 287396, upload-time = "2024-11-23T17:24:03.294Z" },

- { url = "<https://files.pythonhosted.org/packages/35/8f/d1c2c0f0251bee898d508253a437683b0480a1074cfb25ded1f7fdbb925a/fasttext_predict-0.9.2.4-cp313-cp313t-manylinux_2_17_i686.manylinux2014_i686.whl>", hash = "sha256:366ed2ca4f4170418f3585e92059cf17ee2c963bf179111c5b8ba48f06cd69d1", size = 311090, upload-time = "2024-11-23T17:24:04.625Z" },

- { url = "<https://files.pythonhosted.org/packages/5d/52/07d6ed46148662fae84166bc69d944caca87fabc850ebfbd9640b20dafe7/fasttext_predict-0.9.2.4-cp313-cp313t-manylinux_2_17_x86_64.manylinux2014_x86_64.whl>", hash = "sha256:2f1877edbb815a43e7d38cc7332202e759054cf0b5a4b7e34a743c0f5d6e7333", size = 300359, upload-time = "2024-11-23T17:24:06.486Z" },

- { url = "<https://files.pythonhosted.org/packages/fa/a1/751ff471a991e5ed0bae9e7fa6fc8d8ab76b233a7838a27d70d62bed0c8e/fasttext_predict-0.9.2.4-cp313-cp313t-manylinux_2_31_armv7l.whl>", hash = "sha256:f63c31352ba6fc910290b0fe12733770acd8cfa0945fcb9cf3984d241abcfc9d", size = 241164, upload-time = "2024-11-23T17:24:08.501Z" },

- { url = "<https://files.pythonhosted.org/packages/94/19/e251f699a0e9c001fa672ea0929c456160faa68ecfafc19e8def09982b6a/fasttext_predict-0.9.2.4-cp313-cp313t-musllinux_1_2_aarch64.whl>", hash = "sha256:898e14b03fbfb0a8d9a5185a0a00ff656772b3baa37cad122e06e8e4d6da3832", size = 1218629, upload-time = "2024-11-23T17:24:10.04Z" },

- { url = "<https://files.pythonhosted.org/packages/1d/46/1af2f779f8cfd746496a226581f747d3051888e3e3c5b2ca37231e5d04f8/fasttext_predict-0.9.2.4-cp313-cp313t-musllinux_1_2_armv7l.whl>", hash = "sha256:a33bb5832a69fc54d18cadcf015677c1acb5ccc7f0125d261df2a89f8aff01f6", size = 1100535, upload-time = "2024-11-23T17:24:11.5Z" },

- { url = "<https://files.pythonhosted.org/packages/4c/b7/900ccd74a9ba8be7ca6d04bba684e9c43fb0dbed8a3d12ec0536228e2c32/fasttext_predict-0.9.2.4-cp313-cp313t-musllinux_1_2_i686.whl>", hash = "sha256:7fe9e98bd0701d598bf245eb2fbf592145cd03551684a2102a4b301294b9bd87", size = 1387651, upload-time = "2024-11-23T17:24:13.135Z" },

- { url = "<https://files.pythonhosted.org/packages/0b/5a/99fdaed054079f7c96e70df0d7016c4eb6b9e487a614396dd8f849244a52/fasttext_predict-0.9.2.4-cp313-cp313t-musllinux_1_2_x86_64.whl>", hash = "sha256:dcb8c5a74c1785f005fd83d445137437b79ac70a2dfbfe4bb1b09aa5643be545", size = 1286189, upload-time = "2024-11-23T17:24:14.615Z" },

- { url = "<https://files.pythonhosted.org/packages/87/6a/9114d65b3f7a9c20a62b9d2ca3b770ee65de849e4131cc7aa58cdc50cb07/fasttext_predict-0.9.2.4-cp313-cp313t-win32.whl>", hash = "sha256:a85c7de3d4480faa12b930637fca9c23144d1520786fedf9ba8edd8642ed4aea", size = 95905, upload-time = "2024-11-23T17:24:15.868Z" },

- { url = "<https://files.pythonhosted.org/packages/31/fb/6d251f3fdfe3346ee60d091f55106513e509659ee005ad39c914182c96f4/fasttext_predict-0.9.2.4-cp313-cp313t-win_amd64.whl>", hash = "sha256:be0933fa4af7abae09c703d28f9e17c80e7069eb6f92100b21985b777f4ea275", size = 110325, upload-time = "2024-11-23T17:24:16.984Z" },

- { url = "<https://files.pythonhosted.org/packages/ec/97/02d9c533c4c8916948c75195f99ad6bf4baa932ff5b6982b7513fdd37eee/fasttext_predict-0.9.2.4-cp39-cp39-macosx_10_9_x86_64.whl>", hash = "sha256:8ff71f9905567271a760139978dec62f8c224f20c8c42a45addd4830fa3db977", size = 104007, upload-time = "2024-11-23T17:24:18.127Z" },

- { url = "<https://files.pythonhosted.org/packages/a4/36/c030bfc7da05d917d77c38826149d2efb829d0139ab97aba14b6870a502e/fasttext_predict-0.9.2.4-cp39-cp39-macosx_11_0_arm64.whl>", hash = "sha256:89401fa60533a9307bf26c312f3a47c58f9f8daf735532a03b0a88af391a6b7a", size = 96593, upload-time = "2024-11-23T17:24:19.558Z" },

- { url = "<https://files.pythonhosted.org/packages/ae/9f/3594de1953dd70f9b8b00ea03d0b4acfe4dd542998fb4e1877ce84b3b993/fasttext_predict-0.9.2.4-cp39-cp39-manylinux_2_17_aarch64.manylinux2014_aarch64.whl>", hash = "sha256:9b8e51eef5ebb1905b3b10e0f19cec7f0259f9134cfde76e4c172ac5dff3d1f1", size = 282025, upload-time = "2024-11-23T17:24:22.101Z" },

- { url = "<https://files.pythonhosted.org/packages/9e/fe/635175091f2943b8d862144c901cb60b6bdb11ac0dea2ae93844d596c749/fasttext_predict-0.9.2.4-cp39-cp39-manylinux_2_17_i686.manylinux2014_i686.whl>", hash = "sha256:4d4bd0178d295ed898903fc8e1454682a44e9e3db8bc3e777c3e122f2c5d2a39", size = 306291, upload-time = "2024-11-23T17:24:23.542Z" },

- { url = "<https://files.pythonhosted.org/packages/c8/e2/1ea6cd22b308bc2b554a43fd8afaa3800e60a4bd739fe83e7263727a7228/fasttext_predict-0.9.2.4-cp39-cp39-manylinux_2_17_x86_64.manylinux2014_x86_64.whl>", hash = "sha256:37717d593560d2d82911ba644dc0eb0c8d9b270b005d59bc278ae1465b77b50e", size = 294210, upload-time = "2024-11-23T17:24:24.943Z" },

- { url = "<https://files.pythonhosted.org/packages/4d/dd/1700922cec866f214bbb6fbbbfbebbeb77c85838c2049e6866b27cb1ac56/fasttext_predict-0.9.2.4-cp39-cp39-manylinux_2_31_armv7l.whl>", hash = "sha256:144decf434c79b80cacbb14007602ca0e563a951000dc7ca3308d022b1c6a56c", size = 233003, upload-time = "2024-11-23T17:24:26.439Z" },

- { url = "<https://files.pythonhosted.org/packages/b8/a4/07df039c395944fa76333d268c7d23eef6d80c4288d6efda1e5339cddc23/fasttext_predict-0.9.2.4-cp39-cp39-musllinux_1_2_aarch64.whl>", hash = "sha256:abd5f77f491f83f9f2f374c38adb9432fae1e92db28fdd2cf5c0f3db48e1f805", size = 1212044, upload-time = "2024-11-23T17:24:27.943Z" },

- { url = "<https://files.pythonhosted.org/packages/99/20/f1f10a21a649bbed57316a7235464e6758711ed7a2a8e58b6a6878c95d96/fasttext_predict-0.9.2.4-cp39-cp39-musllinux_1_2_armv7l.whl>", hash = "sha256:25f3f82b847a320ce595dc772f5e1054ef0a1aa02e7d39feb0ea6374dc83aa55", size = 1099183, upload-time = "2024-11-23T17:24:30.542Z" },

- { url = "<https://files.pythonhosted.org/packages/42/86/46e65d9326a14541c32f5f14fbda1f37976c2966b51223d822dbb592c517/fasttext_predict-0.9.2.4-cp39-cp39-musllinux_1_2_i686.whl>", hash = "sha256:6390f898bbc83a85447338e1a68d1730d5a5ca68292ea3621718c3f4be39288f", size = 1385701, upload-time = "2024-11-23T17:24:32.862Z" },

- { url = "<https://files.pythonhosted.org/packages/94/b1/c73e3c91915d9fd4556db783e4721d343dec3e46dc7abb9de27acb445b4e/fasttext_predict-0.9.2.4-cp39-cp39-musllinux_1_2_x86_64.whl>", hash = "sha256:038bf374a9b9bd665fe58ef28a9b6a4703f8ba1de93bb747b974d7f78f023222", size = 1276344, upload-time = "2024-11-23T17:24:34.338Z" },

- { url = "<https://files.pythonhosted.org/packages/87/36/0404ed11cb109c94f09e3ec20709e3bc5ad60d6f44ab4a53275dd415df01/fasttext_predict-0.9.2.4-cp39-cp39-win32.whl>", hash = "sha256:639ab150585ceb3832912d9b623122735481cff676876040ca9b08312264634a", size = 90526, upload-time = "2024-11-23T17:24:36.443Z" },

- { url = "<https://files.pythonhosted.org/packages/fa/da/2ab0060f805449bfe22dafbd441d5b94b2d9f4ab185dd2c0436bd1db56ea/fasttext_predict-0.9.2.4-cp39-cp39-win_amd64.whl>", hash = "sha256:91c84cfb18a3a617e785fc9aa3bd4c80ffbe20009beb8f9e63e362160cb71a08", size = 102655, upload-time = "2024-11-23T17:24:37.714Z" },

- { url = "<https://files.pythonhosted.org/packages/20/ba/c7fe88fbf59935118b9a756e3ae671d8ddcdd58170f4e53d60d9863b29e6/fasttext_predict-0.9.2.4-pp310-pypy310_pp73-manylinux_2_17_aarch64.manylinux2014_aarch64.whl>", hash = "sha256:b11ba9414aa71754f798a102cf7d3df53307055b2b0f0b258a3f2d59c5a12cfa", size = 133206, upload-time = "2024-11-23T17:24:38.986Z" },

- { url = "<https://files.pythonhosted.org/packages/50/a6/8807d54b25905d3d91e7b16705632a3ccf4adf6457daae959c4f42987c27/fasttext_predict-0.9.2.4-pp310-pypy310_pp73-manylinux_2_17_i686.manylinux2014_i686.whl>", hash = "sha256:3c89c769e3646bdb341487a68835239f35a4a0959cc1a8d8a7d215f40b22a230", size = 149227, upload-time = "2024-11-23T17:24:40.285Z" },

- { url = "<https://files.pythonhosted.org/packages/27/4a/55ae88864d5711822ecf6f37d54d655dc2e3617ae70d07bf28c08d9bea5f/fasttext_predict-0.9.2.4-pp310-pypy310_pp73-manylinux_2_17_x86_64.manylinux2014_x86_64.whl>", hash = "sha256:5f3b9cd4a2cf4c4853323f57c5da6ecffca6aeb9b6d8751ee40fe611d6edf8dd", size = 140205, upload-time = "2024-11-23T17:24:42.307Z" },

- { url = "<https://files.pythonhosted.org/packages/a0/33/1b5baa8960548100fddc40908780f0c18fddff8a514f9cd3dd0f6676746d/fasttext_predict-0.9.2.4-pp310-pypy310_pp73-win_amd64.whl>", hash = "sha256:1c92905396c74e5cb29ddbfa763b5addec1581b6e0eae4cbe82248dfe733557e", size = 102845, upload-time = "2024-11-23T17:24:43.64Z" },
  +\]

- \[[package]\]
  name = "filelock"
  version = "3.18.0"
  @@ -1773,6 +1871,9 @@ easyocr = \[
  gmft = \[
  { name = "gmft" },
  \]
  +language-detection = \[

- { name = "fast-langdetect" },
  +\]
  paddleocr = \[
  { name = "paddleocr" },
  { name = "paddlepaddle" },
  @@ -1783,6 +1884,10 @@ paddleocr = \[
  dev = \[
  { name = "covdefaults" },
  { name = "mypy" },

- { name = "numpy", version = "2.0.2", source = { registry = "<https://pypi.org/simple>" }, marker = "python_full_version < '3.10'" },

- { name = "numpy", version = "2.2.6", source = { registry = "<https://pypi.org/simple>" }, marker = "python_full_version == '3.10.\*'" },

- { name = "numpy", version = "2.3.1", source = { registry = "<https://pypi.org/simple>" }, marker = "python_full_version >= '3.11'" },

- { name = "pandas" },
  { name = "pre-commit" },
  { name = "pytest" },
  { name = "pytest-cov" },
  @@ -1807,6 +1912,7 @@ requires-dist = \[
  { name = "easyocr", marker = "extra == 'all'", specifier = ">=1.7.2" },
  { name = "easyocr", marker = "extra == 'easyocr'", specifier = ">=1.7.2" },
  { name = "exceptiongroup", marker = "python_full_version < '3.11'", specifier = ">=1.2.2" },

- { name = "fast-langdetect", marker = "extra == 'language-detection'", specifier = ">=0.2.0" },
  { name = "gmft", marker = "extra == 'all'", specifier = ">=0.4.2" },
  { name = "gmft", marker = "extra == 'gmft'", specifier = ">=0.4.2" },
  { name = "html-to-markdown", specifier = ">=1.4.0" },
  @@ -1824,12 +1930,14 @@ requires-dist = \[
  { name = "setuptools", marker = "extra == 'paddleocr'", specifier = ">=80.9.0" },
  { name = "typing-extensions", marker = "python_full_version < '3.12'", specifier = ">=4.14.0" },
  \]
  -provides-extras = ["all", "chunking", "easyocr", "gmft", "paddleocr"]
  +provides-extras = ["all", "chunking", "easyocr", "gmft", "paddleocr", "language-detection"]

[package.metadata.requires-dev]
dev = \[
{ name = "covdefaults", specifier = ">=2.3.0" },
{ name = "mypy", specifier = ">=1.16.1" },

- { name = "numpy", specifier = ">=1.24.0" },
- { name = "pandas", specifier = ">=2.0.0" },
  { name = "pre-commit", specifier = ">=4.2.0" },
  { name = "pytest", specifier = ">=8.4.1" },
  { name = "pytest-cov", specifier = ">=6.2.1" },
  @@ -4441,6 +4549,20 @@ wheels = \[
  { url = "<https://files.pythonhosted.org/packages/3f/51/d4db610ef29373b879047326cbf6fa98b6c1969d6f6dc423279de2b1be2c/requests_toolbelt-1.0.0-py2.py3-none-any.whl>", hash = "sha256:cccfdd665f0a24fcf4726e690f65639d272bb0637b9b92dfd91a5568ccf6bd06", size = 54481, upload-time = "2023-05-01T04:11:28.427Z" },
  \]

+\[[package]\]
+name = "robust-downloader"
+version = "0.0.2"
+source = { registry = "<https://pypi.org/simple>" }
+dependencies = \[

- { name = "colorlog" },
- { name = "requests" },
- { name = "tqdm" },
  +\]
  +sdist = { url = "<https://files.pythonhosted.org/packages/63/20/8d28efa080f58fa06f6378875ac482ee511c076369e5293a2e65128cf9a0/robust-downloader-0.0.2.tar.gz>", hash = "sha256:08c938b96e317abe6b037e34230a91bda9b5d613f009bca4a47664997c61de90", size = 15785, upload-time = "2023-11-13T03:00:20.637Z" }
  +wheels = \[
- { url = "<https://files.pythonhosted.org/packages/56/a1/779e9d0ebbdc704411ce30915a1105eb01aeaa9e402d7e446613ff8fb121/robust_downloader-0.0.2-py3-none-any.whl>", hash = "sha256:8fe08bfb64d714fd1a048a7df6eb7b413eb4e624309a49db2c16fbb80a62869d", size = 15534, upload-time = "2023-11-13T03:00:18.957Z" },
  +\]
- \[[package]\]
  name = "ruamel-yaml"
  version = "0.18.14"
  Error: Process completed with exit code 1.
