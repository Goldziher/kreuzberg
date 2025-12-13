from kreuzberg import extract_file_sync, ExtractionConfig, PageConfig

config = ExtractionConfig(
    pages=PageConfig(extract_pages=True)
)

result = extract_file_sync("document.pdf", config=config)

if result.pages:
    for page in result.pages:
        print(f"Page {page.page_number}:")
        print(f"  Content: {len(page.content)} chars")
        print(f"  Tables: {len(page.tables)}")
        print(f"  Images: {len(page.images)}")
