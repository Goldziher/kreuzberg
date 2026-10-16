//! Trust-boundary validation for caller-owned REST and MCP extraction config.

use serde_json::Value;

const FORBIDDEN_LLM_FIELDS: &[&str] = &[
    "api_key",
    "base_url",
    "load_env",
    "headers",
    "providers",
    "bedrock",
    "credential_provider",
];

const LLM_LOCATIONS: &[(&str, &str)] = &[
    ("/structured_extraction/llm", "structured_extraction.llm"),
    ("/ocr/vlm_config", "ocr.vlm_config"),
    ("/chunking/embedding/model/llm", "chunking.embedding.model.llm"),
    ("/ner/llm", "ner.llm"),
    ("/summarization/llm", "summarization.llm"),
    ("/translation/llm", "translation.llm"),
    ("/page_classification/llm", "page_classification.llm"),
    ("/chunk_classification/llm", "chunk_classification.llm"),
    ("/captioning/llm", "captioning.llm"),
];

/// Reject transport and credential settings supplied by an untrusted request.
pub(crate) fn validate_caller_extraction_config(config: &Value) -> Result<(), String> {
    #[cfg(feature = "url-config-types")]
    if let Some(crawl) = config.pointer("/url/crawl") {
        crawlberg::reject_untrusted_fields(crawl)
            .map_err(|error| format!("Caller extraction config contains forbidden url.crawl setting: {error}"))?;
    }

    for (pointer, path) in LLM_LOCATIONS {
        if let Some(llm) = config.pointer(pointer) {
            validate_llm_fields(llm, path)?;
        }
    }

    if let Some(stages) = config.pointer("/ocr/pipeline/stages").and_then(Value::as_array) {
        for (index, stage) in stages.iter().enumerate() {
            if let Some(llm) = stage.get("vlm_config") {
                validate_llm_fields(llm, &format!("ocr.pipeline.stages[{index}].vlm_config"))?;
            }
        }
    }

    Ok(())
}

fn validate_llm_fields(llm: &Value, path: &str) -> Result<(), String> {
    for field in FORBIDDEN_LLM_FIELDS {
        if llm.get(field).is_some_and(|value| !value.is_null()) {
            return Err(format!("Caller extraction config may not set {path}.{field}"));
        }
    }
    Ok(())
}

pub(crate) fn adopt_operator_crawl_egress(config: &mut super::ExtractionConfig, operator: &super::ExtractionConfig) {
    #[cfg(feature = "url-config-types")]
    config.url.crawl.adopt_operator_egress(&operator.url.crawl);
    #[cfg(not(feature = "url-config-types"))]
    let _ = (config, operator);
}

pub(crate) fn adopt_file_operator_crawl_egress(
    config: &mut super::FileExtractionConfig,
    operator: &super::ExtractionConfig,
) {
    #[cfg(feature = "url-config-types")]
    if let Some(url) = config.url.as_mut() {
        url.crawl.adopt_operator_egress(&operator.url.crawl);
    }
    #[cfg(not(feature = "url-config-types"))]
    let _ = (config, operator);
}

#[cfg(test)]
mod tests {
    use super::*;

    const LLM_PATHS: &[&[&str]] = &[
        &["structured_extraction", "llm"],
        &["ocr", "vlm_config"],
        &["chunking", "embedding", "model", "llm"],
        &["ner", "llm"],
        &["summarization", "llm"],
        &["translation", "llm"],
        &["page_classification", "llm"],
        &["chunk_classification", "llm"],
        &["captioning", "llm"],
    ];

    const FORBIDDEN_FIELDS: &[&str] = &[
        "api_key",
        "base_url",
        "load_env",
        "headers",
        "providers",
        "bedrock",
        "credential_provider",
    ];

    #[cfg(feature = "url-config-types")]
    const FORBIDDEN_CRAWL_FIELDS: &[&str] = &[
        "/ssrf",
        "/ssrf_deny_private_explicit",
        "/max_redirects",
        "/proxy",
        "/browser/proxy",
        "/browser/endpoint",
        "/browser/chrome_path",
        "/browser/chrome_args",
        "/browser/private_pipe",
        "/browser/eval_script",
        "/browser/session_affinity",
        "/browser_profile",
        "/save_browser_profile",
        "/document_output_dir",
        "/warc_output",
    ];

    fn nested_config(path: &[&str], field: &str, value: Value) -> Value {
        let mut current = serde_json::json!({field: value});
        for segment in path.iter().rev() {
            current = serde_json::json!({(*segment): current});
        }
        current
    }

    #[cfg(feature = "url-config-types")]
    fn value_with_pointer(pointer: &str, leaf: Value) -> Value {
        pointer
            .split('/')
            .filter(|segment| !segment.is_empty())
            .rev()
            .fold(leaf, |value, segment| serde_json::json!({segment: value}))
    }

    #[test]
    fn should_reject_each_transport_field_at_every_llm_path() {
        for path in LLM_PATHS {
            for field in FORBIDDEN_FIELDS {
                let config = nested_config(path, field, serde_json::json!("secret"));
                let error = validate_caller_extraction_config(&config).expect_err("field must be rejected");
                assert_eq!(
                    error,
                    format!("Caller extraction config may not set {}.{field}", path.join("."))
                );
            }
        }
    }

    #[test]
    fn should_reject_transport_fields_in_each_ocr_pipeline_stage() {
        let config = serde_json::json!({
            "ocr": {
                "pipeline": {
                    "stages": [
                        {"vlm_config": {"model": "safe"}},
                        {"vlm_config": {"base_url": "http://internal"}}
                    ]
                }
            }
        });

        let error = validate_caller_extraction_config(&config).expect_err("base_url must be rejected");
        assert_eq!(
            error,
            "Caller extraction config may not set ocr.pipeline.stages[1].vlm_config.base_url"
        );
    }

    #[test]
    fn should_allow_null_transport_fields_and_safe_generation_fields() {
        let config = serde_json::json!({
            "structured_extraction": {
                "llm": {
                    "api_key": null,
                    "base_url": null,
                    "load_env": null,
                    "headers": null,
                    "providers": null,
                    "bedrock": null,
                    "credential_provider": null,
                    "model": "openai/gpt-4o-mini",
                    "temperature": 0.2,
                    "max_tokens": 100
                }
            }
        });

        assert_eq!(validate_caller_extraction_config(&config), Ok(()));
    }

    #[test]
    fn should_ignore_forbidden_lookalikes_in_opaque_values() {
        let config = serde_json::json!({
            "ocr": {
                "backend_options": {"api_key": "backend-owned"},
                "pipeline": {
                    "stages": [{"backend": "custom", "backend_options": {"providers": []}}]
                }
            },
            "structured_extraction": {
                "llm": {
                    "model": "openai/gpt-4o-mini",
                    "extra_body": {"base_url": "provider-specific", "headers": {"x": "y"}}
                },
                "schema": {
                    "type": "object",
                    "properties": {"credential_provider": {"type": "string"}}
                }
            }
        });

        assert_eq!(validate_caller_extraction_config(&config), Ok(()));
    }

    #[test]
    #[cfg(feature = "url-config-types")]
    fn should_reject_each_operator_owned_crawl_field_without_leaking_its_value() {
        assert_eq!(crawlberg::UNTRUSTED_CALLER_FORBIDDEN_FIELDS, FORBIDDEN_CRAWL_FIELDS);

        for pointer in FORBIDDEN_CRAWL_FIELDS {
            let secret = format!("secret-for-{pointer}");
            let config = serde_json::json!({
                "url": {"crawl": value_with_pointer(pointer, serde_json::json!(secret))}
            });

            let error =
                validate_caller_extraction_config(&config).expect_err("operator-owned crawl field must be rejected");

            assert_eq!(
                error,
                format!(
                    "Caller extraction config contains forbidden url.crawl setting: \
                     invalid_config: untrusted caller may not set {pointer}"
                )
            );
            assert!(!error.contains(&secret), "rejection must not expose the field value");
        }
    }

    #[test]
    #[cfg(feature = "url-config-types")]
    fn should_allow_null_operator_owned_crawl_fields_and_caller_owned_fields() {
        for pointer in FORBIDDEN_CRAWL_FIELDS {
            let config = serde_json::json!({
                "url": {"crawl": value_with_pointer(pointer, Value::Null)}
            });
            assert_eq!(validate_caller_extraction_config(&config), Ok(()));
        }

        let config = serde_json::json!({
            "url": {
                "crawl": {
                    "max_depth": 2,
                    "auth": {"type": "bearer", "token": "caller-target-token"},
                    "custom_headers": {"Authorization": "caller-target-token"}
                }
            }
        });
        assert_eq!(validate_caller_extraction_config(&config), Ok(()));
    }
}
