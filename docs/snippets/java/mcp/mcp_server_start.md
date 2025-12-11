```java
import java.io.IOException;

public class McpServer {
    public static void main(String[] args) {
        try {
            // Start MCP server using CLI
            ProcessBuilder pb = new ProcessBuilder("kreuzberg", "mcp");
            pb.inheritIO();
            Process process = pb.start();
            process.waitFor();
        } catch (IOException | InterruptedException e) {
            System.err.println("Failed to start MCP server: " + e.getMessage());
        }
    }
}
```
