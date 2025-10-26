"""Integration tests for embedding generation during extraction."""

import pathlib

import pytest

from kreuzberg import extract_bytes_sync, extract_file_sync
from kreuzberg._internal_bindings import (
    ChunkingConfig,
    EmbeddingConfig,
    EmbeddingModelType,
    ExtractionConfig,
)


@pytest.mark.slow
def test_extract_text_with_embeddings_generates_embeddings() -> None:
    """Test that extraction with embeddings enabled generates embeddings."""
    text = "This is a test document for embedding generation. " * 20
    text_bytes = text.encode("utf-8")

    embedding_config = EmbeddingConfig(
        model=EmbeddingModelType.preset("fast"),
        normalize=True,
        batch_size=32,
    )

    chunking_config = ChunkingConfig(max_chars=100, max_overlap=20, embedding=embedding_config)

    config = ExtractionConfig(chunking=chunking_config)

    result = extract_bytes_sync(text_bytes, "text/plain", config)

    assert result.chunks is not None, "Should have chunks"
    assert len(result.chunks) > 0, "Should have at least one chunk"

    for chunk in result.chunks:
        assert chunk["embedding"] is not None, "Each chunk should have an embedding"
        assert len(chunk["embedding"]) == 384, "Fast preset should produce 384-dim embeddings"
        assert all(isinstance(x, float) for x in chunk["embedding"]), "Embeddings should be floats"
        sum_values = sum(abs(x) for x in chunk["embedding"])
        assert sum_values > 0.0, "Embedding should not be all zeros"


@pytest.mark.slow
def test_extract_with_balanced_preset_generates_correct_dimensions() -> None:
    """Test that balanced preset generates 768-dimensional embeddings."""
    text = "Testing balanced preset embedding generation. " * 10
    text_bytes = text.encode("utf-8")

    embedding_config = EmbeddingConfig(
        model=EmbeddingModelType.preset("balanced"),
        normalize=True,
        batch_size=32,
    )

    chunking_config = ChunkingConfig(max_chars=100, max_overlap=20, embedding=embedding_config)

    config = ExtractionConfig(chunking=chunking_config)

    result = extract_bytes_sync(text_bytes, "text/plain", config)

    assert result.chunks is not None
    assert len(result.chunks) > 0

    # Balanced preset should produce 768-dim embeddings
    for chunk in result.chunks:
        assert chunk["embedding"] is not None
        assert len(chunk["embedding"]) == 768, "Balanced preset should produce 768-dim embeddings"


@pytest.mark.slow
def test_extract_with_quality_preset_generates_correct_dimensions() -> None:
    """Test that quality preset generates 1024-dimensional embeddings."""
    text = "Testing quality preset embedding generation. " * 10
    text_bytes = text.encode("utf-8")

    embedding_config = EmbeddingConfig(
        model=EmbeddingModelType.preset("quality"),
        normalize=True,
        batch_size=16,
    )

    chunking_config = ChunkingConfig(max_chars=150, embedding=embedding_config)

    config = ExtractionConfig(chunking=chunking_config)

    result = extract_bytes_sync(text_bytes, "text/plain", config)

    assert result.chunks is not None
    assert len(result.chunks) > 0

    for chunk in result.chunks:
        if chunk["embedding"] is not None:
            assert len(chunk["embedding"]) == 1024, "Quality preset should produce 1024-dim embeddings"


@pytest.mark.slow
def test_extract_with_multilingual_preset() -> None:
    """Test multilingual preset with various language text."""
    # Mix of English, Spanish, and French
    text = (
        "Hello world. This is an English sentence. "
        "Hola mundo. Esta es una oración en español. "
        "Bonjour le monde. Ceci est une phrase en français. "
    ) * 5

    text_bytes = text.encode("utf-8")

    embedding_config = EmbeddingConfig(model=EmbeddingModelType.preset("multilingual"), normalize=True, batch_size=32)

    chunking_config = ChunkingConfig(max_chars=100, max_overlap=20, embedding=embedding_config)

    config = ExtractionConfig(chunking=chunking_config)

    result = extract_bytes_sync(text_bytes, "text/plain", config)

    assert result.chunks is not None
    assert len(result.chunks) > 0

    for chunk in result.chunks:
        assert chunk["embedding"] is not None
        assert len(chunk["embedding"]) == 768, "Multilingual preset should produce 768-dim embeddings"


def test_extract_without_embeddings_has_no_embeddings() -> None:
    """Test that extraction without embedding config doesn't generate embeddings."""
    text = "Test document without embeddings. " * 10
    text_bytes = text.encode("utf-8")

    chunking_config = ChunkingConfig(max_chars=100, max_overlap=20)

    config = ExtractionConfig(chunking=chunking_config)

    result = extract_bytes_sync(text_bytes, "text/plain", config)

    assert result.chunks is not None
    assert len(result.chunks) > 0

    for chunk in result.chunks:
        assert chunk["embedding"] is None, "Should not have embeddings when not configured"


@pytest.mark.slow
def test_embedding_normalization_flag() -> None:
    """Test that normalization flag affects embedding values."""
    text = "Test normalization. " * 10
    text_bytes = text.encode("utf-8")

    # With normalization
    normalized_config = EmbeddingConfig(model=EmbeddingModelType.preset("fast"), normalize=True, batch_size=32)

    chunking_config_normalized = ChunkingConfig(max_chars=100, embedding=normalized_config)

    config_normalized = ExtractionConfig(chunking=chunking_config_normalized)

    result_normalized = extract_bytes_sync(text_bytes, "text/plain", config_normalized)

    assert result_normalized.chunks is not None
    assert len(result_normalized.chunks) > 0
    assert result_normalized.chunks[0]["embedding"] is not None

    # Check that normalized embeddings have L2 norm close to 1
    embedding = result_normalized.chunks[0]["embedding"]
    magnitude = sum(x * x for x in embedding) ** 0.5
    assert abs(magnitude - 1.0) < 0.01, "Normalized embeddings should have magnitude close to 1"


@pytest.mark.slow
def test_batch_size_configuration() -> None:
    """Test that batch_size configuration is accepted."""
    text = "Test batch size configuration. " * 20
    text_bytes = text.encode("utf-8")

    embedding_config_small = EmbeddingConfig(model=EmbeddingModelType.preset("fast"), normalize=True, batch_size=2)

    embedding_config_large = EmbeddingConfig(model=EmbeddingModelType.preset("fast"), normalize=True, batch_size=128)

    # Both should work
    for emb_config in [embedding_config_small, embedding_config_large]:
        chunking_config = ChunkingConfig(max_chars=50, max_overlap=10, embedding=emb_config)
        config = ExtractionConfig(chunking=chunking_config)
        result = extract_bytes_sync(text_bytes, "text/plain", config)

        assert result.chunks is not None
        assert len(result.chunks) > 0
        assert result.chunks[0]["embedding"] is not None


@pytest.mark.slow
def test_embeddings_with_different_chunk_sizes() -> None:
    """Test embeddings work with various chunk sizes."""
    text = "Test different chunk sizes. " * 30
    text_bytes = text.encode("utf-8")

    chunk_sizes = [50, 100, 500, 1000]

    for chunk_size in chunk_sizes:
        embedding_config = EmbeddingConfig(model=EmbeddingModelType.preset("fast"), normalize=True)

        # Use 20% overlap, ensuring it's less than chunk size
        overlap = min(int(chunk_size * 0.2), chunk_size - 1)
        chunking_config = ChunkingConfig(max_chars=chunk_size, max_overlap=overlap, embedding=embedding_config)

        config = ExtractionConfig(chunking=chunking_config)

        result = extract_bytes_sync(text_bytes, "text/plain", config)

        assert result.chunks is not None, f"Should have chunks for size {chunk_size}"
        assert len(result.chunks) > 0, f"Should have at least one chunk for size {chunk_size}"

        for chunk in result.chunks:
            assert chunk["embedding"] is not None, f"Should have embedding for chunk size {chunk_size}"
            assert len(chunk["embedding"]) == 384


def test_fastembed_model_type_custom_dimensions() -> None:
    """Test creating embedding config with custom FastEmbed model."""
    embedding_config = EmbeddingConfig(
        model=EmbeddingModelType.fastembed("BGEBaseENV15", 768),
        normalize=True,
        batch_size=32,
    )

    chunking_config = ChunkingConfig(max_chars=100, max_overlap=20, embedding=embedding_config)

    config = ExtractionConfig(chunking=chunking_config)

    # Just test that configuration is created successfully
    assert config is not None
    assert config.chunking is not None
    assert config.chunking.embedding is not None


def test_custom_cache_directory_configuration() -> None:
    """Test that custom cache directory can be configured."""
    embedding_config = EmbeddingConfig(
        model=EmbeddingModelType.preset("fast"),
        normalize=True,
        cache_dir="/tmp/kreuzberg_test_cache",
    )

    chunking_config = ChunkingConfig(max_chars=100, max_overlap=20, embedding=embedding_config)

    config = ExtractionConfig(chunking=chunking_config)

    assert config is not None
    assert config.chunking is not None


@pytest.mark.slow
def test_embeddings_with_actual_markdown_file(tmp_path: pathlib.Path) -> None:
    """Test embedding generation with actual markdown content."""
    markdown_content = """# Test Document

This is a test document for embedding generation.

## Section 1

Some content in section 1. This has multiple paragraphs to ensure
we get proper chunking and embedding generation.

## Section 2

More content in section 2. We want to verify that embeddings are
generated correctly for real-world markdown content.

### Subsection 2.1

Even more nested content to test chunking behavior.
"""

    md_file = tmp_path / "test.md"
    md_file.write_text(markdown_content)

    embedding_config = EmbeddingConfig(model=EmbeddingModelType.preset("fast"), normalize=True, batch_size=32)

    chunking_config = ChunkingConfig(max_chars=150, embedding=embedding_config)

    config = ExtractionConfig(chunking=chunking_config)

    result = extract_file_sync(str(md_file), None, config)

    assert result.content is not None
    assert result.chunks is not None
    assert len(result.chunks) > 0

    for chunk in result.chunks:
        assert chunk["embedding"] is not None
        assert len(chunk["embedding"]) == 384
