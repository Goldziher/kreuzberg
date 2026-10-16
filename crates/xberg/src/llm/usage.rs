//! Helpers for extracting LLM usage metadata from liter-llm responses.

use crate::types::LlmUsage;

#[derive(Debug, thiserror::Error)]
#[error("LLM attempt metadata")]
struct FailedLlmAttempt {
    usage: LlmUsage,
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

/// Return usage retained on a failed structured LLM response. ~keep
///
/// Inspects the immediate metadata source attached by structured extraction. Returns
/// `None` for unrelated errors or failures before a provider response was decoded.
/// Returns an owned metadata snapshot with no prompt or response content.
///
/// # Examples
///
/// ```
/// use xberg::{XbergError, llm::usage::usage_from_error};
///
/// let error = XbergError::parsing("unrelated parsing failure");
/// assert!(usage_from_error(&error).is_none());
/// ```
#[cfg_attr(alef, alef(skip))]
pub fn usage_from_error(error: &crate::XbergError) -> Option<LlmUsage> {
    std::error::Error::source(error)?
        .downcast_ref::<FailedLlmAttempt>()
        .map(|attempt| attempt.usage.clone())
}

pub(crate) fn with_usage(error: crate::XbergError, usage: Option<LlmUsage>) -> crate::XbergError {
    match (error, usage) {
        (crate::XbergError::Parsing { message, source }, Some(usage)) => {
            crate::XbergError::parsing_with_source(message, FailedLlmAttempt { usage, source })
        }
        (error, _) => error,
    }
}

/// Extract usage metadata from a chat completion response.
pub(crate) fn extract_usage_from_chat(response: &liter_llm::ChatCompletionResponse, source: &str) -> Option<LlmUsage> {
    Some(LlmUsage {
        model: response.model.clone(),
        source: source.to_string(),
        input_tokens: response.usage.as_ref().map(|u| u.prompt_tokens),
        output_tokens: response.usage.as_ref().map(|u| u.completion_tokens),
        total_tokens: response.usage.as_ref().map(|u| u.total_tokens),
        estimated_cost: response.estimated_cost(),
        finish_reason: response
            .choices
            .first()
            .and_then(|c| c.finish_reason.as_ref())
            .map(|fr| format!("{fr:?}").to_lowercase()),
    })
}

/// Extract usage metadata from an embedding response.
#[cfg(all(
    feature = "tokio-runtime",
    any(feature = "embeddings", feature = "static-embeddings"),
    not(target_arch = "wasm32")
))]
pub(crate) fn extract_usage_from_embedding(response: &liter_llm::EmbeddingResponse, source: &str) -> Option<LlmUsage> {
    Some(LlmUsage {
        model: response.model.clone(),
        source: source.to_string(),
        input_tokens: response.usage.as_ref().map(|u| u.prompt_tokens),
        output_tokens: response.usage.as_ref().map(|u| u.completion_tokens),
        total_tokens: response.usage.as_ref().map(|u| u.total_tokens),
        estimated_cost: response.estimated_cost(),
        finish_reason: None,
    })
}

/// Append an `LlmUsage` entry to an `ExtractedDocument`, lazily initializing the vec.
pub(crate) fn push_llm_usage(result: &mut crate::ExtractedDocument, usage: Option<LlmUsage>) {
    if let Some(u) = usage {
        result.llm_usage.get_or_insert_with(Vec::new).push(u);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_attempt_retains_existing_parsing_message_and_source() {
        let original = crate::XbergError::parsing_with_source(
            "safe parsing failure",
            std::io::Error::other("safe underlying failure"),
        );
        let error = with_usage(original, Some(LlmUsage::default()));
        assert_eq!(error.to_string(), "Parsing error: safe parsing failure");
        let metadata = std::error::Error::source(&error).expect("metadata source");
        assert_eq!(metadata.to_string(), "LLM attempt metadata");
        assert_eq!(
            metadata.source().expect("original source").to_string(),
            "safe underlying failure"
        );
        assert!(usage_from_error(&error).is_some());
    }
}
