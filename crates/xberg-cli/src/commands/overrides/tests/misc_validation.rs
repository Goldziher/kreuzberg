use super::super::*;
use super::default_overrides;
use xberg::ExtractionConfig;

#[test]
fn test_validate_target_dpi_out_of_range() {
    let overrides = ExtractionOverrides {
        target_dpi: Some(5),
        ..default_overrides()
    };
    assert!(overrides.validate().is_err());

    let overrides = ExtractionOverrides {
        target_dpi: Some(5000),
        ..default_overrides()
    };
    assert!(overrides.validate().is_err());
}

#[test]
fn test_validate_target_dpi_valid() {
    let overrides = ExtractionOverrides {
        target_dpi: Some(300),
        ..default_overrides()
    };
    assert!(overrides.validate().is_ok());
}

#[test]
fn test_validate_csv_delimiter_valid() {
    let overrides = ExtractionOverrides {
        csv_delimiter: Some(";".to_string()),
        ..default_overrides()
    };
    assert!(overrides.validate().is_ok());
}

#[test]
fn test_validate_csv_delimiter_empty_rejected() {
    let overrides = ExtractionOverrides {
        csv_delimiter: Some(String::new()),
        ..default_overrides()
    };
    let err = overrides.validate().unwrap_err();
    assert_eq!(
        err.to_string(),
        "Invalid CSV delimiter ''. Must be exactly one ASCII character (e.g. ',', ';', '\\t', '|')."
    );
}

#[test]
fn test_validate_csv_delimiter_multi_byte_rejected() {
    let overrides = ExtractionOverrides {
        csv_delimiter: Some("::".to_string()),
        ..default_overrides()
    };
    let err = overrides.validate().unwrap_err();
    assert_eq!(
        err.to_string(),
        "Invalid CSV delimiter '::'. Must be exactly one ASCII character (e.g. ',', ';', '\\t', '|')."
    );
}

#[test]
fn test_apply_csv_delimiter_and_comment_prefixes() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        csv_delimiter: Some(";".to_string()),
        csv_comment_prefix: vec!["#".to_string(), "//".to_string()],
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let csv = config.csv.expect("csv config should be set");
    assert_eq!(csv.delimiter.as_deref(), Some(";"));
    assert_eq!(csv.comment_prefixes, vec!["#".to_string(), "//".to_string()]);
}

#[test]
fn test_apply_csv_no_flags_leaves_config_untouched() {
    let mut config = ExtractionConfig::default();
    let overrides = default_overrides();
    overrides.apply(&mut config);
    assert!(config.csv.is_none());
}
