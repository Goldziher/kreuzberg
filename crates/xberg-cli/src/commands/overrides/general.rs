//! Acceleration, concurrency, page/image extraction, email, cache, and CSV overrides --
//! the flags with no dedicated domain of their own.

use anyhow::{Result, bail};
use xberg::{ExecutionProviderType, ExtractionConfig};

use super::ExtractionOverrides;

/// Hardware acceleration provider for ONNX Runtime models.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum AccelerationArg {
    /// Auto-detect best provider per platform.
    Auto,
    /// CPU execution provider (always available).
    Cpu,
    /// Apple CoreML (macOS/iOS Neural Engine + GPU).
    #[value(name = "coreml")]
    CoreMl,
    /// NVIDIA CUDA GPU acceleration.
    Cuda,
    /// NVIDIA TensorRT (optimized CUDA inference).
    #[value(name = "tensorrt")]
    TensorRt,
}

impl From<AccelerationArg> for ExecutionProviderType {
    fn from(arg: AccelerationArg) -> Self {
        match arg {
            AccelerationArg::Auto => ExecutionProviderType::Auto,
            AccelerationArg::Cpu => ExecutionProviderType::Cpu,
            AccelerationArg::CoreMl => ExecutionProviderType::CoreMl,
            AccelerationArg::Cuda => ExecutionProviderType::Cuda,
            AccelerationArg::TensorRt => ExecutionProviderType::TensorRt,
        }
    }
}

impl ExtractionOverrides {
    /// Reject an out-of-range `--target-dpi` value.
    pub(super) fn validate_target_dpi(&self) -> Result<()> {
        if let Some(dpi) = self.target_dpi
            && (!(36..=2400).contains(&dpi))
        {
            bail!("Invalid target DPI: {dpi}. Value must be between 36 and 2400.");
        }
        Ok(())
    }

    /// Reject a zero value for any of the `--max-concurrent*`/`--max-threads` flags.
    pub(super) fn validate_concurrency(&self) -> Result<()> {
        if let Some(0) = self.max_concurrent {
            bail!("--max-concurrent must be at least 1");
        }
        if let Some(0) = self.max_threads {
            bail!("--max-threads must be at least 1");
        }
        if let Some(0) = self.max_concurrent_ocr {
            bail!("--max-concurrent-ocr must be at least 1");
        }
        Ok(())
    }

    /// Reject an invalid `--csv-delimiter` value.
    pub(super) fn validate_csv(&self) -> Result<()> {
        if let Some(ref delimiter) = self.csv_delimiter
            && !(delimiter.len() == 1 && delimiter.is_ascii())
        {
            bail!(
                "Invalid CSV delimiter '{}'. Must be exactly one ASCII character (e.g. ',', ';', '\\t', '|').",
                delimiter
            );
        }
        Ok(())
    }

    pub(super) fn apply_acceleration(&self, config: &mut ExtractionConfig) {
        if let Some(accel) = self.acceleration {
            let mut accel_config = config.acceleration.clone().unwrap_or_default();
            accel_config.provider = accel.into();
            config.acceleration = Some(accel_config);
        }
    }

    pub(super) fn apply_concurrency(&self, config: &mut ExtractionConfig) {
        if let Some(max_concurrent) = self.max_concurrent {
            config.max_concurrent_extractions = Some(max_concurrent);
        }
        if let Some(max_threads) = self.max_threads {
            let concurrency = config.concurrency.get_or_insert_with(Default::default);
            concurrency.max_threads = Some(max_threads);
        }
        if let Some(max_concurrent_ocr) = self.max_concurrent_ocr {
            let concurrency = config.concurrency.get_or_insert_with(Default::default);
            concurrency.max_concurrent_ocr = Some(max_concurrent_ocr);
        }
    }

    pub(super) fn apply_pages(&self, config: &mut ExtractionConfig) {
        let has_page_flag = self.extract_pages.is_some() || self.page_markers.is_some();
        if has_page_flag {
            let mut page_config = config.pages.clone().unwrap_or_default();
            if let Some(extract) = self.extract_pages {
                page_config.extract_pages = extract;
            }
            if let Some(markers) = self.page_markers {
                page_config.insert_page_markers = markers;
            }
            config.pages = Some(page_config);
        }
    }

    pub(super) fn apply_images(&self, config: &mut ExtractionConfig) {
        let has_image_flag = self.extract_images.is_some() || self.target_dpi.is_some();
        if has_image_flag {
            let mut img = config.images.clone().unwrap_or_default();
            if let Some(extract) = self.extract_images {
                img.extract_images = extract;
            }
            if let Some(dpi) = self.target_dpi {
                img.target_dpi = dpi;
            }
            config.images = Some(img);
        }
    }

    pub(super) fn apply_email(&self, config: &mut ExtractionConfig) {
        if let Some(codepage) = self.msg_codepage {
            let email = config.email.get_or_insert_with(Default::default);
            email.msg_fallback_codepage = Some(codepage);
        }
    }

    pub(super) fn apply_cache(&self, config: &mut ExtractionConfig) {
        if let Some(no_cache_flag) = self.no_cache {
            config.use_cache = !no_cache_flag;
        }
        if let Some(ns) = &self.cache_namespace {
            config.cache_namespace = Some(ns.clone());
        }
        if let Some(ttl) = self.cache_ttl_secs {
            config.cache_ttl_secs = Some(ttl);
        }
    }

    pub(super) fn apply_csv(&self, config: &mut ExtractionConfig) {
        let has_flag = self.csv_delimiter.is_some() || !self.csv_comment_prefix.is_empty();
        if has_flag {
            let mut csv_cfg = config.csv.clone().unwrap_or_default();
            if let Some(ref delimiter) = self.csv_delimiter {
                csv_cfg.delimiter = Some(delimiter.clone());
            }
            if !self.csv_comment_prefix.is_empty() {
                csv_cfg.comment_prefixes = self.csv_comment_prefix.clone();
            }
            config.csv = Some(csv_cfg);
        }
    }
}
