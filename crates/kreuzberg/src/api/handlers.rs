//! API request handlers.

use axum::{
    Json,
    extract::{Multipart, State},
};

use crate::{batch_extract_bytes, cache, extract_bytes};

use super::{
    error::ApiError,
    types::{ApiState, CacheClearResponse, CacheStatsResponse, ExtractResponse, HealthResponse, InfoResponse},
};

/// Extract endpoint handler.
///
/// POST /extract
///
/// Accepts multipart form data with:
/// - `files`: One or more files to extract
/// - `config` (optional): JSON extraction configuration (overrides server defaults)
///
/// Returns a list of extraction results, one per file.
///
/// The server's default config (loaded from kreuzberg.toml/yaml/json via discovery)
/// is used as the base, and any per-request config overrides those defaults.
pub async fn extract_handler(
    State(state): State<ApiState>,
    mut multipart: Multipart,
) -> Result<Json<ExtractResponse>, ApiError> {
    let mut files = Vec::new();
    // Start with server default config (loaded from config file via discovery)
    let mut config = (*state.default_config).clone();

    // Parse multipart form data
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "files" => {
                // Get file content and MIME type
                let file_name = field.file_name().map(|s| s.to_string());
                let content_type = field.content_type().map(|s| s.to_string());
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?;

                // Use provided content type, or default to application/octet-stream
                // MIME detection from bytes can be added later with `infer` crate
                let mime_type = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

                files.push((data.to_vec(), mime_type, file_name));
            }
            "config" => {
                // Parse JSON config
                let config_str = field
                    .text()
                    .await
                    .map_err(|e| ApiError::validation(crate::error::KreuzbergError::validation(e.to_string())))?;

                config = serde_json::from_str(&config_str).map_err(|e| {
                    ApiError::validation(crate::error::KreuzbergError::validation(format!(
                        "Invalid extraction configuration: {}",
                        e
                    )))
                })?;
            }
            _ => {
                // Unknown field, skip
            }
        }
    }

    // Validate that files were provided
    if files.is_empty() {
        return Err(ApiError::validation(crate::error::KreuzbergError::validation(
            "No files provided for extraction",
        )));
    }

    // Single file optimization
    if files.len() == 1 {
        let (data, mime_type, _file_name) = files.into_iter().next().unwrap();
        let result = extract_bytes(&data, mime_type.as_str(), &config).await?;
        return Ok(Json(vec![result]));
    }

    // Batch processing
    let files_data: Vec<(Vec<u8>, String)> = files.into_iter().map(|(data, mime, _name)| (data, mime)).collect();

    // Convert to references for batch_extract_bytes
    let file_refs: Vec<(&[u8], &str)> = files_data
        .iter()
        .map(|(data, mime)| (data.as_slice(), mime.as_str()))
        .collect();

    let results = batch_extract_bytes(file_refs, &config).await?;
    Ok(Json(results))
}

/// Health check endpoint handler.
///
/// GET /health
pub async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Server info endpoint handler.
///
/// GET /info
pub async fn info_handler() -> Json<InfoResponse> {
    Json(InfoResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        rust_backend: true,
    })
}

/// Cache stats endpoint handler.
///
/// GET /cache/stats
pub async fn cache_stats_handler() -> Result<Json<CacheStatsResponse>, ApiError> {
    let cache_dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(".kreuzberg");

    let stats = cache::get_cache_metadata(cache_dir.to_str().unwrap_or(".")).map_err(ApiError::internal)?;

    Ok(Json(CacheStatsResponse {
        directory: cache_dir.to_string_lossy().to_string(),
        total_files: stats.total_files,
        total_size_mb: stats.total_size_mb,
        available_space_mb: stats.available_space_mb,
        oldest_file_age_days: stats.oldest_file_age_days,
        newest_file_age_days: stats.newest_file_age_days,
    }))
}

/// Cache clear endpoint handler.
///
/// DELETE /cache/clear
pub async fn cache_clear_handler() -> Result<Json<CacheClearResponse>, ApiError> {
    let cache_dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(".kreuzberg");

    let (removed_files, freed_mb) =
        cache::clear_cache_directory(cache_dir.to_str().unwrap_or(".")).map_err(ApiError::internal)?;

    Ok(Json(CacheClearResponse {
        directory: cache_dir.to_string_lossy().to_string(),
        removed_files,
        freed_mb,
    }))
}
