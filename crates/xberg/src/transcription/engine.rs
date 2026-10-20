//! Whisper ONNX inference engine.
//!
//! Loads three ONNX sessions (encoder, decoder, decoder_with_past) and runs
//! autoregressive greedy decoding to produce a transcript string from 16 kHz
//! mono f32 PCM audio.
//!
//! # Thread safety
//!
//! `WhisperEngine` is `Send + Sync` — the `ort::Session::run()` API takes
//! `&mut self` as an API-level constraint but its implementation delegates to
//! `run_inner(&self)`, which is thread-safe per the ONNX Runtime documentation.
//! We use the same `&self`-cast pattern established in `reranking/engine.rs`.
//!
//! # Architecture
//!
//! For each 30-second audio chunk the engine:
//! 1. Computes a log-mel spectrogram (shape `[1, n_mels, 3000]`) using `mel_spec`.
//! 2. Runs the encoder to obtain cross-attention key-value states.
//! 3. Seeds the decoder with a prompt
//!    `[<|startoftranscript|>, <|{lang}|>, <|transcribe|>, <|notimestamps|>]`
//!    (the trailing `<|notimestamps|>` token is omitted when timestamps are
//!    requested — see [`build_decoder_prompt_tokens`]).
//! 4. Greedily generates tokens by running `decoder` (step 0) and then
//!    `decoder_with_past` (steps 1…N), accumulating KV-cache tensors.
//! 5. Stops on `<|endoftext|>` or a configurable max-token limit (448).
//! 6. When timestamps were requested, pairs up the emitted `<|x.xx|>` tokens
//!    into per-segment `(start_ms, end_ms, text)` triples (see
//!    [`parse_timestamped_segments`]) and decodes each segment's text tokens
//!    separately; otherwise decodes the whole token stream as one segment.
//!    Whisper timestamp tokens are ordinary (non-`special`) vocabulary
//!    entries, so `Tokenizer::decode(.., skip_special_tokens=true)` does
//!    *not* strip them — they must be parsed out explicitly before decoding
//!    or they leak into the transcript as literal `<|x.xx|>` text.

use std::collections::HashMap;

use mel_spec::mel::{BatchLogMelConfig, BatchLogMelSpectrogram};
use ndarray::{Array2, Array3};
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::Value;
use thiserror::Error;
use tokenizers::Tokenizer;

use crate::transcription::decode::PcmAudio;
use crate::transcription::model::WhisperModelPaths;

/// Whisper operates on 30-second windows at 16 kHz.
const WHISPER_SAMPLE_RATE: usize = 16_000;
/// 30-second window in samples.
const WHISPER_CHUNK_SAMPLES: usize = WHISPER_SAMPLE_RATE * 30;
/// Number of STFT frames in a 30-second window (480000 / 160).
const WHISPER_N_FRAMES: usize = 3_000;
/// Whisper STFT n_fft.
const WHISPER_N_FFT: usize = 400;
/// Whisper STFT hop length.
const WHISPER_HOP_LENGTH: usize = 160;
/// Whisper decoder context length — the maximum number of tokens the decoder
/// can attend over, **including** the prompt.
///
/// Whisper's `n_text_ctx` / `max_target_positions` is 448 for every published
/// export, and the decoder's position-embedding table has exactly that many
/// rows. Feeding a token at index ≥ 448 therefore slices an empty position
/// range, whose empty sequence then fails the self-attention `Reshape`
/// (GH#1944). The generated-token budget is this value minus the prompt
/// length, not this value on its own. ~keep
const WHISPER_MAX_TOKENS: usize = 448;
/// Milliseconds represented by one increment of a Whisper timestamp token ID.
///
/// Whisper's timestamp vocabulary is a contiguous run of IDs starting at
/// `<|0.00|>`; each successive ID represents 20 ms later, up to `<|30.00|>`
/// (1501 tokens spanning the 30 s chunk window).
const WHISPER_TIMESTAMP_TICK_MS: u32 = 20;

/// Number of tokens the greedy loop may generate for a prompt of `prompt_len`
/// tokens, so that prompt + generated never exceeds the decoder context.
fn decoder_token_budget(prompt_len: usize) -> usize {
    WHISPER_MAX_TOKENS.saturating_sub(prompt_len)
}

/// Errors that can occur during Whisper inference.
#[derive(Debug, Error)]
#[cfg_attr(alef, alef(skip))]
pub enum TranscriptionError {
    /// ONNX Runtime returned an error during session build or inference.
    #[error("ONNX Runtime error: {0}")]
    Ort(#[from] ort::Error),
    /// Tokenizer load or decode failed.
    #[error("tokenizer error: {0}")]
    Tokenizer(String),
    /// A tensor shape was not as expected.
    #[error("tensor shape error: {0}")]
    Shape(String),
    /// An I/O error occurred (model file missing, etc.).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// The encoder produced no output.
    #[error("model produced no output")]
    NoOutput,
    /// A required special token was missing from the tokenizer vocabulary.
    #[error("special token not found in tokenizer: {0}")]
    MissingSpecialToken(String),
    /// The mel spectrogram computation failed.
    #[error("mel spectrogram error: {0}")]
    MelSpec(String),
}

/// A transcribed segment: `(start_ms, end_ms, text)`.
type Segment = (u32, u32, String);

/// Outcome of walking the recording in 30-second windows with per-window error
/// isolation: `(segments, warnings, first_error, any_success)`.
type ChunkIsolationOutcome = (
    Vec<Segment>,
    Vec<TranscriptionWindowWarning>,
    Option<TranscriptionError>,
    bool,
);

/// A non-fatal failure while transcribing one 30-second window.
///
/// A window that the decoder cannot process is skipped rather than aborting
/// the whole recording (GH#1944); the remaining windows still produce text.
/// Callers surface these as document-level `ProcessingWarning`s.
#[derive(Debug, Clone)]
#[cfg_attr(alef, alef(skip))]
pub(crate) struct TranscriptionWindowWarning {
    /// Offset of the failed window from the start of the audio, in milliseconds.
    pub start_ms: u32,
    /// End offset of the failed window, in milliseconds.
    pub end_ms: u32,
    /// The decoder error for this window.
    pub message: String,
}

/// Token IDs for the four-token Whisper decode prompt.
#[derive(Debug, Clone)]
struct SpecialTokens {
    /// `<|startoftranscript|>`
    start_of_transcript: u32,
    /// `<|endoftext|>`
    end_of_text: u32,
    /// `<|transcribe|>`
    transcribe: u32,
    /// `<|notimestamps|>`
    no_timestamps: u32,
    /// `<|0.00|>` — the first (lowest-ID) timestamp token. Every ID at or
    /// above this one is a timestamp token; `id - timestamp_begin` ticks of
    /// [`WHISPER_TIMESTAMP_TICK_MS`] gives the offset in milliseconds.
    timestamp_begin: u32,
    /// Language token IDs, keyed by ISO-639-1 code (e.g. `"en"` → token id).
    language_ids: HashMap<String, u32>,
}

impl SpecialTokens {
    /// Resolve all special tokens from the tokenizer.
    ///
    /// Language codes follow Whisper's naming convention: `<|en|>`, `<|de|>`, …
    fn resolve(tokenizer: &Tokenizer) -> Result<Self, TranscriptionError> {
        let resolve = |token: &str| -> Result<u32, TranscriptionError> {
            tokenizer
                .token_to_id(token)
                .ok_or_else(|| TranscriptionError::MissingSpecialToken(token.to_string()))
        };

        let start_of_transcript = resolve("<|startoftranscript|>")?;
        let end_of_text = resolve("<|endoftext|>")?;
        let transcribe = resolve("<|transcribe|>")?;
        let no_timestamps = resolve("<|notimestamps|>")?;
        let timestamp_begin = resolve("<|0.00|>")?;

        let language_codes = [
            "af", "am", "ar", "as", "az", "ba", "be", "bg", "bn", "bo", "br", "bs", "ca", "cs", "cy", "da", "de", "el",
            "en", "es", "et", "eu", "fa", "fi", "fo", "fr", "gl", "gu", "ha", "haw", "he", "hi", "hr", "ht", "hu",
            "hy", "id", "is", "it", "ja", "jw", "ka", "kk", "km", "kn", "ko", "la", "lb", "lo", "lt", "lv", "mg", "mi",
            "mk", "ml", "mn", "mr", "ms", "mt", "my", "ne", "nl", "nn", "no", "oc", "pa", "pl", "ps", "pt", "ro", "ru",
            "sa", "sd", "si", "sk", "sl", "sn", "so", "sq", "sr", "su", "sv", "sw", "ta", "te", "tg", "th", "tk", "tl",
            "tr", "tt", "uk", "ur", "uz", "vi", "yi", "yo", "zh",
        ];

        let mut language_ids = HashMap::new();
        for code in language_codes {
            let token = format!("<|{code}|>");
            if let Some(id) = tokenizer.token_to_id(&token) {
                language_ids.insert(code.to_string(), id);
            }
        }

        tracing::debug!(
            start_of_transcript,
            end_of_text,
            transcribe,
            no_timestamps,
            timestamp_begin,
            language_count = language_ids.len(),
            "Resolved Whisper special tokens",
        );

        Ok(Self {
            start_of_transcript,
            end_of_text,
            transcribe,
            no_timestamps,
            timestamp_begin,
            language_ids,
        })
    }

    /// Look up the language token ID for `lang` (e.g. `"en"`).
    ///
    /// Falls back to English when the language code is unrecognised.
    fn language_id(&self, lang: &str) -> u32 {
        if let Some(&id) = self.language_ids.get(lang) {
            return id;
        }
        tracing::warn!(language = lang, "Unknown language code; falling back to English",);
        *self.language_ids.get("en").unwrap_or(&self.start_of_transcript)
    }
}

/// Build the four (or three) token Whisper decoder prompt.
///
/// The canonical Whisper prompt is
/// `[<|startoftranscript|>, <|{lang}|>, <|transcribe|>, <|notimestamps|>]`.
/// When `timestamps` is `true`, the trailing `no_timestamps` token is omitted
/// so the model is free to emit `<|x.xx|>` timestamp tokens in its output
/// instead of being forced to suppress them.
pub fn build_decoder_prompt_tokens(
    start_of_transcript: u32,
    lang_id: u32,
    transcribe: u32,
    no_timestamps: u32,
    timestamps: bool,
) -> Vec<i64> {
    let mut prompt: Vec<i64> = vec![start_of_transcript as i64, lang_id as i64, transcribe as i64];
    if !timestamps {
        prompt.push(no_timestamps as i64);
    }
    prompt
}

/// Convert a raw Whisper timestamp token ID to a millisecond offset from the
/// start of the 30-second chunk it was decoded in.
///
/// `token_id` must be `>= timestamp_begin_id`; IDs below that are ordinary
/// vocabulary tokens, not timestamps.
pub fn timestamp_token_to_ms(token_id: u32, timestamp_begin_id: u32) -> u32 {
    token_id.saturating_sub(timestamp_begin_id) * WHISPER_TIMESTAMP_TICK_MS
}

/// Split a generated token stream into per-segment `(start_id, end_id, text_token_ids)`
/// triples using Whisper's timestamp-token convention.
///
/// Whisper (when not suppressing timestamps via `<|notimestamps|>`) emits
/// timestamp tokens in pairs bracketing each spoken segment:
/// `<|t0|> tok tok tok <|t1|>`, with the next segment's opening timestamp
/// often following immediately. This function pairs up consecutive timestamp
/// tokens (IDs `>= timestamp_begin_id`) and returns the plain-vocabulary
/// tokens found strictly between each pair as that segment's text tokens.
///
/// Tokens before the first timestamp token, and a trailing unpaired
/// timestamp token (generation stopped before the model closed its final
/// segment) together with any text after it, are dropped — there is no
/// complete `(start, end)` pair to report them under.
///
/// Returns an empty `Vec` when `token_ids` contains no timestamp tokens at
/// all (e.g. `timestamps` was `false`, so `<|notimestamps|>` suppressed
/// them) — callers should fall back to treating the whole sequence as one
/// untimed block of text in that case.
pub fn parse_timestamped_segments(token_ids: &[u32], timestamp_begin_id: u32) -> Vec<(u32, u32, Vec<u32>)> {
    let mut segments = Vec::new();
    let mut open_start: Option<u32> = None;
    let mut current_text: Vec<u32> = Vec::new();

    for &token in token_ids {
        if token >= timestamp_begin_id {
            match open_start {
                None => {
                    open_start = Some(token);
                    current_text.clear();
                }
                Some(start) => {
                    segments.push((start, token, std::mem::take(&mut current_text)));
                    open_start = None;
                }
            }
        } else if open_start.is_some() {
            current_text.push(token);
        }
    }

    segments
}

/// Build an ONNX Runtime session from a model file path.
///
/// Uses the same builder configuration as `reranking/mod.rs`:
/// all-graph optimization, intra-thread budget from the concurrency
/// resolver, and the bundled ORT execution providers.
fn build_session(path: &std::path::Path) -> Result<Session, TranscriptionError> {
    crate::ort_discovery::ensure_ort_available();
    let thread_budget = crate::core::config::concurrency::resolve_thread_budget(None);

    let mut builder = Session::builder(crate::ort_discovery::ort_environment()?)?;
    builder = builder
        .with_optimization_level(GraphOptimizationLevel::All)
        .map_err(|e| ort::Error::new(e.message()))?;
    builder = builder
        .with_intra_threads(thread_budget)
        .map_err(|e| ort::Error::new(e.message()))?;
    builder = builder
        .with_inter_threads(1)
        .map_err(|e| ort::Error::new(e.message()))?;
    builder = crate::ort_discovery::apply_execution_providers(builder, None)?;
    let session = builder.commit_from_file(path)?;
    Ok(session)
}

/// Whisper ONNX inference engine.
///
/// Holds three sessions (encoder, decoder, decoder_with_past) and a tokenizer.
/// Call [`WhisperEngine::transcribe`] to produce a transcript from PCM audio.
#[cfg_attr(alef, alef(skip))]
pub struct WhisperEngine {
    encoder: Session,
    decoder: Session,
    decoder_with_past: Session,
    tokenizer: Tokenizer,
    special_tokens: SpecialTokens,
    mel_frontend: BatchLogMelSpectrogram,
    n_mels: u32,
}

#[allow(unsafe_code)]
unsafe impl Send for WhisperEngine {}
#[allow(unsafe_code)]
unsafe impl Sync for WhisperEngine {}

impl WhisperEngine {
    /// Load a Whisper engine from the given model paths.
    ///
    /// Builds three ONNX sessions and resolves special-token IDs from the
    /// bundled tokenizer. This is a blocking, CPU-heavy operation — callers
    /// on an async runtime should wrap it in `tokio::task::spawn_blocking`.
    pub fn load(paths: &WhisperModelPaths) -> Result<Self, TranscriptionError> {
        tracing::debug!(
            encoder = ?paths.encoder,
            decoder = ?paths.decoder,
            decoder_with_past = ?paths.decoder_with_past,
            n_mels = paths.n_mels,
            "Loading WhisperEngine sessions",
        );

        let encoder = build_session(&paths.encoder)?;
        let decoder = build_session(&paths.decoder)?;
        let decoder_with_past = build_session(&paths.decoder_with_past)?;

        tracing::debug!(
            inputs = ?encoder.inputs().iter().map(|i| i.name().to_string()).collect::<Vec<_>>(),
            outputs = ?encoder.outputs().iter().map(|o| o.name().to_string()).collect::<Vec<_>>(),
            "Encoder session I/O",
        );
        tracing::debug!(
            inputs = ?decoder.inputs().iter().map(|i| i.name().to_string()).collect::<Vec<_>>(),
            outputs = ?decoder.outputs().iter().map(|o| o.name().to_string()).collect::<Vec<_>>(),
            "Decoder (no past) session I/O",
        );
        tracing::debug!(
            inputs = ?decoder_with_past.inputs().iter().map(|i| i.name().to_string()).collect::<Vec<_>>(),
            outputs = ?decoder_with_past.outputs().iter().map(|o| o.name().to_string()).collect::<Vec<_>>(),
            "Decoder (with past) session I/O",
        );

        let tokenizer =
            Tokenizer::from_file(&paths.tokenizer).map_err(|e| TranscriptionError::Tokenizer(e.to_string()))?;

        let special_tokens = SpecialTokens::resolve(&tokenizer)?;

        let mel_frontend = BatchLogMelSpectrogram::new(BatchLogMelConfig {
            sample_rate: WHISPER_SAMPLE_RATE,
            n_fft: WHISPER_N_FFT,
            win_length: WHISPER_N_FFT,
            hop_length: WHISPER_HOP_LENGTH,
            n_mels: paths.n_mels as usize,
            f_min: 0.0,
            f_max: None,
            htk: false,
            norm: true,
            preemphasis: 0.0,
            center: true,
            log_zero_guard: 1e-10_f32,
            pad_to: 0,
            normalize_per_feature: false,
        })
        .map_err(|e| TranscriptionError::MelSpec(e.to_string()))?;

        Ok(Self {
            encoder,
            decoder,
            decoder_with_past,
            tokenizer,
            special_tokens,
            mel_frontend,
            n_mels: paths.n_mels,
        })
    }

    /// Transcribe PCM audio to a string.
    ///
    /// The `pcm` input **must** already be 16 kHz mono f32 as produced by
    /// [`crate::transcription::decode::decode_audio_to_pcm`]. Passing audio
    /// at a different sample rate will produce garbage output without an error.
    ///
    /// For audio longer than 30 seconds the input is split into 30-second
    /// chunks; each chunk is transcribed independently and the results are
    /// joined with a single space.
    ///
    /// When `timestamps` is `true` the decoder prompt omits `<|notimestamps|>`,
    /// letting the model emit `<|x.xx|>`-style timestamp tokens in its raw
    /// output. This convenience method discards that timing information and
    /// returns only the concatenated transcript text — use
    /// [`WhisperEngine::transcribe_segments`] to get per-segment start/end
    /// timestamps. When `timestamps` is `false` (the default), the prompt
    /// includes `<|notimestamps|>` and there is only ever one segment per
    /// chunk, so the two methods produce the same text.
    pub fn transcribe(
        &self,
        pcm: &PcmAudio,
        language: Option<&str>,
        timestamps: bool,
    ) -> Result<String, TranscriptionError> {
        let segments = self.transcribe_segments(pcm, language, timestamps)?;
        Ok(segments
            .into_iter()
            .map(|(_, _, text)| text)
            .collect::<Vec<_>>()
            .join(" "))
    }

    /// Transcribe PCM audio to a sequence of `(start_ms, end_ms, text)` segments.
    ///
    /// The `pcm` input **must** already be 16 kHz mono f32 as produced by
    /// [`crate::transcription::decode::decode_audio_to_pcm`]. Passing audio
    /// at a different sample rate will produce garbage output without an error.
    ///
    /// For audio longer than 30 seconds the input is split into 30-second
    /// chunks; each chunk is transcribed independently. Timestamps in the
    /// returned segments are absolute — measured from the start of the full
    /// `pcm` buffer, not from the start of each chunk.
    ///
    /// When `timestamps` is `false`, each chunk with non-empty text produces
    /// exactly one segment spanning the chunk's full duration (there is no
    /// finer-grained timing available without `<|x.xx|>` tokens in the
    /// decoder output).
    pub fn transcribe_segments(
        &self,
        pcm: &PcmAudio,
        language: Option<&str>,
        timestamps: bool,
    ) -> Result<Vec<(u32, u32, String)>, TranscriptionError> {
        let (segments, _warnings) = self.transcribe_segments_with_warnings(pcm, language, timestamps)?;
        Ok(segments)
    }

    /// Like [`WhisperEngine::transcribe_segments`], but also returns a warning
    /// for every 30-second window the decoder could not process.
    ///
    /// A single undecodable window must not discard the whole recording
    /// (GH#1944): its error is collected as a warning and the remaining windows
    /// still contribute text. An error is returned only when *every* window
    /// failed, so a systemic failure is never silently rendered as an empty
    /// transcript.
    pub(crate) fn transcribe_segments_with_warnings(
        &self,
        pcm: &PcmAudio,
        language: Option<&str>,
        timestamps: bool,
    ) -> Result<(Vec<Segment>, Vec<TranscriptionWindowWarning>), TranscriptionError> {
        if pcm.samples.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        let lang = language.unwrap_or("en");
        let ms_per_sample = 1000_f64 / pcm.sample_rate_hz.max(1) as f64;

        let (segments, warnings, first_error, any_success) =
            transcribe_chunks_with_isolation(&pcm.samples, ms_per_sample, |chunk, duration| {
                self.transcribe_chunk(chunk, lang, timestamps, duration)
            });

        if !any_success && let Some(error) = first_error {
            return Err(error);
        }
        Ok((segments, warnings))
    }

    /// Transcribe a single chunk of PCM (at most 30 seconds of audio).
    ///
    /// The chunk is zero-padded to exactly [`WHISPER_CHUNK_SAMPLES`] samples
    /// so that the encoder always receives a `[1, n_mels, 3000]` tensor.
    ///
    /// Returns chunk-relative `(start_ms, end_ms, text)` segments — relative
    /// to the start of *this* chunk, not the overall PCM buffer. When
    /// `timestamps` is `false`, or the model fails to emit a complete
    /// timestamp pair, this returns a single segment spanning
    /// `0..chunk_duration_ms`.
    fn transcribe_chunk(
        &self,
        chunk: &[f32],
        lang: &str,
        timestamps: bool,
        chunk_duration_ms: u32,
    ) -> Result<Vec<(u32, u32, String)>, TranscriptionError> {
        let padded = if chunk.len() == WHISPER_CHUNK_SAMPLES {
            chunk.to_vec()
        } else {
            let mut v = chunk.to_vec();
            v.resize(WHISPER_CHUNK_SAMPLES, 0.0_f32);
            v
        };

        let mel_flat = self.compute_log_mel(&padded)?;

        let encoder_hidden_states = self.run_encoder(mel_flat)?;

        let lang_id = self.special_tokens.language_id(lang);
        let prompt = build_decoder_prompt_tokens(
            self.special_tokens.start_of_transcript,
            lang_id,
            self.special_tokens.transcribe,
            self.special_tokens.no_timestamps,
            timestamps,
        );

        let token_ids = self.greedy_decode(prompt, &encoder_hidden_states)?;

        if timestamps {
            let timed = parse_timestamped_segments(&token_ids, self.special_tokens.timestamp_begin);
            if !timed.is_empty() {
                let mut out = Vec::with_capacity(timed.len());
                for (start_id, end_id, text_tokens) in timed {
                    let text = self
                        .tokenizer
                        .decode(&text_tokens, true)
                        .map_err(|e| TranscriptionError::Tokenizer(e.to_string()))?;
                    let start_ms = timestamp_token_to_ms(start_id, self.special_tokens.timestamp_begin);
                    let end_ms = timestamp_token_to_ms(end_id, self.special_tokens.timestamp_begin);
                    out.push((start_ms, end_ms, text.trim().to_string()));
                }
                return Ok(out);
            }
            // Fall through: the model didn't emit a complete timestamp pair
            // (e.g. it hit WHISPER_MAX_TOKENS before closing a segment).
            // Decode everything, dropping any stray timestamp tokens
            // ourselves — they are ordinary vocabulary entries in the
            // tokenizer (not marked `special`), so `skip_special_tokens`
            // would not filter them out.
        }

        let timestamp_begin = self.special_tokens.timestamp_begin;
        let filtered_ids: Vec<u32> = token_ids.into_iter().filter(|&t| t < timestamp_begin).collect();

        let text = self
            .tokenizer
            .decode(&filtered_ids, true)
            .map_err(|e| TranscriptionError::Tokenizer(e.to_string()))?;
        let text = text.trim().to_string();

        if text.is_empty() {
            Ok(Vec::new())
        } else {
            Ok(vec![(0, chunk_duration_ms, text)])
        }
    }

    /// Compute the Whisper log-mel spectrogram for a 30-second padded chunk.
    ///
    /// Returns a flat Vec<f32> laid out row-major `[n_mels, 3000]` ready for
    /// wrapping into a `[1, n_mels, 3000]` tensor.
    ///
    /// We use `compute_flat()` (which returns raw `Vec<f32>`) to avoid touching
    /// mel_spec's ndarray 0.16 types — ort requires ndarray 0.17 types.
    fn compute_log_mel(&self, samples: &[f32]) -> Result<Vec<f32>, TranscriptionError> {
        const LN10: f32 = std::f32::consts::LN_10;

        let output = self
            .mel_frontend
            .compute_flat(samples)
            .map_err(|e| TranscriptionError::MelSpec(e.to_string()))?;

        let n_mels = output.rows;
        let n_frames = output.cols;
        let mut flat = output.data;

        for v in flat.iter_mut() {
            *v /= LN10;
        }

        let max_val = flat.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let floor = max_val - 8.0_f32;
        for v in flat.iter_mut() {
            *v = (v.max(floor) + 4.0_f32) / 4.0_f32;
        }

        let target_frames = WHISPER_N_FRAMES;
        let log_mel_flat = if n_frames == target_frames {
            flat
        } else if n_frames < target_frames {
            let mut padded = vec![0.0_f32; n_mels * target_frames];
            for mel_idx in 0..n_mels {
                let src_start = mel_idx * n_frames;
                let dst_start = mel_idx * target_frames;
                padded[dst_start..dst_start + n_frames].copy_from_slice(&flat[src_start..src_start + n_frames]);
            }
            padded
        } else {
            let mut trimmed = vec![0.0_f32; n_mels * target_frames];
            for mel_idx in 0..n_mels {
                let src_start = mel_idx * n_frames;
                let dst_start = mel_idx * target_frames;
                trimmed[dst_start..dst_start + target_frames]
                    .copy_from_slice(&flat[src_start..src_start + target_frames]);
            }
            trimmed
        };

        Ok(log_mel_flat)
    }

    /// Run the encoder and return the `last_hidden_state` value.
    fn run_encoder(&self, mel_flat: Vec<f32>) -> Result<Value, TranscriptionError> {
        let n_mels = self.n_mels as usize;
        let mel_nd = Array3::from_shape_vec((1, n_mels, WHISPER_N_FRAMES), mel_flat)
            .map_err(|e| TranscriptionError::Shape(e.to_string()))?;

        let mel_value: Value = Value::from_array(mel_nd)?.into();

        #[allow(unsafe_code)]
        let outputs = unsafe {
            let ptr = &self.encoder as *const Session as *mut Session;
            (*ptr).run(ort::inputs!["input_features" => mel_value])
        }?;

        let encoder_output_name = self
            .encoder
            .outputs()
            .first()
            .map(|o| o.name().to_string())
            .unwrap_or_else(|| "last_hidden_state".to_string());

        let hidden: Value = outputs
            .into_iter()
            .find(|(name, _)| *name == "last_hidden_state" || *name == encoder_output_name)
            .map(|(_, v)| v)
            .ok_or(TranscriptionError::NoOutput)?;

        Ok(hidden)
    }

    /// Run the greedy decode loop.
    ///
    /// Returns the token IDs produced **after** the prompt (i.e. the transcribed
    /// tokens), excluding the final `<|endoftext|>` token.
    fn greedy_decode(&self, prompt: Vec<i64>, encoder_hidden_states: &Value) -> Result<Vec<u32>, TranscriptionError> {
        let names = self.decoder_io_names();
        // The position-embedding table has WHISPER_MAX_TOKENS rows (0..448); a
        // token fed at index ≥ that is out of range, slices an empty position
        // range, and crashes the self-attention Reshape (GH#1944). Reserve the
        // prompt's positions from the context budget so the loop can never feed
        // a token past the table. `max_new_tokens` includes step 0's token. ~keep
        let max_new_tokens = decoder_token_budget(prompt.len());
        if max_new_tokens == 0 {
            return Ok(Vec::new());
        }
        let outputs = self.run_decoder_prompt(prompt, encoder_hidden_states, &names)?;
        let first_token = decoder_output_token(&outputs, &names.logits_output)?;
        if first_token == self.special_tokens.end_of_text {
            return Ok(Vec::new());
        }
        let cache = WhisperDecoderCache::from_prompt(outputs, &names.logits_output);
        self.decode_with_past(first_token, encoder_hidden_states, &names, cache, max_new_tokens)
    }

    fn decoder_io_names(&self) -> DecoderIoNames {
        let dec_input_names: Vec<String> = self.decoder.inputs().iter().map(|i| i.name().to_string()).collect();
        let dec_output_names: Vec<String> = self.decoder.outputs().iter().map(|o| o.name().to_string()).collect();
        let dwp_input_names: Vec<String> = self
            .decoder_with_past
            .inputs()
            .iter()
            .map(|i| i.name().to_string())
            .collect();
        let dwp_output_names: Vec<String> = self
            .decoder_with_past
            .outputs()
            .iter()
            .map(|o| o.name().to_string())
            .collect();
        tracing::debug!(?dec_input_names, ?dec_output_names, "Decoder I/O names");
        tracing::debug!(?dwp_input_names, ?dwp_output_names, "Decoder-with-past I/O names");
        DecoderIoNames {
            encoder_hidden_states_input: dec_input_names
                .iter()
                .find(|n| n.contains("encoder_hidden_states"))
                .cloned()
                .unwrap_or_else(|| "encoder_hidden_states".to_string()),
            logits_output: dec_output_names
                .iter()
                .find(|n| *n == "logits")
                .cloned()
                .unwrap_or_else(|| "logits".to_string()),
            with_past_logits_output: dwp_output_names
                .iter()
                .find(|n| *n == "logits")
                .cloned()
                .unwrap_or_else(|| "logits".to_string()),
            with_past_wants_encoder_states: dwp_input_names.iter().any(|n| n.contains("encoder_hidden_states")),
        }
    }

    fn run_decoder_prompt(
        &self,
        prompt: Vec<i64>,
        encoder_hidden_states: &Value,
        names: &DecoderIoNames,
    ) -> Result<ort::session::SessionOutputs<'_>, TranscriptionError> {
        let input_ids =
            Array2::from_shape_vec((1, prompt.len()), prompt).map_err(|e| TranscriptionError::Shape(e.to_string()))?;
        let ids_value: Value = Value::from_array(input_ids)?.into();
        let encoder_states = clone_value_f32(encoder_hidden_states)?;
        let inputs = ort::inputs![
            "input_ids" => ids_value,
            &names.encoder_hidden_states_input => encoder_states,
        ];
        #[allow(unsafe_code)]
        let outputs = unsafe {
            // SAFETY: The module's shared-session contract relies on ORT run delegating to run_inner(&self). ~keep
            // Outputs stay tied to &self, so the session outlives their borrow. ~keep
            let ptr = &self.decoder as *const Session as *mut Session;
            (*ptr).run(inputs)
        }?;
        Ok(outputs)
    }

    fn decode_with_past(
        &self,
        first_token: u32,
        encoder_hidden_states: &Value,
        names: &DecoderIoNames,
        mut cache: WhisperDecoderCache,
        max_new_tokens: usize,
    ) -> Result<Vec<u32>, TranscriptionError> {
        let mut generated = vec![first_token];
        for _ in 1..max_new_tokens {
            let last_token = *generated.last().expect("generated is non-empty; qed");
            let outputs = self.run_decoder_with_past(last_token, encoder_hidden_states, names, &cache)?;
            let next_token = decoder_output_token(&outputs, &names.with_past_logits_output)?;
            if next_token == self.special_tokens.end_of_text {
                break;
            }
            generated.push(next_token);
            let new_decoder_kvs = decoder_cache_outputs(outputs, &names.with_past_logits_output);
            if !new_decoder_kvs.is_empty() {
                cache.decoder_kvs = new_decoder_kvs;
            }
        }
        Ok(generated)
    }

    fn run_decoder_with_past(
        &self,
        last_token: u32,
        encoder_hidden_states: &Value,
        names: &DecoderIoNames,
        cache: &WhisperDecoderCache,
    ) -> Result<ort::session::SessionOutputs<'_>, TranscriptionError> {
        let last_id_arr = Array2::from_shape_vec((1, 1), vec![last_token as i64])
            .map_err(|e| TranscriptionError::Shape(e.to_string()))?;
        let ids_value: Value = Value::from_array(last_id_arr)?.into();
        let mut inputs = ort::inputs!["input_ids" => ids_value];
        if names.with_past_wants_encoder_states {
            let encoder_states = clone_value_f32(encoder_hidden_states)?;
            inputs.push((names.encoder_hidden_states_input.as_str().into(), encoder_states.into()));
        }
        for (kv_name, kv_val) in &cache.decoder_kvs {
            let kv_clone = clone_value_f32(kv_val)?;
            inputs.push((kv_name.as_str().into(), kv_clone.into()));
        }
        for (kv_name, kv_val) in &cache.encoder_kvs {
            let kv_clone = clone_value_f32(kv_val)?;
            inputs.push((kv_name.as_str().into(), kv_clone.into()));
        }
        #[allow(unsafe_code)]
        let outputs = unsafe {
            // SAFETY: The module's shared-session contract relies on ORT run delegating to run_inner(&self). ~keep
            // Outputs stay tied to &self, so the session outlives their borrow. ~keep
            let ptr = &self.decoder_with_past as *const Session as *mut Session;
            (*ptr).run(inputs)
        }?;
        Ok(outputs)
    }
}

struct DecoderIoNames {
    encoder_hidden_states_input: String,
    logits_output: String,
    with_past_logits_output: String,
    with_past_wants_encoder_states: bool,
}

struct WhisperDecoderCache {
    encoder_kvs: Vec<(String, Value)>,
    decoder_kvs: Vec<(String, Value)>,
}

impl WhisperDecoderCache {
    fn from_prompt(outputs: ort::session::SessionOutputs<'_>, logits_name: &str) -> Self {
        let mut encoder_kvs = Vec::new();
        let mut decoder_kvs = Vec::new();
        for (name, value) in decoder_cache_outputs(outputs, logits_name) {
            if name.contains(".encoder.") {
                encoder_kvs.push((name, value));
            } else {
                decoder_kvs.push((name, value));
            }
        }
        Self {
            encoder_kvs,
            decoder_kvs,
        }
    }
}

fn decoder_output_token(
    outputs: &ort::session::SessionOutputs<'_>,
    logits_name: &str,
) -> Result<u32, TranscriptionError> {
    let logits = outputs
        .iter()
        .find(|(name, _)| *name == logits_name)
        .map(|(_, value)| value)
        .ok_or(TranscriptionError::NoOutput)?;
    greedy_argmax_last(&logits)
}

fn decoder_cache_outputs(outputs: ort::session::SessionOutputs<'_>, logits_name: &str) -> Vec<(String, Value)> {
    outputs
        .into_iter()
        .filter(|(name, _)| *name != logits_name)
        .map(|(name, value)| (name.replacen("present", "past_key_values", 1), value))
        .collect()
}

/// Walk the recording in 30-second windows, calling `transcribe_chunk` for
/// each, and isolate per-window failures.
///
/// Returns `(segments, warnings, first_error, any_success)`. A failing window
/// contributes a warning and is skipped; its error is retained only so the
/// caller can fail the whole call when *every* window failed (a systemic
/// failure) rather than returning an empty transcript. Extracted from
/// [`WhisperEngine::transcribe_segments_with_warnings`] so the isolation
/// behaviour is unit-testable with an injected failing transcriber.
fn transcribe_chunks_with_isolation<F>(
    samples: &[f32],
    ms_per_sample: f64,
    mut transcribe_chunk: F,
) -> ChunkIsolationOutcome
where
    F: FnMut(&[f32], u32) -> Result<Vec<Segment>, TranscriptionError>,
{
    let mut segments: Vec<Segment> = Vec::new();
    let mut warnings: Vec<TranscriptionWindowWarning> = Vec::new();
    let mut first_error: Option<TranscriptionError> = None;
    let mut any_success = false;
    let mut offset = 0_usize;

    while offset < samples.len() {
        let chunk_end = (offset + WHISPER_CHUNK_SAMPLES).min(samples.len());
        let chunk = &samples[offset..chunk_end];
        let chunk_offset_ms = (offset as f64 * ms_per_sample) as u32;
        let chunk_duration_ms = ((chunk_end - offset) as f64 * ms_per_sample) as u32;

        match transcribe_chunk(chunk, chunk_duration_ms) {
            Ok(chunk_segments) => {
                any_success = true;
                for (start_ms, end_ms, text) in chunk_segments {
                    if text.is_empty() {
                        continue;
                    }
                    segments.push((chunk_offset_ms + start_ms, chunk_offset_ms + end_ms, text));
                }
            }
            Err(error) => {
                tracing::warn!(
                    window_start_ms = chunk_offset_ms,
                    window_end_ms = chunk_offset_ms + chunk_duration_ms,
                    error = %error,
                    "Whisper could not decode a 30-second window; skipping it and continuing",
                );
                warnings.push(TranscriptionWindowWarning {
                    start_ms: chunk_offset_ms,
                    end_ms: chunk_offset_ms + chunk_duration_ms,
                    message: error.to_string(),
                });
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }

        offset += WHISPER_CHUNK_SAMPLES;
    }

    (segments, warnings, first_error, any_success)
}

/// Extract the token with the highest logit from the last position of a
/// `[batch=1, seq_len, vocab_size]` logits tensor.
///
/// Whisper vocab is ~51865 tokens. A plain argmax over `Vec<f32>` is fast
/// enough; no softmax is required for greedy decoding.
fn greedy_argmax_last(logits: &Value) -> Result<u32, TranscriptionError> {
    let tensor = logits.try_extract_array::<f32>().map_err(TranscriptionError::Ort)?;
    let shape = tensor.shape();
    if shape.len() < 3 {
        return Err(TranscriptionError::Shape(format!(
            "Expected logits tensor rank 3, got rank {}",
            shape.len()
        )));
    }
    let seq_len = shape[shape.len() - 2];
    let vocab_size = shape[shape.len() - 1];
    let last_pos_offset = (seq_len - 1) * vocab_size;

    let flat: Vec<f32> = tensor.iter().cloned().collect();
    if flat.len() < last_pos_offset + vocab_size {
        return Err(TranscriptionError::Shape(format!(
            "Logits flat length {} too short for offset {} + vocab {}",
            flat.len(),
            last_pos_offset,
            vocab_size
        )));
    }
    let last_logits = &flat[last_pos_offset..last_pos_offset + vocab_size];

    let best = last_logits
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(idx, _)| idx as u32)
        .ok_or_else(|| TranscriptionError::Shape("logits slice was empty".to_string()))?;

    Ok(best)
}

/// Clone an `ort::value::Value` by extracting its f32 data and rebuilding a
/// new tensor with the same shape.
///
/// This is necessary because `ort::value::Value` is not `Clone` and ORT
/// session inputs consume values by move.
fn clone_value_f32(value: &Value) -> Result<Value, TranscriptionError> {
    let arr = value.try_extract_array::<f32>().map_err(TranscriptionError::Ort)?;
    let shape: Vec<usize> = arr.shape().to_vec();
    let flat: Vec<f32> = arr.iter().cloned().collect();
    let owned = ndarray::ArrayD::from_shape_vec(ndarray::IxDyn(&shape), flat)
        .map_err(|e| TranscriptionError::Shape(e.to_string()))?;
    let result: Value = Value::from_array(owned)?.into();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greedy_argmax_last_picks_highest() {
        let data = Array3::from_shape_vec((1, 2, 4), vec![0.1_f32, 0.9, 0.2, 0.0, 0.1, 0.2, 0.3, 0.8]).unwrap();
        let val: Value = Value::from_array(data).unwrap().into();
        let tok = greedy_argmax_last(&val).unwrap();
        assert_eq!(tok, 3, "expected argmax at index 3 (last position)");
    }

    #[test]
    fn greedy_argmax_last_single_position() {
        let data = Array3::from_shape_vec((1, 1, 5), vec![0.0_f32, 0.0, 100.0, 0.0, 0.0]).unwrap();
        let val: Value = Value::from_array(data).unwrap().into();
        let tok = greedy_argmax_last(&val).unwrap();
        assert_eq!(tok, 2);
    }

    #[test]
    fn greedy_argmax_last_preserves_last_tie_and_rank_errors() {
        let tied = Array3::from_shape_vec((1, 1, 4), vec![0.0_f32, 2.0, 2.0, 1.0]).unwrap();
        let logits: Value = Value::from_array(tied).unwrap().into();
        assert_eq!(greedy_argmax_last(&logits).unwrap(), 2);

        let rank_two = Array2::from_shape_vec((1, 4), vec![0.0_f32; 4]).unwrap();
        let logits: Value = Value::from_array(rank_two).unwrap().into();
        let error = greedy_argmax_last(&logits).unwrap_err();
        assert_eq!(
            error.to_string(),
            "tensor shape error: Expected logits tensor rank 3, got rank 2"
        );
    }

    #[test]
    fn cloned_decoder_state_preserves_tensor_shape_and_values() {
        let values = vec![1.0_f32, -2.0, 3.0, 4.0];
        let tensor = Array3::from_shape_vec((1, 2, 2), values.clone()).unwrap();
        let original: Value = Value::from_array(tensor).unwrap().into();
        let cloned = clone_value_f32(&original).unwrap();

        let observed = cloned.try_extract_array::<f32>().unwrap();
        assert_eq!(observed.shape(), &[1, 2, 2]);
        assert_eq!(observed.iter().copied().collect::<Vec<_>>(), values);
        assert_eq!(original.try_extract_array::<f32>().unwrap(), observed);
    }

    #[test]
    fn special_tokens_resolve_from_tiny_vocab() {
        use crate::core::config::transcription::WhisperModel;
        use crate::transcription::model::ensure_whisper_model;

        let paths = match ensure_whisper_model(WhisperModel::Tiny, None, false, false) {
            Ok(p) => p,
            Err(_) => return,
        };

        let tokenizer = Tokenizer::from_file(&paths.tokenizer).expect("tokenizer load");
        let st = SpecialTokens::resolve(&tokenizer).expect("special token resolution");

        assert!(st.end_of_text > 0, "end_of_text token should be a valid non-zero ID");
        assert!(
            st.language_ids.contains_key("en"),
            "English language token must be present"
        );
    }

    /// The decoder's position-embedding table has [`WHISPER_MAX_TOKENS`] rows,
    /// so the prompt must be subtracted from the budget. Regression for
    /// GH#1944: an unsubtracted budget let a repetition loop feed a token at
    /// position 448, slicing an empty position range and crashing Reshape.
    #[test]
    fn decoder_token_budget_reserves_prompt_positions() {
        assert_eq!(decoder_token_budget(4), WHISPER_MAX_TOKENS - 4);
        assert_eq!(decoder_token_budget(WHISPER_MAX_TOKENS), 0);
        assert!(decoder_token_budget(WHISPER_MAX_TOKENS + 1) == 0, "never underflows");
        assert!(4 + decoder_token_budget(4) <= WHISPER_MAX_TOKENS);
    }

    /// One undecodable window must not discard the others (GH#1944).
    #[test]
    fn a_failing_window_is_skipped_and_the_rest_are_kept() {
        let samples = vec![0.0_f32; WHISPER_CHUNK_SAMPLES * 2 + 1];
        let ms_per_sample = 1000.0 / 16_000.0;
        let mut calls = 0usize;

        let (segments, warnings, first_error, any_success) =
            transcribe_chunks_with_isolation(&samples, ms_per_sample, |_chunk, _duration| {
                let index = calls;
                calls += 1;
                if index == 1 {
                    Err(TranscriptionError::Shape("decoder rejected window 1".into()))
                } else {
                    Ok(vec![(0, 30_000, format!("chunk-{index}"))])
                }
            });

        assert!(any_success, "chunks 0 and 2 succeeded");
        assert_eq!(calls, 3, "expected three 30-second windows");
        assert_eq!(warnings.len(), 1, "exactly the middle window must warn");
        assert_eq!(warnings[0].start_ms, 30_000);
        assert_eq!(warnings[0].end_ms, 60_000);
        assert!(warnings[0].message.contains("window 1"));
        assert!(first_error.is_some());
        let texts: Vec<&str> = segments.iter().map(|(_, _, text)| text.as_str()).collect();
        assert_eq!(texts, vec!["chunk-0", "chunk-2"]);
        assert_eq!(segments[1].0, 60_000, "third chunk keeps its absolute offset");
    }

    /// A systemic decoder failure must still surface as an error rather than an
    /// empty transcript.
    #[test]
    fn every_window_failing_reports_the_first_error() {
        let samples = vec![0.0_f32; WHISPER_CHUNK_SAMPLES + 1];
        let ms_per_sample = 1000.0 / 16_000.0;

        let (segments, warnings, first_error, any_success) =
            transcribe_chunks_with_isolation(&samples, ms_per_sample, |_chunk, _duration| {
                Err(TranscriptionError::Shape("boom".into()))
            });

        assert!(!any_success, "no window succeeded");
        assert!(segments.is_empty(), "no partial text to return");
        assert_eq!(warnings.len(), 2, "both windows must warn");
        assert!(first_error.is_some(), "the caller must be able to fail the whole call");
    }
}
