package dev.kreuzberg;

/**
 * Interface for validating extraction results.
 *
 * <p>Validators check extraction results for quality, completeness, or
 * other criteria. If validation fails, they throw a {@link ValidationException}.</p>
 *
 * <h2>Usage</h2>
 * <pre>{@code
 * Validator minLengthValidator = result -> {
 *     if (result.content().length() < 10) {
 *         throw new ValidationException("Content too short: " + result.content().length());
 *     }
 * };
 *
 * ExtractionResult result = Kreuzberg.extractFile("document.pdf");
 * minLengthValidator.validate(result);
 * }</pre>
 *
 * <h2>Combining Validators</h2>
 * <pre>{@code
 * Validator combined = validator1.andThen(validator2);
 * combined.validate(result); // Runs both validators
 * }</pre>
 *
 * @since 4.0.0
 */
@FunctionalInterface
public interface Validator {
    /**
     * Validate an extraction result.
     *
     * <p>If validation passes, this method returns normally.
     * If validation fails, it throws a {@link ValidationException}
     * with a descriptive message.</p>
     *
     * @param result the extraction result to validate
     * @throws ValidationException if validation fails
     */
    void validate(ExtractionResult result) throws ValidationException;

    /**
     * Returns a composed validator that first runs this validator,
     * then runs the {@code after} validator.
     *
     * @param after the validator to run after this one
     * @return a composed validator
     * @throws NullPointerException if after is null
     */
    default Validator andThen(Validator after) {
        if (after == null) {
            throw new NullPointerException("after validator must not be null");
        }
        return result -> {
            this.validate(result);
            after.validate(result);
        };
    }
}
