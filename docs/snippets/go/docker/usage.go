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
	"os/exec"
	"path/filepath"
	"time"
)

type DockerKreuzbergClient struct {
	containerName string
	containerImage string
	apiPort       int
}

func NewDockerKreuzbergClient(containerName, image string, port int) *DockerKreuzbergClient {
	return &DockerKreuzbergClient{
		containerName: containerName,
		containerImage: image,
		apiPort:       port,
	}
}

func (c *DockerKreuzbergClient) StartContainer() error {
	fmt.Println("Starting Kreuzberg Docker container...")
	cmd := exec.Command("docker", "run", "-d",
		"--name", c.containerName,
		"-p", fmt.Sprintf("%d:8000", c.apiPort),
		c.containerImage)

	if err := cmd.Run(); err != nil {
		return fmt.Errorf("failed to start container: %w", err)
	}

	fmt.Printf("Container started on http://localhost:%d\n", c.apiPort)
	return nil
}

func (c *DockerKreuzbergClient) ExtractFile(filePath string) (string, error) {
	fileBytes, err := os.ReadFile(filePath)
	if err != nil {
		return "", err
	}

	var buf bytes.Buffer
	writer := multipart.NewWriter(&buf)

	part, err := writer.CreateFormFile("file", filepath.Base(filePath))
	if err != nil {
		return "", err
	}

	if _, err := io.Copy(part, bytes.NewReader(fileBytes)); err != nil {
		return "", err
	}

	if err := writer.Close(); err != nil {
		return "", err
	}

	resp, err := http.Post(
		fmt.Sprintf("http://localhost:%d/api/extract", c.apiPort),
		writer.FormDataContentType(),
		&buf,
	)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	var result map[string]string
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return "", err
	}

	return result["content"], nil
}

func (c *DockerKreuzbergClient) StopContainer() error {
	fmt.Println("Stopping Kreuzberg Docker container...")
	if err := exec.Command("docker", "stop", c.containerName).Run(); err != nil {
		return err
	}
	if err := exec.Command("docker", "rm", c.containerName).Run(); err != nil {
		return err
	}
	fmt.Println("Container stopped and removed")
	return nil
}

func main() {
	client := NewDockerKreuzbergClient("kreuzberg-api", "kreuzberg:latest", 8000)

	if err := client.StartContainer(); err != nil {
		panic(err)
	}

	time.Sleep(2 * time.Second)

	content, err := client.ExtractFile("document.pdf")
	if err != nil {
		panic(err)
	}

	fmt.Printf("Extracted content:\n%s\n", content)

	if err := client.StopContainer(); err != nil {
		panic(err)
	}
}
```
