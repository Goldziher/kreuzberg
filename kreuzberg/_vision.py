from __future__ import annotations

from pathlib import Path
from typing import Final

from anyio import Path as AsyncPath
from huggingface_hub import snapshot_download
from onnxruntime_genai import Config, Generator, GeneratorParams, Images, Model, MultiModalProcessor, TokenizerStream

from kreuzberg._sync import run_sync

DEFAULT_DIR: Final[Path] = Path(__file__).parent.parent / "models"
DEFAULT_PROMPT: Final[str] = """Extract all text from this image and convert it to properly formatted markdown.

Follow these rules precisely:
        1. Convert headers to markdown headers using the appropriate number of # symbols
        2. Preserve paragraph structure with proper line breaks
        3. Convert bullet points and numbered lists to markdown format
        4. Format tables using markdown table syntax with aligned columns
        5. Convert URLs to markdown links
        6. Preserve footnotes using markdown footnote syntax
        7. Format citations properly
        8. Maintain text emphasis (bold, italic, underline) using markdown syntax
        9. Include code blocks with appropriate language tags if applicable
        10. Preserve the reading order of columns, sidebars, and other layout elements

Extract ALL text content exactly as it appears, maintaining the original language.
Do not summarize or omit any textual content.
"""


def normalize_model_name(model_id: str) -> str:
    value = model_id.split("/")[1]
    return "_".join(value.split(" "))


async def get_genai_config_folder(path: AsyncPath) -> str:
    """Find the first directory containing genai_config.json configuration file.

    Args:
        path: Directory path to search in

    Returns:
        String path to first directory containing genai_config.json

    Raises:
        FileNotFoundError: If no genai_config.json file is found in the directory tree
    """
    async for p in path.iterdir():
        if await p.is_file() and p.name == "genai_config.json":
            return str(p.parent)
        if await p.is_dir() and (value := await get_genai_config_folder(p)):
            return value

    raise FileNotFoundError(f"No genai_config.json found in directory tree starting at {path}")


async def get_hf_model(*, model_id: str, models_dir: Path | str = DEFAULT_DIR) -> str:
    target_dir = AsyncPath(models_dir) / normalize_model_name(model_id)
    await target_dir.mkdir(parents=True, exist_ok=True)

    # Download model files first
    await run_sync(
        snapshot_download,
        model_id,
        local_dir=str(target_dir),  # Download directly to target_dir
        allow_patterns="cpu_and_mobile/*",
    )

    # Now search for genai.json in the downloaded files
    return await get_genai_config_folder(target_dir)


async def get_model_config(*, model_id: str, provider: str, models_dir: Path | str = DEFAULT_DIR) -> Model:
    model_path = await get_hf_model(model_id=model_id, models_dir=models_dir)

    config = Config(model_path)
    config.clear_providers()

    if provider != "cpu":
        config.append_provider(provider)

    return Model(config)


async def extract_image_text(
    *,
    image_path: str,
    prompt: str,
    model: Model,
    processor: MultiModalProcessor,
    tokenizer_stream: TokenizerStream,
    max_length: int = 1024,
) -> str:
    """Extract text from an image using an ONNX Runtime model.

    Args:
        image_path: The path to the image file.
        prompt: The prompt to use for text generation.
        model: The ONNX Runtime model.
        processor: The multimodal processor.
        tokenizer_stream: The tokenizer stream.
        max_length: The maximum length of the generated text.

    Returns:
        The extracted text content.
    """
    prompt_text = f"<|user|>\n<|image_1|>\n{prompt}<|end|>\n<|assistant|>\n"

    onnx_image = await run_sync(Images.open, image_path)

    inputs = processor(prompt_text, images=onnx_image)

    params = GeneratorParams(model)
    params.set_inputs(inputs)
    params.set_search_options(max_length=max_length)

    generator = Generator(model, params)
    generated_text = ""

    while not generator.is_done():
        generator.compute_logits()
        generator.generate_next_token()
        if tokens := generator.get_next_tokens():
            for token in tokens:
                generated_text += tokenizer_stream.decode(token)
        else:
            break

    return generated_text


async def extract_text_and_layout(
    *,
    execution_provider: str = "cpu",
    image_path: Path,
    model_id: str,
    models_dir: Path | str = DEFAULT_DIR,
    prompt: str = DEFAULT_PROMPT,
    max_length: int = 1024,
) -> str:
    """Extract text and layout information from images using an ONNX Runtime model.

    Args:
        execution_provider: The execution provider to use for the model.
        image_path: The path to the image file.
        model_id: The Hugging Face model ID.
        models_dir: The directory to store the downloaded model.
        prompt: The prompt to use for text generation.
        max_length: The maximum length of the generated text.

    Returns:
        A list of extracted text content from each image
    """
    model = await get_model_config(model_id=model_id, provider=execution_provider, models_dir=models_dir)

    processor: MultiModalProcessor = model.create_multimodal_processor()
    tokenizer_stream: TokenizerStream = processor.create_stream()

    return await extract_image_text(
        image_path=str(image_path),
        prompt=prompt,
        model=model,
        processor=processor,
        tokenizer_stream=tokenizer_stream,
        max_length=max_length,
    )
