//! Structured data extraction using LLM-based schema-guided generation.
//!
//! Uses liter-llm's JSON schema response format to extract structured data
//! from document content. The LLM is constrained to produce output conforming
//! to the caller's JSON schema.

use crate::core::config::LlmConfig;
use crate::core::config::llm::StructuredExtractionConfig;
use crate::types::LlmUsage;
use serde_json::Value;

/// Send a free-form prompt to the configured LLM with a JSON-schema response
/// constraint and return the parsed JSON value plus captured usage.
///
/// This is the shared helper used by LLM-backed post-processors (page
/// classification, LLM-driven NER, etc.) that need structured output but do not
/// want to depend on [`StructuredExtractionConfig`]'s schema/prompt machinery.
///
/// # Arguments
///
/// * `llm_config` — provider/model configuration.
/// * `prompt` — fully-rendered user prompt (no Jinja substitution performed).
/// * `schema_name` — name for the JSON schema (passed to providers that
///   distinguish multiple structured outputs).
/// * `schema` — the JSON schema the LLM is required to obey.
/// * `source` — label used for the returned [`LlmUsage`] entry.
///
/// # Errors
///
/// Returns an error if the LLM client cannot be constructed, the request fails,
/// the response contains no content, or the response is not parseable JSON.
/// Usage from a decoded response is available through [`super::usage::usage_from_error`].
#[allow(clippy::field_reassign_with_default)]
#[cfg_attr(alef, alef(skip))]
pub async fn complete_with_json_schema(
    llm_config: &LlmConfig,
    prompt: &str,
    schema_name: &str,
    schema: &Value,
    source: &str,
) -> crate::Result<(Value, Option<LlmUsage>)> {
    use liter_llm::LlmClient;

    let client = super::client::create_client(llm_config)?;

    let sanitized_schema = sanitize_schema_for_provider(schema, &llm_config.model);

    let mut request = liter_llm::ChatCompletionRequest::default();
    request.model = llm_config.model.clone();
    request.messages = vec![liter_llm::Message::User(liter_llm::UserMessage {
        content: liter_llm::UserContent::Text(prompt.to_string()),
        name: None,
    })];
    super::client::apply_request_time_params(&mut request, llm_config)?;
    request.response_format = Some(liter_llm::ResponseFormat::JsonSchema {
        json_schema: liter_llm::JsonSchemaFormat {
            name: schema_name.to_string(),
            description: None,
            schema: sanitized_schema,
            strict: Some(false),
        },
    });

    let response = client
        .chat(request)
        .await
        .map_err(|e| crate::XbergError::parsing(format!("LLM JSON-schema request failed ({source}): {e}")))?;

    let usage = super::usage::extract_usage_from_chat(&response, source);

    let text = response
        .choices
        .first()
        .and_then(|c| c.message.content.as_ref().and_then(|m| m.as_text()))
        .ok_or_else(|| {
            super::usage::with_usage(
                crate::XbergError::parsing(format!(
                    "LLM JSON-schema response ({source}) returned no content (model={}, {} choices)",
                    llm_config.model,
                    response.choices.len()
                )),
                usage.clone(),
            )
        })?;

    let cleaned = strip_markdown_fence(&text);
    let value = serde_json::from_str(cleaned).map_err(|e| {
        super::usage::with_usage(
            crate::XbergError::parsing(format!(
                "LLM JSON-schema response ({source}) returned invalid JSON (model={}) at line {}, column {}",
                llm_config.model,
                e.line(),
                e.column()
            )),
            usage.clone(),
        )
    })?;

    Ok((value, usage))
}

/// Strip ```json ... ``` markdown code fences some providers wrap JSON in.
fn strip_markdown_fence(text: &str) -> &str {
    let trimmed = text.trim();
    let stripped = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|s| s.strip_suffix("```"))
        .map(str::trim);
    stripped.unwrap_or(trimmed)
}

/// Strip JSON Schema fields that some providers don't support.
///
/// ~keep `additionalProperties` is rejected by Gemini but required by native JSON
/// schema output in OpenAI and Anthropic. Strip it only for Google providers.
fn sanitize_schema_for_provider(schema: &Value, model: &str) -> Value {
    let needs_strip =
        model.starts_with("gemini/") || model.starts_with("google_ai/") || model.starts_with("vertex_ai/");

    if needs_strip {
        strip_additional_properties(schema)
    } else {
        schema.clone()
    }
}

fn strip_additional_properties(schema: &Value) -> Value {
    match schema {
        Value::Object(map) => {
            let mut cleaned = serde_json::Map::new();
            for (key, value) in map {
                if key == "additionalProperties" {
                    continue;
                }
                cleaned.insert(key.clone(), strip_additional_properties(value));
            }
            Value::Object(cleaned)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(strip_additional_properties).collect()),
        other => other.clone(),
    }
}

/// Extract structured data from document content using an LLM with JSON schema.
///
/// Sends the document content to the configured LLM with a JSON schema constraint,
/// validating the returned data locally when strict mode is enabled.
///
/// # Arguments
///
/// * `content` - The extracted document text to send to the LLM.
/// * `config` - Structured extraction configuration including schema and LLM settings.
///
/// # Returns
///
/// A `serde_json::Value`, validated against the original schema in strict mode.
///
/// # Errors
///
/// Returns an error if:
/// - The LLM client cannot be created (invalid provider/credentials).
/// - The LLM request fails (network, rate-limit, etc.).
/// - The LLM response cannot be parsed as valid JSON.
/// - In strict mode, the schema cannot be compiled offline or the output fails validation.
#[allow(clippy::field_reassign_with_default)]
pub(crate) async fn extract_structured(
    content: &str,
    config: &StructuredExtractionConfig,
) -> crate::Result<(serde_json::Value, Option<crate::types::LlmUsage>)> {
    use liter_llm::LlmClient;

    let validator = compile_strict_schema(config).await?;

    let client = super::client::create_client(&config.llm)?;

    let template = config
        .prompt
        .as_deref()
        .unwrap_or(super::prompts::STRUCTURED_EXTRACTION_TEMPLATE);

    let schema_json = serde_json::to_string_pretty(&config.schema)
        .map_err(|e| crate::XbergError::validation(format!("Failed to serialize schema for prompt: {e}")))?;

    let ctx = minijinja::context! {
        content => content,
        schema => schema_json,
        schema_name => &config.schema_name,
        schema_description => config.schema_description.as_deref().unwrap_or(""),
    };

    let prompt = super::prompts::render_template(template, &ctx)?;

    let sanitized_schema = sanitize_schema_for_provider(&config.schema, &config.llm.model);

    let mut request = liter_llm::ChatCompletionRequest::default();
    request.model = config.llm.model.clone();
    request.messages = vec![liter_llm::Message::User(liter_llm::UserMessage {
        content: liter_llm::UserContent::Text(prompt),
        name: None,
    })];
    super::client::apply_request_time_params(&mut request, &config.llm)?;
    request.response_format = Some(liter_llm::ResponseFormat::JsonSchema {
        json_schema: liter_llm::JsonSchemaFormat {
            name: config.schema_name.clone(),
            description: config.schema_description.clone(),
            schema: sanitized_schema,
            strict: Some(config.strict),
        },
    });

    let response = client
        .chat(request)
        .await
        .map_err(|e| crate::XbergError::parsing(format!("LLM structured extraction request failed: {e}")))?;

    let usage = super::usage::extract_usage_from_chat(&response, "structured_extraction");

    let text = response
        .choices
        .first()
        .and_then(|c| c.message.content.as_ref().and_then(|m| m.as_text()))
        .ok_or_else(|| {
            super::usage::with_usage(
                crate::XbergError::parsing(format!(
                    "LLM structured extraction returned no content (model={}, {} choices)",
                    config.llm.model,
                    response.choices.len()
                )),
                usage.clone(),
            )
        })?;

    let cleaned = strip_markdown_fence(&text);

    let value = serde_json::from_str(cleaned).map_err(|e| {
        super::usage::with_usage(
            crate::XbergError::parsing(format!(
                "LLM structured extraction returned invalid JSON at line {}, column {}",
                e.line(),
                e.column()
            )),
            usage.clone(),
        )
    })?;

    let value = validate_structured_output(value, validator)
        .await
        .map_err(|error| super::usage::with_usage(error, usage.clone()))?;

    Ok((value, usage))
}

async fn compile_strict_schema(config: &StructuredExtractionConfig) -> crate::Result<Option<jsonschema::Validator>> {
    if !config.strict {
        return Ok(None);
    }
    let schema = config.schema.clone();
    tokio::task::spawn_blocking(move || {
        jsonschema::options()
            .offline()
            .build(&schema)
            .map(Some)
            .map_err(|_| crate::XbergError::validation("invalid structured extraction schema".to_owned()))
    })
    .await
    .map_err(|_| crate::XbergError::parsing("Structured extraction schema compilation task failed".to_owned()))?
}

async fn validate_structured_output(value: Value, validator: Option<jsonschema::Validator>) -> crate::Result<Value> {
    let Some(validator) = validator else {
        return Ok(value);
    };
    tokio::task::spawn_blocking(move || {
        if validator.is_valid(&value) {
            Ok(value)
        } else {
            Err(crate::XbergError::parsing(
                "LLM structured extraction schema validation failed".to_owned(),
            ))
        }
    })
    .await
    .map_err(|_| crate::XbergError::parsing("Structured extraction schema validation task failed".to_owned()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_sanitize_schema_strips_for_google_providers() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "nested": {
                    "type": "object",
                    "properties": { "x": { "type": "integer" } },
                    "additionalProperties": false
                }
            },
            "required": ["name"],
            "additionalProperties": false
        });

        for model in [
            "gemini/gemini-2.5-flash",
            "google_ai/gemini-2.5-flash",
            "vertex_ai/gemini-2.5-flash",
        ] {
            let sanitized = sanitize_schema_for_provider(&schema, model);
            assert!(sanitized.get("additionalProperties").is_none(), "model={model}");
            assert!(
                sanitized["properties"]["nested"].get("additionalProperties").is_none(),
                "model={model}"
            );
            assert_eq!(sanitized["type"], "object", "model={model}");
            assert_eq!(sanitized["required"], json!(["name"]), "model={model}");
        }
    }

    #[test]
    fn test_sanitize_schema_preserves_for_openai() {
        let schema = json!({
            "type": "object",
            "properties": { "a": { "type": "string" } },
            "required": ["a"],
            "additionalProperties": false
        });

        let sanitized = sanitize_schema_for_provider(&schema, "openai/gpt-4o");
        assert_eq!(sanitized, schema);
    }

    #[test]
    fn test_sanitize_schema_preserves_for_anthropic_native_structured_output() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "nested": {
                    "type": "object",
                    "properties": { "x": { "type": "integer" } },
                    "additionalProperties": false
                }
            },
            "required": ["name"],
            "additionalProperties": false
        });

        let sanitized = sanitize_schema_for_provider(&schema, "anthropic/claude-haiku-4-5-20251001");
        assert_eq!(sanitized, schema);
    }
}
