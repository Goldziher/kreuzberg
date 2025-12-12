require 'kreuzberg'

result = Kreuzberg.extract_file_sync("document.pdf")

if result.metadata.pages&.boundaries
  content_bytes = result.content.bytes

  result.metadata.pages.boundaries.take(3).each do |boundary|
    page_bytes = content_bytes[boundary.byte_start...boundary.byte_end]
    page_text = page_bytes.pack('C*').force_encoding('UTF-8')

    puts "Page #{boundary.page_number}:"
    puts "  Byte range: #{boundary.byte_start}-#{boundary.byte_end}"
    puts "  Preview: #{page_text[0..100]}..."
  end
end
