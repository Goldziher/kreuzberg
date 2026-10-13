//! Configurable allowlist for the MCP HTTP transport's `Host` header validation.
//!
//! rmcp's `StreamableHttpServerConfig` defaults `allowed_hosts` to a loopback-only list
//! (`localhost`, `127.0.0.1`, `::1`) to guard against DNS-rebinding attacks (see
//! GHSA-89vp-x53w-74fx). That default has no override, so `xberg mcp --transport http`
//! cannot run behind a reverse proxy or ingress that forwards a different `Host` header.
//!
//! This module resolves *additional* hosts to extend (never replace) that default, using
//! the precedence cascade: CLI flag > `XBERG_MCP_ALLOWED_HOSTS` env var > `[mcp]
//! allowed_hosts` config-file key > default (loopback only, unchanged).
//!
//! Deliberately kept as a standalone `[mcp]` table rather than a field on
//! [`crate::ExtractionConfig`], so this MCP-transport-only setting never appears in the
//! alef-generated binding surface.

use crate::{Result, XbergError};
use serde::Deserialize;
use std::path::Path;

/// Environment variable providing a comma-separated list of additional allowed hosts for
/// the MCP HTTP transport.
#[cfg_attr(alef, alef(skip))]
pub const MCP_ALLOWED_HOSTS_ENV: &str = "XBERG_MCP_ALLOWED_HOSTS";

/// Resolve the extra allowed hosts for the MCP HTTP transport using the precedence
/// cascade: `cli_hosts` > `env_value` > `config_hosts` > default (empty).
///
/// The first non-empty tier wins in full; tiers are never merged with each other. Hosts
/// are trimmed, empty entries are dropped, and duplicates are removed while preserving
/// order. An empty result means rmcp's built-in loopback-only default is used unchanged.
#[cfg_attr(alef, alef(skip))]
pub fn resolve_extra_allowed_hosts(
    cli_hosts: &[String],
    env_value: Option<&str>,
    config_hosts: &[String],
) -> Vec<String> {
    if !cli_hosts.is_empty() {
        return clean_hosts(cli_hosts.iter().map(String::as_str));
    }
    if let Some(env_value) = env_value
        && !env_value.trim().is_empty()
    {
        return clean_hosts(env_value.split(','));
    }
    if !config_hosts.is_empty() {
        return clean_hosts(config_hosts.iter().map(String::as_str));
    }
    Vec::new()
}

/// Trim, drop empty entries from, and de-duplicate (preserving order) a host list.
fn clean_hosts<'a>(hosts: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    hosts
        .map(str::trim)
        .filter(|host| !host.is_empty())
        .filter(|host| seen.insert((*host).to_string()))
        .map(str::to_string)
        .collect()
}

/// Read the `allowed_hosts` list from an `[mcp]` table in a TOML, YAML, or JSON config
/// file.
///
/// Returns an empty vector if the file has no `[mcp]` table or `allowed_hosts` key.
///
/// # Errors
///
/// Returns `XbergError::Validation` if the file cannot be read, has an unsupported
/// extension, or contains invalid syntax for its detected format.
#[cfg_attr(alef, alef(skip))]
pub fn read_mcp_allowed_hosts_from_file(path: impl AsRef<Path>) -> Result<Vec<String>> {
    #[derive(Debug, Default, Deserialize)]
    struct McpSection {
        #[serde(default)]
        allowed_hosts: Vec<String>,
    }

    #[derive(Debug, Default, Deserialize)]
    struct ConfigFile {
        #[serde(default)]
        mcp: McpSection,
    }

    let path = path.as_ref();
    let content = std::fs::read_to_string(path)
        .map_err(|e| XbergError::validation(format!("Failed to read config file {}: {}", path.display(), e)))?;
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default();

    let parsed: ConfigFile = match extension.as_str() {
        "toml" => toml::from_str(&content)
            .map_err(|e| XbergError::validation(format!("Invalid TOML in {}: {}", path.display(), e)))?,
        "yaml" | "yml" => yaml_serde::from_str(&content)
            .map_err(|e| XbergError::validation(format!("Invalid YAML in {}: {}", path.display(), e)))?,
        "json" => serde_json::from_str(&content)
            .map_err(|e| XbergError::validation(format!("Invalid JSON in {}: {}", path.display(), e)))?,
        other => {
            return Err(XbergError::validation(format!(
                "Unsupported config file format: .{}. Supported formats: .toml, .yaml, .yml, .json",
                other
            )));
        }
    };

    Ok(parsed.mcp.allowed_hosts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_extra_allowed_hosts_returns_empty_when_all_tiers_unset() {
        let result = resolve_extra_allowed_hosts(&[], None, &[]);
        assert_eq!(
            result,
            Vec::<String>::new(),
            "unset cascade must preserve rmcp's default"
        );
    }

    #[test]
    fn resolve_extra_allowed_hosts_prefers_cli_over_env_and_config() {
        let cli = vec!["cli.example.com".to_string()];
        let config = vec!["config.example.com".to_string()];
        let result = resolve_extra_allowed_hosts(&cli, Some("env.example.com"), &config);
        assert_eq!(result, vec!["cli.example.com".to_string()]);
    }

    #[test]
    fn resolve_extra_allowed_hosts_prefers_env_over_config() {
        let result = resolve_extra_allowed_hosts(&[], Some("env.example.com"), &["config.example.com".to_string()]);
        assert_eq!(result, vec!["env.example.com".to_string()]);
    }

    #[test]
    fn resolve_extra_allowed_hosts_falls_back_to_config() {
        let config = vec!["config.example.com".to_string()];
        let result = resolve_extra_allowed_hosts(&[], None, &config);
        assert_eq!(result, vec!["config.example.com".to_string()]);
    }

    #[test]
    fn resolve_extra_allowed_hosts_parses_comma_separated_env_var_with_whitespace() {
        let result = resolve_extra_allowed_hosts(&[], Some("a.com, b.com"), &[]);
        assert_eq!(result, vec!["a.com".to_string(), "b.com".to_string()]);
    }

    #[test]
    fn resolve_extra_allowed_hosts_treats_blank_env_var_as_unset() {
        let config = vec!["config.example.com".to_string()];
        let result = resolve_extra_allowed_hosts(&[], Some("   "), &config);
        assert_eq!(
            result,
            vec!["config.example.com".to_string()],
            "whitespace-only env var must fall through to config tier"
        );
    }

    #[test]
    fn resolve_extra_allowed_hosts_drops_empty_entries_and_deduplicates() {
        let cli = vec![
            "a.com".to_string(),
            "".to_string(),
            " a.com ".to_string(),
            "b.com".to_string(),
        ];
        let result = resolve_extra_allowed_hosts(&cli, None, &[]);
        assert_eq!(result, vec!["a.com".to_string(), "b.com".to_string()]);
    }

    #[test]
    fn read_mcp_allowed_hosts_from_file_parses_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("xberg.toml");
        std::fs::write(&path, "[mcp]\nallowed_hosts = [\"a.com\", \"b.com\"]\n").expect("write");

        let hosts = read_mcp_allowed_hosts_from_file(&path).expect("parse toml");
        assert_eq!(hosts, vec!["a.com".to_string(), "b.com".to_string()]);
    }

    #[test]
    fn read_mcp_allowed_hosts_from_file_parses_yaml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("xberg.yaml");
        std::fs::write(&path, "mcp:\n  allowed_hosts:\n    - a.com\n    - b.com\n").expect("write");

        let hosts = read_mcp_allowed_hosts_from_file(&path).expect("parse yaml");
        assert_eq!(hosts, vec!["a.com".to_string(), "b.com".to_string()]);
    }

    #[test]
    fn read_mcp_allowed_hosts_from_file_parses_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("xberg.json");
        std::fs::write(&path, r#"{"mcp": {"allowed_hosts": ["a.com", "b.com"]}}"#).expect("write");

        let hosts = read_mcp_allowed_hosts_from_file(&path).expect("parse json");
        assert_eq!(hosts, vec!["a.com".to_string(), "b.com".to_string()]);
    }

    #[test]
    fn read_mcp_allowed_hosts_from_file_returns_empty_when_mcp_section_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("xberg.toml");
        std::fs::write(&path, "use_cache = true\n").expect("write");

        let hosts = read_mcp_allowed_hosts_from_file(&path).expect("parse toml without mcp section");
        assert!(hosts.is_empty());
    }

    #[test]
    fn read_mcp_allowed_hosts_from_file_rejects_unsupported_extension() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("xberg.ini");
        std::fs::write(&path, "[mcp]\nallowed_hosts = a.com\n").expect("write");

        let result = read_mcp_allowed_hosts_from_file(&path);
        assert!(result.is_err(), "unsupported extension must be rejected");
    }
}
