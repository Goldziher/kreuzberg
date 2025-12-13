```rust title="Rust"
use kreuzberg::plugins::registry::*;

let registry = get_document_extractor_registry();
let extractors = registry.list()?;
println!("Registered extractors: {:?}", extractors);

let registry = get_post_processor_registry();
let processors = registry.list()?;
println!("Registered processors: {:?}", processors);

let registry = get_ocr_backend_registry();
let backends = registry.list()?;
println!("Registered OCR backends: {:?}", backends);

let registry = get_validator_registry();
let validators = registry.list()?;
println!("Registered validators: {:?}", validators);
```
