# Structured Extraction with Vision Models

Kreuzberg v4 introduces a powerful, model-agnostic structured extraction feature that lets you extract typed data from documents using any vision-capable Large Language Model (LLM). Instead of depending on a specific library like `litellm`, Kreuzberg gives you full control over the extraction process by allowing you to provide your own asynchronous Python callable.

This design is flexible, performant, and deeply integrated with Python's typing ecosystem.

## Core Concepts

The structured extraction API is built around a few key components:

1.  **User-Provided Extractor**: You provide an `async` Python function that takes `prompt` and `images` as input and returns a JSON string or bytes. This function is responsible for making the actual call to your chosen LLM (OpenAI, Anthropic, Ollama, etc.).
2.  **Rust Orchestrator**: A high-performance Rust core orchestrates the extraction process. It calls your Python extractor, validates the output against a JSON schema, and performs retries with structured error feedback if validation fails.
3.  **JSON Schema Validation**: The Rust core uses the `jsonschema` crate to validate the LLM's output. This ensures that the extracted data conforms to the expected structure before it's returned to your Python code.
4.  **Automatic Schema Derivation**: You can provide a `pydantic` model or `msgspec` struct as the `output_type`, and Kreuzberg will automatically derive the JSON schema for you.
5.  **Typed Deserialization**: When an `output_type` is provided, Kreuzberg automatically decodes the validated JSON into an instance of your `pydantic` model or `msgspec` struct.

## Python API

The primary entry points are `extract_structured_async` and its synchronous counterpart, `extract_structured`.

### `extract_structured_async`

```python
async def extract_structured_async(
    images: list[bytes],
    prompt: str,
    extractor: Callable[[str, list[bytes]], Awaitable[str | bytes]],
    *,
    schema_json: str | None = None,
    output_type: Any | None = None,
    max_retries: int = 2,
    include_error_in_retry: bool = True,
) -> Any:
```

-   `images`: A list of byte arrays, where each byte array is a PNG, JPEG, or other image file.
-   `prompt`: The prompt to send to the vision model.
-   `extractor`: An `async` callable that receives the prompt and images and returns the model's raw JSON output.
-   `schema_json`: An optional JSON schema to validate the output against.
-   `output_type`: An optional type (e.g., a `pydantic` model or `msgspec` struct) to derive the schema from and decode the result into.
-   `max_retries`: The number of times to retry if validation fails.
-   `include_error_in_retry`: If `True`, the validation error is appended to the prompt on retries to help the model correct its output.

### `extract_structured`

A synchronous wrapper around the `async` version, suitable for use in non-async code.

## Usage Example

Here is a complete example demonstrating how to extract invoice data using OpenAI's GPT-4 Vision model and a `pydantic` model.

First, define your desired output structure using `pydantic`:

```python
from pydantic import BaseModel, Field
from typing import List

class LineItem(BaseModel):
    description: str = Field(..., description="Description of the line item.")
    quantity: int = Field(..., description="Quantity of the line item.")
    unit_price: float = Field(..., description="Unit price of the line item.")
    total: float = Field(..., description="Total price of the line item.")

class Invoice(BaseModel):
    invoice_id: str = Field(..., description="The invoice ID.")
    vendor_name: str = Field(..., description="The name of the vendor.")
    customer_name: str = Field(..., description="The name of the customer.")
    line_items: List[LineItem] = Field(..., description="A list of line items on the invoice.")
```

Next, create an `async` extractor function that calls the OpenAI API:

```python
import os
from openai import AsyncOpenAI

async def openai_extractor(prompt: str, images: list[bytes]) -> str:
    """
    An extractor that uses OpenAI's API to extract structured data from images.
    """
    client = AsyncOpenAI(api_key=os.environ.get("OPENAI_API_KEY"))

    messages = [
        {
            "role": "user",
            "content": [
                {"type": "text", "text": prompt},
            ]
            + [
                {
                    "type": "image_url",
                    "image_url": {
                        "url": f"data:image/png;base64,{image_data.hex()}",
                    },
                }
                for image_data in images
            ],
        }
    ]

    response = await client.chat.completions.create(
        model="gpt-4-vision-preview",
        messages=messages,
        max_tokens=4096,
    )
    return response.choices[0].message.content
```

Finally, use `extract_structured_async` to orchestrate the extraction:

```python
import asyncio
from kreuzberg import extract_structured_async

async def main():
    with open("invoice.png", "rb") as f:
        image_bytes = f.read()

    prompt = "Extract the invoice data from the image. Respond with JSON that conforms to the provided schema."

    invoice: Invoice = await extract_structured_async(
        images=[image_bytes],
        prompt=prompt,
        extractor=openai_extractor,
        output_type=Invoice,
    )

    print(f"Vendor: {invoice.vendor_name}")
    for item in invoice.line_items:
        print(f"- {item.description}: {item.quantity} x {item.unit_price} = {item.total}")

if __name__ == "__main__":
    asyncio.run(main())
```

## Error Handling

-   `ValidationError`: Raised if `schema_json` is invalid or if a schema cannot be derived from `output_type`.
-   `ExtractionValidationError`: Raised if the model's output fails validation after all retries. The error context contains `attempts` and a list of `errors`.