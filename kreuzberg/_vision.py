from __future__ import annotations

from pathlib import Path
from typing import Any, Final

import torch
from huggingface_hub import snapshot_download
from PIL import Image
from transformers import AutoModelForCausalLM, AutoProcessor

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


async def download_hf_model(*, model_id: str, models_dir: Path | str = DEFAULT_DIR) -> Path:
    """Download model files from Hugging Face.

    Args:
        model_id: The Hugging Face model ID.
        models_dir: The directory to store the downloaded model.

    Returns:
        Path to the downloaded model directory.
    """
    target_dir = Path(models_dir) / normalize_model_name(model_id)
    target_dir.mkdir(parents=True, exist_ok=True)

    # Download model files
    await run_sync(
        snapshot_download,
        model_id,
        local_dir=str(target_dir),
    )

    return target_dir


async def load_vision_model(model_dir: Path, device: str, num_crops: int) -> tuple[AutoModelForCausalLM, AutoProcessor]:
    """Load the vision model and processor.

    Args:
        model_dir: Path to the model directory
        device: Device to load the model on ("cpu", "cuda", "mps")
        num_crops: Number of crops to use in the processor

    Returns:
        Tuple of (model, processor)
    """
    # Only try flash attention when using GPU
    if device == "cuda" and torch.cuda.is_available():
        try:
            model: AutoModelForCausalLM = await run_sync(
                AutoModelForCausalLM.from_pretrained,
                model_dir,
                device_map=device,
                trust_remote_code=True,
                torch_dtype="auto",
                _attn_implementation="flash_attention_2",
            )
        except ImportError:
            # Fall back to eager implementation if flash attention is not available
            model: AutoModelForCausalLM = await run_sync(
                AutoModelForCausalLM.from_pretrained,
                model_dir,
                device_map=device,
                trust_remote_code=True,
                torch_dtype="auto",
                _attn_implementation="eager",
            )
    else:
        # For CPU or other devices, use eager implementation directly
        model: AutoModelForCausalLM = await run_sync(
            AutoModelForCausalLM.from_pretrained,
            model_dir,
            device_map=device,
            trust_remote_code=True,
            torch_dtype="auto",
            _attn_implementation="eager",
        )

    # Load processor with optimal num_crops setting
    processor: AutoProcessor = await run_sync(
        AutoProcessor.from_pretrained,
        model_dir,
        trust_remote_code=True,
        num_crops=num_crops,
    )

    return model, processor


async def process_images(image_path: Path, prompt: str, processor: AutoProcessor) -> dict[str, Any]:
    """Process a single image and create input tensors.

    Args:
        image_path: Path to image file
        prompt: Text prompt
        processor: Model processor

    Returns:
        Processed inputs for the model
    """
    # Load single image
    image = await run_sync(Image.open, str(image_path))  # type: ignore[return-value]
    image = await run_sync(lambda img: img.convert("RGB"), image)  # type: ignore[return-value]

    # Prepare prompt with image placeholder
    messages = [{"role": "user", "content": f"<|image_1|>\n{prompt}"}]

    # Apply chat template
    prompt_text = await run_sync(
        processor.tokenizer.apply_chat_template,
        messages,
        tokenize=False,
        add_generation_prompt=True,
    )

    # Process inputs
    return await run_sync(
        processor,
        prompt_text,
        image,
        return_tensors="pt",
    )


async def generate_text(
    model: AutoModelForCausalLM,
    processor: AutoProcessor,
    inputs: dict[str, torch.Tensor],
    device: str,
    max_new_tokens: int = 1024,
) -> str:
    """Generate text based on processed inputs.

    Args:
        model: The model
        processor: The processor
        inputs: Processed inputs
        device: Device to run generation on
        max_new_tokens: Maximum new tokens to generate

    Returns:
        Generated text
    """
    # Move inputs to device if needed
    if device != "cpu":
        inputs = await run_sync(
            lambda x: {k: v.to(device) for k, v in x.items()},
            inputs,
        )

    # Generate text
    with torch.no_grad():
        output_ids = model.generate(
            **inputs,
            eos_token_id=processor.tokenizer.eos_token_id,
            max_new_tokens=max_new_tokens,
            do_sample=False,
            temperature=0.0,
        )

    # Get only the newly generated tokens (skip the input tokens)
    output_ids = output_ids[:, inputs["input_ids"].shape[1] :]

    # Decode the output
    generated_text = processor.batch_decode(
        output_ids,
        skip_special_tokens=True,
        clean_up_tokenization_spaces=False,
    )

    return generated_text[0] if generated_text else ""


async def extract_image_text(
    *,
    image_path: Path,
    model: AutoModelForCausalLM,
    processor: AutoProcessor,
    prompt: str,
    device: str,
    max_new_tokens: int = 1024,
) -> str:
    """Extract text from images using a vision model.

    Args:
        image_path: Path to image file(s)
        model: The vision model
        processor: The vision processor
        prompt: Text prompt to guide extraction
        device: Device to run inference on
        max_new_tokens: Maximum tokens to generate

    Returns:
        Extracted text
    """
    # Process the images
    inputs = await process_images(image_path, prompt, processor)

    # Generate text from the processed inputs
    return await generate_text(model, processor, inputs, device, max_new_tokens)


async def extract_text_and_layout(
    *,
    execution_provider: str = "cpu",
    image_path: Path,
    model_id: str,
    models_dir: Path | str = DEFAULT_DIR,
    prompt: str = DEFAULT_PROMPT,
    max_length: int = 1024,
) -> str:
    """Extract text and layout information from images using transformers.

    Args:
        execution_provider: The device to use ("cpu", "cuda", "mps")
        image_path: Path to the image file or list of paths for multiple images
        model_id: The Hugging Face model ID
        models_dir: The directory to store the downloaded model
        prompt: The prompt to guide text extraction
        max_length: Maximum tokens to generate

    Returns:
        Extracted text content from the image(s)
    """
    # Map execution_provider to torch device
    device = "cpu"
    if execution_provider.lower() != "cpu":
        if execution_provider.lower() == "cudaexecutionprovider" and torch.cuda.is_available():
            device = "cuda"
        elif (
            execution_provider.lower() == "mpsexecutionprovider"
            and hasattr(torch.backends, "mps")
            and torch.backends.mps.is_available()
        ):
            device = "mps"

    # Clear memory
    import gc

    gc.collect()
    if torch.cuda.is_available():
        torch.cuda.empty_cache()

    try:
        # Download model (if needed)
        model_dir = await download_hf_model(model_id=model_id, models_dir=models_dir)

        # Use optimal num_crops for single image
        num_crops = 16  # As recommended in the docs

        # Load model and processor
        model, processor = await load_vision_model(model_dir, device, num_crops)

        # Extract text
        return await extract_image_text(
            image_path=image_path,
            model=model,
            processor=processor,
            prompt=prompt,
            device=device,
            max_new_tokens=max_length,
        )
    finally:
        # Clean up
        gc.collect()
        if torch.cuda.is_available():
            torch.cuda.empty_cache()
