use crate::error::{KreuzbergError, Result};
use once_cell::sync::OnceCell;
use regex::Regex;
use tokio::process::Command;

static PANDOC_VALIDATED: OnceCell<bool> = OnceCell::new();

/// Validate that Pandoc version 2 or above is installed and available
pub async fn validate_pandoc_version() -> Result<()> {
    if PANDOC_VALIDATED.get().is_some() {
        return Ok(());
    }

    let output = Command::new("pandoc").arg("--version").output().await.map_err(|e| {
        KreuzbergError::MissingDependency(format!(
            "Pandoc version 2 or above is required but not found in PATH: {}",
            e
        ))
    })?;

    if !output.status.success() {
        return Err(KreuzbergError::MissingDependency(
            "Pandoc version 2 or above is required but command failed".to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let version = extract_version(&stdout).ok_or_else(|| {
        KreuzbergError::MissingDependency(format!("Could not parse Pandoc version from output: {}", stdout))
    })?;

    if version.major < 2 {
        return Err(KreuzbergError::MissingDependency(format!(
            "Pandoc version 2 or above is required, found version {}.{}.{}",
            version.major, version.minor, version.patch
        )));
    }

    let _ = PANDOC_VALIDATED.set(true);

    Ok(())
}

#[derive(Debug, Clone)]
struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

fn extract_version(output: &str) -> Option<Version> {
    let patterns = [
        r"pandoc(?:\.exe)?(?:\s+|\s+v|\s+version\s+)(\d+)\.(\d+)(?:\.(\d+))?",
        r"pandoc\s+\(version\s+(\d+)\.(\d+)(?:\.(\d+))?\)",
        r"pandoc-(\d+)\.(\d+)(?:\.(\d+))?",
        r"^(\d+)\.(\d+)(?:\.(\d+)(?:\.(\d+))?)?",
        r"(?:^|\s)(\d+)\.(\d+)(?:\.(\d+))?(?:\s|$)",
    ];

    for pattern in &patterns {
        if let Ok(re) = Regex::new(pattern)
            && let Some(caps) = re.captures(output)
        {
            let major = caps.get(1)?.as_str().parse().ok()?;
            let minor = caps.get(2)?.as_str().parse().ok()?;
            let patch = caps.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);

            return Some(Version { major, minor, patch });
        }
    }

    for line in output.lines() {
        for token in line.split_whitespace() {
            if let Some(version) = parse_version_token(token) {
                return Some(version);
            }
        }
    }

    None
}

fn parse_version_token(token: &str) -> Option<Version> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() >= 2
        && let (Ok(major), Ok(minor)) = (parts[0].parse(), parts[1].parse())
    {
        let patch = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
        return Some(Version { major, minor, patch });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version_standard_format() {
        let output = "pandoc 3.1.2";
        let version = extract_version(output).unwrap();
        assert_eq!(version.major, 3);
        assert_eq!(version.minor, 1);
        assert_eq!(version.patch, 2);
    }

    #[test]
    fn test_extract_version_with_parens() {
        let output = "pandoc (version 2.19.2)";
        let version = extract_version(output).unwrap();
        assert_eq!(version.major, 2);
        assert_eq!(version.minor, 19);
    }

    #[test]
    fn test_extract_version_with_exe() {
        let output = "pandoc.exe 3.0";
        let version = extract_version(output).unwrap();
        assert_eq!(version.major, 3);
        assert_eq!(version.minor, 0);
    }

    #[test]
    fn test_extract_version_multiline() {
        let output = "pandoc 3.1.2\nCopyright (C) 2006-2023 John MacFarlane";
        let version = extract_version(output).unwrap();
        assert_eq!(version.major, 3);
        assert_eq!(version.minor, 1);
    }

    #[test]
    fn test_extract_version_no_patch() {
        let output = "pandoc 2.5";
        let version = extract_version(output).unwrap();
        assert_eq!(version.major, 2);
        assert_eq!(version.minor, 5);
        assert_eq!(version.patch, 0);
    }

    #[test]
    fn test_parse_version_token() {
        let version = parse_version_token("2.19.2").unwrap();
        assert_eq!(version.major, 2);
        assert_eq!(version.minor, 19);
        assert_eq!(version.patch, 2);
    }

    #[test]
    fn test_parse_version_token_no_patch() {
        let version = parse_version_token("3.1").unwrap();
        assert_eq!(version.major, 3);
        assert_eq!(version.minor, 1);
        assert_eq!(version.patch, 0);
    }

    #[test]
    fn test_parse_version_token_invalid() {
        let version = parse_version_token("abc");
        assert!(version.is_none());
    }
}
