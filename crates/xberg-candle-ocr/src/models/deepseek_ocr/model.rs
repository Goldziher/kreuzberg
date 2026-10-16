//! DeepSeek-OCR vision-language model.
//!
//! Vendored from jhqxxx/aha (Apache-2.0). See repo-root ATTRIBUTIONS.md § jhqxxx/aha.
//!
//! Combines a SAM-based vision encoder, CLIP embeddings, and a Qwen2/DeepSeek V2
//! language decoder for optical character recognition with vision-language understanding.
//!
//! # Architecture
//!
//! 1. **Vision Encoder**: SAM ViT-B processes input images into spatial features.
//! 2. **Vision Embedding**: ViT or Qwen2 (v2) refines vision features with cross-modal grounding.
//! 3. **Projector**: Linear layer adapts vision embeddings to language model dimension.
//! 4. **Language Decoder**: Qwen2 or DeepSeek V2 processes fused vision+text embeddings.

use candle_core::{D, IndexOp, Tensor};
use candle_nn::{
    Activation, Conv2d, Embedding, Init, LayerNorm, Linear, Module, RmsNorm, VarBuilder, embedding, linear,
    linear_no_bias,
    ops::{sigmoid, softmax},
};
use candle_transformers::models::segment_anything::LayerNorm2d;

use crate::error::{CandleOcrError, Result};
use crate::vendor::aha::{
    InferenceModel, MultiModalData,
    modules::{
        GateUpDownMLP, NaiveAttention, QKVCatAttention, TwoLinearMLP, eager_attention_forward, get_conv2d,
        get_layer_norm, quick_gelu,
    },
    rope::RoPE,
};

use super::config::{DeepseekOCRConfig, DeepseekV2Config};
use super::utils::{
    attn_masked_fill, index_select_2d, interpolate_bicubic, interpolate_linear_1d, masked_scatter_dim0, nonzero,
    onehot, prepare_causal_attention_mask, topk,
};

/// Patch embedding layer for vision encoder.
#[derive(Debug)]
pub struct PatchEmbed {
    proj: Conv2d,
}

impl PatchEmbed {
    /// Create a new patch embedding layer.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if layer initialization fails.
    pub fn new(
        vb: VarBuilder,
        in_chans: usize,
        embed_dim: usize,
        kernel_size: usize,
        stride: usize,
        padding: usize,
    ) -> Result<Self> {
        let proj = get_conv2d(
            vb.pp("proj"),
            in_chans,
            embed_dim,
            kernel_size,
            padding,
            stride,
            1,
            1,
            true,
        )?;
        Ok(Self { proj })
    }

    /// Forward pass through patch embedding.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if forward computation fails.
    pub fn forward(&self, xs: &Tensor) -> Result<Tensor> {
        let xs = self.proj.forward(xs)?;
        let xs = xs.permute((0, 2, 3, 1))?;
        Ok(xs)
    }
}

/// SAM-style attention block with relative positional encoding.
#[derive(Debug)]
pub struct Attention {
    num_heads: usize,
    qkv: Linear,
    proj: Linear,
    scaling: f64,
    use_rel_pos: bool,
    rel_pos_h: Option<Tensor>,
    rel_pos_w: Option<Tensor>,
}

impl Attention {
    /// Create a new attention layer.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if layer initialization fails.
    pub fn new(
        vb: VarBuilder,
        dim: usize,
        num_heads: usize,
        qkv_bias: bool,
        use_rel_pos: bool,
        input_size: Option<(usize, usize)>,
    ) -> Result<Self> {
        let head_dim = dim / num_heads;
        let scaling = 1.0 / (head_dim as f64).sqrt();
        let qkv = candle_nn::linear_b(dim, dim * 3, qkv_bias, vb.pp("qkv"))?;
        let proj = candle_nn::linear(dim, dim, vb.pp("proj"))?;

        let mut rel_pos_h = None;
        let mut rel_pos_w = None;
        if use_rel_pos {
            let input_size = input_size.ok_or_else(|| {
                CandleOcrError::UnsupportedConfig(
                    "Input size must be provided if using relative positional encoding.".to_string(),
                )
            })?;
            let h_len = 2 * input_size.0 - 1;
            let w_len = 2 * input_size.1 - 1;
            rel_pos_h = Some(vb.get_with_hints((h_len, head_dim), "rel_pos_h", Init::Const(0.))?);
            rel_pos_w = Some(vb.get_with_hints((w_len, head_dim), "rel_pos_w", Init::Const(0.))?);
        }

        Ok(Self {
            num_heads,
            qkv,
            proj,
            scaling,
            use_rel_pos,
            rel_pos_h,
            rel_pos_w,
        })
    }

    fn get_rel_pos(&self, q_size: usize, k_size: usize, rel_pos: &Tensor) -> Result<Tensor> {
        let max_rel_dist = 2 * std::cmp::max(q_size, k_size) - 1;
        let rel_pos_resized = if rel_pos.dim(0)? != max_rel_dist {
            let rel_pos_t = rel_pos
                .to_dtype(candle_core::DType::F32)?
                .t()?
                .unsqueeze(0)?
                .contiguous()?;
            let rel_pos_resized = interpolate_linear_1d(&rel_pos_t, max_rel_dist, None)?;
            rel_pos_resized
                .squeeze(0)?
                .t()?
                .contiguous()?
                .to_dtype(rel_pos.dtype())?
        } else {
            rel_pos.clone()
        };
        let device = rel_pos.device();
        let q_coords = Tensor::arange(0u32, q_size as u32, device)?
            .to_dtype(candle_core::DType::F32)?
            .unsqueeze(D::Minus1)?
            .affine((k_size as f64 / q_size as f64).max(1.0), 0.0)?;
        let k_coords = Tensor::arange(0u32, k_size as u32, device)?
            .to_dtype(candle_core::DType::F32)?
            .unsqueeze(0)?
            .affine((q_size as f64 / k_size as f64).max(1.0), 0.0)?;
        let relative_coords = q_coords
            .broadcast_sub(&k_coords)?
            .affine(1.0, (k_size - 1) as f64)?
            .affine((q_size as f64 / k_size as f64).max(1.0), 0.0)?;
        let relative_coords = relative_coords.to_dtype(candle_core::DType::U32)?.contiguous()?;
        let rel_pos_resized = rel_pos_resized.contiguous()?;
        let res = index_select_2d(&rel_pos_resized, &relative_coords)?;
        Ok(res)
    }

    fn add_decomposed_rel_pos(
        &self,
        q: &Tensor,
        rel_pos_h: &Tensor,
        rel_pos_w: &Tensor,
        q_size: (usize, usize),
        k_size: (usize, usize),
    ) -> Result<(Tensor, Tensor)> {
        let (q_h, q_w) = q_size;
        let (k_h, k_w) = k_size;
        let rh = self.get_rel_pos(q_h, k_h, rel_pos_h)?;
        let rw = self.get_rel_pos(q_w, k_w, rel_pos_w)?;
        let (b, _, dim) = q.dims3()?;
        let r_q = q.reshape((b, q_h, q_w, dim))?.contiguous()?;
        let r_q_ = r_q.unsqueeze(D::Minus2)?;
        let rh_ = rh.unsqueeze(1)?.unsqueeze(0)?;
        let rel_h = r_q_.broadcast_mul(&rh_)?.sum(D::Minus1)?;
        let rw_ = rw.unsqueeze(0)?.unsqueeze(0)?;
        let rel_w = r_q_.broadcast_mul(&rw_)?.sum(D::Minus1)?;
        let rel_h = rel_h.unsqueeze(D::Minus1)?.reshape((b, q_h * q_w, k_h, 1))?;
        let rel_w = rel_w.unsqueeze(D::Minus2)?.reshape((b, q_h * q_w, 1, k_w))?;
        Ok((rel_h, rel_w))
    }

    /// Forward pass through attention.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if forward computation fails.
    pub fn forward(&self, xs: &Tensor) -> Result<Tensor> {
        let (b, h, w, _) = xs.dims4()?;
        let qkv = self
            .qkv
            .forward(xs)?
            .reshape((b, h * w, 3, self.num_heads, ()))?
            .permute((2, 0, 3, 1, 4))?
            .contiguous()?;
        let query_states = qkv.i(0)?.contiguous()?;
        let key_states = qkv.i(1)?.contiguous()?;
        let value_states = qkv.i(2)?.contiguous()?;
        let xs = if self.use_rel_pos {
            let rel_pos_h = self.rel_pos_h.as_ref().ok_or_else(|| {
                CandleOcrError::InferenceFailed("SAM attention: use_rel_pos set but rel_pos_h is missing".into())
            })?;
            let rel_pos_w = self.rel_pos_w.as_ref().ok_or_else(|| {
                CandleOcrError::InferenceFailed("SAM attention: use_rel_pos set but rel_pos_w is missing".into())
            })?;
            let q_reshape = query_states.reshape((b * self.num_heads, h * w, ()))?;
            let (rel_h, rel_w) = self.add_decomposed_rel_pos(&q_reshape, rel_pos_h, rel_pos_w, (h, w), (h, w))?;
            let (_, rel_h_dim1, rel_h_dim2, rel_h_dim3) = rel_h.dims4()?;
            let rel_h = rel_h.reshape((b, self.num_heads, rel_h_dim1, rel_h_dim2, rel_h_dim3))?;
            let (_, rel_w_dim1, rel_w_dim2, rel_w_dim3) = rel_w.dims4()?;
            let rel_w = rel_w.reshape((b, self.num_heads, rel_w_dim1, rel_w_dim2, rel_w_dim3))?;
            let attn_bias =
                rel_h
                    .broadcast_add(&rel_w)?
                    .reshape((b, self.num_heads, rel_h_dim1, rel_h_dim2 * rel_w_dim3))?;
            eager_attention_forward(
                &query_states,
                &key_states,
                &value_states,
                None,
                Some(&attn_bias),
                self.scaling,
            )?
        } else {
            eager_attention_forward(&query_states, &key_states, &value_states, None, None, self.scaling)?
        };
        let xs = xs.reshape((b, h * w, ()))?.reshape((b, h, w, ()))?;
        let xs = self.proj.forward(&xs)?;
        Ok(xs)
    }
}

/// Transformer block with attention and MLP.
#[derive(Debug)]
pub struct Block {
    norm1: LayerNorm,
    attn: Attention,
    norm2: LayerNorm,
    mlp: TwoLinearMLP,
    window_size: usize,
}

impl Block {
    /// Create a new transformer block.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if layer initialization fails.
    pub fn new(
        vb: VarBuilder,
        dim: usize,
        num_heads: usize,
        mlp_ratio: f32,
        qkv_bias: bool,
        eps: f64,
        act: Activation,
        use_rel_pos: bool,
        window_size: usize,
        input_size: Option<(usize, usize)>,
    ) -> Result<Self> {
        let norm1 = get_layer_norm(vb.pp("norm1"), eps, dim, true)?;
        let input_size = if window_size == 0 {
            input_size
        } else {
            Some((window_size, window_size))
        };
        let attn = Attention::new(vb.pp("attn"), dim, num_heads, qkv_bias, use_rel_pos, input_size)?;
        let norm2 = get_layer_norm(vb.pp("norm2"), eps, dim, true)?;
        let mlp_dim = (dim as f32 * mlp_ratio) as usize;
        let mlp = TwoLinearMLP::new(vb.pp("mlp"), dim, mlp_dim, dim, act, true, "lin1", "lin2")?;
        Ok(Self {
            norm1,
            attn,
            norm2,
            mlp,
            window_size,
        })
    }

    fn window_partition(&self, x: &Tensor, window_size: usize) -> Result<(Tensor, (usize, usize))> {
        let (b, h, w, c) = x.dims4()?;
        let pad_h = (window_size - h % window_size) % window_size;
        let pad_w = (window_size - w % window_size) % window_size;
        let x = if pad_h > 0 || pad_w > 0 {
            let x = x.pad_with_zeros(1, 0, pad_h)?;
            x.pad_with_zeros(2, 0, pad_w)?
        } else {
            x.clone()
        };
        let hp = h + pad_h;
        let wp = w + pad_w;
        let x = x.reshape((b, hp / window_size, window_size, wp / window_size, window_size, c))?;
        let windows = x
            .permute((0, 1, 3, 2, 4, 5))?
            .contiguous()?
            .reshape(((), window_size, window_size, c))?;
        Ok((windows, (hp, wp)))
    }

    fn window_unpartition(
        &self,
        windows: &Tensor,
        window_size: usize,
        pad_hw: (usize, usize),
        hw: (usize, usize),
    ) -> Result<Tensor> {
        let (hp, wp) = pad_hw;
        let (h, w) = hw;
        let b = windows.dim(0)? / (hp * wp / window_size / window_size);
        let last_dim = windows.dim(D::Minus1)?;
        let x = windows.reshape(&[
            b,
            hp / window_size,
            wp / window_size,
            window_size,
            window_size,
            last_dim,
        ])?;
        let mut x = x.permute((0, 1, 3, 2, 4, 5))?.contiguous()?.reshape((b, hp, wp, ()))?;
        if hp > h || wp > w {
            x = x.i((.., 0..h, 0..w, ..))?
        }
        Ok(x)
    }

    /// Forward pass through block.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if forward computation fails.
    pub fn forward(&self, xs: &Tensor) -> Result<Tensor> {
        let shortcut = xs.clone();
        let xs = self.norm1.forward(xs)?;
        let xs = if self.window_size > 0 {
            let h = xs.dim(1)?;
            let w = xs.dim(2)?;
            let (x, (hp, wp)) = self.window_partition(&xs, self.window_size)?;
            let x = self.attn.forward(&x)?;
            self.window_unpartition(&x, self.window_size, (hp, wp), (h, w))?
        } else {
            self.attn.forward(&xs)?
        };
        let x = shortcut.add(&xs)?;
        let x = x.add(&self.mlp.forward(&self.norm2.forward(&x)?)?)?;
        Ok(x)
    }
}

/// Neck (feature pyramid) layer.
#[derive(Debug)]
pub struct Neck {
    conv2d_0: Conv2d,
    layernorm_1: LayerNorm2d,
    conv2d_2: Conv2d,
    layernorm_3: LayerNorm2d,
}

impl Neck {
    /// Create a new neck layer.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if layer initialization fails.
    pub fn new(vb: VarBuilder, embed_dim: usize, out_chans: usize) -> Result<Self> {
        let conv2d_0 = get_conv2d(vb.pp("0"), embed_dim, out_chans, 1, 0, 1, 1, 1, false)?;
        let layernorm_1 = LayerNorm2d::new(out_chans, 0.000001, vb.pp("1"))?;
        let conv2d_2 = get_conv2d(vb.pp("2"), out_chans, out_chans, 3, 1, 1, 1, 1, false)?;
        let layernorm_3 = LayerNorm2d::new(out_chans, 0.000001, vb.pp("3"))?;
        Ok(Self {
            conv2d_0,
            layernorm_1,
            conv2d_2,
            layernorm_3,
        })
    }

    /// Forward pass through neck.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if forward computation fails.
    pub fn forward(&self, xs: &Tensor) -> Result<Tensor> {
        let xs = self.conv2d_0.forward(xs)?;
        let xs = self.layernorm_1.forward(&xs)?;
        let xs = self.conv2d_2.forward(&xs)?;
        let xs = self.layernorm_3.forward(&xs)?;
        Ok(xs)
    }
}

/// SAM vision encoder (Vision Transformer with patch embedding).
#[derive(Debug)]
pub struct ImageEncoderViT {
    patch_embed: PatchEmbed,
    pos_embed: Option<Tensor>,
    blocks: Vec<Block>,
    neck: Neck,
    net_2: Conv2d,
    net_3: Conv2d,
}

impl ImageEncoderViT {
    /// Create a new image encoder.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if layer initialization fails.
    pub fn new(
        vb: VarBuilder,
        img_size: usize,
        patch_size: usize,
        in_chans: usize,
        embed_dim: usize,
        depth: usize,
        num_heads: usize,
        mlp_ratio: f32,
        out_chans: usize,
        qkv_bias: bool,
        act: Activation,
        use_abs_pos: bool,
        use_rel_pos: bool,
        window_size: usize,
        global_attn_indexes: Vec<usize>,
        version: usize,
    ) -> Result<Self> {
        let patch_embed = PatchEmbed::new(vb.pp("patch_embed"), in_chans, embed_dim, patch_size, patch_size, 0)?;
        let pos_embed = if use_abs_pos {
            Some(vb.get_with_hints(
                (1, img_size / patch_size, img_size / patch_size, embed_dim),
                "pos_embed",
                Init::Const(0.),
            )?)
        } else {
            None
        };
        let mut blocks = Vec::new();
        let vb_blocks = vb.pp("blocks");
        for i in 0..depth {
            let window_size = if global_attn_indexes.contains(&i) {
                0
            } else {
                window_size
            };

            let block = Block::new(
                vb_blocks.pp(i),
                embed_dim,
                num_heads,
                mlp_ratio,
                qkv_bias,
                1e-6,
                act,
                use_rel_pos,
                window_size,
                Some((img_size / patch_size, img_size / patch_size)),
            )?;
            blocks.push(block);
        }

        let neck = Neck::new(vb.pp("neck"), embed_dim, out_chans)?;
        let net_2 = get_conv2d(vb.pp("net_2"), 256, 512, 3, 1, 2, 1, 1, false)?;
        let net_3_out_c = if version == 2 { 1024 } else { 896 };
        let net_3 = get_conv2d(vb.pp("net_3"), 512, net_3_out_c, 3, 1, 2, 1, 1, false)?;
        Ok(Self {
            patch_embed,
            pos_embed,
            blocks,
            neck,
            net_2,
            net_3,
        })
    }

    fn get_abs_pos_sam(&self, abs_pos: &Tensor, tgt_size: usize) -> Result<Tensor> {
        let src_size = abs_pos.dim(1)?;
        if src_size != tgt_size {
            let old_pos_embed = abs_pos.permute((0, 3, 1, 2))?;
            let new_pos_embed = interpolate_bicubic(&old_pos_embed, (tgt_size, tgt_size), Some(false), Some(true))?;
            let new_pos_embed = new_pos_embed.permute((0, 2, 3, 1))?;
            Ok(new_pos_embed)
        } else {
            Ok(abs_pos.clone())
        }
    }

    /// Forward pass through vision encoder.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if forward computation fails.
    pub fn forward(&self, xs: &Tensor) -> Result<Tensor> {
        let mut x = self.patch_embed.forward(xs)?;
        if let Some(pos_emb) = &self.pos_embed {
            let dim1 = x.dim(1)?;
            let pos = self.get_abs_pos_sam(pos_emb, dim1)?;
            x = x.broadcast_add(&pos)?;
        }
        for blk in &self.blocks {
            x = blk.forward(&x)?;
        }
        let x = x.permute((0, 3, 1, 2))?;
        let x = self.neck.forward(&x)?;
        let x = self.net_2.forward(&x)?;
        let x = self.net_3.forward(&x)?;
        Ok(x)
    }
}

/// CLIP vision embeddings.
#[derive(Debug)]
pub struct CLIPVisionEmbeddings {
    class_embedding: Tensor,
    patch_embedding: Conv2d,
    pos_embeds: Tensor,
    embed_dim: usize,
}

impl CLIPVisionEmbeddings {
    /// Create a new CLIP vision embeddings layer.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if layer initialization fails.
    pub fn new(
        vb: VarBuilder,
        hidden_size: usize,
        image_size: usize,
        patch_size: usize,
        num_channels: usize,
    ) -> Result<Self> {
        let class_embedding = vb.get_with_hints(hidden_size, "class_embedding", Init::Const(0.0))?;

        let patch_embedding = get_conv2d(
            vb.pp("patch_embedding"),
            num_channels,
            hidden_size,
            patch_size,
            0,
            patch_size,
            1,
            1,
            false,
        )?;

        let num_patches = (image_size / patch_size).pow(2);
        let num_positions = num_patches + 1;
        let position_embedding = embedding(num_positions, hidden_size, vb.pp("position_embedding"))?;
        let position_ids = Tensor::arange(0u32, num_positions as u32, vb.device())?;
        let pos_embeds = position_embedding.forward(&position_ids)?;
        Ok(Self {
            class_embedding,
            patch_embedding,
            pos_embeds,
            embed_dim: hidden_size,
        })
    }

    fn get_abs_pos(&self, tgt_size: usize) -> Result<Tensor> {
        let abs_pos_new = self.pos_embeds.clone();
        let (len, dim) = abs_pos_new.dims2()?;
        let src_size = ((len - 1) as f32).sqrt() as usize;
        let tgt_size = (tgt_size as f32).sqrt() as usize;
        let pos_embeds = if src_size != tgt_size {
            let cls_token = abs_pos_new.i(0)?.unsqueeze(0)?;
            let old_pos_embed = abs_pos_new.i(1..)?;
            let old_pos_embed = old_pos_embed
                .reshape((1, src_size, src_size, dim))?
                .permute((0, 3, 1, 2))?
                .contiguous()?;
            let new_pos_embed = interpolate_bicubic(&old_pos_embed, (tgt_size, tgt_size), Some(false), Some(true))?;
            let new_pos_embed = new_pos_embed
                .permute((0, 2, 3, 1))?
                .reshape((tgt_size * tgt_size, dim))?;
            Tensor::cat(&[cls_token, new_pos_embed], 0)?.unsqueeze(0)?
        } else {
            self.pos_embeds.clone()
        };
        Ok(pos_embeds)
    }

    /// Forward pass through CLIP embeddings.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if forward computation fails.
    pub fn forward(&self, pixel_values: &Tensor, patch_embeds: Option<&Tensor>) -> Result<Tensor> {
        let bs = pixel_values.dim(0)?;
        let patch_embeds = match patch_embeds {
            Some(t) => t.clone(),
            None => self.patch_embedding.forward(pixel_values)?,
        };

        let patch_embeds = patch_embeds.flatten(2, D::Minus1)?.transpose(1, 2)?;
        let class_embeds = self.class_embedding.expand((bs, 1, self.embed_dim))?;
        let embeddings = Tensor::cat(&[class_embeds, patch_embeds], 1)?;
        let pos_embeds = self.get_abs_pos(embeddings.dim(1)?)?;
        let embeddings = embeddings.broadcast_add(&pos_embeds)?;
        Ok(embeddings)
    }
}

/// Feed-forward layer without projection.
#[derive(Debug)]
pub struct NoTPFeedForward {
    fc1: Linear,
    fc2: Linear,
}

impl NoTPFeedForward {
    /// Create a new feed-forward layer.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if layer initialization fails.
    pub fn new(vb: VarBuilder, dim: usize, hidden_dim: usize) -> Result<Self> {
        let fc1 = candle_nn::linear(dim, hidden_dim, vb.pp("fc1"))?;
        let fc2 = candle_nn::linear(hidden_dim, dim, vb.pp("fc2"))?;
        Ok(Self { fc1, fc2 })
    }

    /// Forward pass through feed-forward.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if forward computation fails.
    pub fn forward(&self, xs: &Tensor) -> Result<Tensor> {
        let output = self.fc1.forward(xs)?;
        let output = quick_gelu(&output)?;
        let output = self.fc2.forward(&output)?;
        Ok(output)
    }
}

/// NoTP transformer block (pre-norm with attention and MLlayer).
#[derive(Debug)]
pub struct NoTPTransformerBlock {
    self_attn: QKVCatAttention,
    mlp: NoTPFeedForward,
    layer_norm1: LayerNorm,
    layer_norm2: LayerNorm,
}

impl NoTPTransformerBlock {
    /// Create a new NoTP transformer block.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if layer initialization fails.
    pub fn new(vb: VarBuilder, hidden_size: usize, num_heads: usize, ffn_hidden_size: usize, eps: f64) -> Result<Self> {
        let self_attn = QKVCatAttention::new(
            vb.pp("self_attn"),
            hidden_size,
            num_heads,
            None,
            true,
            Some("qkv_proj"),
            Some("out_proj"),
        )?;
        let mlp = NoTPFeedForward::new(vb.pp("mlp"), hidden_size, ffn_hidden_size)?;
        let layer_norm1 = get_layer_norm(vb.pp("layer_norm1"), eps, hidden_size, true)?;
        let layer_norm2 = get_layer_norm(vb.pp("layer_norm2"), eps, hidden_size, true)?;
        Ok(Self {
            self_attn,
            mlp,
            layer_norm1,
            layer_norm2,
        })
    }

    /// Forward pass through block.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if forward computation fails.
    pub fn forward(&self, xs: &Tensor) -> Result<Tensor> {
        let x = self.layer_norm1.forward(xs)?;
        let x = self.self_attn.forward(&x, None, None, None, false, false)?;
        let res = x.add(xs)?;
        let x = self.layer_norm2.forward(&res)?;
        let x = self.mlp.forward(&x)?;
        let out = x.add(&res)?;
        Ok(out)
    }
}

/// NoTP transformer stack.
#[derive(Debug)]
pub struct NoTPTransformer {
    layers: Vec<NoTPTransformerBlock>,
}

impl NoTPTransformer {
    /// Create a new NoTP transformer stack.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if layer initialization fails.
    pub fn new(
        vb: VarBuilder,
        num_layers: usize,
        hidden_size: usize,
        num_heads: usize,
        ffn_hidden_size: usize,
        eps: f64,
    ) -> Result<Self> {
        let mut layers = Vec::new();
        let vb_layers = vb.pp("layers");
        for i in 0..num_layers {
            let blocks = NoTPTransformerBlock::new(vb_layers.pp(i), hidden_size, num_heads, ffn_hidden_size, eps)?;
            layers.push(blocks);
        }
        Ok(Self { layers })
    }

    /// Forward pass through transformer.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if forward computation fails.
    pub fn forward(&self, xs: &Tensor) -> Result<Tensor> {
        let mut x = xs.clone();
        for layer in &self.layers {
            x = layer.forward(&x)?;
        }
        Ok(x)
    }
}

/// ViT (Vision Transformer) model with CLIP-style embeddings.
#[derive(Debug)]
pub struct VitModel {
    embeddings: CLIPVisionEmbeddings,
    transformer: NoTPTransformer,
    pre_layernorm: LayerNorm,
}

impl VitModel {
    /// Create a new ViT model.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if layer initialization fails.
    pub fn new(
        vb: VarBuilder,
        image_size: usize,
        patch_size: usize,
        num_channels: usize,
        num_layers: usize,
        hidden_size: usize,
        num_heads: usize,
        ffn_hidden_size: usize,
        eps: f64,
    ) -> Result<Self> {
        let embeddings =
            CLIPVisionEmbeddings::new(vb.pp("embeddings"), hidden_size, image_size, patch_size, num_channels)?;
        let transformer = NoTPTransformer::new(
            vb.pp("transformer"),
            num_layers,
            hidden_size,
            num_heads,
            ffn_hidden_size,
            eps,
        )?;
        let pre_layernorm = if vb.contains_tensor("pre_layrnorm.weight") {
            get_layer_norm(vb.pp("pre_layrnorm"), eps, hidden_size, true)?
        } else {
            get_layer_norm(vb.pp("pre_layernorm"), eps, hidden_size, true)?
        };
        Ok(Self {
            embeddings,
            transformer,
            pre_layernorm,
        })
    }

    /// Forward pass through ViT.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if forward computation fails.
    pub fn forward(&self, xs: &Tensor, patch_embeds: Option<&Tensor>) -> Result<Tensor> {
        let x = self.embeddings.forward(xs, patch_embeds)?;
        let hidden_states = self.pre_layernorm.forward(&x)?;
        let output = self.transformer.forward(&hidden_states)?;
        Ok(output)
    }
}

/// Mixture-of-Experts (MoE) gating mechanism.
#[derive(Debug)]
pub struct MoEGate {
    top_k: usize,
    routed_scaling_factor: f64,
    scoring_func: String,
    topk_method: String,
    norm_topk_prob: bool,
    linear: Linear,
}

impl MoEGate {
    /// Create a new MoE gate.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if layer initialization fails.
    pub fn new(vb: VarBuilder, config: &DeepseekV2Config) -> Result<Self> {
        let linear = linear_no_bias(config.hidden_size, config.n_routed_experts, vb)?;
        Ok(Self {
            top_k: config.num_experts_per_tok,
            routed_scaling_factor: config.routed_scaling_factor,
            scoring_func: config.scoring_func.clone(),
            topk_method: config.topk_method.clone(),
            norm_topk_prob: config.norm_topk_prob,
            linear,
        })
    }

    /// Forward pass through MoE gate.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if forward computation fails.
    pub fn forward(&self, xs: &Tensor) -> Result<(Tensor, Tensor)> {
        let (_, _, dim) = xs.dims3()?;
        let xs = xs.reshape(((), dim))?;
        let logits = self.linear.forward(&xs)?.to_dtype(candle_core::DType::F32)?;
        let scores = if self.scoring_func == "softmax" {
            softmax(&logits, D::Minus1)?
        } else if self.scoring_func == "sigmoid" {
            sigmoid(&logits)?
        } else {
            return Err(CandleOcrError::UnsupportedConfig(format!(
                "unsupported scoring function for MoE gating: {}",
                self.scoring_func
            )));
        };
        let (topk_idx, topk_weight) = if self.topk_method == "greedy" {
            topk(&scores, self.top_k)?
        } else {
            return Err(CandleOcrError::UnsupportedConfig(format!(
                "unsupported topk_method for MoE gating: {}",
                self.topk_method
            )));
        };
        let topk_weight = if self.top_k > 1 && self.norm_topk_prob {
            topk_weight
                .broadcast_div(&topk_weight.sum_keepdim(D::Minus1)?.affine(1.0, 1e-20)?)?
                .affine(self.routed_scaling_factor, 0.0)?
        } else {
            topk_weight.affine(self.routed_scaling_factor, 0.0)?
        };
        let topk_weight = topk_weight.to_dtype(xs.dtype())?;
        Ok((topk_idx, topk_weight))
    }
}

/// DeepSeek V2 Mixture-of-Experts layer.
#[derive(Debug)]
pub struct DeepseekV2MoE {
    experts: Vec<GateUpDownMLP>,
    gate: MoEGate,
    shared_experts: GateUpDownMLP,
}

impl DeepseekV2MoE {
    /// Create a new DeepSeek V2 MoE layer.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if layer initialization fails.
    pub fn new(vb: VarBuilder, config: &DeepseekV2Config) -> Result<Self> {
        let mut experts = Vec::new();
        let vb_experts = vb.pp("experts");
        for i in 0..config.n_routed_experts {
            let mlp = GateUpDownMLP::new(
                vb_experts.pp(i),
                config.hidden_size,
                config.moe_intermediate_size,
                Activation::Silu,
                false,
                None,
                None,
                None,
            )?;
            experts.push(mlp);
        }
        let gate = MoEGate::new(vb.pp("gate"), config)?;
        let shared_experts = GateUpDownMLP::new(
            vb.pp("shared_experts"),
            config.hidden_size,
            config.moe_intermediate_size * config.n_shared_experts,
            Activation::Silu,
            false,
            None,
            None,
            None,
        )?;
        Ok(Self {
            experts,
            gate,
            shared_experts,
        })
    }

    fn moe_infer(&self, xs: &Tensor, topk_idx: &Tensor, topk_weight: &Tensor) -> Result<Tensor> {
        let expert_mask = onehot(topk_idx, self.experts.len())?
            .permute((2, 1, 0))?
            .to_dtype(candle_core::DType::U32)?;
        let expert_hit = expert_mask.sum((D::Minus1, D::Minus2))?;
        let expert_hit_vec = expert_hit.to_vec1::<u32>()?;
        let expert_hit_vec: Vec<usize> = expert_hit_vec
            .iter()
            .enumerate()
            .filter_map(|(i, &val)| if val > 0 { Some(i) } else { None })
            .collect();
        let mut final_xs = xs.zeros_like()?;
        for i in expert_hit_vec {
            let expert = &self.experts[i];
            let tokens = expert_mask.i(i)?;
            let (topk_id, token_id) = nonzero(&tokens)?;
            let token_id_u32: Vec<u32> = token_id.iter().map(|x| *x as u32).collect();
            let token_id_tensor = Tensor::new(token_id_u32.as_slice(), xs.device())?;
            let select_tokens = xs.index_select(&token_id_tensor, 0)?;
            let select_xs = expert.forward(&select_tokens)?;
            let topk_id_u32: Vec<u32> = topk_id.iter().map(|x| *x as u32).collect();
            let select_weight = topk_weight.index_select(&token_id_tensor, 0)?.gather(
                &Tensor::new(topk_id_u32.as_slice(), xs.device())?.unsqueeze(D::Minus1)?,
                D::Minus1,
            )?;
            let select_xs = select_xs.broadcast_mul(&select_weight)?;
            final_xs = final_xs.index_add(&token_id_tensor, &select_xs, 0)?;
        }
        Ok(final_xs)
    }
}

impl Module for DeepseekV2MoE {
    fn forward(&self, xs: &Tensor) -> candle_core::Result<Tensor> {
        let identity = xs.clone();
        let (_bs, _seq_len, embedding_dim) = xs.dims3()?;
        let (topk_idx, topk_weight) = self
            .gate
            .forward(xs)
            .map_err(|e| candle_core::Error::Msg(format!("{e}")))?;
        let xs = xs.reshape(((), embedding_dim))?;
        let xs = self
            .moe_infer(&xs, &topk_idx, &topk_weight)
            .map_err(|e| candle_core::Error::Msg(format!("{e}")))?;
        let xs = xs.reshape(identity.dims())?;
        let xs_shared_experts = self
            .shared_experts
            .forward(&identity)
            .map_err(|e| candle_core::Error::Msg(format!("{e}")))?;
        let xs = xs.add(&xs_shared_experts)?;
        Ok(xs)
    }
}

/// Projection layer (MoE or MLP).
#[derive(Debug)]
pub enum DeepseekV2Proj {
    /// Mixture-of-Experts variant.
    MOE(DeepseekV2MoE),
    /// Dense MLP variant.
    MLP(GateUpDownMLP),
}

impl DeepseekV2Proj {
    /// Forward pass through projection.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if forward computation fails.
    pub fn forward(&self, xs: &Tensor) -> Result<Tensor> {
        match self {
            DeepseekV2Proj::MLP(model) => model
                .forward(xs)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("MLP forward failed: {e}"))),
            DeepseekV2Proj::MOE(model) => model
                .forward(xs)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("MoE forward failed: {e}"))),
        }
    }
}

/// DeepSeek V2 decoder layer with attention and projection.
#[derive(Debug)]
pub struct DeepseekV2DecoderLayer {
    self_attn: NaiveAttention,
    mlp: DeepseekV2Proj,
    input_layernorm: RmsNorm,
    post_attention_layernorm: RmsNorm,
}

impl DeepseekV2DecoderLayer {
    /// Create a new decoder layer.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if layer initialization fails.
    pub fn new(vb: VarBuilder, config: &DeepseekV2Config, layer_id: usize) -> Result<Self> {
        let self_attn = NaiveAttention::new(
            vb.pp("self_attn"),
            config.hidden_size,
            config.num_attention_heads,
            config.num_key_value_heads,
            None,
            false,
            None,
            None,
            None,
            None,
        )?;
        let mlp = if layer_id >= config.first_k_dense_replace && layer_id.is_multiple_of(config.moe_layer_freq) {
            DeepseekV2Proj::MOE(DeepseekV2MoE::new(vb.pp("mlp"), config)?)
        } else {
            DeepseekV2Proj::MLP(GateUpDownMLP::new(
                vb.pp("mlp"),
                config.hidden_size,
                config.intermediate_size,
                Activation::Silu,
                false,
                None,
                None,
                None,
            )?)
        };
        let input_layernorm = candle_nn::rms_norm(config.hidden_size, config.rms_norm_eps, vb.pp("input_layernorm"))?;
        let post_attention_layernorm = candle_nn::rms_norm(
            config.hidden_size,
            config.rms_norm_eps,
            vb.pp("post_attention_layernorm"),
        )?;
        Ok(Self {
            self_attn,
            mlp,
            input_layernorm,
            post_attention_layernorm,
        })
    }

    /// Forward pass through decoder layer.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if forward computation fails.
    pub fn forward(
        &mut self,
        xs: &Tensor,
        cos: &Tensor,
        sin: &Tensor,
        attention_mask: Option<&Tensor>,
    ) -> Result<Tensor> {
        let residual = xs.clone();
        let xs = self.input_layernorm.forward(xs)?;

        let xs = self
            .self_attn
            .forward_with_cache(&xs, Some(cos), Some(sin), attention_mask, false)?;
        let residual = residual.add(&xs)?;
        let xs = self.post_attention_layernorm.forward(&residual)?;
        let xs = self.mlp.forward(&xs)?;
        let xs = residual.add(&xs)?;
        Ok(xs)
    }

    /// Clear KV cache.
    pub fn clear_kv_cache(&mut self) {
        self.self_attn.clear_kv_cache();
    }
}

/// DeepSeek V2 model with MoE layers.
#[derive(Debug)]
pub struct DeepseekV2Model {
    embed_tokens: Embedding,
    layers: Vec<DeepseekV2DecoderLayer>,
    rope: RoPE,
    norm: RmsNorm,
}

impl DeepseekV2Model {
    /// Create a new DeepSeek V2 model.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if layer initialization fails.
    pub fn new(vb: VarBuilder, config: DeepseekV2Config) -> Result<Self> {
        let embed_tokens = embedding(config.vocab_size, config.hidden_size, vb.pp("embed_tokens"))?;
        let mut layers = Vec::new();
        let vb_layers = vb.pp("layers");
        for i in 0..config.num_hidden_layers {
            let layer = DeepseekV2DecoderLayer::new(vb_layers.pp(i), &config, i)?;
            layers.push(layer);
        }
        let head_dim = config.hidden_size / config.num_attention_heads;
        let rope = RoPE::new(head_dim, 10000.0, vb.device())?;
        let norm = candle_nn::rms_norm(config.hidden_size, config.rms_norm_eps, vb.pp("norm"))?;
        Ok(Self {
            embed_tokens,
            layers,
            rope,
            norm,
        })
    }

    /// Forward pass through model.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if forward computation fails.
    pub fn forward(&mut self, xs: &Tensor, seqlen_offset: usize) -> Result<Tensor> {
        let (_bs, seq_len, _) = xs.dims3()?;
        let (cos, sin) = self.rope.forward(seqlen_offset, seq_len, xs.device())?;

        let attention_mask: Option<Tensor> = {
            if seq_len <= 1 {
                None
            } else {
                Some(prepare_causal_attention_mask(xs.dim(0)?, seq_len, 0, xs.device())?)
            }
        };
        let mut xs = xs.clone();
        for layer in &mut self.layers {
            xs = layer.forward(&xs, &cos, &sin, attention_mask.as_ref())?;
        }
        let xs = self.norm.forward(&xs)?;
        Ok(xs)
    }

    /// Clear KV caches from all layers.
    pub fn clear_kv_cache(&mut self) {
        for layer in &mut self.layers {
            layer.clear_kv_cache();
        }
    }
}

/// Qwen2 decoder wrapped as encoder (vision tower).
#[derive(Debug)]
pub struct Qwen2Decoder2Encoder {
    model: crate::vendor::aha::qwen2::Qwen2Decoder,
    query_768: Embedding,
    query_1024: Embedding,
}

impl Qwen2Decoder2Encoder {
    /// Create a new Qwen2 encoder.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if layer initialization fails.
    pub fn new(vb: VarBuilder) -> Result<Self> {
        let qwen2_config = crate::vendor::aha::qwen2::Qwen2Config {
            vocab_size: 151936,
            hidden_size: 896,
            intermediate_size: 4864,
            num_hidden_layers: 24,
            num_attention_heads: 14,
            num_key_value_heads: 2,
            max_position_embeddings: 131072,
            sliding_window: 32768,
            max_window_layers: 21,
            tie_word_embeddings: true,
            rope_theta: 1000000.0,
            rms_norm_eps: 1e-06,
            use_sliding_window: false,
            hidden_act: Activation::Silu,
        };
        let model = crate::vendor::aha::qwen2::Qwen2Decoder::new(vb.pp("model.model"), &qwen2_config)?;
        let query_768 = embedding(144, qwen2_config.hidden_size, vb.pp("query_768"))?;
        let query_1024 = embedding(256, qwen2_config.hidden_size, vb.pp("query_1024"))?;

        Ok(Self {
            model,
            query_768,
            query_1024,
        })
    }

    /// Forward pass through Qwen2 encoder.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if forward computation fails.
    pub fn forward(&self, xs: &Tensor) -> Result<Tensor> {
        let xs = xs.flatten_from(2)?.transpose(1, 2)?;
        let (bs, n_query, _) = xs.dims3()?;
        let param_img = if n_query == 144 {
            self.query_768.embeddings()
        } else if n_query == 256 {
            self.query_1024.embeddings()
        } else {
            return Err(CandleOcrError::UnsupportedConfig(format!(
                "only support 144/256 seq_len, got {n_query}"
            )));
        };
        let branch_query_imgs = param_img.unsqueeze(0)?.repeat((bs, 1, 1))?;
        let x_combined = Tensor::cat(&[&xs, &branch_query_imgs], 1)?;
        let device = xs.device();
        let token_type_ids = Tensor::cat(
            &[
                Tensor::ones(n_query, candle_core::DType::U32, device)?,
                Tensor::zeros(n_query, candle_core::DType::U32, device)?,
            ],
            0,
        )?
        .unsqueeze(0)?;
        let mask_up = token_type_ids.repeat((n_query, 1))?;
        let mask_down_1 = Tensor::ones((n_query, n_query), candle_core::DType::U32, device)?;
        let mask_down_2 = Tensor::tril2(n_query, candle_core::DType::U32, device)?;
        let mask_down = Tensor::cat(&[mask_down_1, mask_down_2], 1)?;
        let mask = Tensor::cat(&[mask_up, mask_down], 0)?;
        let on_true = mask
            .zeros_like()?
            .unsqueeze(0)?
            .unsqueeze(0)?
            .to_dtype(candle_core::DType::F32)?;
        let attn_mask = attn_masked_fill(&on_true, &mask, f32::NEG_INFINITY)?;
        let xs = self.model.forward_no_cache(&x_combined, Some(&attn_mask), 0)?;
        let xs = xs.narrow(1, n_query, n_query)?;
        Ok(xs)
    }
}

/// Vision model variant enum.
#[derive(Debug)]
pub enum VisionModel {
    /// ViT-based vision tower.
    Vit(VitModel),
    /// Qwen2-based vision tower (DeepSeek-OCR v2).
    Qwen2(Qwen2Decoder2Encoder),
}

/// Complete DeepSeek-OCR model with vision+language fusion.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug)]
pub struct DeepseekOCRModel {
    sam_model: ImageEncoderViT,
    vision_model: VisionModel,
    projector: Linear,
    language_model: DeepseekV2Model,
    image_newline: Option<Tensor>,
    view_separator: Tensor,
    lm_head: Linear,
    stop_token_ids: Vec<u32>,
}

impl DeepseekOCRModel {
    /// Create a new DeepSeek-OCR model.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError`] if weight loading or model initialization fails.
    pub fn new(vb: VarBuilder, config: DeepseekOCRConfig, version: usize) -> Result<Self> {
        let vb_m = vb.pp("model");
        let sam_model = ImageEncoderViT::new(
            vb_m.pp("sam_model"),
            1024,
            16,
            3,
            768,
            12,
            12,
            4.0,
            256,
            true,
            Activation::Gelu,
            true,
            true,
            14,
            config.vision_config.width.sam_vit_b.global_attn_indexes.clone(),
            version,
        )?;
        let has_qwen2_tower = vb_m.contains_tensor("qwen2_model.model.model.layers.0.self_attn.q_proj.weight");
        let (vision_model, image_newline) = if has_qwen2_tower {
            let qwen2 = Qwen2Decoder2Encoder::new(vb_m.pp("qwen2_model"))?;
            (VisionModel::Qwen2(qwen2), None)
        } else {
            let vision_model = VitModel::new(vb_m.pp("vision_model"), 224, 14, 3, 24, 1024, 16, 4096, 1e-5)?;
            let image_newline = vb_m.get_with_hints(1280, "image_newline", Init::Const(0.))?;
            (VisionModel::Vit(vision_model), Some(image_newline))
        };

        let projector = linear(
            config.projector_config.input_dim,
            config.projector_config.n_embed,
            vb_m.pp("projector.layers"),
        )?;

        let view_separator = if vb_m.contains_tensor("view_seperator") {
            vb_m.get_with_hints(1280, "view_seperator", Init::Const(0.))?
        } else {
            vb_m.get_with_hints(1280, "view_separator", Init::Const(0.))?
        };
        let language_model = DeepseekV2Model::new(vb_m, config.language_config.clone())?;
        let lm_head = linear_no_bias(config.hidden_size, config.vocab_size, vb.pp("lm_head"))?;
        let stop_token_ids = vec![config.eos_token_id, config.bos_token_id];
        Ok(Self {
            sam_model,
            vision_model,
            projector,
            language_model,
            image_newline,
            view_separator,
            lm_head,
            stop_token_ids,
        })
    }

    fn forward(
        &mut self,
        input_ids: &Tensor,
        images_ori: Option<&Tensor>,
        image_crop: Option<&Tensor>,
        images_seq_mask: Option<&Tensor>,
        images_spatial_crop: Option<&Tensor>,
        seqlen_offset: usize,
    ) -> Result<Tensor> {
        let mut input_embeds = self.language_model.embed_tokens.forward(input_ids)?;
        if input_ids.dim(1)? > 1
            && let Some(images_ori) = images_ori
            && let Some(image_crop) = image_crop
            && let Some(images_seq_mask) = images_seq_mask
            && let Some(images_spatial_crop) = images_spatial_crop
        {
            let image_num = images_ori.dim(0)?;
            let mut last_crop_num = 0;
            let mut images_in_this_batch = Vec::new();
            for i in 0..image_num {
                let image_ori_i = images_ori.i(i)?.unsqueeze(0)?;
                let global_local_features = if image_crop
                    .sum_all()?
                    .to_dtype(candle_core::DType::F32)?
                    .to_scalar::<f32>()?
                    != 0.0
                {
                    let images_spatial_crop_i = images_spatial_crop.i(i)?;
                    let width_crop_num = images_spatial_crop_i.i(0)?.to_scalar::<u32>()? as usize;
                    let height_crop_num = images_spatial_crop_i.i(1)?.to_scalar::<u32>()? as usize;
                    let crop_num = width_crop_num * height_crop_num;
                    let image_crop_i = image_crop.i(last_crop_num..last_crop_num + crop_num)?;
                    last_crop_num += crop_num;
                    let local_feature_1 = self.sam_model.forward(&image_crop_i)?;
                    let local_features = match &self.vision_model {
                        VisionModel::Vit(vit) => {
                            let local_feature_2 = vit.forward(&image_crop_i, Some(&local_feature_1))?;
                            let local_feature_1 = local_feature_1.flatten(2, 3)?.permute((0, 2, 1))?;
                            let local_feature_2 = local_feature_2.i((.., 1..))?;
                            Tensor::cat(&[local_feature_2, local_feature_1], D::Minus1)?.contiguous()?
                        }
                        VisionModel::Qwen2(qwen2) => qwen2.forward(&local_feature_1)?,
                    };
                    let local_features = self.projector.forward(&local_features)?;
                    let global_features_1 = self.sam_model.forward(&image_ori_i)?;
                    let global_features = match &self.vision_model {
                        VisionModel::Vit(vit) => {
                            let global_features_2 = vit.forward(&image_ori_i, Some(&global_features_1))?;
                            let global_features_1 = global_features_1.flatten(2, 3)?.permute((0, 2, 1))?;
                            let global_features_2 = global_features_2.i((.., 1..))?;
                            Tensor::cat(&[global_features_2, global_features_1], D::Minus1)?
                        }
                        VisionModel::Qwen2(qwen2) => qwen2.forward(&global_features_1)?,
                    };
                    let global_features = self.projector.forward(&global_features)?;
                    let (_, hw, n_dim) = global_features.dims3()?;
                    let (_, hw2, n_dim2) = local_features.dims3()?;
                    let (global_features, local_features) = if let Some(image_newline) = &self.image_newline {
                        let h = (hw as f32).sqrt() as usize;
                        let w = h;
                        let h2 = (hw2 as f32).sqrt() as usize;
                        let w2 = h2;
                        let global_features = global_features.reshape((h, w, n_dim))?;
                        let image_newline = image_newline.unsqueeze(0)?.unsqueeze(0)?;
                        let global_cat = image_newline.expand((h, 1, n_dim))?;
                        let global_features = Tensor::cat(&[&global_features, &global_cat], 1)?;
                        let local_features = local_features
                            .reshape((height_crop_num, width_crop_num, h2, w2, n_dim2))?
                            .permute((0, 2, 1, 3, 4))?
                            .reshape((height_crop_num * h2, width_crop_num * w2, n_dim2))?;
                        let local_cat = image_newline.expand((height_crop_num * h2, 1, n_dim2))?;
                        let local_features = Tensor::cat(&[&local_features, &local_cat], 1)?;
                        (global_features, local_features)
                    } else {
                        (global_features, local_features)
                    };

                    let global_features = global_features.reshape(((), n_dim))?;
                    let local_features = local_features.reshape(((), n_dim2))?;
                    Tensor::cat(&[local_features, global_features, self.view_separator.unsqueeze(0)?], 0)?
                } else {
                    let global_features_1 = self.sam_model.forward(&image_ori_i)?;
                    let global_features = match &self.vision_model {
                        VisionModel::Vit(vit) => {
                            let global_features_2 = vit.forward(&image_ori_i, Some(&global_features_1))?;
                            let global_features_1 = global_features_1.flatten(2, 3)?.permute((0, 2, 1))?;
                            let global_features_2 = global_features_2.i((.., 1..))?;
                            Tensor::cat(&[global_features_2, global_features_1], D::Minus1)?
                        }
                        VisionModel::Qwen2(qwen2) => qwen2.forward(&global_features_1)?,
                    };
                    let global_features = self.projector.forward(&global_features)?;
                    let (_, hw, n_dim) = global_features.dims3()?;
                    let global_features = if let Some(image_newline) = &self.image_newline {
                        let h = (hw as f32).sqrt() as usize;
                        let w = h;
                        let global_features = global_features.reshape((h, w, n_dim))?;
                        let image_newline = image_newline.unsqueeze(0)?.unsqueeze(0)?;
                        let global_cat = image_newline.expand((h, 1, n_dim))?;
                        Tensor::cat(&[&global_features, &global_cat], 1)?
                    } else {
                        global_features
                    };

                    let global_features = global_features.reshape(((), n_dim))?;
                    Tensor::cat(&[global_features, self.view_separator.unsqueeze(0)?], 0)?
                };
                images_in_this_batch.push(global_local_features);
            }
            let images_in_this_batch = Tensor::cat(&images_in_this_batch, 0)?;
            input_embeds = masked_scatter_dim0(&input_embeds, &images_in_this_batch, images_seq_mask)?;
        }
        let outputs = self.language_model.forward(&input_embeds, seqlen_offset)?;
        let seq_len = outputs.dim(1)?;
        let hidden_state = outputs.narrow(1, seq_len - 1, 1)?;
        let logits = self.lm_head.forward(&hidden_state)?;
        Ok(logits)
    }

    /// Clear KV caches from language model.
    pub fn clear_kv_cache(&mut self) {
        self.language_model.clear_kv_cache();
    }
}

impl InferenceModel for DeepseekOCRModel {
    fn forward_initial(&mut self, input_ids: &Tensor, seqlen_offset: usize, data: MultiModalData) -> Result<Tensor> {
        if data.data_vec.len() != 4 {
            return Err(CandleOcrError::InferenceFailed(
                "DeepseekOCR requires exactly 4 data items: images_ori, image_crop, images_seq_mask, images_spatial_crop".to_string(),
            ));
        }
        let images_ori = &data.data_vec[0];
        let image_crop = &data.data_vec[1];
        let images_seq_mask = &data.data_vec[2];
        let images_spatial_crop = &data.data_vec[3];
        self.forward(
            input_ids,
            images_ori.as_ref(),
            image_crop.as_ref(),
            images_seq_mask.as_ref(),
            images_spatial_crop.as_ref(),
            seqlen_offset,
        )
    }

    fn forward_step(&mut self, input_ids: &Tensor, seqlen_offset: usize) -> Result<Tensor> {
        self.forward(input_ids, None, None, None, None, seqlen_offset)
    }

    fn clear_cache(&mut self) {
        self.clear_kv_cache();
    }

    fn stop_token_ids(&self) -> Vec<u32> {
        self.stop_token_ids.clone()
    }
}

#[cfg(test)]
mod tests {
    use candle_core::{DType, Device};

    use super::*;

    /// Minimal JSON snippet for `DeepseekOCRConfig` round-trip testing.
    const MINIMAL_CONFIG_JSON: &str = r#"{
        "language_config": {
            "bos_token_id": 100256,
            "eos_token_id": 100257,
            "first_k_dense_replace": 21,
            "hidden_size": 4096,
            "intermediate_size": 14336,
            "lm_head": true,
            "max_position_embeddings": 4096,
            "moe_intermediate_size": 1408,
            "n_group": 8,
            "n_routed_experts": 64,
            "n_shared_experts": 2,
            "num_attention_heads": 32,
            "num_experts_per_tok": 6,
            "num_hidden_layers": 30,
            "num_key_value_heads": 8,
            "qk_nope_head_dim": 128,
            "qk_rope_head_dim": 64,
            "rm_head": false,
            "topk_group": 4,
            "topk_method": "greedy",
            "torch_dtype": "torch.float32",
            "use_mla": true,
            "v_head_dim": 128,
            "vocab_size": 100264
        },
        "projector_config": {
            "input_dim": 768,
            "model_type": "linear",
            "n_embed": 1280,
            "projector_type": "linear"
        },
        "torch_dtype": "torch.float32",
        "vision_config": {
            "image_size": 1024,
            "mlp_ratio": 4.0,
            "width": {
                "sam_vit_b": {
                    "downsample_channels": [96, 192, 384, 768],
                    "global_attn_indexes": [2, 5, 8, 11],
                    "heads": 12,
                    "layers": 12,
                    "width": 768
                }
            }
        },
        "bos_token_id": 100256,
        "eos_token_id": 100257,
        "first_k_dense_replace": 21,
        "hidden_size": 4096,
        "intermediate_size": 14336,
        "lm_head": true,
        "max_position_embeddings": 4096,
        "moe_intermediate_size": 1408,
        "n_group": 8,
        "n_routed_experts": 64,
        "n_shared_experts": 2,
        "num_attention_heads": 32,
        "num_experts_per_tok": 6,
        "num_hidden_layers": 30,
        "num_key_value_heads": 8,
        "qk_nope_head_dim": 128,
        "qk_rope_head_dim": 64,
        "rm_head": false,
        "topk_group": 4,
        "topk_method": "greedy",
        "use_mla": true,
        "v_head_dim": 128,
        "vocab_size": 100264
    }"#;

    /// Deserializing a representative JSON snippet must succeed and produce
    /// exact field values matching the source document.
    #[test]
    fn config_deserializes_with_exact_field_values() {
        let cfg: DeepseekOCRConfig = serde_json::from_str(MINIMAL_CONFIG_JSON).expect("config must deserialize");

        assert_eq!(cfg.vocab_size, 100264, "vocab_size mismatch");
        assert_eq!(cfg.hidden_size, 4096, "hidden_size mismatch");
        assert_eq!(cfg.eos_token_id, 100257, "eos_token_id mismatch");
        assert_eq!(cfg.bos_token_id, 100256, "bos_token_id mismatch");
        assert_eq!(cfg.projector_config.input_dim, 768, "projector input_dim mismatch");
        assert_eq!(cfg.projector_config.n_embed, 1280, "projector n_embed mismatch");
        assert_eq!(cfg.vision_config.image_size, 1024, "vision image_size mismatch");
        assert_eq!(
            cfg.vision_config.width.sam_vit_b.global_attn_indexes,
            vec![2usize, 5, 8, 11],
            "global_attn_indexes mismatch"
        );
        assert_eq!(cfg.language_config.hidden_size, 4096, "language hidden_size mismatch");
        assert_eq!(cfg.language_config.n_routed_experts, 64, "n_routed_experts mismatch");
        assert_eq!(cfg.language_config.topk_method, "greedy", "topk_method mismatch");
    }

    /// Fields with serde defaults must be populated correctly when absent from JSON.
    #[test]
    fn config_serde_defaults_are_applied_when_fields_absent() {
        let cfg: DeepseekOCRConfig = serde_json::from_str(MINIMAL_CONFIG_JSON).expect("config must deserialize");

        assert_eq!(
            cfg.language_config.moe_layer_freq, 1,
            "default moe_layer_freq should be 1"
        );
        assert!(
            (cfg.language_config.routed_scaling_factor - 1.0_f64).abs() < 1e-9,
            "default routed_scaling_factor should be 1.0"
        );
        assert_eq!(
            cfg.language_config.scoring_func, "softmax",
            "default scoring_func should be 'softmax'"
        );
        assert!(
            !cfg.language_config.norm_topk_prob,
            "default norm_topk_prob should be false"
        );
    }

    /// Both VisionModel variants must compile and be matchable at runtime.
    /// Uses zero-weight VarBuilders to avoid any weight download.
    #[test]
    fn vision_model_enum_vit_and_qwen2_variants_are_dispatchable() {
        let dev = Device::Cpu;
        let vb = VarBuilder::zeros(DType::F32, &dev);

        let vit =
            VitModel::new(vb.pp("vit"), 28, 14, 3, 1, 16, 2, 128, 1e-5).expect("VitModel must construct from zeros");

        let vit_model = VisionModel::Vit(vit);
        let is_vit = matches!(vit_model, VisionModel::Vit(_));
        assert!(is_vit, "VisionModel::Vit variant must match");
        let is_qwen2 = matches!(vit_model, VisionModel::Qwen2(_));
        assert!(!is_qwen2, "VisionModel::Vit must not match Qwen2 arm");
    }

    /// ImageEncoderViT with tiny dimensions must produce a tensor of the
    /// expected shape on a synthetic zero-valued input.
    ///
    /// Setup (version 1):
    ///   img_size=64, patch=16 → 4×4 patch grid
    ///   embed_dim=16, depth=1, out_chans=256 (hardcoded for net_2)
    ///   net_2: 256→512, stride 2 → 2×2
    ///   net_3 (v1): 512→896, stride 2 → 1×1
    ///   Expected output: (1, 896, 1, 1)
    #[test]
    fn image_encoder_vit_forward_produces_expected_output_shape() {
        let dev = Device::Cpu;
        let vb = VarBuilder::zeros(DType::F32, &dev);

        let encoder = ImageEncoderViT::new(
            vb,
            64,
            16,
            3,
            16,
            1,
            2,
            4.0,
            256,
            true,
            Activation::Gelu,
            true,
            false,
            0,
            vec![0usize],
            1,
        )
        .expect("ImageEncoderViT must construct from zeros");

        let input =
            Tensor::zeros((1usize, 3usize, 64usize, 64usize), DType::F32, &dev).expect("synthetic input must allocate");

        let output = encoder.forward(&input).expect("forward must succeed");

        let shape = output.dims().to_vec();
        assert_eq!(
            shape,
            vec![1, 896, 1, 1],
            "ImageEncoderViT output shape mismatch: got {shape:?}"
        );
    }

    /// Version 2 net_3 channel count (1024) is selected when version=2.
    #[test]
    fn image_encoder_vit_version2_produces_1024_output_channels() {
        let dev = Device::Cpu;
        let vb = VarBuilder::zeros(DType::F32, &dev);

        let encoder = ImageEncoderViT::new(
            vb,
            64,
            16,
            3,
            16,
            1,
            2,
            4.0,
            256,
            true,
            Activation::Gelu,
            true,
            false,
            0,
            vec![0usize],
            2,
        )
        .expect("ImageEncoderViT v2 must construct");

        let input = Tensor::zeros((1usize, 3usize, 64usize, 64usize), DType::F32, &dev).expect("input allocation");
        let output = encoder.forward(&input).expect("v2 forward must succeed");

        let shape = output.dims().to_vec();
        assert_eq!(
            shape,
            vec![1, 1024, 1, 1],
            "version=2 net_3 must output 1024 channels, got {shape:?}"
        );
    }

    /// Qwen2Decoder2Encoder must construct from zero-weight VarBuilder without
    /// panicking. Construction validates that the hardcoded Qwen2 config
    /// (hidden_size=896, 24 layers, query embeddings at 144/256) is
    /// architecturally consistent.
    ///
    /// Forward pass is deliberately omitted here: 24-layer Qwen2 on CPU
    /// exceeds the <1 s unit-test budget. Forward is exercised by the
    /// network-gated integration test in deepseek_ocr_integration.rs.
    #[test]
    fn qwen2_decoder2encoder_constructs_from_zeros() {
        let dev = Device::Cpu;
        let vb = VarBuilder::zeros(DType::F32, &dev);

        let encoder = Qwen2Decoder2Encoder::new(vb);
        assert!(
            encoder.is_ok(),
            "Qwen2Decoder2Encoder must construct without errors: {:?}",
            encoder.err()
        );
    }

    /// Qwen2Decoder2Encoder rejects sequence lengths other than 144 and 256.
    ///
    /// This verifies the guard at the top of `Qwen2Decoder2Encoder::forward`.
    /// We use a zero-weight encoder and an intentionally-wrong token count to
    /// confirm the error path is covered without a full forward pass.
    #[test]
    fn qwen2_decoder2encoder_forward_rejects_invalid_seq_len() {
        let dev = Device::Cpu;
        let vb = VarBuilder::zeros(DType::F32, &dev);

        let encoder = Qwen2Decoder2Encoder::new(vb).expect("Qwen2Decoder2Encoder must construct");

        let bad_input =
            Tensor::zeros((1usize, 896usize, 2usize, 2usize), DType::F32, &dev).expect("bad input allocation");

        let result = encoder.forward(&bad_input);
        assert!(
            result.is_err(),
            "forward must reject seq_len=4 (not 144 or 256), but returned Ok"
        );

        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("only support 144/256"),
            "error message must describe the valid seq lengths, got: {err_msg}"
        );
    }
}
