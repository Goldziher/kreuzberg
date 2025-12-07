```bash
# Start API server (default mode)
docker run -p 8000:8000 goldziher/kreuzberg:latest

# Test the API
curl -F "files=@document.pdf" http://localhost:8000/extract
```
