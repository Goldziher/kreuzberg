```go
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"mime/multipart"
	"net/http"
	"os"
)

func main() {
	file, err := os.Open("scanned.pdf")
	if err != nil {
		panic(err)
	}
	defer file.Close()

	body := &bytes.Buffer{}
	writer := multipart.NewWriter(body)
	part, err := writer.CreateFormFile("files", "scanned.pdf")
	if err != nil {
		panic(err)
	}

	if _, err := io.Copy(part, file); err != nil {
		panic(err)
	}

	config := map[string]interface{}{
		"ocr": map[string]string{
			"language": "eng",
		},
		"force_ocr": true,
	}
	configJSON, _ := json.Marshal(config)
	writer.WriteField("config", string(configJSON))

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
