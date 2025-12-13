```java title="Java"
Validator qualityValidator = result -> {
    double score = result.getMetadata().containsKey("quality_score")
        ? ((Number) result.getMetadata().get("quality_score")).doubleValue()
        : 0.0;

    if (score < 0.5) {
        throw new ValidationException(
            String.format("Quality score too low: %.2f < 0.50", score)
        );
    }
};
```
