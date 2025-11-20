```bash
# Extract a single file
docker run -v $(pwd):/data goldziher/kreuzberg:v4-core \
  extract /data/document.pdf

# Batch process multiple files
docker run -v $(pwd):/data goldziher/kreuzberg:v4-core \
  batch /data/*.pdf --output-format json

# Detect MIME type
docker run -v $(pwd):/data goldziher/kreuzberg:v4-core \
  detect /data/unknown-file.bin
```
