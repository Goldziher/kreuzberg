//! Stable content digest of a chunking result, used to compare output across implementations.

use xberg::chunking::{ChunkerType, ChunkingConfig, ChunkingResult, chunk_text};
use xberg::types::PageBoundary;

struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn bytes(&mut self, data: &[u8]) {
        for &b in data {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
        self.0 ^= 0xff;
        self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
    }

    fn num(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }
}

/// Hashes everything observable about each chunk: text, offsets, page range and spans, heading
/// context, classification and indices.
pub fn digest(result: &ChunkingResult) -> u64 {
    let mut hash = Fnv::new();
    hash.num(result.chunk_count as u64);
    for chunk in &result.chunks {
        let meta = &chunk.metadata;
        hash.bytes(chunk.content.as_bytes());
        hash.bytes(format!("{:?}", chunk.chunk_type).as_bytes());
        hash.num(meta.byte_start as u64);
        hash.num(meta.byte_end as u64);
        hash.num(meta.chunk_index as u64);
        hash.num(meta.total_chunks as u64);
        hash.num(u64::from(meta.first_page.unwrap_or(0)));
        hash.num(u64::from(meta.last_page.unwrap_or(0)));
        for span in &meta.page_spans {
            hash.num(u64::from(span.page));
        }
        hash.num(0xfeed);
        match &meta.heading_context {
            None => hash.num(0),
            Some(context) => {
                hash.num(context.headings.len() as u64 + 1);
                for heading in &context.headings {
                    hash.num(u64::from(heading.level));
                    hash.bytes(heading.text.as_bytes());
                }
            }
        }
        for part in &meta.heading_path {
            hash.bytes(part.as_bytes());
        }
    }
    hash.0
}

pub const CHUNKERS: &[(&str, ChunkerType)] = &[
    ("markdown", ChunkerType::Markdown),
    ("text", ChunkerType::Text),
    ("semantic", ChunkerType::Semantic),
];

/// [`CHUNKERS`] plus the YAML/JSON section chunker, which falls back to text splitting otherwise.
pub const ALL_CHUNKERS: &[(&str, ChunkerType)] = &[
    ("markdown", ChunkerType::Markdown),
    ("text", ChunkerType::Text),
    ("semantic", ChunkerType::Semantic),
    ("yaml", ChunkerType::Yaml),
];

/// `(max_characters, overlap)` pairs exercised by every corpus-wide test.
pub const CAPACITIES: &[(usize, usize)] = &[(120, 0), (500, 100), (1_000, 200), (4_000, 0)];

pub fn config(chunker_type: ChunkerType, max_characters: usize, overlap: usize) -> ChunkingConfig {
    ChunkingConfig {
        max_characters,
        overlap,
        trim: true,
        chunker_type,
        ..Default::default()
    }
}

pub fn run(text: &str, config: &ChunkingConfig, pages: Option<&[PageBoundary]>) -> ChunkingResult {
    chunk_text(text, config, pages).expect("chunking succeeds")
}
