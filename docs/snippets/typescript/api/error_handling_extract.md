```typescript title="TypeScript"
try {
    const response = await fetch("http://localhost:8000/extract", {
        method: "POST",
        body: formData,
    });

    if (!response.ok) {
        const error = await response.json();
        console.error(`Error: ${error.error_type}: ${error.message}`);
    } else {
        const results = await response.json();
        console.log(results);
    }
} catch (e) {
    console.error("Request failed:", e);
}
```
