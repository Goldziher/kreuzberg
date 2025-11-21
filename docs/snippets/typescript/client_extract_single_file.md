```typescript
// Using fetch API
const formData = new FormData();
formData.append("files", fileInput.files[0]);

const response = await fetch("http://localhost:8000/extract", {
  method: "POST",
  body: formData,
});

const results = await response.json();
console.log(results[0].content);
```
