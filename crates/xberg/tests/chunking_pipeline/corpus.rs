//! Deterministic document shapes and real-document loading for the chunking pipeline tests.
//!
//! Generators are seeded and offline; nothing here is committed as a large file. Real documents
//! come from the shared `test_documents/` submodule and are skipped when it is absent.

use xberg::types::PageBoundary;

/// SplitMix64: tiny, seedable, and stable across platforms and Rust versions.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }

    pub fn range(&mut self, low: usize, high: usize) -> usize {
        low + self.below(high - low + 1)
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }

    pub fn pick_str(&mut self, items: &[&'static str]) -> &'static str {
        items[self.below(items.len())]
    }
}

pub const WORDS: &[&str] = &[
    "the",
    "quick",
    "brown",
    "fox",
    "jumps",
    "over",
    "lazy",
    "dog",
    "pipeline",
    "chunk",
    "boundary",
    "semantic",
    "section",
    "extraction",
    "document",
    "markdown",
    "table",
    "heading",
    "capacity",
    "overlap",
    "token",
    "index",
    "stream",
    "buffer",
    "parser",
    "grammar",
    "unicode",
    "segment",
    "offset",
    "cursor",
];

pub fn words(rng: &mut Rng, count: usize) -> String {
    let mut out = String::new();
    for i in 0..count {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(rng.pick_str(WORDS));
    }
    out
}

fn sentence(rng: &mut Rng) -> String {
    let count = rng.range(5, 18);
    let mut text = words(rng, count);
    text[..1].make_ascii_uppercase();
    text.push_str(rng.pick_str(&[". ", ". ", ". ", "! ", "? "]));
    text
}

fn paragraph(rng: &mut Rng) -> String {
    let count = rng.range(2, 7);
    let mut out = String::new();
    for _ in 0..count {
        out.push_str(&sentence(rng));
    }
    out.truncate(out.trim_end().len());
    out
}

fn fill(size: usize, mut next: impl FnMut() -> String) -> String {
    let mut out = String::with_capacity(size + 256);
    while out.len() < size {
        out.push_str(&next());
    }
    out
}

pub fn flat_prose(size: usize, seed: u64) -> String {
    let mut rng = Rng::new(seed);
    fill(size, || format!("{}\n\n", paragraph(&mut rng)))
}

pub fn long_table(size: usize, seed: u64) -> String {
    let mut rng = Rng::new(seed);
    let mut out = String::from("| id | name | score | note |\n|---|---|---|---|\n");
    let mut id = 0;
    while out.len() < size {
        id += 1;
        let name = words(&mut rng, 2);
        let note = words(&mut rng, 3);
        out.push_str(&format!("| {id} | {name} | {} | {note} |\n", rng.below(1000)));
    }
    out
}

pub fn nested_lists(size: usize, seed: u64) -> String {
    let mut rng = Rng::new(seed);
    fill(size, || {
        let depth = rng.below(4);
        let marker = rng.pick_str(&["-", "*", "1."]);
        let item = words(&mut rng, 6);
        format!("{}{marker} {item}\n", "  ".repeat(depth))
    })
}

pub fn heading_dense(size: usize, seed: u64) -> String {
    let mut rng = Rng::new(seed);
    fill(size, || {
        let level = rng.range(1, 6);
        let title = words(&mut rng, 3);
        let body = if rng.below(3) == 0 {
            String::new()
        } else {
            sentence(&mut rng)
        };
        format!("{} {title}\n\n{body}\n\n", "#".repeat(level))
    })
}

pub fn code_fences(size: usize, seed: u64) -> String {
    let mut rng = Rng::new(seed);
    fill(size, || {
        let prose = paragraph(&mut rng);
        let lines = rng.range(3, 25);
        let mut code = String::new();
        for n in 0..lines {
            code.push_str(&format!(
                "    let value_{n} = compute({}, {});\n",
                rng.below(100),
                rng.below(100)
            ));
        }
        format!("{prose}\n\n```rust\nfn demo() {{\n{code}}}\n```\n\n")
    })
}

pub fn unicode_mix(size: usize, seed: u64) -> String {
    let mut rng = Rng::new(seed);
    const PIECES: &[&str] = &[
        "日本語のテキストを分割します。",
        "中文文本需要被正确切分，不能在字符中间断开。",
        "한국어 문장도 포함됩니다. ",
        "emoji 🎉🚀👨\u{200d}👩\u{200d}👧\u{200d}👦 family ",
        "combining e\u{301}a\u{300}o\u{302} marks ",
        "العربية نص ",
        "Ünïcödé wörds ",
        "\n\n",
    ];
    fill(size, || {
        let mut out = String::new();
        for _ in 0..rng.range(2, 6) {
            out.push_str(rng.pick_str(PIECES));
        }
        out
    })
}

pub fn crlf_prose(size: usize, seed: u64) -> String {
    flat_prose(size, seed).replace('\n', "\r\n")
}

pub fn one_long_line(size: usize, seed: u64) -> String {
    let mut rng = Rng::new(seed);
    fill(size, || format!("{} ", words(&mut rng, 8)))
}

pub fn blank_line_flood(size: usize, seed: u64) -> String {
    let mut rng = Rng::new(seed);
    fill(size, || {
        let blanks = "\n".repeat(rng.range(2, 40));
        format!("{}{blanks}", words(&mut rng, 4))
    })
}

pub fn yaml_like(size: usize, seed: u64) -> String {
    let mut rng = Rng::new(seed);
    let mut n = 0;
    fill(size, || {
        n += 1;
        let a = words(&mut rng, 2);
        let b = words(&mut rng, 3);
        format!(
            "section_{n}:\n  name: {a}\n  items:\n    - {b}\n    - {}\n",
            rng.below(500)
        )
    })
}

pub fn csv_like(size: usize, seed: u64) -> String {
    let mut rng = Rng::new(seed);
    let mut out = String::from("id,name,city,amount\n");
    let mut id = 0;
    while out.len() < size {
        id += 1;
        out.push_str(&format!(
            "{id},{},{},{}.{:02}\n",
            words(&mut rng, 2),
            rng.pick(WORDS),
            rng.below(10_000),
            rng.below(100)
        ));
    }
    out
}

pub fn json_like(size: usize, seed: u64) -> String {
    let mut rng = Rng::new(seed);
    let mut out = String::from("[\n");
    while out.len() < size {
        out.push_str(&format!(
            "  {{\"id\": {}, \"label\": \"{}\", \"tags\": [\"{}\", \"{}\"]}},\n",
            rng.below(100_000),
            words(&mut rng, 2),
            rng.pick(WORDS),
            rng.pick(WORDS)
        ));
    }
    out.push_str("  {}\n]\n");
    out
}

pub fn mixed_markdown(size: usize, seed: u64) -> String {
    let mut rng = Rng::new(seed);
    fill(size, || match rng.below(5) {
        0 => heading_dense(300, rng.next_u64()),
        1 => flat_prose(600, rng.next_u64()),
        2 => long_table(400, rng.next_u64()),
        3 => code_fences(500, rng.next_u64()),
        _ => nested_lists(300, rng.next_u64()),
    })
}

pub type Generator = fn(usize, u64) -> String;

/// Every scaling shape, in a stable order. Names appear in golden digests.
pub const SHAPES: &[(&str, Generator)] = &[
    ("flat_prose", flat_prose),
    ("long_table", long_table),
    ("nested_lists", nested_lists),
    ("heading_dense", heading_dense),
    ("code_fences", code_fences),
    ("unicode_mix", unicode_mix),
    ("crlf_prose", crlf_prose),
    ("one_long_line", one_long_line),
    ("blank_line_flood", blank_line_flood),
    ("yaml_like", yaml_like),
    ("csv_like", csv_like),
    ("json_like", json_like),
    ("mixed_markdown", mixed_markdown),
    ("adversarial_mix", fuzz_document),
];

/// Contiguous page boundaries of roughly `page_bytes` bytes each, aligned to char boundaries.
pub fn pages(text: &str, page_bytes: usize) -> Vec<PageBoundary> {
    let mut boundaries = Vec::new();
    let mut start = 0;
    let mut page = 1;
    while start < text.len() {
        let mut end = (start + page_bytes).min(text.len());
        while !text.is_char_boundary(end) {
            end += 1;
        }
        boundaries.push(PageBoundary {
            byte_start: start,
            byte_end: end,
            page_number: page,
        });
        start = end;
        page += 1;
    }
    boundaries
}

/// Adversarial prose assembled from pieces that stress sentence, word and grapheme segmentation:
/// unterminated runs, long single words, CJK, emoji, mixed and trailing terminators, decimals,
/// abbreviations, digit-and-period runs, whitespace runs and CRLF.
pub fn fuzz_document(size: usize, seed: u64) -> String {
    let mut rng = Rng::new(seed);
    let mode = rng.below(6);
    fill(size, || match rng.below(16) {
        0 => {
            let count = rng_len(&mut rng, 1, 400);
            format!("{} ", words(&mut rng, count))
        }
        1 => format!("{} ", "x".repeat(rng_len(&mut rng, 1, 3_000))),
        2 => "日本語のテキスト、中文文本没有标点".repeat(rng_len(&mut rng, 1, 80)),
        3 => "🎉🚀👨\u{200d}👩\u{200d}👧 ".repeat(rng_len(&mut rng, 1, 60)),
        4 => format!(
            "{}{} ",
            words(&mut rng, 4),
            rng.pick_str(&[".", "!", "?", "。", "？", "...", "…", ".\"", "!)", "?!", ". . ."])
        ),
        5 => format!("{}. ", rng.below(100_000)),
        6 => format!("3.14 e.g. i.e. Dr. Smith v1.2.3 {} ", rng.pick_str(WORDS)),
        7 => " ".repeat(rng_len(&mut rng, 1, 500)),
        8 => format!("1. 2. 3 4 5 6 7 8 9 {}. ", rng.below(1000)),
        9 if mode < 2 => "\r\n".repeat(rng_len(&mut rng, 1, 4)),
        9 => "\n".repeat(rng_len(&mut rng, 1, 3)),
        10 => "e\u{301}a\u{300} ".repeat(rng_len(&mut rng, 1, 100)),
        11 => format!(
            "https://example.com/{}?q={} ",
            words(&mut rng, 3).replace(' ', "/"),
            rng.below(1000)
        ),
        12 => format!("{}\t{}\t{} ", rng.pick_str(WORDS), rng.below(99), rng.pick_str(WORDS)),
        13 => "؟ ، ۔ ".repeat(rng_len(&mut rng, 1, 20)),
        _ => {
            let count = rng_len(&mut rng, 1, 30);
            format!("{} ", words(&mut rng, count))
        }
    })
}

fn rng_len(rng: &mut Rng, low: usize, high: usize) -> usize {
    rng.range(low, high)
}
