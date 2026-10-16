//! PEFT-format LoRA adapter loading + merge-at-load.
//!
//! Adapters live as a 2-file directory:
//!
//! ```text
//! <adapter>/
//! ├── adapter_config.json     {target_modules, r, alpha, fan_in_fan_out, ...}
//! └── adapter_model.safetensors  {layer_path → W_down, W_up}
//! ```
//!
//! Safetensors keys follow the PEFT convention:
//!
//! ```text
//! base_model.model.<module_path>.lora_A.weight   shape [r, in]
//! base_model.model.<module_path>.lora_B.weight   shape [out, r]
//! ```
//!
//! Merge operation (per target module):
//!
//! ```text
//! W_merged = W_base + (alpha / r) * (lora_B @ lora_A)
//! ```
//!
//! With `fan_in_fan_out: true` (HF Conv1D-style layers), the delta is
//! transposed before adding. Most modern adapters use `false` (default).
//!
//! ## References
//!
//! - PEFT layer.py: <https://github.com/huggingface/peft/blob/main/src/peft/tuners/lora/layer.py>
//! - LoRA paper: arXiv:2106.09685
//! - GLiNER2Swift's merge-at-load pattern (verified equivalent to PyTorch
//!   `peft.merge_and_unload`).

use std::collections::HashMap;
use std::path::Path;

use candle_core::{DType, Device, Tensor};
use safetensors::SafeTensors;
use serde::Deserialize;

/// Subset of PEFT's `adapter_config.json` schema we need for inference-time
/// merge. Other fields (peft_type, task_type, lora_dropout, bias, ...) are
/// accepted-and-ignored via `#[serde(default)]` + extra fields ignored by
/// serde_json's default behavior.
#[derive(Debug, Clone, Deserialize)]
pub struct LoraConfig {
    /// LoRA rank.
    pub r: usize,
    /// LoRA scaling factor (alpha). Final scale is `alpha / r`.
    pub lora_alpha: f64,
    /// Target module names or regex patterns. Matched against the
    /// per-key path AFTER the `base_model.model.` prefix is stripped.
    /// May be `None` if the adapter sets `target_modules: null` and
    /// matching is implicit (rare).
    #[serde(default)]
    #[allow(dead_code)]
    pub target_modules: Option<Vec<String>>,
    /// Base model identifier this adapter was trained on.
    #[serde(default)]
    pub base_model_name_or_path: Option<String>,
    /// Whether the base layer's weight is stored in `(in, out)` order
    /// (Conv1D / GPT2-style). For standard `nn.Linear` it's `false` (the
    /// default).
    #[serde(default)]
    pub fan_in_fan_out: bool,
}

/// Per-module LoRA delta (the two matrices) parsed from
/// `adapter_model.safetensors`.
pub struct LoraModule {
    /// Down-projection. Shape `[r, in]` (PEFT default).
    pub lora_a: Tensor,
    /// Up-projection. Shape `[out, r]` (PEFT default).
    pub lora_b: Tensor,
}

/// A loaded PEFT adapter: config + per-module deltas keyed by HF
/// parameter path (e.g. `encoder.encoder.layer.0.attention.self.query_proj`).
pub struct LoraAdapter {
    pub config: LoraConfig,
    pub modules: HashMap<String, LoraModule>,
}

impl LoraAdapter {
    /// Load a PEFT adapter from a directory.
    pub fn load(adapter_dir: &Path, device: &Device) -> crate::candle::Result<Self> {
        let config_path = adapter_dir.join("adapter_config.json");
        let cfg_str = std::fs::read_to_string(&config_path).map_err(|e| {
            crate::candle::GlinerCandleError::Backend(format!("lora: read {}: {e}", config_path.display()))
        })?;
        let config: LoraConfig = serde_json::from_str(&cfg_str).map_err(|e| {
            crate::candle::GlinerCandleError::Backend(format!("lora: parse {}: {e}", config_path.display()))
        })?;
        if config.r == 0 {
            return Err(crate::candle::GlinerCandleError::Backend(
                "lora: adapter_config.json has r=0; refusing to merge".into(),
            ));
        }

        let weights_path = if adapter_dir.join("adapter_model.safetensors").exists() {
            adapter_dir.join("adapter_model.safetensors")
        } else if adapter_dir.join("adapter_weights.safetensors").exists() {
            adapter_dir.join("adapter_weights.safetensors")
        } else {
            return Err(crate::candle::GlinerCandleError::Backend(format!(
                "lora: no adapter_model.safetensors or adapter_weights.safetensors in {}",
                adapter_dir.display()
            )));
        };

        let bytes = std::fs::read(&weights_path).map_err(|e| {
            crate::candle::GlinerCandleError::Backend(format!("lora: read {}: {e}", weights_path.display()))
        })?;
        let st = SafeTensors::deserialize(&bytes).map_err(|e| {
            crate::candle::GlinerCandleError::Backend(format!("lora: deserialize {}: {e}", weights_path.display()))
        })?;

        // Walk keys, group by module path, slot lora_A/lora_B.
        // Keys: base_model.model.<path>.lora_{A,B}.weight ~keep
        let mut by_module: HashMap<String, (Option<Tensor>, Option<Tensor>)> = HashMap::new();
        for (key, view) in st.tensors() {
            let (module_path, slot) = parse_lora_key(&key)?;
            let shape: Vec<usize> = view.shape().to_vec();
            // safetensors gives us a byte slice; load into a Candle tensor.
            // PEFT adapters are typically fp32; if the dtype is fp16/bf16 we'd
            // need to convert. Phase 4 supports fp32 only; error otherwise. ~keep
            if view.dtype() != safetensors::Dtype::F32 {
                return Err(crate::candle::GlinerCandleError::Backend(format!(
                    "lora: {key}: dtype {:?} not supported (Phase 4 ships fp32 only)",
                    view.dtype()
                )));
            }
            let bytes = view.data();
            let n = bytes.len() / 4;
            let mut data = Vec::with_capacity(n);
            for chunk in bytes.as_chunks::<4>().0 {
                data.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
            }
            let tensor = Tensor::from_vec(data, shape, device)
                .map_err(|e| crate::candle::GlinerCandleError::Backend(format!("lora: tensor {key}: {e}")))?;
            let entry = by_module.entry(module_path).or_default();
            match slot {
                LoraSlot::A => entry.0 = Some(tensor),
                LoraSlot::B => entry.1 = Some(tensor),
            }
        }

        let mut modules = HashMap::new();
        for (path, (a, b)) in by_module {
            let lora_a = a.ok_or_else(|| {
                crate::candle::GlinerCandleError::Backend(format!("lora: missing lora_A for module {path}"))
            })?;
            let lora_b = b.ok_or_else(|| {
                crate::candle::GlinerCandleError::Backend(format!("lora: missing lora_B for module {path}"))
            })?;
            modules.insert(path, LoraModule { lora_a, lora_b });
        }

        Ok(Self { config, modules })
    }
}

#[derive(Debug, Clone, Copy)]
enum LoraSlot {
    A,
    B,
}

/// Parse `base_model.model.<module_path>.lora_{A,B}.weight` →
/// `(<module_path>, slot)`. Strict; rejects keys not matching the
/// PEFT convention.
fn parse_lora_key(key: &str) -> crate::candle::Result<(String, LoraSlot)> {
    let stripped = key.strip_prefix("base_model.model.").ok_or_else(|| {
        crate::candle::GlinerCandleError::Backend(format!("lora: key {key} does not start with 'base_model.model.'"))
    })?;
    if let Some(path) = stripped.strip_suffix(".lora_A.weight") {
        Ok((path.to_string(), LoraSlot::A))
    } else if let Some(path) = stripped.strip_suffix(".lora_B.weight") {
        Ok((path.to_string(), LoraSlot::B))
    } else {
        Err(crate::candle::GlinerCandleError::Backend(format!(
            "lora: key {key} does not end with '.lora_A.weight' or '.lora_B.weight'"
        )))
    }
}

/// Merge a loaded adapter into base safetensors weights, returning a
/// `HashMap<key, Tensor>` of the merged base weights ready to feed into
/// a `VarBuilder::from_tensors`.
///
/// For every base weight tensor:
/// - If a target module's path matches (the base key minus its
///   `.weight` suffix), apply `W += alpha/r * (lora_B @ lora_A)`.
/// - Otherwise, return the base weight unchanged.
///
/// Cost: O(base_safetensors_size) for the read + O(num_target_modules ×
/// rank × max_dim²) for the matmuls. Typically ~100ms for a 280M-param
/// model with 50 target modules at rank 8.
pub(crate) fn merge_into_base(
    base_safetensors: &Path,
    adapter: &LoraAdapter,
    device: &Device,
) -> crate::candle::Result<HashMap<String, Tensor>> {
    let bytes = std::fs::read(base_safetensors).map_err(|e| {
        crate::candle::GlinerCandleError::Backend(format!("lora_merge: read {}: {e}", base_safetensors.display()))
    })?;
    let st = SafeTensors::deserialize(&bytes).map_err(|e| {
        crate::candle::GlinerCandleError::Backend(format!(
            "lora_merge: deserialize {}: {e}",
            base_safetensors.display()
        ))
    })?;

    let scale = adapter.config.lora_alpha / (adapter.config.r as f64);
    let mut out: HashMap<String, Tensor> = HashMap::with_capacity(st.tensors().len());
    // Owned because we exit the loop scope before the validation pass below. ~keep
    let mut applied: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (key, view) in st.tensors() {
        let shape: Vec<usize> = view.shape().to_vec();
        let mut tensor = decode_view(&view, shape, device)
            .map_err(|e| crate::candle::GlinerCandleError::Backend(format!("lora_merge: decode {key}: {e}")))?;

        if let Some(mod_path) = key.strip_suffix(".weight")
            && let Some(lora_mod) = adapter.modules.get(mod_path)
        {
            tensor = apply_lora_delta(
                &tensor,
                &lora_mod.lora_a,
                &lora_mod.lora_b,
                scale,
                adapter.config.fan_in_fan_out,
            )
            .map_err(|e| {
                crate::candle::GlinerCandleError::Backend(format!("lora_merge: apply delta to {mod_path}: {e}"))
            })?;
            applied.insert(mod_path.to_string());
        }

        out.insert(key.to_string(), tensor);
    }

    // Sanity: every adapter module should have matched a base key. If
    // the adapter targets a module that doesn't exist in the base, that's
    // a config error (e.g. trained on a different model). ~keep
    for adapter_path in adapter.modules.keys() {
        if !applied.contains(adapter_path) {
            return Err(crate::candle::GlinerCandleError::Backend(format!(
                "lora_merge: adapter targets module '{adapter_path}' but no \
                 matching key '{adapter_path}.weight' found in base safetensors"
            )));
        }
    }

    Ok(out)
}

/// Decode a SafeTensors view into a Candle Tensor. Supports fp32 + i64.
fn decode_view(
    view: &safetensors::tensor::TensorView<'_>,
    shape: Vec<usize>,
    device: &Device,
) -> candle_core::Result<Tensor> {
    use safetensors::Dtype as ST;
    match view.dtype() {
        ST::F32 => {
            let bytes = view.data();
            let n = bytes.len() / 4;
            let mut data = Vec::with_capacity(n);
            for chunk in bytes.as_chunks::<4>().0 {
                data.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
            }
            Tensor::from_vec(data, shape, device)
        }
        ST::I64 => {
            let bytes = view.data();
            let n = bytes.len() / 8;
            let mut data: Vec<i64> = Vec::with_capacity(n);
            for chunk in bytes.as_chunks::<8>().0 {
                data.push(i64::from_le_bytes([
                    chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
                ]));
            }
            Tensor::from_vec(data, shape, device)
        }
        other => Err(candle_core::Error::Msg(format!(
            "lora_merge: dtype {other:?} not supported (only F32, I64; the GLiNER2 \
             base safetensors has only F32 and I64)"
        ))),
    }
}

/// Apply `W += scale * (lora_B @ lora_A)`. If `fan_in_fan_out`, the
/// base weight is `[in, out]` instead of `[out, in]` and the delta is
/// transposed before adding.
fn apply_lora_delta(
    base: &Tensor,
    lora_a: &Tensor,
    lora_b: &Tensor,
    scale: f64,
    fan_in_fan_out: bool,
) -> candle_core::Result<Tensor> {
    let delta = lora_b.matmul(lora_a)?;
    let delta = (delta * scale)?;
    let delta = if fan_in_fan_out {
        delta.t()?.contiguous()?
    } else {
        delta
    };
    if base.shape().dims() != delta.shape().dims() {
        return Err(candle_core::Error::Msg(format!(
            "lora_merge: base shape {:?} != delta shape {:?} (fan_in_fan_out={fan_in_fan_out})",
            base.shape().dims(),
            delta.shape().dims(),
        )));
    }
    base.add(&delta)
}

// Note: DType is referenced via fully-qualified candle_core::DType in the
// public API to avoid an unused import when the trait isn't otherwise used. ~keep
#[allow(dead_code)]
fn _dtype_marker() -> DType {
    DType::F32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lora_key_strict() {
        let (path, slot) = parse_lora_key("base_model.model.encoder.layer.0.attention.self.query.lora_A.weight")
            .expect("valid PEFT key should parse");
        assert_eq!(path, "encoder.layer.0.attention.self.query");
        assert!(matches!(slot, LoraSlot::A));

        let (path_b, slot_b) = parse_lora_key("base_model.model.encoder.layer.0.attention.self.query.lora_B.weight")
            .expect("valid PEFT key (B) should parse");
        assert_eq!(path_b, "encoder.layer.0.attention.self.query");
        assert!(matches!(slot_b, LoraSlot::B));

        assert!(
            parse_lora_key("encoder.layer.0.attention.self.query.lora_A.weight").is_err(),
            "missing 'base_model.model.' prefix should fail"
        );
        assert!(
            parse_lora_key("base_model.model.encoder.layer.0.weight").is_err(),
            "missing '.lora_A.weight'/'.lora_B.weight' suffix should fail"
        );
    }

    #[test]
    fn apply_lora_delta_shape() {
        let device = Device::Cpu;
        let base = Tensor::zeros((4, 3), DType::F32, &device).unwrap();
        let lora_a = Tensor::ones((2, 3), DType::F32, &device).unwrap();
        let lora_b = Tensor::ones((4, 2), DType::F32, &device).unwrap();
        let merged = apply_lora_delta(&base, &lora_a, &lora_b, 0.5, false).unwrap();
        assert_eq!(merged.shape().dims(), &[4, 3]);
        // Each entry of (lora_b @ lora_a) is r=2 ones, so delta = 2 * 0.5 = 1.0 everywhere. ~keep
        let v = merged.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        for x in v {
            assert!((x - 1.0).abs() < 1e-6);
        }
    }
}
