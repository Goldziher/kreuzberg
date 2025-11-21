```go
package main

import (
	"bytes"
	"fmt"
	"io"
	"mime/multipart"
	"net/http"
	"os"
)

func main() {
	file1, err := os.Open("doc1.pdf")
	if err != nil {
		panic(err)
	}
	defer file1.Close()

	file2, err := os.Open("doc2.docx")
	if err != nil {
		panic(err)
	}
	defer file2.Close()

	body := &bytes.Buffer{}
	writer := multipart.NewWriter(body)

	part1, err := writer.CreateFormFile("files", "doc1.pdf")
	if err != nil {
		panic(err)
	}
	if _, err := io.Copy(part1, file1); err != nil {
		panic(err)
	}

	part2, err := writer.CreateFormFile("files", "doc2.docx")
	if err != nil {
		panic(err)
	}
	if _, err := io.Copy(part2, file2); err != nil {
		panic(err)
	}

	writer.Close()

	resp, err := http.Post("http://localhost:8000/extract",
		writer.FormDataContentType(), body)
	if err != nil {
		panic(err)
	}
	defer resp.Body.Close()

	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		panic(err)
	}

	fmt.Println(string(bodyBytes))
}
```
