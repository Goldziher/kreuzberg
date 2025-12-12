```typescript title="TypeScript"
// Multiple files
const multipleFiles = new FormData();
for (const file of fileInput.files) {
  multipleFiles.append("files", file);
}

const response3 = await fetch("http://localhost:8000/extract", {
  method: "POST",
  body: multipleFiles,
});
```
