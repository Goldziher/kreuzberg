```java
import dev.kreuzberg.*;
import java.lang.foreign.Arena;

public class MinLengthValidatorExample {
    public static void main(String[] args) {
        int minLength = 100;

        try (Arena arena = Arena.ofConfined()) {
            // Define validator
            Validator minLengthValidator = result -> {
                if (result.content().length() < minLength) {
                    throw new ValidationException(
                        "Content too short: " + result.content().length() +
                        " < " + minLength
                    );
                }
            };

            // Register with priority 100 (run early - fast check)
            Kreuzberg.registerValidator("min-length", minLengthValidator, 100, arena);

            // Use in extraction - will throw ValidationException if content too short
            try {
                ExtractionResult result = Kreuzberg.extractFileSync("document.pdf");
                System.out.println("Validation passed!");
            } catch (ValidationException e) {
                System.err.println("Validation failed: " + e.getMessage());
            }
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}
```
