require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  chunking: Kreuzberg::ChunkingConfig.new(chunk_size: 500, overlap: 50),
  pages: Kreuzberg::PageConfig.new(extract_pages: true)
)

result = Kreuzberg.extract_file_sync("document.pdf", config: config)

result.chunks&.each do |chunk|
  if chunk.metadata.first_page
    page_range = if chunk.metadata.first_page == chunk.metadata.last_page
      "Page #{chunk.metadata.first_page}"
    else
      "Pages #{chunk.metadata.first_page}-#{chunk.metadata.last_page}"
    end

    puts "Chunk: #{chunk.text[0..50]}... (#{page_range})"
  end
end
