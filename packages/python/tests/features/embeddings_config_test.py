"""Tests for embedding configuration in Python bindings."""

from kreuzberg._internal_bindings import (
    ChunkingConfig,
    EmbeddingConfig,
    EmbeddingModelType,
)


def test_embedding_model_type_preset_creates_successfully() -> None:
    """Test creating an EmbeddingModelType with preset."""
    model = EmbeddingModelType.preset("balanced")
    assert model is not None
    assert "balanced" in repr(model)


def test_embedding_model_type_preset_all_presets() -> None:
    """Test creating all available preset models."""
    presets = ["fast", "balanced", "quality", "multilingual"]
    for preset_name in presets:
        model = EmbeddingModelType.preset(preset_name)
        assert model is not None
        assert preset_name in repr(model)


def test_embedding_model_type_fastembed_creates_successfully() -> None:
    """Test creating an EmbeddingModelType with fastembed."""
    model = EmbeddingModelType.fastembed("BGEBaseENV15", 768)
    assert model is not None
    assert "BGEBaseENV15" in repr(model)
    assert "768" in repr(model)


def test_embedding_model_type_custom_creates_successfully() -> None:
    """Test creating an EmbeddingModelType with custom model."""
    model = EmbeddingModelType.custom("my-custom-model", 512)
    assert model is not None
    assert "my-custom-model" in repr(model)
    assert "512" in repr(model)


def test_embedding_config_creates_with_defaults() -> None:
    """Test creating EmbeddingConfig with default values."""
    config = EmbeddingConfig()
    assert config is not None
    assert config.normalize is True
    assert config.batch_size == 32


def test_embedding_config_creates_with_custom_values() -> None:
    """Test creating EmbeddingConfig with custom values."""
    model = EmbeddingModelType.preset("quality")
    config = EmbeddingConfig(
        model=model,
        normalize=False,
        batch_size=64,
        show_download_progress=True,
        cache_dir="/tmp/models",
    )
    assert config is not None
    assert config.normalize is False
    assert config.batch_size == 64


def test_embedding_config_normalize_property() -> None:
    """Test EmbeddingConfig normalize property."""
    config_true = EmbeddingConfig(normalize=True)
    assert config_true.normalize is True

    config_false = EmbeddingConfig(normalize=False)
    assert config_false.normalize is False


def test_embedding_config_batch_size_property() -> None:
    """Test EmbeddingConfig batch_size property."""
    config_default = EmbeddingConfig()
    assert config_default.batch_size == 32

    config_custom = EmbeddingConfig(batch_size=128)
    assert config_custom.batch_size == 128


def test_chunking_config_creates_with_defaults() -> None:
    """Test creating ChunkingConfig with default values."""
    config = ChunkingConfig()
    assert config is not None
    assert config.max_chars == 1000
    assert config.max_overlap == 200


def test_chunking_config_creates_with_embedding() -> None:
    """Test creating ChunkingConfig with embedding configuration."""
    emb_config = EmbeddingConfig(normalize=True, batch_size=16)
    chunk_config = ChunkingConfig(max_chars=2000, max_overlap=400, embedding=emb_config)
    assert chunk_config is not None
    assert chunk_config.max_chars == 2000
    assert chunk_config.max_overlap == 400
    assert chunk_config.embedding is not None


def test_chunking_config_creates_with_preset() -> None:
    """Test creating ChunkingConfig with preset."""
    config = ChunkingConfig(preset="balanced")
    assert config is not None
    assert config.preset == "balanced"


def test_chunking_config_with_both_preset_and_embedding() -> None:
    """Test creating ChunkingConfig with both preset and embedding."""
    emb_config = EmbeddingConfig()
    config = ChunkingConfig(preset="quality", embedding=emb_config)
    assert config is not None
    assert config.preset == "quality"
    assert config.embedding is not None


def test_chunking_config_embedding_property() -> None:
    """Test ChunkingConfig embedding property."""
    emb_config = EmbeddingConfig()
    config = ChunkingConfig(embedding=emb_config)
    retrieved_config = config.embedding
    assert retrieved_config is not None


def test_chunking_config_without_embedding() -> None:
    """Test ChunkingConfig without embedding."""
    config = ChunkingConfig()
    assert config.embedding is None


def test_chunking_config_preset_property() -> None:
    """Test ChunkingConfig preset property."""
    config_with_preset = ChunkingConfig(preset="fast")
    assert config_with_preset.preset == "fast"


def test_chunking_config_without_preset() -> None:
    """Test ChunkingConfig without preset."""
    config = ChunkingConfig()
    assert config.preset is None


def test_embedding_config_with_all_preset_models() -> None:
    """Test EmbeddingConfig with all preset models."""
    presets = ["fast", "balanced", "quality", "multilingual"]
    for preset_name in presets:
        model = EmbeddingModelType.preset(preset_name)
        config = EmbeddingConfig(model=model)
        assert config is not None


def test_embedding_config_with_fastembed_models() -> None:
    """Test EmbeddingConfig with FastEmbed models."""
    models = [
        ("AllMiniLML6V2Q", 384),
        ("BGEBaseENV15", 768),
        ("BGELargeENV15", 1024),
        ("MultilingualE5Base", 768),
    ]
    for model_name, dimensions in models:
        model = EmbeddingModelType.fastembed(model_name, dimensions)
        config = EmbeddingConfig(model=model)
        assert config is not None


def test_chunking_config_all_presets() -> None:
    """Test ChunkingConfig with all available presets."""
    presets = ["fast", "balanced", "quality", "multilingual"]
    for preset_name in presets:
        config = ChunkingConfig(preset=preset_name)
        assert config is not None
        assert config.preset == preset_name
