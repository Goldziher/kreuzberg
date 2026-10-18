//! Smoke test for the `xberg-gliner` ONNX NER backend.
//!
//! The default `xberg-io/gliner-models` alias downloads ONNX weights and a
//! tokenizer on first run, which makes this test slow and network-dependent.
//! It is marked `#[ignore]` so it does not run in CI by default.

#![cfg(feature = "ner-onnx")]

use xberg::text::ner::{default_model_name, download_model};

#[test]
fn shared_backend_loader_is_public() {
    let _loader = xberg::text::ner::gline::get_or_init_backend;
}

#[tokio::test]
#[ignore = "downloads ~100MB GLiNER model from HuggingFace"]
async fn concurrent_shared_backend_loads_return_the_same_arc() {
    let (first, second) = tokio::join!(
        xberg::text::ner::gline::get_or_init_backend(None),
        xberg::text::ner::gline::get_or_init_backend(None),
    );
    let first = first.expect("first backend load succeeds");
    let second = second.expect("second backend load succeeds");

    assert!(std::sync::Arc::ptr_eq(&first, &second));
}

#[test]
#[ignore = "downloads ~100MB GLiNER model from HuggingFace"]
fn download_default_model_succeeds() {
    let model_path = download_model(default_model_name(), None).expect("model download succeeds");

    let metadata = std::fs::metadata(&model_path).expect("model file exists");
    assert!(metadata.len() > 0, "model file should not be empty");
}

#[tokio::test]
#[ignore = "downloads ~100MB GLiNER model from HuggingFace; runs inference"]
async fn detects_person_org_location_in_canonical_sentence() {
    use xberg::text::ner::NerBackend;
    use xberg::text::ner::gline::GlineBackend;
    use xberg::types::entity::EntityCategory;

    let backend = GlineBackend::new(None).expect("backend construction");
    let text = "Cristiano Ronaldo plays for Al Nassr in Saudi Arabia.";
    let categories = vec![
        EntityCategory::Person,
        EntityCategory::Organization,
        EntityCategory::Location,
    ];

    let entities = backend
        .detect(text, &categories)
        .await
        .expect("entity detection succeeds");

    assert!(entities.iter().any(|entity| entity.category == EntityCategory::Person));
    assert!(
        entities
            .iter()
            .any(|entity| entity.category == EntityCategory::Organization)
    );
    assert!(
        entities
            .iter()
            .any(|entity| entity.category == EntityCategory::Location)
    );
}

/// Verifies that `NerConfig::custom_labels` participates in backend dispatch.
///
/// The backend must accept custom labels and route them through the GLiNER
/// zero-shot input path.
#[tokio::test]
async fn custom_labels_route_through_backend() {
    use xberg::core::config::ner::NerConfig;
    use xberg::text::ner::NerBackend;
    use xberg::text::ner::gline::GlineBackend;
    use xberg::types::entity::EntityCategory;

    let cfg = NerConfig {
        categories: vec![EntityCategory::Person, EntityCategory::Organization],
        custom_labels: vec!["Product".to_string(), "Treatment".to_string()],
        ..NerConfig::default()
    };

    let backend = match GlineBackend::new(None) {
        Ok(b) => b,
        Err(xberg::XbergError::MissingDependency(_)) | Err(xberg::XbergError::Plugin { .. }) => {
            return;
        }
        Err(other) => panic!("unexpected backend construction error: {other:?}"),
    };

    let text = "Aspirin treats headaches and is manufactured by Bayer.";
    let result = backend
        .detect_with_custom(text, &cfg.categories, &cfg.custom_labels)
        .await;
    match result {
        Ok(_entities) => {}
        Err(other) => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
#[ignore = "downloads ~100MB GLiNER model from HuggingFace; runs inference"]
async fn invoice_identifier_is_not_reported_as_an_email() {
    use xberg::text::ner::NerBackend;
    use xberg::text::ner::gline::GlineBackend;
    use xberg::types::entity::EntityCategory;

    let backend = GlineBackend::new(None).expect("backend construction");
    let text = "Marie Curie visited Paris. Photosynthesis converts sunlight into chemical energy. Photosynthesis feeds plants.Invoice number SYN-004. Total amount due is USD 125.50. Please pay this invoice.";

    let entities = backend.detect(text, &[]).await.expect("entity detection succeeds");

    let emails: Vec<_> = entities
        .iter()
        .filter(|entity| entity.category == EntityCategory::Email)
        .collect();
    assert!(emails.is_empty(), "no address in the text, got {emails:?}");
    assert!(entities.iter().any(|entity| entity.category == EntityCategory::Person));
    assert!(
        entities
            .iter()
            .any(|entity| entity.category == EntityCategory::Location)
    );
}
