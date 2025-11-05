"""Custom PostProcessor Example.

Demonstrates implementing custom post-processor plugins.
"""

import re
from datetime import datetime

from kreuzberg import (
    ExtractionResult,
    clear_post_processors,
    extract_file_sync,
    register_post_processor,
    unregister_post_processor,
)


class MetadataEnricher:
    """Post-processor that enriches extraction results with metadata."""

    def name(self) -> str:
        return "metadata_enricher"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Add statistical metadata to the result."""
        content = result.content

        # Calculate statistics
        result.metadata["processed_at"] = datetime.now().isoformat()
        result.metadata["word_count"] = len(content.split())
        result.metadata["char_count"] = len(content)
        result.metadata["line_count"] = len(content.splitlines())
        result.metadata["has_content"] = bool(content.strip())

        # Content type hints
        result.metadata["has_urls"] = bool(re.search(r"https?://", content))
        result.metadata["has_emails"] = bool(re.search(r"\S+@\S+\.\S+", content))
        result.metadata["has_phone_numbers"] = bool(re.search(r"\d{3}[-.]?\d{3}[-.]?\d{4}", content))

        return result


class PIIRedactor:
    """Post-processor that redacts Personally Identifiable Information."""

    def name(self) -> str:
        return "pii_redactor"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Redact PII from content."""
        content = result.content

        # Redact emails
        content = re.sub(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b", "[EMAIL REDACTED]", content)

        # Redact phone numbers (various formats)
        content = re.sub(r"\(\d{3}\)\s*\d{3}[-.]?\d{4}", "[PHONE REDACTED]", content)
        content = re.sub(r"\d{3}[-.]?\d{3}[-.]?\d{4}", "[PHONE REDACTED]", content)

        # Redact SSN
        content = re.sub(r"\b\d{3}-\d{2}-\d{4}\b", "[SSN REDACTED]", content)

        # Redact credit card numbers
        content = re.sub(r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b", "[CARD REDACTED]", content)

        result.content = content
        result.metadata["pii_redacted"] = True

        return result


class TextNormalizer:
    """Post-processor that normalizes text formatting."""

    def name(self) -> str:
        return "text_normalizer"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Normalize text formatting."""
        content = result.content

        # Normalize whitespace
        content = re.sub(r" +", " ", content)  # Multiple spaces -> single space
        content = re.sub(r"\n{3,}", "\n\n", content)  # Multiple newlines -> max 2

        # Remove empty lines
        lines = [line for line in content.splitlines() if line.strip()]
        content = "\n".join(lines)

        # Normalize Unicode
        content = content.strip()

        result.content = content
        result.metadata["text_normalized"] = True

        return result


class LanguageTranslator:
    """Post-processor that translates content to target language.

    Note: This is a mock example. In production, use a translation API.
    """

    def __init__(self, target_language: str = "en") -> None:
        self.target_language = target_language

    def name(self) -> str:
        return "language_translator"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Translate content to target language."""
        # In production: use translation API
        # from googletrans import Translator
        # translator = Translator()
        # result.content = translator.translate(
        #     result.content,
        #     dest=self.target_language
        # ).text

        result.metadata["translated_to"] = self.target_language
        result.metadata["original_language"] = result.detected_languages[0] if result.detected_languages else "unknown"

        return result


class SummaryGenerator:
    """Post-processor that generates a summary of the content."""

    def __init__(self, max_summary_length: int = 500) -> None:
        self.max_summary_length = max_summary_length

    def name(self) -> str:
        return "summary_generator"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Generate content summary."""
        content = result.content

        # Simple extractive summarization
        summary = content[: self.max_summary_length]

        # Try to break at sentence boundary
        if len(content) > self.max_summary_length:
            last_period = summary.rfind(".")
            last_newline = summary.rfind("\n")
            break_point = max(last_period, last_newline)

            if break_point > 0:
                summary = summary[: break_point + 1]
            else:
                summary += "..."

        result.metadata["summary"] = summary.strip()
        result.metadata["is_truncated"] = len(content) > self.max_summary_length

        return result


class KeywordExtractor:
    """Post-processor that extracts keywords from content."""

    def name(self) -> str:
        return "keyword_extractor"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Extract keywords from content."""
        # Simple word frequency-based keyword extraction
        content = result.content.lower()

        # Remove common words (stopwords)
        stopwords = {
            "the",
            "a",
            "an",
            "and",
            "or",
            "but",
            "in",
            "on",
            "at",
            "to",
            "for",
            "of",
            "with",
            "is",
            "was",
            "are",
            "were",
            "be",
            "been",
            "being",
        }

        # Extract words
        words = re.findall(r"\b[a-z]{4,}\b", content)

        # Count word frequencies
        word_freq: dict[str, int] = {}
        for word in words:
            if word not in stopwords:
                word_freq[word] = word_freq.get(word, 0) + 1

        # Get top 10 keywords
        keywords = sorted(word_freq.items(), key=lambda x: x[1], reverse=True)[:10]
        result.metadata["keywords"] = [word for word, _ in keywords]

        return result


def main() -> None:
    # Register post-processors
    register_post_processor(MetadataEnricher())
    register_post_processor(PIIRedactor())
    register_post_processor(TextNormalizer())
    register_post_processor(LanguageTranslator(target_language="en"))
    register_post_processor(SummaryGenerator(max_summary_length=300))
    register_post_processor(KeywordExtractor())

    # Extract with all post-processors
    extract_file_sync("document.pdf")

    # Unregister specific post-processor
    unregister_post_processor("pii_redactor")

    extract_file_sync("document.pdf")

    # Clear all post-processors
    clear_post_processors()

    extract_file_sync("document.pdf")

    # Register selective post-processors
    register_post_processor(MetadataEnricher())
    register_post_processor(SummaryGenerator(max_summary_length=200))

    extract_file_sync("document.pdf")


if __name__ == "__main__":
    main()
