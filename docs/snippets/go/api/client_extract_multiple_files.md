```go title="Go"
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
	file1, err := os.Open("doc1.pdf")
	if err != nil {
		log.Fatalf("open doc1.pdf: %v", err)
	}
	defer file1.Close()

	file2, err := os.Open("doc2.docx")
	if err != nil {
		log.Fatalf("open doc2.docx: %v", err)
	}
	defer file2.Close()

	body := &bytes.Buffer{}
	writer := multipart.NewWriter(body)

	part1, err := writer.CreateFormFile("files", "doc1.pdf")
	if err != nil {
		log.Fatalf("create form file 1: %v", err)
	}
	if _, err := io.Copy(part1, file1); err != nil {
		log.Fatalf("copy file 1: %v", err)
	}

	part2, err := writer.CreateFormFile("files", "doc2.docx")
	if err != nil {
		log.Fatalf("create form file 2: %v", err)
	}
	if _, err := io.Copy(part2, file2); err != nil {
		log.Fatalf("copy file 2: %v", err)
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
