```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.Validator;
import dev.kreuzberg.ValidationException;
import dev.kreuzberg.KreuzbergException;
import java.io.IOException;

public class MinLengthValidatorExample {
    public static void main(String[] args) {
        int minLength = 100;

        Validator minLengthValidator = result -> {
            if (result.getContent().length() < minLength) {
                throw new ValidationException(
                    "Content too short: " + result.getContent().length() +
                    " < " + minLength
                );
            }
        };

        try {
            Kreuzberg.registerValidator("min-length", minLengthValidator, 100);

            ExtractionResult result = Kreuzberg.extractFile("document.pdf");
            System.out.println("Validation passed!");
        } catch (ValidationException e) {
            System.err.println("Validation failed: " + e.getMessage());
        } catch (IOException | KreuzbergException e) {
            e.printStackTrace();
        }
    }
}
```
