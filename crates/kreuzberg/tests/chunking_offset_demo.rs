#[cfg(feature = "chunking")]
#[test]
fn demonstrate_correct_offset_calculation() {
    use kreuzberg::chunking::{ChunkerType, ChunkingConfig, chunk_text};

    println!("\n=== Demonstrating Correct Chunking Offset Calculation ===\n");

    let config_with_overlap = ChunkingConfig {
        max_characters: 20,
        overlap: 5,
        trim: false,
        chunker_type: ChunkerType::Text,
    };

    let text = "AAAAA BBBBB CCCCC DDDDD EEEEE FFFFF";
    println!("Text: \"{}\"", text);
    println!(
        "Max characters: {}, Overlap: {}\n",
        config_with_overlap.max_characters, config_with_overlap.overlap
    );

    let result = chunk_text(text, &config_with_overlap).unwrap();

    println!("WITH OVERLAP (5 chars):");
    for (i, chunk) in result.chunks.iter().enumerate() {
        println!(
            "  Chunk {}: [{:3} - {:3}] = \"{}\"",
            i,
            chunk.metadata.char_start,
            chunk.metadata.char_end,
            chunk.content.replace('\n', "\\n")
        );
    }

    println!("\nOverlap verification:");
    for i in 0..result.chunks.len() - 1 {
        let current = &result.chunks[i];
        let next = &result.chunks[i + 1];
        let overlap_size = current.metadata.char_end - next.metadata.char_start;
        println!(
            "  Chunks {} and {}: overlap = {} chars (next starts at {} while current ends at {})",
            i,
            i + 1,
            overlap_size,
            next.metadata.char_start,
            current.metadata.char_end
        );
        assert!(
            overlap_size > 0 && overlap_size <= config_with_overlap.overlap + 10,
            "Overlap should exist and be reasonable"
        );
    }

    println!("\n\n=== Without Overlap ===\n");
    let config_no_overlap = ChunkingConfig {
        max_characters: 20,
        overlap: 0,
        trim: false,
        chunker_type: ChunkerType::Text,
    };

    let result_no_overlap = chunk_text(text, &config_no_overlap).unwrap();

    println!("WITHOUT OVERLAP:");
    for (i, chunk) in result_no_overlap.chunks.iter().enumerate() {
        println!(
            "  Chunk {}: [{:3} - {:3}] = \"{}\"",
            i,
            chunk.metadata.char_start,
            chunk.metadata.char_end,
            chunk.content.replace('\n', "\\n")
        );
    }

    println!("\nAdjacency verification:");
    for i in 0..result_no_overlap.chunks.len() - 1 {
        let current = &result_no_overlap.chunks[i];
        let next = &result_no_overlap.chunks[i + 1];
        let gap = next.metadata.char_start as i32 - current.metadata.char_end as i32;
        println!(
            "  Chunks {} and {}: gap = {} (next starts at {}, current ends at {})",
            i,
            i + 1,
            gap,
            next.metadata.char_start,
            current.metadata.char_end
        );
        assert!(gap >= 0, "Should have no overlap (gap >= 0)");
    }

    println!("\n✓ All offset calculations are correct!");
}
