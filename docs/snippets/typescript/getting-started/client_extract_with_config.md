```typescript
// With configuration
const formDataWithConfig = new FormData();
formDataWithConfig.append("files", fileInput.files[0]);
formDataWithConfig.append("config", JSON.stringify({
  ocr: { language: "eng" },
  force_ocr: true
}));

const response2 = await fetch("http://localhost:8000/extract", {
  method: "POST",
  body: formDataWithConfig,
});
```
