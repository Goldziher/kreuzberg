import asyncio
import os
from typing import List

from pydantic import BaseModel, Field
from openai import AsyncOpenAI

from kreuzberg import extract_structured_async

# 1. Define your desired output structure using Pydantic
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

# 2. Create an async extractor function that calls your preferred LLM API
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

# 3. Use extract_structured_async to orchestrate the extraction
async def main():
    # For this example, we'll create a dummy invoice image.
    # In a real application, you would load your own image file.
    try:
        from PIL import Image, ImageDraw, ImageFont

        img = Image.new('RGB', (800, 1000), color = 'white')
        d = ImageDraw.Draw(img)
        fnt = ImageFont.load_default()

        d.text((50, 50), "INVOICE", font=fnt, fill=(0,0,0))
        d.text((50, 100), "Invoice ID: INV-123", font=fnt, fill=(0,0,0))
        d.text((50, 120), "Vendor: ACME Corp.", font=fnt, fill=(0,0,0))
        d.text((50, 140), "Customer: John Doe", font=fnt, fill=(0,0,0))

        d.text((50, 200), "Description", font=fnt, fill=(0,0,0))
        d.text((300, 200), "Qty", font=fnt, fill=(0,0,0))
        d.text((400, 200), "Unit Price", font=fnt, fill=(0,0,0))
        d.text((500, 200), "Total", font=fnt, fill=(0,0,0))

        d.text((50, 220), "Product A", font=fnt, fill=(0,0,0))
        d.text((300, 220), "2", font=fnt, fill=(0,0,0))
        d.text((400, 220), "10.00", font=fnt, fill=(0,0,0))
        d.text((500, 220), "20.00", font=fnt, fill=(0,0,0))

        d.text((50, 240), "Product B", font=fnt, fill=(0,0,0))
        d.text((300, 240), "3", font=fnt, fill=(0,0,0))
        d.text((400, 240), "15.00", font=fnt, fill=(0,0,0))
        d.text((500, 240), "45.00", font=fnt, fill=(0,0,0))

        img.save("invoice.png")

    except ImportError:
        print("Pillow not installed, skipping dummy image creation.")
        print("Please create a file named invoice.png for this example to work.")
        return

    with open("invoice.png", "rb") as f:
        image_bytes = f.read()

    prompt = "Extract the invoice data from the image. Respond with JSON that conforms to the provided schema."

    try:
        invoice: Invoice = await extract_structured_async(
            images=[image_bytes],
            prompt=prompt,
            extractor=openai_extractor,
            output_type=Invoice,
        )

        print("--- Successfully Extracted Invoice Data ---")
        print(f"Invoice ID: {invoice.invoice_id}")
        print(f"Vendor: {invoice.vendor_name}")
        print(f"Customer: {invoice.customer_name}")
        print("Line Items:")
        for item in invoice.line_items:
            print(f"  - {item.description}: {item.quantity} x ${item.unit_price:.2f} = ${item.total:.2f}")
        print("-----------------------------------------")

    except Exception as e:
        print(f"An error occurred during extraction: {e}")

if __name__ == "__main__":
    if not os.environ.get("OPENAI_API_KEY"):
        print("Please set the OPENAI_API_KEY environment variable to run this example.")
    else:
        asyncio.run(main())