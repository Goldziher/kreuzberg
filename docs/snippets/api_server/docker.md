```bash
# Run server on port 8000
docker run -d \n  -p 8000:8000 \n  goldziher/kreuzberg:latest \n  serve -H 0.0.0.0 -p 8000

# With environment variables
docker run -d \n  -e KREUZBERG_CORS_ORIGINS="https://myapp.com" \n  -e KREUZBERG_MAX_UPLOAD_SIZE_MB=200 \n  -p 8000:8000 \n  goldziher/kreuzberg:latest \n  serve -H 0.0.0.0 -p 8000

```
