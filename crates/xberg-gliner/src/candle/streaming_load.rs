//! Memory-efficient safetensors loading for wasm32.
//!
//! `candle_core::safetensors::load_buffer` (used by the original
//! `from_buffered_safetensors` implementations) deserializes the safetensors
//! header, then eagerly materializes EVERY tensor in the file as an owned
//! F32 `Tensor` into one `HashMap<String, Tensor>` -- all at once, before any
//! dtype conversion happens. For a large encoder (this crate targets
//! DeBERTa-v2-scale GLiNER2 models with big embedding tables), that means
//! holding the full F32-sized model in memory simultaneously with the raw
//! safetensors byte buffer it was just downloaded into -- confirmed via a
//! live browser reproduction to exhaust wasm32 linear memory and crash with
//! `RuntimeError: unreachable` inside `std::alloc::rust_oom`, even though
//! the model's final (post-conversion) memory footprint would fit
//! comfortably.
//!
//! This loader instead uses `safetensors::SafeTensors::deserialize` directly
//! (zero-copy header parse -- no tensor allocation yet), then converts each
//! tensor to the target `dtype` ONE AT A TIME, dropping the transient F32
//! copy before moving to the next tensor. Peak memory is therefore: the raw
//! byte buffer (already resident, already paid for) + one transient F32
//! tensor + the growing target-dtype `HashMap` -- not raw buffer + full F32
//! HashMap + target-dtype HashMap all at once.
//!
//! For the F32->F16 case specifically (the only conversion this crate needs
//! on wasm32), even that one-transient-F32-tensor peak turned out to be too
//! much for large tensors: `candle_core::safetensors::Load::load` first
//! copies the view's raw bytes into an owned F32 `Tensor`/`Vec<f32>`, then
//! `to_dtype` allocates a SECOND owned buffer (`Vec<f16>`) and converts
//! element-by-element before the F32 one is dropped -- confirmed via a live
//! browser reproduction to intermittently OOM (`RuntimeError: unreachable`
//! in `rust_oom`) inside exactly this call, on exactly the largest tensor
//! (the token embedding table), even with the one-tensor-at-a-time loop
//! above. `convert_view_to_f16` below reads the view's raw little-endian
//! bytes directly and writes straight into the target `Vec<half::f16>` --
//! only ONE new heap allocation per tensor (the raw bytes are a borrow into
//! the already-resident safetensors buffer, not a copy), roughly halving
//! the transient peak for the file's largest tensors.

use std::collections::HashMap;

use candle_core::{DType, Device, Tensor};
use safetensors::tensor::{Dtype as StDtype, SafeTensors, TensorView};

/// Load every tensor in a safetensors buffer, converting each to `dtype`
/// immediately after loading rather than materializing the whole file at
/// its native (F32) dtype first. See module docs for why this matters on
/// wasm32.
pub(crate) fn load_buffer_streaming(
    bytes: &[u8],
    device: &Device,
    dtype: DType,
) -> crate::candle::Result<HashMap<String, Tensor>> {
    let st = SafeTensors::deserialize(bytes)
        .map_err(|e| crate::candle::GlinerCandleError::Backend(format!("safetensors deserialize: {e}")))?;

    let mut tensors = HashMap::with_capacity(st.len());
    for (name, view) in st.iter() {
        let converted = if dtype == DType::F16 && view.dtype() == StDtype::F32 {
            convert_view_to_f16(&view, device)?
        } else {
            let native = candle_core::safetensors::Load::load(&view, device)?;
            if native.dtype() == dtype {
                native
            } else {
                let converted = native.to_dtype(dtype)?;
                drop(native);
                converted
            }
        };
        tensors.insert(name.to_string(), converted);
    }
    Ok(tensors)
}

/// Convert an F32 safetensors view directly to an F16 `Tensor` without ever
/// materializing an intermediate F32 `Tensor`/`Vec<f32>`. See module docs.
fn convert_view_to_f16(view: &TensorView<'_>, device: &Device) -> crate::candle::Result<Tensor> {
    let bytes = view.data();
    let mut out: Vec<half::f16> = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.as_chunks::<4>().0 {
        let f = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        out.push(half::f16::from_f32(f));
    }
    Tensor::from_vec(out, view.shape(), device).map_err(crate::candle::GlinerCandleError::from)
}
