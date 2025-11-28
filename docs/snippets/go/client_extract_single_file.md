```go
package main

import (
	"bytes"
	"fmt"
	"io"
	"log"
	"mime/multipart"
	"net/http"
	"os"
)

func main() {
	file, err := os.Open("document.pdf")
	if err != nil {
		log.Fatalf("open file: %v", err)
	}
	defer file.Close()

	body := &bytes.Buffer{}
	writer := multipart.NewWriter(body)
	part, err := writer.CreateFormFile("files", "document.pdf")
	if err != nil {
		log.Fatalf("create form file: %v", err)
	}

	if _, err := io.Copy(part, file); err != nil {
		log.Fatalf("copy file: %v", err)
	}
	writer.Close()

	resp, err := http.Post("http://localhost:8000/extract",
		writer.FormDataContentType(), body)
	if err != nil {
		log.Fatalf("http post: %v", err)
	}
	defer resp.Body.Close()

	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		log.Fatalf("read response: %v", err)
	}

	fmt.Println(string(bodyBytes))
}
```
