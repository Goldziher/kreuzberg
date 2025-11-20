```java
import java.io.IOException;

public class ApiServer {
    public static void main(String[] args) {
        try {
            ProcessBuilder pb = new ProcessBuilder(
                "kreuzberg", "serve", "-H", "0.0.0.0", "-p", "8000"
            );
            pb.inheritIO();
            Process process = pb.start();
            process.waitFor();
        } catch (IOException | InterruptedException e) {
            System.err.println("Failed to start server: " + e.getMessage());
        }
    }
}

```
